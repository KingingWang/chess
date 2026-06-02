//! A thin session layer coordinating the handshake and color assignment on top
//! of a raw [`Connection`]. Move validation against the rules engine is left to
//! the application (which owns the authoritative [`chess_core::Game`]), but this
//! type provides the handshake and typed send helpers.
//!
//! The same handshake runs over both transports: a direct LAN socket
//! ([`Session::host`] / [`Session::join`]) and a relayed WSS tunnel
//! ([`Session::host_relay`] / [`Session::join_relay`]).

use chess_core::{Color, Move};

use crate::connection::{Connection, InboundEvent, NetError, Server};
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
    #[error("peer left during handshake (likely wrong password or aborted)")]
    PeerLeft,
}

impl Session {
    /// Run the host side of the color-assignment handshake over an established
    /// connection: announce the guest's color, then await its echo.
    pub async fn handshake_as_host(
        mut conn: Connection,
        host_color: Color,
        name: &str,
    ) -> Result<Session, HandshakeError> {
        host_handshake_on(&mut conn, host_color, name).await?;
        Ok(Session {
            conn,
            my_color: host_color,
            role: Role::Host,
        })
    }

    /// Run the guest side of the handshake: receive our assigned color, echo it.
    pub async fn handshake_as_guest(
        mut conn: Connection,
        name: &str,
    ) -> Result<Session, HandshakeError> {
        let my_color = guest_handshake_on(&mut conn, name).await?;
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

    /// Send the host's authoritative [`chess_core::Game`] to the guest. Used
    /// when a guest (re)connects so it picks up the move history in progress.
    pub async fn send_sync(&mut self, game: chess_core::Game) -> Result<(), NetError> {
        self.conn
            .send(&Message::Sync {
                game: Box::new(game),
            })
            .await
    }

    /// Await the next message from the peer.
    pub async fn recv(&mut self) -> Result<Message, NetError> {
        self.conn.recv().await
    }
}

/// Run the host side of the handshake on an already-established
/// [`Connection`] held by the caller. Lets a persistent host task run the
/// handshake against successive guests over the same underlying transport
/// (relay WS or a fresh LAN socket) without giving up ownership of the
/// connection on each iteration.
pub async fn host_handshake_on(
    conn: &mut Connection,
    host_color: Color,
    name: &str,
) -> Result<(), HandshakeError> {
    conn.send(&Message::Hello {
        version: PROTOCOL_VERSION,
        your_color: host_color.opponent(),
        name: name.to_string(),
    })
    .await?;

    match conn.recv_event().await? {
        InboundEvent::Message(Message::Hello { version, .. }) if version == PROTOCOL_VERSION => {}
        InboundEvent::Message(Message::Hello { version, .. }) => {
            return Err(HandshakeError::Version {
                local: PROTOCOL_VERSION,
                remote: version,
            })
        }
        InboundEvent::Message(other) => return Err(HandshakeError::Unexpected(other)),
        // Relay told us the would-be guest dropped before completing the
        // handshake (most often: wrong password → guest couldn't decrypt our
        // Hello and disconnected). The conn (our link to the server) is still
        // alive; the caller may loop back and wait for the next guest.
        InboundEvent::PeerLeft | InboundEvent::PeerJoined => return Err(HandshakeError::PeerLeft),
    }
    Ok(())
}

/// Run the guest side of the handshake on a borrowed [`Connection`], returning
/// the color assigned by the host on success.
pub async fn guest_handshake_on(
    conn: &mut Connection,
    name: &str,
) -> Result<Color, HandshakeError> {
    let my_color = match conn.recv_event().await? {
        InboundEvent::Message(Message::Hello {
            version,
            your_color,
            ..
        }) if version == PROTOCOL_VERSION => your_color,
        InboundEvent::Message(Message::Hello { version, .. }) => {
            return Err(HandshakeError::Version {
                local: PROTOCOL_VERSION,
                remote: version,
            })
        }
        InboundEvent::Message(other) => return Err(HandshakeError::Unexpected(other)),
        InboundEvent::PeerLeft | InboundEvent::PeerJoined => return Err(HandshakeError::PeerLeft),
    };

    conn.send(&Message::Hello {
        version: PROTOCOL_VERSION,
        your_color: my_color.opponent(),
        name: name.to_string(),
    })
    .await?;

    Ok(my_color)
}

/// Wait for the relay server to signal that a peer has joined the room
/// (skipping any stray `PeerLeft` or stale game frames left over from a
/// previous guest). Returns `Err(NetError::Closed)` if the connection dies
/// before that happens — a fatal condition for a persistent host loop.
pub async fn wait_for_peer_joined(conn: &mut Connection) -> Result<(), NetError> {
    loop {
        match conn.recv_event().await? {
            InboundEvent::PeerJoined => return Ok(()),
            // PeerLeft or any stale message from a defunct guest: ignore and
            // keep waiting for the next real arrival.
            InboundEvent::PeerLeft | InboundEvent::Message(_) => continue,
        }
    }
}
