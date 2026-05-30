//! A thin session layer coordinating the handshake and color assignment on top
//! of a raw [`Connection`]. Move validation against the rules engine is left to
//! the application (which owns the authoritative [`chess_core::Game`]), but this
//! type provides the handshake and typed send helpers.

use chess_core::{Color, Move};

use crate::connection::{Connection, NetError, Server};
use crate::protocol::{Message, PROTOCOL_VERSION};

/// Identifies which side of the network link we are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Hosts the game and assigns colors (defaults to playing Red).
    Host,
    /// Joins a hosted game.
    Guest,
}

/// A negotiated session: a connection plus the color we play.
pub struct Session {
    pub conn: Connection,
    pub my_color: Color,
    pub role: Role,
}

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("protocol version mismatch: local {local}, remote {remote}")]
    Version { local: u32, remote: u32 },
    #[error("unexpected message during handshake: {0:?}")]
    Unexpected(Message),
}

impl Session {
    /// Host a game on `addr`, accept one guest, and assign colors. The host
    /// plays `host_color` (Red by convention) and tells the guest the opposite.
    pub async fn host(
        addr: impl tokio::net::ToSocketAddrs,
        host_color: Color,
        name: &str,
        password: &str,
    ) -> Result<Session, HandshakeError> {
        let server = Server::bind(addr).await?;
        let mut conn = server.accept_one(password).await?;

        // Tell the guest its color.
        conn.send(&Message::Hello {
            version: PROTOCOL_VERSION,
            your_color: host_color.opponent(),
            name: name.to_string(),
        })
        .await?;

        // Expect the guest's Hello back.
        match conn.recv().await? {
            Message::Hello { version, .. } if version == PROTOCOL_VERSION => {}
            Message::Hello { version, .. } => {
                return Err(HandshakeError::Version {
                    local: PROTOCOL_VERSION,
                    remote: version,
                })
            }
            other => return Err(HandshakeError::Unexpected(other)),
        }

        Ok(Session {
            conn,
            my_color: host_color,
            role: Role::Host,
        })
    }

    /// Join a hosted game; the host assigns our color.
    pub async fn join(
        addr: impl tokio::net::ToSocketAddrs,
        name: &str,
        password: &str,
    ) -> Result<Session, HandshakeError> {
        let mut conn = crate::connection::connect(addr, password).await?;

        let my_color = match conn.recv().await? {
            Message::Hello {
                version,
                your_color,
                ..
            } if version == PROTOCOL_VERSION => your_color,
            Message::Hello { version, .. } => {
                return Err(HandshakeError::Version {
                    local: PROTOCOL_VERSION,
                    remote: version,
                })
            }
            other => return Err(HandshakeError::Unexpected(other)),
        };

        conn.send(&Message::Hello {
            version: PROTOCOL_VERSION,
            your_color: my_color.opponent(),
            name: name.to_string(),
        })
        .await?;

        Ok(Session {
            conn,
            my_color,
            role: Role::Guest,
        })
    }

    pub async fn send_move(&mut self, mv: Move) -> Result<(), NetError> {
        self.conn.send(&Message::Move { mv: mv.into() }).await
    }

    pub async fn resign(&mut self) -> Result<(), NetError> {
        self.conn.send(&Message::Resign).await
    }

    pub async fn offer_draw(&mut self) -> Result<(), NetError> {
        self.conn.send(&Message::DrawOffer).await
    }

    pub async fn respond_draw(&mut self, accept: bool) -> Result<(), NetError> {
        self.conn.send(&Message::DrawResponse { accept }).await
    }

    /// Await the next message from the peer.
    pub async fn recv(&mut self) -> Result<Message, NetError> {
        self.conn.recv().await
    }
}
