//! Message transport for two-player play: a thin, async, frame-based
//! [`Connection`] that runs over either a direct LAN TCP socket or a relayed
//! WebSocket-over-TLS tunnel. The two transports differ only in framing:
//!
//! * **LAN TCP** — base64 of the sealed frame, newline-delimited. A one-line
//!   plaintext salt prelude is exchanged first so both peers derive the same
//!   key (see [`crate::crypto`]).
//! * **Relay WSS** — one binary WebSocket message per sealed frame; the salt is
//!   exchanged out-of-band via the relay control protocol (see [`crate::relay`]).
//!
//! Either way, every frame is sealed with a password-derived AEAD cipher, so
//! the relay server only ever sees opaque ciphertext (end-to-end encryption).

use std::net::SocketAddr;

use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;

use crate::crypto::Cipher;
use crate::protocol::Message;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// The relay-side WebSocket stream type (client end of a TLS tunnel).
pub(crate) type WsClient = WebSocketStream<tokio_rustls::client::TlsStream<TcpStream>>;

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("connection closed by peer")]
    Closed,
    #[error("malformed message: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("decryption/authentication failed (wrong password?)")]
    Crypto,
}

/// What a single inbound poll yielded.
///
/// Game-data frames carry an end-to-end-encrypted [`Message`]; relay-only
/// control frames are exposed as their own variants so the host task can
/// react to a peer joining or leaving without tearing down the WebSocket.
#[derive(Debug)]
pub enum InboundEvent {
    /// A normal end-to-end-encrypted [`Message`] from the peer.
    Message(Message),
    /// Relay control: a new peer has joined the room and the next handshake
    /// should run. Never produced on a LAN (TCP) connection.
    PeerJoined,
    /// Relay control: the previously connected peer has disconnected. The
    /// caller may keep the connection alive and wait for the next
    /// [`InboundEvent::PeerJoined`].
    PeerLeft,
}

/// The underlying framed byte transport beneath the AEAD layer.
enum Wire {
    Tcp {
        reader: BufReader<OwnedReadHalf>,
        writer: OwnedWriteHalf,
        buf: String,
    },
    Ws(Box<WsClient>),
}

impl Wire {
    /// Send one already-sealed frame.
    async fn send_frame(&mut self, sealed: &[u8]) -> Result<(), NetError> {
        match self {
            Wire::Tcp { writer, .. } => {
                let encoded = B64.encode(sealed);
                writer.write_all(encoded.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
                Ok(())
            }
            Wire::Ws(ws) => {
                ws.send(WsMessage::Binary(sealed.to_vec().into()))
                    .await
                    .map_err(|_| NetError::Closed)?;
                Ok(())
            }
        }
    }

    /// Receive the next "raw" inbound: either a sealed binary frame or, on a
    /// relay WebSocket, a plaintext control text frame. LAN TCP always yields
    /// a binary frame.
    async fn recv_inbound(&mut self) -> Result<WireInbound, NetError> {
        match self {
            Wire::Tcp { reader, buf, .. } => {
                buf.clear();
                let n = reader.read_line(buf).await?;
                if n == 0 {
                    return Err(NetError::Closed);
                }
                let bytes = B64
                    .decode(buf.trim_end().as_bytes())
                    .map_err(|_| NetError::Crypto)?;
                Ok(WireInbound::Frame(bytes))
            }
            Wire::Ws(ws) => loop {
                match ws.next().await {
                    Some(Ok(m)) if m.is_binary() => {
                        return Ok(WireInbound::Frame(m.into_data().to_vec()))
                    }
                    Some(Ok(m)) if m.is_text() => {
                        let txt = m.into_text().map_err(|_| NetError::Closed)?;
                        return Ok(WireInbound::Text(txt.to_string()));
                    }
                    Some(Ok(m)) if m.is_close() => return Err(NetError::Closed),
                    Some(Ok(_)) => continue,
                    Some(Err(_)) => return Err(NetError::Closed),
                    None => return Err(NetError::Closed),
                }
            },
        }
    }
}

/// Output of [`Wire::recv_inbound`], private to this module.
enum WireInbound {
    /// A sealed binary frame ready to be decrypted into a [`Message`].
    Frame(Vec<u8>),
    /// A plaintext text frame (only ever produced by the relay control plane).
    Text(String),
}

/// A bidirectional, message-framed, end-to-end encrypted connection to one peer.
pub struct Connection {
    wire: Wire,
    cipher: Cipher,
    peer: Option<SocketAddr>,
}

impl Connection {
    /// Wrap an established relay WebSocket tunnel with the room cipher.
    pub(crate) fn from_ws(ws: WsClient, cipher: Cipher) -> Connection {
        Connection {
            wire: Wire::Ws(Box::new(ws)),
            cipher,
            peer: None,
        }
    }

    /// The peer's socket address, if known (LAN only; `None` over a relay).
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer
    }

    /// Send one message: serialize -> seal -> framed transport.
    pub async fn send(&mut self, msg: &Message) -> Result<(), NetError> {
        let sealed = self.cipher.seal(msg.to_line().as_bytes());
        self.wire.send_frame(&sealed).await
    }

    /// Receive the next message, blocking until one arrives or the peer closes.
    ///
    /// Relay control frames (peer join/leave) are silently skipped here; use
    /// [`Connection::recv_event`] to observe them instead.
    pub async fn recv(&mut self) -> Result<Message, NetError> {
        loop {
            match self.recv_event().await? {
                InboundEvent::Message(m) => return Ok(m),
                InboundEvent::PeerJoined | InboundEvent::PeerLeft => continue,
            }
        }
    }

    /// Receive the next inbound, including relay control signals. On a LAN
    /// (TCP) connection only [`InboundEvent::Message`] is ever produced.
    pub async fn recv_event(&mut self) -> Result<InboundEvent, NetError> {
        loop {
            match self.wire.recv_inbound().await? {
                WireInbound::Frame(sealed) => {
                    let plain = self.cipher.open(&sealed).ok_or(NetError::Crypto)?;
                    let line = std::str::from_utf8(&plain).map_err(|_| NetError::Crypto)?;
                    return Ok(InboundEvent::Message(Message::from_line(line)?));
                }
                WireInbound::Text(txt) => {
                    use crate::relay::ControlMsg;
                    match serde_json::from_str::<ControlMsg>(&txt) {
                        Ok(ControlMsg::PeerJoined) => return Ok(InboundEvent::PeerJoined),
                        Ok(ControlMsg::PeerLeft) => return Ok(InboundEvent::PeerLeft),
                        Ok(_) | Err(_) => continue,
                    }
                }
            }
        }
    }
}

/// Host a LAN game: bind, then accept exactly one opponent.
pub struct Server {
    listener: TcpListener,
}

impl Server {
    /// Bind to `addr` (e.g. `"0.0.0.0:9696"`).
    pub async fn bind(addr: impl tokio::net::ToSocketAddrs) -> Result<Server, NetError> {
        Ok(Server {
            listener: TcpListener::bind(addr).await?,
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Wait for one client to connect and return the encrypted connection. The
    /// host generates a fresh salt and sends it as a plaintext prelude before
    /// any sealed frame, so the guest can derive the matching key.
    pub async fn accept_one(&self, password: &str) -> Result<Connection, NetError> {
        let (stream, _) = self.listener.accept().await?;
        stream.set_nodelay(true)?;
        let peer = stream.peer_addr().ok();
        let salt = crate::crypto::random_salt();
        let (r, w) = stream.into_split();
        let mut writer = w;
        writer.write_all(B64.encode(salt).as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        let cipher = Cipher::from_password_salt(password, &salt);
        Ok(Connection {
            wire: Wire::Tcp {
                reader: BufReader::new(r),
                writer,
                buf: String::new(),
            },
            cipher,
            peer,
        })
    }
}

/// Connect to a LAN host as the guest, using `password` for the link key. The
/// guest reads the plaintext salt prelude first, then derives the room key.
pub async fn connect(
    addr: impl tokio::net::ToSocketAddrs,
    password: &str,
) -> Result<Connection, NetError> {
    let stream = TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    let peer = stream.peer_addr().ok();
    let (r, w) = stream.into_split();
    let mut reader = BufReader::new(r);
    let mut line = String::new();
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Err(NetError::Closed);
    }
    let salt = B64
        .decode(line.trim_end().as_bytes())
        .map_err(|_| NetError::Crypto)?;
    let cipher = Cipher::from_password_salt(password, &salt);
    Ok(Connection {
        wire: Wire::Tcp {
            reader,
            writer: w,
            buf: String::new(),
        },
        cipher,
        peer,
    })
}
