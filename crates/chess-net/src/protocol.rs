//! Wire protocol for LAN play.
//!
//! Messages are newline-delimited JSON ("JSON Lines"). This keeps the protocol
//! human-readable and trivial to frame over TCP: one `\n`-terminated JSON
//! object per message. Only the basics required by the spec are modelled:
//! handshake, move, resign, draw offer/response.

use chess_core::{Color, Move};
use serde::{Deserialize, Serialize};

/// Protocol version for a minimal compatibility check during handshake.
pub const PROTOCOL_VERSION: u32 = 1;

/// A single move encoded as its ICCS string (e.g. `"h2e2"`), so the wire format
/// is independent of internal square indexing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WireMove(pub String);

impl From<Move> for WireMove {
    fn from(m: Move) -> Self {
        WireMove(m.to_iccs())
    }
}

impl WireMove {
    pub fn parse(&self) -> Option<Move> {
        Move::from_iccs(&self.0)
    }
}

/// Messages exchanged between the two peers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// Sent by both sides on connect. The server assigns colors; the client
    /// echoes to confirm. `version` must match [`PROTOCOL_VERSION`].
    Hello {
        version: u32,
        /// The color the recipient will play (server -> client). Ignored when
        /// sent client -> server.
        your_color: Color,
        /// Free-form display name.
        name: String,
    },
    /// A played move. The receiver must validate it against its own board.
    Move { mv: WireMove },
    /// The sender resigns; the opponent wins.
    Resign,
    /// The sender offers a draw.
    DrawOffer,
    /// Response to a [`Message::DrawOffer`].
    DrawResponse { accept: bool },
    /// Application-level keepalive / no-op (optional).
    Ping,
}

impl Message {
    /// Serialize to a single JSON line (no trailing newline).
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("Message is always serializable")
    }

    /// Parse one JSON line.
    pub fn from_line(line: &str) -> Result<Message, serde_json::Error> {
        serde_json::from_str(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_roundtrips_through_wire() {
        let m = Move::from_iccs("h2e2").unwrap();
        let wire = WireMove::from(m);
        assert_eq!(wire.parse(), Some(m));
    }

    #[test]
    fn message_json_roundtrip() {
        let msgs = vec![
            Message::Hello {
                version: PROTOCOL_VERSION,
                your_color: Color::Black,
                name: "guest".into(),
            },
            Message::Move {
                mv: WireMove("b0c2".into()),
            },
            Message::Resign,
            Message::DrawOffer,
            Message::DrawResponse { accept: true },
        ];
        for m in msgs {
            let line = m.to_line();
            assert!(!line.contains('\n'));
            assert_eq!(Message::from_line(&line).unwrap(), m);
        }
    }
}
