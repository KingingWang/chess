//! `chess-net` — minimal LAN networking for two-player Xiangqi over TCP.
//!
//! Layers:
//! * [`protocol`] — newline-delimited JSON [`Message`]s (move / resign / draw).
//! * [`connection`] — async, line-framed [`Connection`] with `Server`/`connect`.
//! * [`session`] — handshake + color assignment producing a [`Session`].
//!
//! The application owns the authoritative [`chess_core::Game`] and validates
//! every received move against it, so a malformed or illegal peer move is
//! rejected locally rather than trusted. Reconnection and matchmaking are
//! intentionally out of scope per the spec.

pub mod connection;
pub mod crypto;
pub mod protocol;
pub mod relay;
pub mod session;

pub use connection::InboundEvent;
pub use connection::{connect, Connection, NetError, Server};
pub use crypto::Cipher;
pub use protocol::{Message, WireMove, PROTOCOL_VERSION};
pub use relay::{
    ControlMsg, PendingRelayHost, RelayClientConfig, RelayError, DEFAULT_RELAY_HOST,
    DEFAULT_RELAY_PORT,
};
pub use session::{
    guest_handshake_on, host_handshake_on, wait_for_peer_joined, HandshakeError, Role, Session,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::{Color, Game, Move};

    #[tokio::test]
    async fn full_handshake_and_move_sync_over_tcp() {
        // Bind to an ephemeral port on loopback.
        let server = Server::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        // Host side runs concurrently with the guest.
        let host = tokio::spawn(async move {
            let mut conn = server.accept_one("pw").await.unwrap();
            conn.send(&Message::Hello {
                version: PROTOCOL_VERSION,
                your_color: Color::Black,
                name: "host".into(),
            })
            .await
            .unwrap();
            // guest hello
            let _ = conn.recv().await.unwrap();
            // Host (Red) plays first.
            let mut game = Game::new();
            let mv = Move::from_iccs("h2e2").unwrap();
            game.make_move(mv).unwrap();
            conn.send(&Message::Move { mv: mv.into() }).await.unwrap();
            // Receive guest's reply move and validate it.
            match conn.recv().await.unwrap() {
                Message::Move { mv } => {
                    let m = mv.parse().unwrap();
                    assert!(game.make_move(m).is_ok(), "peer move must be legal");
                }
                other => panic!("unexpected: {other:?}"),
            }
        });

        // Guest side.
        let mut session = Session::join(addr, "guest", "pw").await.unwrap();
        assert_eq!(session.my_color, Color::Black);

        let mut game = Game::new();
        // Receive host's move and apply.
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

        host.await.unwrap();
    }

    #[tokio::test]
    async fn join_with_wrong_password_fails_handshake() {
        let server = Server::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        let host = tokio::spawn(async move {
            // Host seals its Hello with the correct password.
            let mut conn = server.accept_one("right-pw").await.unwrap();
            let _ = conn
                .send(&Message::Hello {
                    version: PROTOCOL_VERSION,
                    your_color: Color::Black,
                    name: "host".into(),
                })
                .await;
        });

        // The guest uses the WRONG password, so it cannot decrypt the very
        // first handshake frame and the join fails.
        let res = Session::join(addr, "guest", "wrong-pw").await;
        assert!(res.is_err(), "a wrong password must fail the handshake");
        let _ = host.await;
    }

    #[tokio::test]
    async fn session_host_join_assigns_colors() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let host = tokio::spawn(async move {
            // Bind first so the port is known, then hand it to the guest.
            let server = Server::bind("127.0.0.1:0").await.unwrap();
            let addr = server.local_addr().unwrap();
            tx.send(addr).unwrap();
            let mut conn = server.accept_one("pw").await.unwrap();
            conn.send(&Message::Hello {
                version: PROTOCOL_VERSION,
                your_color: Color::Black,
                name: "host".into(),
            })
            .await
            .unwrap();
            match conn.recv().await.unwrap() {
                Message::Hello { .. } => Color::Red,
                _ => panic!(),
            }
        });
        let addr = rx.await.unwrap();
        let session = Session::join(addr, "guest", "pw").await.unwrap();
        assert_eq!(session.my_color, Color::Black);
        assert_eq!(host.await.unwrap(), Color::Red);
    }
}
