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
