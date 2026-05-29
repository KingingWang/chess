//! TCP transport for LAN play: a thin, async, line-framed [`Connection`] plus
//! server/client constructors. No reconnection or matchmaking — just the basic
//! synchronous-feeling request/response a two-player game needs.

use std::net::SocketAddr;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};

use crate::protocol::Message;

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("connection closed by peer")]
    Closed,
    #[error("malformed message: {0}")]
    Decode(#[from] serde_json::Error),
}

/// A bidirectional, message-framed connection to one peer.
pub struct Connection {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    peer: SocketAddr,
    buf: String,
}

impl Connection {
    fn new(stream: TcpStream) -> std::io::Result<Connection> {
        stream.set_nodelay(true)?; // low-latency moves
        let peer = stream.peer_addr()?;
        let (r, w) = stream.into_split();
        Ok(Connection {
            reader: BufReader::new(r),
            writer: w,
            peer,
            buf: String::new(),
        })
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    /// Send one message (appends the framing newline and flushes).
    pub async fn send(&mut self, msg: &Message) -> Result<(), NetError> {
        let line = msg.to_line();
        self.writer.write_all(line.as_bytes()).await?;
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
        Ok(Message::from_line(self.buf.trim_end())?)
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

    /// Wait for one client to connect and return the connection.
    pub async fn accept_one(&self) -> Result<Connection, NetError> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Connection::new(stream)?)
    }
}

/// Connect to a host as the client/guest.
pub async fn connect(addr: impl tokio::net::ToSocketAddrs) -> Result<Connection, NetError> {
    let stream = TcpStream::connect(addr).await?;
    Ok(Connection::new(stream)?)
}
