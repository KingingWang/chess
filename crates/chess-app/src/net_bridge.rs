//! Non-blocking networking bridge (LAN direct or relayed over the internet).
//!
//! A Tokio task owns the [`chess_net::Session`]; it forwards locally-issued
//! commands (move/resign/draw) to the peer and pushes received peer events back
//! to Bevy. Bevy systems only ever touch the two lock-free channels, so the
//! render loop is never blocked on socket IO.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, Move};
use chess_net::{Message, RelayClientConfig, Role, Session};
use crossbeam_channel::{Receiver, Sender, TryRecvError};

/// Resolved relay client configuration, loaded once at startup and shared with
/// the networking task whenever a server (internet) game starts.
#[derive(Resource, Clone)]
pub struct RelayConfig(pub RelayClientConfig);

/// Where/how to establish the networked session.
pub enum NetTarget {
    /// Direct LAN connection (host binds / guest connects to `addr`).
    Lan { role: Role, addr: String },
    /// Create a relayed room on the relay server.
    RelayHost { cfg: RelayClientConfig },
    /// Join a relayed room by number on the relay server.
    RelayJoin { cfg: RelayClientConfig, room: String },
}

/// Command sent from Bevy -> the network task.
#[derive(Debug, Clone)]
pub enum NetCommand {
    Move(Move),
    Resign,
    DrawOffer,
    DrawResponse(bool),
}

/// Event sent from the network task -> Bevy.
#[derive(Debug, Clone)]
pub enum NetEvent {
    /// Relay host only: the server assigned this room number to share.
    RoomCreated(String),
    Connected { my_color: ChessColor },
    PeerMove(Move),
    PeerResign,
    PeerDrawOffer,
    PeerDrawResponse(bool),
    Error(String),
    Disconnected,
}

/// Resource holding the live link to the network task.
#[derive(Resource)]
pub struct NetLink {
    pub out: Sender<NetCommand>,
    pub inbound: Receiver<NetEvent>,
}

/// Spawn the network task for the given target, returning the [`NetLink`].
pub fn start_net(
    runtime: &tokio::runtime::Runtime,
    target: NetTarget,
    host_color: ChessColor,
    name: String,
    password: String,
) -> NetLink {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<NetCommand>();
    let (evt_tx, evt_rx) = crossbeam_channel::unbounded::<NetEvent>();

    runtime.spawn(async move {
        let session = establish(target, host_color, &name, &password, &evt_tx).await;
        let mut session = match session {
            Some(s) => s,
            None => return, // an error was already reported
        };
        let _ = evt_tx.send(NetEvent::Connected {
            my_color: session.my_color,
        });

        run_session_loop(&mut session, cmd_rx, evt_tx).await;
    });

    NetLink {
        out: cmd_tx,
        inbound: evt_rx,
    }
}

/// Establish the session for `target`, reporting errors via `evt_tx`. Returns
/// `None` (after sending an `Error`) on failure.
async fn establish(
    target: NetTarget,
    host_color: ChessColor,
    name: &str,
    password: &str,
    evt_tx: &Sender<NetEvent>,
) -> Option<Session> {
    let result = match target {
        NetTarget::Lan { role, addr } => match role {
            Role::Host => Session::host(addr.as_str(), host_color, name, password).await,
            Role::Guest => Session::join(addr.as_str(), name, password).await,
        },
        NetTarget::RelayHost { cfg } => {
            // Two phases: create (learn the room number to share), then wait
            // for the guest, then run the color-assignment handshake.
            match Session::relay_create(&cfg, password).await {
                Ok(pending) => {
                    let _ = evt_tx.send(NetEvent::RoomCreated(pending.room.clone()));
                    match pending.await_guest().await {
                        Ok(conn) => Session::handshake_as_host(conn, host_color, name).await,
                        Err(e) => Err(e.into()),
                    }
                }
                Err(e) => Err(e),
            }
        }
        NetTarget::RelayJoin { cfg, room } => {
            Session::join_relay(&cfg, &room, name, password).await
        }
    };

    match result {
        Ok(s) => Some(s),
        Err(e) => {
            let _ = evt_tx.send(NetEvent::Error(format!("handshake failed: {e}")));
            let _ = evt_tx.send(NetEvent::Disconnected);
            None
        }
    }
}

/// Drive the session: interleave outbound commands and inbound messages.
async fn run_session_loop(
    session: &mut Session,
    cmd_rx: Receiver<NetCommand>,
    evt_tx: Sender<NetEvent>,
) {
    use tokio::time::{sleep, Duration};
    loop {
        // Drain any pending local commands (non-blocking).
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => {
                    let res = match cmd {
                        NetCommand::Move(mv) => session.send_move(mv).await,
                        NetCommand::Resign => session.resign().await,
                        NetCommand::DrawOffer => session.offer_draw().await,
                        NetCommand::DrawResponse(a) => session.respond_draw(a).await,
                    };
                    if let Err(e) = res {
                        let _ = evt_tx.send(NetEvent::Error(format!("send failed: {e}")));
                        let _ = evt_tx.send(NetEvent::Disconnected);
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        // Poll the socket with a short timeout so we stay responsive to local
        // commands without busy-spinning.
        match tokio::time::timeout(Duration::from_millis(20), session.recv()).await {
            Ok(Ok(msg)) => {
                let evt = match msg {
                    Message::Move { mv } => match mv.parse() {
                        Some(m) => NetEvent::PeerMove(m),
                        None => NetEvent::Error("peer sent unparseable move".into()),
                    },
                    Message::Resign => NetEvent::PeerResign,
                    Message::DrawOffer => NetEvent::PeerDrawOffer,
                    Message::DrawResponse { accept } => NetEvent::PeerDrawResponse(accept),
                    Message::Hello { .. } | Message::Ping => continue,
                };
                if evt_tx.send(evt).is_err() {
                    return;
                }
            }
            Ok(Err(e)) => {
                let _ = evt_tx.send(NetEvent::Error(format!("recv failed: {e}")));
                let _ = evt_tx.send(NetEvent::Disconnected);
                return;
            }
            Err(_) => {
                // timeout: just loop again to check commands
                sleep(Duration::from_millis(1)).await;
            }
        }
    }
}

/// Bevy system: drain peer events and apply them to the authoritative game.
pub fn poll_net_events(
    link: Option<Res<NetLink>>,
    mut core: ResMut<crate::app_state::CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let Some(link) = link else {
        return;
    };
    while let Ok(evt) = link.inbound.try_recv() {
        match evt {
            NetEvent::RoomCreated(room) => {
                info!(%room, "relay room created");
                core.room_code = Some(room);
                core.awaiting_peer = true;
            }
            NetEvent::Connected { my_color } => {
                core.local_color = my_color;
                core.awaiting_peer = false;
                info!(?my_color, "connected to peer");
            }
            NetEvent::PeerMove(mv) => {
                crate::moves::apply_local_move(&mut core, mv);
                dirty.0 = true;
            }
            NetEvent::PeerResign => {
                let winner = core.local_color;
                core.game.resign(winner.opponent());
                dirty.0 = true;
            }
            NetEvent::PeerDrawOffer => {
                info!("peer offered a draw");
                core.draw_offer_from_peer = true;
            }
            NetEvent::PeerDrawResponse(accept) => {
                if accept {
                    core.game.agree_draw();
                }
            }
            NetEvent::Error(e) => warn!(error = %e, "network error"),
            NetEvent::Disconnected => {
                core.awaiting_peer = false;
                warn!("peer disconnected");
            }
        }
    }
}
