//! TCP transport for LAN play: a thin, async, line-framed [`Connection`] plus
//! server/client constructors. No reconnection or matchmaking — just the basic
//! synchronous-feeling request/response a two-player game needs.
//!
//! Every frame is encrypted with a password-derived AEAD cipher (see
//! [`crate::crypto`]) and then base64-encoded so it still fits the simple
//! newline-delimited framing.

use std::net::SocketAddr;

use base64::Engine as _;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};

use crate::crypto::Cipher;
use crate::protocol::Message;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

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

/// A bidirectional, message-framed, encrypted connection to one peer.
pub struct Connection {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    peer: SocketAddr,
    cipher: Cipher,
    buf: String,
}

impl Connection {
    fn new(stream: TcpStream, cipher: Cipher) -> std::io::Result<Connection> {
        stream.set_nodelay(true)?; // low-latency moves
        let peer = stream.peer_addr()?;
        let (r, w) = stream.into_split();
        Ok(Connection {
            reader: BufReader::new(r),
            writer: w,
            peer,
            cipher,
            buf: String::new(),
        })
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    /// Send one message: serialize -> seal -> base64 -> newline-framed.
    pub async fn send(&mut self, msg: &Message) -> Result<(), NetError> {
        let line = msg.to_line();
        let sealed = self.cipher.seal(line.as_bytes());
        let encoded = B64.encode(&sealed);
        self.writer.write_all(encoded.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// Receive the next message, blocking until one arrives or the peer closes.
    pub async fn recv(&mut self) -> Result<Message, NetError> {
        self.buf.clear();
        let n = self.reader.read_line(&mut self.buf).await?;
        if n == 0 {
            return Err(NetError::Closed);
        }
        let sealed = B64
            .decode(self.buf.trim_end().as_bytes())
            .map_err(|_| NetError::Crypto)?;
        let plain = self.cipher.open(&sealed).ok_or(NetError::Crypto)?;
        let line = std::str::from_utf8(&plain).map_err(|_| NetError::Crypto)?;
        Ok(Message::from_line(line)?)
    }
}

/// Host a game: bind, then accept exactly one opponent.
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

    /// Wait for one client to connect and return the encrypted connection,
    /// keyed by `password`.
    pub async fn accept_one(&self, password: &str) -> Result<Connection, NetError> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Connection::new(stream, Cipher::from_password(password))?)
    }
}

/// Connect to a host as the client/guest, using `password` for the link key.
pub async fn connect(
    addr: impl tokio::net::ToSocketAddrs,
    password: &str,
) -> Result<Connection, NetError> {
    let stream = TcpStream::connect(addr).await?;
    Ok(Connection::new(stream, Cipher::from_password(password))?)
}
