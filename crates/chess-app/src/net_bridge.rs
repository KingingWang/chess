//! Non-blocking networking bridge (LAN direct or relayed over the internet).
//!
//! A Tokio task owns the [`chess_net::Session`]; it forwards locally-issued
//! commands (move/resign/draw/sync) to the peer and pushes received peer
//! events back to Bevy. Bevy systems only ever touch the two lock-free
//! channels, so the render loop is never blocked on socket IO.
//!
//! **Persistent host loops.** LAN and relay hosts run a loop: when a guest
//! disconnects, the host stays alive and waits for the next joiner (with the
//! same room number / port and password). The Bevy side is notified via
//! [`NetEvent::PeerDisconnected`] and re-synchronised by a fresh
//! [`NetEvent::Connected`] + [`NetCommand::Sync`] round once the new peer
//! completes the handshake.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, Game, Move};

use crate::app_state::{AppState, BoardOrientation, GameMode};
use crate::lan_dialog::LanDialog;
use chess_net::{
    host_handshake_on, wait_for_peer_joined, Connection, HandshakeError, InboundEvent, Message,
    NetError, RelayClientConfig, Role, Server, Session,
};
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
    RelayJoin {
        cfg: RelayClientConfig,
        room: String,
    },
}

/// Command sent from Bevy -> the network task.
#[derive(Debug, Clone)]
pub enum NetCommand {
    Move(Move),
    Resign,
    DrawOffer,
    DrawResponse(bool),
    /// Push the host's authoritative [`Game`] to the (just-(re)connected) guest.
    Sync(Box<Game>),
}

/// Event sent from the network task -> Bevy.
#[derive(Debug, Clone)]
pub enum NetEvent {
    /// Relay host only: the server assigned this room number to share.
    RoomCreated(String),
    /// A peer has completed the handshake. Sent on the first connection and
    /// again on every host-side reconnect.
    Connected {
        my_color: ChessColor,
    },
    /// The currently-connected peer disconnected; the host is waiting for the
    /// next joiner. Does **not** tear down the session.
    PeerDisconnected,
    /// The host re-synchronised this client to its authoritative game state
    /// (sent on guest reconnect to restore the in-progress position).
    PeerSync(Box<Game>),
    PeerMove(Move),
    PeerResign,
    PeerDrawOffer,
    PeerDrawResponse(bool),
    /// A non-fatal network error (logged; does not bounce mid-game).
    Error(String),
    /// The session is terminally dead — bounce back to the menu.
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
        match target {
            NetTarget::Lan {
                role: Role::Host,
                addr,
            } => run_host_lan(addr, host_color, name, password, cmd_rx, evt_tx).await,
            NetTarget::Lan {
                role: Role::Guest,
                addr,
            } => run_guest_lan(addr, name, password, cmd_rx, evt_tx).await,
            NetTarget::RelayHost { cfg } => {
                run_host_relay(cfg, host_color, name, password, cmd_rx, evt_tx).await
            }
            NetTarget::RelayJoin { cfg, room } => {
                run_guest_relay(cfg, room, name, password, cmd_rx, evt_tx).await
            }
        }
    });

    NetLink {
        out: cmd_tx,
        inbound: evt_rx,
    }
}

/// Outcome of one peer-relaying iteration of the inner loop.
enum InnerOutcome {
    /// The peer disconnected cleanly; the host loop should wait for the next
    /// joiner.
    PeerLeft,
    /// The session is terminally dead (transport gone, our channel closed,
    /// fatal protocol error). The task must return.
    Fatal,
}

/// Drive a connected session: interleave outbound commands and inbound
/// messages until the peer disconnects (returns [`InnerOutcome::PeerLeft`])
/// or a fatal error occurs (returns [`InnerOutcome::Fatal`]).
async fn run_inner_loop(
    conn: &mut Connection,
    cmd_rx: &Receiver<NetCommand>,
    evt_tx: &Sender<NetEvent>,
) -> InnerOutcome {
    use tokio::time::{sleep, Duration};
    loop {
        // Drain any pending local commands first (non-blocking).
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => {
                    let res: Result<(), NetError> = match cmd {
                        NetCommand::Move(mv) => conn.send(&Message::Move { mv: mv.into() }).await,
                        NetCommand::Resign => conn.send(&Message::Resign).await,
                        NetCommand::DrawOffer => conn.send(&Message::DrawOffer).await,
                        NetCommand::DrawResponse(a) => {
                            conn.send(&Message::DrawResponse { accept: a }).await
                        }
                        NetCommand::Sync(game) => conn.send(&Message::Sync { game }).await,
                    };
                    if let Err(e) = res {
                        let _ = evt_tx.send(NetEvent::Error(format!("send failed: {e}")));
                        return InnerOutcome::Fatal;
                    }
                }
                Err(TryRecvError::Empty) => break,
                // Bevy dropped the link (e.g. menu teardown): treat as fatal.
                Err(TryRecvError::Disconnected) => return InnerOutcome::Fatal,
            }
        }

        // Poll the socket with a short timeout so we stay responsive to local
        // commands without busy-spinning.
        match tokio::time::timeout(Duration::from_millis(20), conn.recv_event()).await {
            Ok(Ok(InboundEvent::Message(msg))) => {
                let evt = match msg {
                    Message::Move { mv } => match mv.parse() {
                        Some(m) => NetEvent::PeerMove(m),
                        None => NetEvent::Error("peer sent unparseable move".into()),
                    },
                    Message::Resign => NetEvent::PeerResign,
                    Message::DrawOffer => NetEvent::PeerDrawOffer,
                    Message::DrawResponse { accept } => NetEvent::PeerDrawResponse(accept),
                    Message::Sync { game } => NetEvent::PeerSync(game),
                    // Hello and Ping arriving mid-session are stale/keepalive
                    // noise and silently ignored.
                    Message::Hello { .. } | Message::Ping => continue,
                };
                if evt_tx.send(evt).is_err() {
                    return InnerOutcome::Fatal;
                }
            }
            // Relay told us the peer left: end this iteration cleanly so the
            // host loop can await the next joiner.
            Ok(Ok(InboundEvent::PeerLeft)) => return InnerOutcome::PeerLeft,
            // A stray PeerJoined here would mean the server's already routed a
            // new guest before we exited this iteration; treat the same — let
            // the host re-handshake.
            Ok(Ok(InboundEvent::PeerJoined)) => return InnerOutcome::PeerLeft,
            Ok(Err(NetError::Closed)) => return InnerOutcome::PeerLeft,
            Ok(Err(e)) => {
                let _ = evt_tx.send(NetEvent::Error(format!("recv failed: {e}")));
                // A decryption / decode error from this peer is not fatal to
                // the host loop (the peer just sent garbage); but for a guest
                // there's no recovery path either way, and the host loop will
                // see the peer drop on its next read. Bail this iteration.
                return InnerOutcome::PeerLeft;
            }
            Err(_) => {
                // recv timeout: loop again to check commands.
                sleep(Duration::from_millis(1)).await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// LAN
// ---------------------------------------------------------------------------

/// Persistent LAN host task: bind once, then loop accepting guests on the same
/// port. When a guest drops, emit [`NetEvent::PeerDisconnected`] and wait for
/// the next joiner.
async fn run_host_lan(
    addr: String,
    host_color: ChessColor,
    name: String,
    password: String,
    cmd_rx: Receiver<NetCommand>,
    evt_tx: Sender<NetEvent>,
) {
    let server = match Server::bind(addr.as_str()).await {
        Ok(s) => s,
        Err(e) => {
            let _ = evt_tx.send(NetEvent::Error(format!("handshake failed: {e}")));
            let _ = evt_tx.send(NetEvent::Disconnected);
            return;
        }
    };
    loop {
        let mut conn = match server.accept_one(&password).await {
            Ok(c) => c,
            Err(e) => {
                // The listener died (or a one-off OS error). For the very first
                // attempt this is a true connect failure; for a re-accept it
                // means the listening socket is gone and we cannot recover.
                let _ = evt_tx.send(NetEvent::Error(format!("handshake failed: {e}")));
                let _ = evt_tx.send(NetEvent::Disconnected);
                return;
            }
        };
        if let Err(e) = host_handshake_on(&mut conn, host_color, &name).await {
            // The listener is still fine — the joiner just couldn't complete
            // the handshake (wrong password, abort, protocol mismatch). Log
            // and keep waiting for the next attempt. We never bail here.
            let _ = evt_tx.send(NetEvent::Error(format!("rejected joiner: {e}")));
            continue;
        }
        if evt_tx
            .send(NetEvent::Connected {
                my_color: host_color,
            })
            .is_err()
        {
            return;
        }
        match run_inner_loop(&mut conn, &cmd_rx, &evt_tx).await {
            InnerOutcome::PeerLeft => {
                if evt_tx.send(NetEvent::PeerDisconnected).is_err() {
                    return;
                }
                continue;
            }
            InnerOutcome::Fatal => {
                let _ = evt_tx.send(NetEvent::Disconnected);
                return;
            }
        }
    }
}

/// LAN guest: single-shot connect; on disconnect the session ends.
async fn run_guest_lan(
    addr: String,
    name: String,
    password: String,
    cmd_rx: Receiver<NetCommand>,
    evt_tx: Sender<NetEvent>,
) {
    use tokio::time::{timeout, Duration};
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);

    let res = timeout(
        CONNECT_TIMEOUT,
        Session::join(addr.as_str(), &name, &password),
    )
    .await;
    let mut session = match res {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let _ = evt_tx.send(NetEvent::Error(format!("handshake failed: {e}")));
            let _ = evt_tx.send(NetEvent::Disconnected);
            return;
        }
        Err(_) => {
            let _ = evt_tx.send(NetEvent::Error(
                "handshake failed: timed out while connecting".into(),
            ));
            let _ = evt_tx.send(NetEvent::Disconnected);
            return;
        }
    };
    if evt_tx
        .send(NetEvent::Connected {
            my_color: session.my_color,
        })
        .is_err()
    {
        return;
    }
    let _ = run_inner_loop(&mut session.conn, &cmd_rx, &evt_tx).await;
    let _ = evt_tx.send(NetEvent::Disconnected);
}

// ---------------------------------------------------------------------------
// Relay (internet)
// ---------------------------------------------------------------------------

/// Persistent relay host task: create the room, then loop accepting guests on
/// the same WebSocket. The relay server keeps the room number reserved for
/// the lifetime of the host's WSS connection.
async fn run_host_relay(
    cfg: RelayClientConfig,
    host_color: ChessColor,
    name: String,
    password: String,
    cmd_rx: Receiver<NetCommand>,
    evt_tx: Sender<NetEvent>,
) {
    use tokio::time::{timeout, Duration};
    const CREATE_TIMEOUT: Duration = Duration::from_secs(12);

    let pending = match timeout(CREATE_TIMEOUT, Session::relay_create(&cfg, &password)).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            let _ = evt_tx.send(NetEvent::Error(format!("handshake failed: {e}")));
            let _ = evt_tx.send(NetEvent::Disconnected);
            return;
        }
        Err(_) => {
            let _ = evt_tx.send(NetEvent::Error(
                "handshake failed: timed out while connecting".into(),
            ));
            let _ = evt_tx.send(NetEvent::Disconnected);
            return;
        }
    };
    if evt_tx
        .send(NetEvent::RoomCreated(pending.room.clone()))
        .is_err()
    {
        return;
    }

    let mut conn = match pending.await_guest().await {
        Ok(c) => c,
        Err(e) => {
            let _ = evt_tx.send(NetEvent::Error(format!("handshake failed: {e}")));
            let _ = evt_tx.send(NetEvent::Disconnected);
            return;
        }
    };

    let mut first = true;
    loop {
        if !first {
            // Wait for the next PeerJoined notice from the relay server.
            if let Err(e) = wait_for_peer_joined(&mut conn).await {
                let _ = evt_tx.send(NetEvent::Error(format!("relay closed: {e}")));
                let _ = evt_tx.send(NetEvent::Disconnected);
                return;
            }
        }
        // A failed handshake consumes the "we already have a peer ready"
        // state, so set `first = false` first: the next iteration must wait
        // for the relay to signal a fresh PeerJoined before trying again
        // (otherwise we'd re-send our stale Hello into wait_next_guest and
        // never line up with the new joiner — they'd both be parked waiting
        // for each other).
        first = false;
        match host_handshake_on(&mut conn, host_color, &name).await {
            Ok(()) => {}
            // PeerLeft: relay told us the joiner gave up (typical for wrong
            // password — the guest couldn't decrypt our Hello and dropped).
            // The relay WS is still healthy: loop and wait for the next.
            Err(HandshakeError::PeerLeft) => {
                let _ = evt_tx.send(NetEvent::Error("rejected joiner: peer left".into()));
                continue;
            }
            Err(HandshakeError::Net(NetError::Closed)) => {
                // The relay link itself died — fatal.
                let _ = evt_tx.send(NetEvent::Error("relay connection closed".into()));
                let _ = evt_tx.send(NetEvent::Disconnected);
                return;
            }
            Err(e) => {
                // Protocol mismatch / decode error from the joiner: keep the
                // room open, log, and wait for the next.
                let _ = evt_tx.send(NetEvent::Error(format!("rejected joiner: {e}")));
                continue;
            }
        }
        if evt_tx
            .send(NetEvent::Connected {
                my_color: host_color,
            })
            .is_err()
        {
            return;
        }
        match run_inner_loop(&mut conn, &cmd_rx, &evt_tx).await {
            InnerOutcome::PeerLeft => {
                if evt_tx.send(NetEvent::PeerDisconnected).is_err() {
                    return;
                }
                continue;
            }
            InnerOutcome::Fatal => {
                let _ = evt_tx.send(NetEvent::Disconnected);
                return;
            }
        }
    }
}

/// Relay guest: single-shot connect; on disconnect the session ends.
async fn run_guest_relay(
    cfg: RelayClientConfig,
    room: String,
    name: String,
    password: String,
    cmd_rx: Receiver<NetCommand>,
    evt_tx: Sender<NetEvent>,
) {
    use tokio::time::{timeout, Duration};
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);

    let res = timeout(
        CONNECT_TIMEOUT,
        Session::join_relay(&cfg, &room, &name, &password),
    )
    .await;
    let mut session = match res {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let _ = evt_tx.send(NetEvent::Error(format!("handshake failed: {e}")));
            let _ = evt_tx.send(NetEvent::Disconnected);
            return;
        }
        Err(_) => {
            let _ = evt_tx.send(NetEvent::Error(
                "handshake failed: timed out while connecting".into(),
            ));
            let _ = evt_tx.send(NetEvent::Disconnected);
            return;
        }
    };
    if evt_tx
        .send(NetEvent::Connected {
            my_color: session.my_color,
        })
        .is_err()
    {
        return;
    }
    // Guest doesn't reconnect; one inner loop and we're done.
    let _ = run_inner_loop(&mut session.conn, &cmd_rx, &evt_tx).await;
    let _ = evt_tx.send(NetEvent::Disconnected);
}

// ---------------------------------------------------------------------------
// Bevy poll system
// ---------------------------------------------------------------------------

/// A user-facing message for a connection that failed *before* it was ever
/// established (server down, room missing, wrong password, host absent, ...).
fn connect_error_message(mode: GameMode) -> String {
    match mode {
        GameMode::RelayJoin => "加入失败：服务器未启动、房间不存在或密码错误".to_string(),
        GameMode::RelayHost => "创建失败：无法连接中继服务器".to_string(),
        GameMode::LanJoin => {
            "连接失败：找不到主机（请确认对方已创建房间，且 IP/端口正确）".to_string()
        }
        GameMode::LanHost => "监听失败：端口可能已被占用".to_string(),
        _ => "网络连接失败".to_string(),
    }
}

/// Bevy system: drain peer events and apply them to the authoritative game.
pub fn poll_net_events(
    mut commands: Commands,
    link: Option<Res<NetLink>>,
    mut core: ResMut<crate::app_state::CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    mut orient: ResMut<BoardOrientation>,
    mut next: ResMut<NextState<AppState>>,
    mut dialog: ResMut<LanDialog>,
) {
    let Some(link) = link else {
        return;
    };

    // Drain first so we never hold the channel borrow across a state change.
    let events: Vec<NetEvent> = link.inbound.try_iter().collect();

    for evt in events {
        // A terminal `Disconnected` or `Error` *before we ever connected* means
        // the room could not be joined/created: abort back to the menu with a
        // user-visible reason instead of sitting on an empty board forever.
        if matches!(evt, NetEvent::Error(_) | NetEvent::Disconnected) && !core.connected {
            if let NetEvent::Error(ref e) = evt {
                warn!(error = %e, "connection failed before established");
            }
            dialog.error = Some(connect_error_message(core.mode));
            dialog.open = true;
            dialog.rebuild = true;
            core.awaiting_peer = false;
            core.room_code = None;
            commands.remove_resource::<NetLink>();
            next.set(AppState::Menu);
            return;
        }

        match evt {
            NetEvent::RoomCreated(room) => {
                info!(%room, "relay room created");
                core.room_code = Some(room);
                core.awaiting_peer = true;
            }
            NetEvent::Connected { my_color } => {
                core.local_color = my_color;
                core.awaiting_peer = false;
                core.peer_disconnected = false;
                core.connected = true;
                // Flip the board so the local player always sits at the bottom.
                *orient = BoardOrientation::from_color(my_color);
                dirty.0 = true;
                info!(?my_color, "connected to peer");
                // Hosts push their authoritative game state so a (re)joining
                // guest can resume the match from the current position.
                if core.mode.is_net_host() {
                    let _ = link.out.send(NetCommand::Sync(Box::new(core.game.clone())));
                }
            }
            NetEvent::PeerSync(game) => {
                core.game = *game;
                core.awaiting_peer = false;
                core.peer_disconnected = false;
                dirty.0 = true;
                info!("synchronised with host's game state");
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
            NetEvent::PeerDisconnected => {
                core.peer_disconnected = true;
                warn!("peer disconnected; awaiting reconnect");
            }
            NetEvent::Error(e) => warn!(error = %e, "network error"),
            NetEvent::Disconnected => {
                // Session terminated after at least one successful connect.
                // We don't auto-bounce here so the player can see the final
                // position; they return to the menu via the HUD button.
                core.awaiting_peer = false;
                core.peer_disconnected = true;
                warn!("session ended");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::CoreGame;
    use crate::board_view::RenderDirty;
    use chess_core::Game;

    fn base_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<AppState>();
        app.insert_resource(RenderDirty::default());
        app.insert_resource(BoardOrientation::default());
        app.insert_resource(LanDialog::default());
        app.add_systems(Update, poll_net_events);
        app
    }

    fn link_with(event: NetEvent) -> (NetLink, Receiver<NetCommand>) {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<NetCommand>();
        let (evt_tx, evt_rx) = crossbeam_channel::unbounded::<NetEvent>();
        evt_tx.send(event).unwrap();
        (
            NetLink {
                out: cmd_tx,
                inbound: evt_rx,
            },
            cmd_rx,
        )
    }

    #[test]
    fn failure_before_connect_returns_to_menu_with_error() {
        let mut app = base_app();
        let core = CoreGame {
            mode: GameMode::RelayJoin,
            awaiting_peer: true,
            connected: false,
            ..Default::default()
        };
        app.insert_resource(core);
        let (link, _cmd_rx) = link_with(NetEvent::Error("connection refused".into()));
        app.insert_resource(link);

        app.update();

        let dialog = app.world().resource::<LanDialog>();
        assert!(dialog.open, "dialog should reopen");
        assert!(dialog.error.is_some(), "an error message should be shown");
        assert!(
            app.world().get_resource::<NetLink>().is_none(),
            "the dead NetLink must be removed"
        );
        match app.world().resource::<NextState<AppState>>() {
            NextState::Pending(s) => assert_eq!(*s, AppState::Menu),
            _ => panic!("expected a pending transition back to the menu"),
        }
    }

    #[test]
    fn lan_join_to_refused_address_bounces_to_menu() {
        use std::time::{Duration, Instant};

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        // Port 1 on loopback refuses immediately: a real connect failure.
        let link = start_net(
            &rt,
            NetTarget::Lan {
                role: Role::Guest,
                addr: "127.0.0.1:1".into(),
            },
            ChessColor::Red,
            "guest".into(),
            "pw".into(),
        );

        let mut app = base_app();
        let core = CoreGame {
            mode: GameMode::LanJoin,
            awaiting_peer: true,
            connected: false,
            ..Default::default()
        };
        app.insert_resource(core);
        app.insert_resource(link);

        let start = Instant::now();
        loop {
            app.update();
            if app.world().get_resource::<NetLink>().is_none() {
                break; // bounced: dead link removed
            }
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "join to a refused address must bounce back, but it never did"
            );
            std::thread::sleep(Duration::from_millis(20));
        }

        let dialog = app.world().resource::<LanDialog>();
        assert!(dialog.open, "menu dialog should reopen on failure");
        assert!(dialog.error.is_some(), "an error must be shown");
        assert!(
            !app.world().resource::<CoreGame>().connected,
            "must never have been marked connected"
        );
    }

    #[test]
    fn connected_event_activates_game_and_assigns_color() {
        let mut app = base_app();
        let core = CoreGame {
            mode: GameMode::RelayJoin,
            awaiting_peer: true,
            connected: false,
            ..Default::default()
        };
        app.insert_resource(core);
        let (link, _cmd_rx) = link_with(NetEvent::Connected {
            my_color: ChessColor::Black,
        });
        app.insert_resource(link);

        app.update();

        let core = app.world().resource::<CoreGame>();
        assert!(!core.awaiting_peer, "a connected game must stop waiting");
        assert!(core.connected, "connected flag must be set");
        assert_eq!(core.local_color, ChessColor::Black, "color comes from peer");
    }

    #[test]
    fn connected_event_on_host_pushes_sync_command() {
        let mut app = base_app();
        let core = CoreGame {
            mode: GameMode::RelayHost,
            awaiting_peer: true,
            connected: false,
            ..Default::default()
        };
        app.insert_resource(core);
        let (link, cmd_rx) = link_with(NetEvent::Connected {
            my_color: ChessColor::Red,
        });
        app.insert_resource(link);

        app.update();

        match cmd_rx.try_recv() {
            Ok(NetCommand::Sync(_)) => {}
            other => panic!("host must push Sync after Connected, got {other:?}"),
        }
    }

    #[test]
    fn peer_sync_overrides_local_game() {
        let mut app = base_app();
        let core = CoreGame {
            mode: GameMode::RelayJoin,
            connected: true,
            peer_disconnected: false,
            ..Default::default()
        };
        app.insert_resource(core);
        // Build a host-side game with one move played.
        let mut host_game = Game::new();
        host_game
            .make_move(Move::from_iccs("h2e2").unwrap())
            .unwrap();
        let (link, _cmd_rx) = link_with(NetEvent::PeerSync(Box::new(host_game.clone())));
        app.insert_resource(link);

        app.update();

        let core = app.world().resource::<CoreGame>();
        assert_eq!(
            core.game.history_len(),
            1,
            "PeerSync should replace local game with host's state"
        );
        assert!(!core.peer_disconnected);
    }

    #[test]
    fn peer_disconnected_does_not_bounce_to_menu() {
        let mut app = base_app();
        let core = CoreGame {
            mode: GameMode::RelayHost,
            connected: true, // already mid-game
            ..Default::default()
        };
        app.insert_resource(core);
        let (link, _cmd_rx) = link_with(NetEvent::PeerDisconnected);
        app.insert_resource(link);

        app.update();

        let core = app.world().resource::<CoreGame>();
        assert!(core.peer_disconnected, "flag must be set");
        assert!(
            app.world().get_resource::<NetLink>().is_some(),
            "link must stay alive so the host can serve a reconnect"
        );
    }

    #[test]
    fn error_after_connect_does_not_bounce() {
        let mut app = base_app();
        let core = CoreGame {
            mode: GameMode::RelayJoin,
            connected: true, // already in a live game
            ..Default::default()
        };
        app.insert_resource(core);
        let (link, _cmd_rx) = link_with(NetEvent::Error("transient".into()));
        app.insert_resource(link);

        app.update();

        assert!(
            app.world().get_resource::<NetLink>().is_some(),
            "a mid-game error must not tear down the link"
        );
        let dialog = app.world().resource::<LanDialog>();
        assert!(!dialog.open, "the menu dialog must stay closed mid-game");
    }
}
