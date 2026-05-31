//! End-to-end test: a real `chess-relay` server (over self-signed TLS) pairs a
//! host and a guest by room number, and forwards their end-to-end-encrypted
//! moves. The server only relays opaque bytes — it never sees the password,
//! the salt-derived key, or any game data.

use std::path::PathBuf;

use chess_core::{Color, Game, Move};
use chess_net::{Message, RelayClientConfig, Session};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Generate a self-signed cert/key for localhost, write them to a temp dir, and
/// return their paths.
fn write_dev_cert() -> (PathBuf, PathBuf) {
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "chess-relay-test-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");
    std::fs::write(&cert_path, cert.pem()).unwrap();
    std::fs::write(&key_path, signing_key.serialize_pem()).unwrap();
    (cert_path, key_path)
}

/// Start a relay server on an ephemeral loopback port; returns the port.
async fn start_server() -> u16 {
    let (cert, key) = write_dev_cert();
    let acceptor = chess_relay::build_acceptor(&cert, &key).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(chess_relay::serve(
        listener,
        acceptor,
        chess_relay::Rooms::default(),
    ));
    port
}

/// Dev client config: connect to loopback, skip cert verification.
fn client_cfg(port: u16) -> RelayClientConfig {
    RelayClientConfig {
        host: "localhost".to_string(),
        port,
        ca_path: None,
        insecure: true,
    }
}

#[tokio::test]
async fn relay_pairs_and_forwards_moves_end_to_end() {
    let port = start_server().await;
    let cfg = client_cfg(port);

    // The host shares the room number with the guest over this channel as soon
    // as the server assigns it (before the guest joins).
    let (room_tx, room_rx) = oneshot::channel::<String>();

    let host_cfg = cfg.clone();
    let host = tokio::spawn(async move {
        let pending = Session::relay_create(&host_cfg, "room-pw").await.unwrap();
        room_tx.send(pending.room.clone()).unwrap();

        let conn = pending.await_guest().await.unwrap();
        let mut session = Session::handshake_as_host(conn, Color::Red, "host")
            .await
            .unwrap();
        assert_eq!(session.my_color, Color::Red);

        // Host (Red) plays first.
        let mut game = Game::new();
        let mv = Move::from_iccs("h2e2").unwrap();
        game.make_move(mv).unwrap();
        session.send_move(mv).await.unwrap();

        // Receive and validate the guest's reply.
        match session.recv().await.unwrap() {
            Message::Move { mv } => {
                let m = mv.parse().unwrap();
                assert!(game.make_move(m).is_ok(), "peer move must be legal");
            }
            other => panic!("unexpected: {other:?}"),
        }
    });

    let guest_cfg = cfg.clone();
    let guest = tokio::spawn(async move {
        let room = room_rx.await.unwrap();
        let mut session = Session::join_relay(&guest_cfg, &room, "guest", "room-pw")
            .await
            .unwrap();
        assert_eq!(session.my_color, Color::Black);

        // Receive the host's move and apply it.
        let mut game = Game::new();
        match session.recv().await.unwrap() {
            Message::Move { mv } => {
                let m = mv.parse().unwrap();
                game.make_move(m).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }

        // Guest (Black) replies with a legal move.
        let reply = Move::from_iccs("b9c7").unwrap();
        assert!(game.make_move(reply).is_ok());
        session.send_move(reply).await.unwrap();
    });

    host.await.unwrap();
    guest.await.unwrap();
}

#[tokio::test]
async fn joining_with_wrong_password_fails_handshake() {
    let port = start_server().await;
    let cfg = client_cfg(port);

    let (room_tx, room_rx) = oneshot::channel::<String>();

    let host_cfg = cfg.clone();
    let host = tokio::spawn(async move {
        let pending = Session::relay_create(&host_cfg, "right-pw").await.unwrap();
        room_tx.send(pending.room.clone()).unwrap();
        let conn = pending.await_guest().await.unwrap();
        // The host seals its Hello with the correct password; the guest using a
        // wrong password cannot decrypt it. Either side may then error out.
        let _ = Session::handshake_as_host(conn, Color::Red, "host").await;
    });

    let guest_cfg = cfg.clone();
    let guest = tokio::spawn(async move {
        let room = room_rx.await.unwrap();
        let res = Session::join_relay(&guest_cfg, &room, "guest", "wrong-pw").await;
        assert!(res.is_err(), "a wrong password must fail the handshake");
    });

    let _ = host.await;
    guest.await.unwrap();
}

#[tokio::test]
async fn joining_unknown_room_is_rejected() {
    let port = start_server().await;
    let cfg = client_cfg(port);
    let res = Session::join_relay(&cfg, "00000001", "guest", "pw").await;
    assert!(res.is_err(), "joining a non-existent room must fail");
}

/// Guest #1 connects, plays a move, drops; guest #2 then joins the SAME room
/// with the SAME password. The relay must keep the room alive across the
/// disconnect and the host must be able to continue serving moves to the new
/// guest after a fresh handshake.
#[tokio::test]
async fn room_survives_guest_reconnect() {
    let port = start_server().await;
    let cfg = client_cfg(port);
    let (room_tx, room_rx) = oneshot::channel::<String>();

    let host_cfg = cfg.clone();
    let host = tokio::spawn(async move {
        let pending = Session::relay_create(&host_cfg, "pw").await.unwrap();
        room_tx.send(pending.room.clone()).unwrap();

        // First guest.
        let mut conn = pending.await_guest().await.unwrap();
        chess_net::host_handshake_on(&mut conn, Color::Red, "host")
            .await
            .unwrap();

        // Exchange one move with guest #1, then notice they leave.
        let mv = Move::from_iccs("h2e2").unwrap();
        conn.send(&chess_net::Message::Move { mv: mv.into() })
            .await
            .unwrap();
        // Drain until the relay tells us the guest is gone.
        loop {
            match conn.recv_event().await.unwrap() {
                chess_net::InboundEvent::PeerLeft => break,
                chess_net::InboundEvent::Message(_) => continue,
                chess_net::InboundEvent::PeerJoined => continue,
            }
        }

        // Second guest joins on the same room.
        chess_net::wait_for_peer_joined(&mut conn).await.unwrap();
        chess_net::host_handshake_on(&mut conn, Color::Red, "host")
            .await
            .unwrap();
        // Resync (the second guest needs the host's current position).
        let mut game = Game::new();
        game.make_move(mv).unwrap();
        conn.send(&chess_net::Message::Sync {
            game: Box::new(game.clone()),
        })
        .await
        .unwrap();
        // Receive guest #2's reply move.
        match conn.recv().await.unwrap() {
            Message::Move { mv } => {
                let m = mv.parse().unwrap();
                assert!(game.make_move(m).is_ok(), "guest #2 move must be legal");
            }
            other => panic!("unexpected: {other:?}"),
        }
    });

    let guest1_cfg = cfg.clone();
    let guest1 = tokio::spawn(async move {
        let room = room_rx.await.unwrap();
        let mut session = Session::join_relay(&guest1_cfg, &room, "g1", "pw")
            .await
            .unwrap();
        // Receive the host's first move, then disconnect immediately.
        match session.recv().await.unwrap() {
            Message::Move { .. } => {}
            other => panic!("unexpected: {other:?}"),
        }
        drop(session);
        room
    });
    let room = guest1.await.unwrap();

    // Now guest #2 joins on the same room number.
    let guest2_cfg = cfg.clone();
    let guest2 = tokio::spawn(async move {
        // Reconnect can race with the relay finishing teardown of guest #1;
        // retry briefly until the room registers as available.
        let mut attempt = 0u32;
        let mut session = loop {
            match Session::join_relay(&guest2_cfg, &room, "g2", "pw").await {
                Ok(s) => break s,
                Err(_) if attempt < 50 => {
                    attempt += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                Err(e) => panic!("guest #2 could not rejoin room {room}: {e}"),
            }
        };

        // Expect a Sync from the host, then play a follow-up move.
        let mut local_game;
        match session.recv().await.unwrap() {
            Message::Sync { game } => {
                local_game = *game;
                assert_eq!(
                    local_game.history_len(),
                    1,
                    "Sync must carry the host's in-progress game (one move played)"
                );
                assert_eq!(
                    local_game.side_to_move(),
                    Color::Black,
                    "after host's h2e2 it should be Black to move"
                );
            }
            other => panic!("expected Sync, got {other:?}"),
        }
        let reply = Move::from_iccs("b9c7").unwrap();
        assert!(local_game.make_move(reply).is_ok());
        session.send_move(reply).await.unwrap();
    });

    host.await.unwrap();
    guest2.await.unwrap();
}
