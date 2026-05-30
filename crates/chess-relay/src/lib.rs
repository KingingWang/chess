//! `chess-relay` library: an end-to-end-encrypted relay that pairs two clients
//! by room number and forwards opaque binary frames between them over
//! WebSocket-over-TLS. The server never learns the room password and never sees
//! any game data (everything is encrypted end-to-end by `chess-net`).
//!
//! The binary entry point is [`run`]; [`serve`] is exposed for tests that wire
//! up their own listener and TLS acceptor.

pub mod config;
mod room;
mod tls;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use chess_net::ControlMsg;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{info, warn};

pub use config::Config;
pub use room::Rooms;
pub use tls::build_acceptor;

/// The server-side WebSocket stream type (server end of a TLS tunnel).
pub type WsServer = tokio_tungstenite::WebSocketStream<tokio_rustls::server::TlsStream<TcpStream>>;

/// How long a created room waits for a guest before timing out.
pub const ROOM_TIMEOUT: Duration = Duration::from_secs(600);

/// Parse args, load config, build TLS, bind, and serve forever.
pub async fn run() -> anyhow::Result<()> {
    let config_path = parse_config_arg()?;
    let cfg = Config::load(config_path.as_deref())?;
    let acceptor =
        build_acceptor(&cfg.cert, &cfg.key).context("building TLS acceptor (check cert/key paths)")?;

    let addr = cfg.listen_addr();
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    info!(%addr, cert = %cfg.cert.display(), "chess-relay listening (WSS)");

    serve(listener, acceptor, Rooms::default()).await
}

/// Accept connections forever, spawning a handler per connection.
pub async fn serve(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    rooms: Rooms,
) -> anyhow::Result<()> {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "accept failed");
                continue;
            }
        };
        let acceptor = acceptor.clone();
        let rooms = rooms.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(stream, acceptor, rooms).await {
                warn!(%peer, error = %e, "connection ended with error");
            }
        });
    }
}

/// Parse an optional `--config <path>` / `-c <path>` argument.
pub fn parse_config_arg() -> anyhow::Result<Option<PathBuf>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                let path = args
                    .get(i + 1)
                    .context("--config requires a file path argument")?;
                config = Some(PathBuf::from(path));
                i += 2;
            }
            other if other.starts_with("--config=") => {
                config = Some(PathBuf::from(&other["--config=".len()..]));
                i += 1;
            }
            "-h" | "--help" => {
                eprintln!("Usage: chess-relay [--config <path>]");
                std::process::exit(0);
            }
            other => anyhow::bail!("unexpected argument: {other}"),
        }
    }
    Ok(config)
}

/// Handle one client connection: TLS, WebSocket, then the control protocol.
async fn handle(stream: TcpStream, acceptor: TlsAcceptor, rooms: Rooms) -> anyhow::Result<()> {
    stream.set_nodelay(true).ok();
    let tls = acceptor.accept(stream).await.context("TLS handshake")?;
    let mut ws = accept_async(tls).await.context("WebSocket handshake")?;

    match recv_control(&mut ws).await? {
        ControlMsg::Create { salt } => handle_create(ws, salt, rooms).await,
        ControlMsg::Join { room } => handle_join(ws, room, rooms).await,
        other => {
            let _ = send_control(
                &mut ws,
                &ControlMsg::Error {
                    msg: format!("unexpected first control message: {other:?}"),
                },
            )
            .await;
            Ok(())
        }
    }
}

/// Host side: register a room, wait for a guest, then relay.
async fn handle_create(mut ws: WsServer, salt: String, rooms: Rooms) -> anyhow::Result<()> {
    let (room, rx) = rooms.create(salt);
    info!(%room, active = rooms.active(), "room created");
    send_control(&mut ws, &ControlMsg::Created { room: room.clone() }).await?;

    match tokio::time::timeout(ROOM_TIMEOUT, rx).await {
        Ok(Ok(guest)) => {
            send_control(&mut ws, &ControlMsg::PeerJoined).await?;
            info!(%room, "peer joined; relaying");
            relay(ws, guest).await;
            info!(%room, "session ended");
        }
        Ok(Err(_)) => {
            // Sender dropped without sending (join handler failed). Clean up.
            rooms.remove(&room);
        }
        Err(_) => {
            rooms.remove(&room);
            let _ = send_control(
                &mut ws,
                &ControlMsg::Error {
                    msg: "timed out waiting for a guest".into(),
                },
            )
            .await;
            info!(%room, "room timed out");
        }
    }
    Ok(())
}

/// Guest side: claim a waiting room and hand our stream to the host task.
async fn handle_join(mut ws: WsServer, room: String, rooms: Rooms) -> anyhow::Result<()> {
    match rooms.take(&room) {
        None => {
            let _ = send_control(
                &mut ws,
                &ControlMsg::Error {
                    msg: format!("no such active room: {room}"),
                },
            )
            .await;
            Ok(())
        }
        Some((salt, tx)) => {
            // Tell the guest the host's salt before handing off the stream.
            send_control(&mut ws, &ControlMsg::Joined { salt }).await?;
            if let Err(mut ws) = tx.send(ws) {
                warn!(%room, "host left before guest could be paired");
                let _ = ws.close(None).await;
            }
            Ok(())
        }
    }
}

/// Pure byte relay between the two paired peers. The server forwards binary
/// frames verbatim and never inspects or decrypts their contents.
async fn relay(host: WsServer, guest: WsServer) {
    let (mut host_tx, mut host_rx) = host.split();
    let (mut guest_tx, mut guest_rx) = guest.split();

    let h2g = async {
        while let Some(msg) = host_rx.next().await {
            match msg {
                Ok(m) if m.is_binary() => {
                    if guest_tx.send(m).await.is_err() {
                        break;
                    }
                }
                Ok(m) if m.is_close() => break,
                Ok(_) => {} // ignore ping/pong/text after pairing
                Err(_) => break,
            }
        }
        let _ = guest_tx.close().await;
    };

    let g2h = async {
        while let Some(msg) = guest_rx.next().await {
            match msg {
                Ok(m) if m.is_binary() => {
                    if host_tx.send(m).await.is_err() {
                        break;
                    }
                }
                Ok(m) if m.is_close() => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = host_tx.close().await;
    };

    tokio::join!(h2g, g2h);
}

async fn send_control(ws: &mut WsServer, msg: &ControlMsg) -> anyhow::Result<()> {
    let txt = serde_json::to_string(msg)?;
    ws.send(WsMessage::Text(txt.into())).await?;
    Ok(())
}

async fn recv_control(ws: &mut WsServer) -> anyhow::Result<ControlMsg> {
    loop {
        match ws.next().await {
            Some(Ok(m)) if m.is_text() => {
                let txt = m.into_text()?;
                return Ok(serde_json::from_str(txt.as_str())?);
            }
            Some(Ok(m)) if m.is_close() => anyhow::bail!("closed before control message"),
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(e.into()),
            None => anyhow::bail!("closed before control message"),
        }
    }
}
