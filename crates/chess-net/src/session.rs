//! A thin session layer coordinating the handshake and color assignment on top
//! of a raw [`Connection`]. Move validation against the rules engine is left to
//! the application (which owns the authoritative [`chess_core::Game`]), but this
//! type provides the handshake and typed send helpers.
//!
//! The same handshake runs over both transports: a direct LAN socket
//! ([`Session::host`] / [`Session::join`]) and a relayed WSS tunnel
//! ([`Session::host_relay`] / [`Session::join_relay`]).

use chess_core::{Color, Move};

use crate::connection::{Connection, NetError, Server};
use crate::protocol::{Message, PROTOCOL_VERSION};
use crate::relay::{self, PendingRelayHost, RelayClientConfig};

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
    #[error(transparent)]
    Relay(#[from] relay::RelayError),
    #[error("protocol version mismatch: local {local}, remote {remote}")]
    Version { local: u32, remote: u32 },
    #[error("unexpected message during handshake: {0:?}")]
    Unexpected(Message),
    #[error("timed out while connecting")]
    Timeout,
}

impl Session {
    /// Run the host side of the color-assignment handshake over an established
    /// connection: announce the guest's color, then await its echo.
    pub async fn handshake_as_host(
        mut conn: Connection,
        host_color: Color,
        name: &str,
    ) -> Result<Session, HandshakeError> {
        conn.send(&Message::Hello {
            version: PROTOCOL_VERSION,
            your_color: host_color.opponent(),
            name: name.to_string(),
        })
        .await?;

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

    /// Run the guest side of the handshake: receive our assigned color, echo it.
    pub async fn handshake_as_guest(mut conn: Connection, name: &str) -> Result<Session, HandshakeError> {
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

    /// Host a LAN game on `addr`, accept one guest, and assign colors. The host
    /// plays `host_color` (Red by convention) and tells the guest the opposite.
    pub async fn host(
        addr: impl tokio::net::ToSocketAddrs,
        host_color: Color,
        name: &str,
        password: &str,
    ) -> Result<Session, HandshakeError> {
        let server = Server::bind(addr).await?;
        let conn = server.accept_one(password).await?;
        Self::handshake_as_host(conn, host_color, name).await
    }

    /// Join a hosted LAN game; the host assigns our color.
    pub async fn join(
        addr: impl tokio::net::ToSocketAddrs,
        name: &str,
        password: &str,
    ) -> Result<Session, HandshakeError> {
        let conn = crate::connection::connect(addr, password).await?;
        Self::handshake_as_guest(conn, name).await
    }

    /// Create a relayed (internet) room. Returns a [`PendingRelayHost`] whose
    /// `room` number can be shared immediately; call
    /// [`PendingRelayHost::await_guest`] then [`Session::handshake_as_host`] to
    /// complete the connection once a guest joins.
    pub async fn relay_create(
        cfg: &RelayClientConfig,
        password: &str,
    ) -> Result<PendingRelayHost, HandshakeError> {
        Ok(relay::relay_create(cfg, password).await?)
    }

    /// Join a relayed (internet) game by room number and password.
    pub async fn join_relay(
        cfg: &RelayClientConfig,
        room: &str,
        name: &str,
        password: &str,
    ) -> Result<Session, HandshakeError> {
        let conn = relay::relay_join(cfg, room, password).await?;
        Self::handshake_as_guest(conn, name).await
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
