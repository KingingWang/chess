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
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{info, warn};

pub use config::Config;
pub use room::{JoinOutcome, Rooms};
pub use tls::build_acceptor;

/// The server-side WebSocket stream type (server end of a TLS tunnel).
pub type WsServer = tokio_tungstenite::WebSocketStream<tokio_rustls::server::TlsStream<TcpStream>>;

/// Read half of a [`WsServer`] (used while a guest session is being relayed).
type WsServerRx = SplitStream<WsServer>;
/// Write half of a [`WsServer`] (used while a guest session is being relayed).
type WsServerTx = SplitSink<WsServer, WsMessage>;

/// How long a created room waits for a guest before timing out.
pub const ROOM_TIMEOUT: Duration = Duration::from_secs(600);

/// Parse args, load config, build TLS, bind, and serve forever.
pub async fn run() -> anyhow::Result<()> {
    let config_path = parse_config_arg()?;
    let cfg = Config::load(config_path.as_deref())?;
    let acceptor = build_acceptor(&cfg.cert, &cfg.key)
        .context("building TLS acceptor (check cert/key paths)")?;

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

/// Host side: register a room, then loop accepting guests until the host
/// itself disconnects.
///
/// The room number is allocated up front and shared with the host so it can
/// display it to the user immediately. Each guest pairing runs a single byte
/// relay round; when the guest disconnects, the host is told via
/// `ControlMsg::PeerLeft` and the loop awaits the next guest. The room number
/// (and registry entry) survive across guest reconnects and are only released
/// when the host's WebSocket closes or the first-guest timeout fires.
async fn handle_create(ws: WsServer, salt: String, rooms: Rooms) -> anyhow::Result<()> {
    let (room, mut rx) = rooms.create(salt);
    info!(%room, active = rooms.active(), "room created");
    // Split the host WebSocket up front so the read and write halves can be
    // borrowed independently by the byte-relay tasks (otherwise the two halves
    // of the relay would both mutably borrow `ws` and deadlock the borrow
    // checker).
    let (mut host_tx, mut host_rx) = ws.split();
    send_control_tx(&mut host_tx, &ControlMsg::Created { room: room.clone() }).await?;

    // Only the *initial* wait has a timeout (preventing zombie rooms when the
    // host creates one and then walks away). Once at least one guest has
    // joined we keep the room alive for arbitrary reconnects, as long as the
    // host stays online.
    let mut first = true;
    loop {
        let next = if first {
            match tokio::time::timeout(ROOM_TIMEOUT, wait_next_guest(&mut rx, &mut host_rx)).await {
                Ok(v) => v,
                Err(_) => {
                    let _ = send_control_tx(
                        &mut host_tx,
                        &ControlMsg::Error {
                            msg: "timed out waiting for a guest".into(),
                        },
                    )
                    .await;
                    info!(%room, "room timed out");
                    break;
                }
            }
        } else {
            wait_next_guest(&mut rx, &mut host_rx).await
        };

        let guest = match next {
            Some(g) => g,
            None => break, // sender dropped — should not happen while room is registered
        };

        if let Err(e) = send_control_tx(&mut host_tx, &ControlMsg::PeerJoined).await {
            warn!(%room, error = %e, "failed to notify host of peer join; host gone");
            break;
        }
        info!(%room, "peer joined; relaying");
        let outcome = relay(&mut host_tx, &mut host_rx, guest).await;
        info!(%room, ?outcome, "guest session ended");

        match outcome {
            RelayOutcome::GuestLeft => {
                // Host stays online — tell it the guest is gone and loop back
                // to await the next reconnect.
                if let Err(e) = send_control_tx(&mut host_tx, &ControlMsg::PeerLeft).await {
                    warn!(%room, error = %e, "host gone while sending peer_left");
                    break;
                }
                first = false;
                continue;
            }
            RelayOutcome::HostLeft => break,
        }
    }

    rooms.release(&room);
    info!(%room, active = rooms.active(), "room released");
    Ok(())
}

/// Guest side: locate an active room and hand our stream to the host task.
async fn handle_join(mut ws: WsServer, room: String, rooms: Rooms) -> anyhow::Result<()> {
    match rooms.join(&room) {
        JoinOutcome::NotFound => {
            let _ = send_control(
                &mut ws,
                &ControlMsg::Error {
                    msg: format!("no such active room: {room}"),
                },
            )
            .await;
            Ok(())
        }
        JoinOutcome::Busy => {
            let _ = send_control(
                &mut ws,
                &ControlMsg::Error {
                    msg: "another player is already in this room; try again later".to_string(),
                },
            )
            .await;
            Ok(())
        }
        JoinOutcome::Ready { salt, tx } => {
            // Tell the guest the host's salt before handing off the stream.
            send_control(&mut ws, &ControlMsg::Joined { salt }).await?;
            // try_send (capacity 1) — Ready means it was free a moment ago.
            // If a race put another guest in front of us, treat the same as
            // Busy: cleanly close and let the joiner retry.
            if let Err(err) = tx.try_send(ws) {
                use tokio::sync::mpsc::error::TrySendError;
                let mut ws = match err {
                    TrySendError::Full(ws) | TrySendError::Closed(ws) => ws,
                };
                warn!(%room, "host busy or gone before guest could be paired");
                let _ = ws.close(None).await;
            }
            Ok(())
        }
    }
}

/// Which side caused the byte-relay session to end.
#[derive(Debug, Clone, Copy)]
enum RelayOutcome {
    /// The guest disconnected; the host's WebSocket is still alive and the
    /// caller may await the next guest.
    GuestLeft,
    /// The host's connection died first; the room must be released.
    HostLeft,
}

/// Pure byte relay between the host and a single guest, forwarding binary
/// frames verbatim in both directions. The server never inspects, buffers, or
/// decrypts payloads. Closes the guest's stream on the way out, but leaves
/// the host's WebSocket open so the next guest can be served on the same
/// connection.
async fn relay(
    host_tx: &mut WsServerTx,
    host_rx: &mut WsServerRx,
    guest: WsServer,
) -> RelayOutcome {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let (mut guest_tx, mut guest_rx) = guest.split();
    let host_gone = Arc::new(AtomicBool::new(false));

    let hg1 = host_gone.clone();
    let hg2 = host_gone.clone();

    // host -> guest: pull from host's read half, push to guest. If the host
    // side errors first we mark host_gone and break.
    let h2g = async {
        loop {
            match host_rx.next().await {
                Some(Ok(m)) if m.is_binary() => {
                    if guest_tx.send(m).await.is_err() {
                        break;
                    }
                }
                Some(Ok(m)) if m.is_close() => {
                    hg1.store(true, Ordering::Relaxed);
                    break;
                }
                Some(Ok(_)) => continue, // ignore ping/pong/text
                Some(Err(_)) | None => {
                    hg1.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        let _ = guest_tx.close().await;
    };

    // guest -> host: pull from guest, push to host. If the host write fails we
    // mark host_gone (the host's connection is dead).
    let g2h = async {
        loop {
            match guest_rx.next().await {
                Some(Ok(m)) if m.is_binary() => {
                    if host_tx.send(m).await.is_err() {
                        hg2.store(true, Ordering::Relaxed);
                        break;
                    }
                }
                Some(Ok(m)) if m.is_close() => break, // guest left
                Some(Ok(_)) => continue,
                Some(Err(_)) | None => break, // guest left
            }
        }
        // Important: do *not* close the host stream here; the outer handler
        // keeps it alive for the next guest.
    };

    // `select!` (not `join!`): when either side ends — typically the guest
    // closing its WS while the host is idle waiting for the next move — we
    // must return immediately so the outer loop can notify the host with
    // `PeerLeft`. Waiting for the still-blocked side would deadlock the host
    // task (the host stays parked on `recv_event` until it sees `PeerLeft`,
    // which only fires after this function returns).
    tokio::select! {
        _ = h2g => {},
        _ = g2h => {},
    }

    if host_gone.load(Ordering::Relaxed) {
        RelayOutcome::HostLeft
    } else {
        RelayOutcome::GuestLeft
    }
}

async fn send_control(ws: &mut WsServer, msg: &ControlMsg) -> anyhow::Result<()> {
    let txt = serde_json::to_string(msg)?;
    ws.send(WsMessage::Text(txt.into())).await?;
    Ok(())
}

/// `send_control` variant operating on the write half of a split WebSocket
/// (used by the host-side relay loop, which keeps the WS split for the
/// duration of the room so the read and write halves can be borrowed by the
/// byte-relay independently).
async fn send_control_tx(ws: &mut WsServerTx, msg: &ControlMsg) -> anyhow::Result<()> {
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

/// Wait for the next guest stream while also watching the host's WebSocket so
/// the loop terminates if the host disconnects mid-wait (otherwise the host
/// could be gone for minutes while we keep blocking on the channel). Returns
/// `Some(guest)` if a guest arrived, or `None` if the host left first.
async fn wait_next_guest(
    rx: &mut tokio::sync::mpsc::Receiver<WsServer>,
    host_rx: &mut WsServerRx,
) -> Option<WsServer> {
    loop {
        tokio::select! {
            biased;
            g = rx.recv() => return g,
            ev = host_rx.next() => match ev {
                Some(Ok(m)) if m.is_close() => return None,
                Some(Err(_)) | None => return None,
                // Ignore any text/ping/pong frames the host might send while
                // it has no active peer; keep waiting for either side.
                Some(Ok(_)) => continue,
            }
        }
    }
}
