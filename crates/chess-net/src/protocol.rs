//! Wire protocol for LAN play.
//!
//! Messages are newline-delimited JSON ("JSON Lines"). This keeps the protocol
//! human-readable and trivial to frame over TCP: one `\n`-terminated JSON
//! object per message. Only the basics required by the spec are modelled:
//! handshake, move, resign, draw offer/response.

use chess_core::{Color, Game, Move};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Full game-state snapshot pushed by the host so the guest matches the
    /// host's authoritative position. Sent automatically when a (re)connecting
    /// guest finishes its handshake, and whenever the host starts a new game
    /// while a guest is already connected. Boxed to keep [`Message`] small.
    Sync { game: Box<Game> },
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
            Message::Sync {
                game: Box::new(Game::new()),
            },
        ];
        for m in msgs {
            let line = m.to_line();
            assert!(!line.contains('\n'), "wire line must not contain newlines");
            // Game has no PartialEq, so compare via the canonical wire form.
            let round = Message::from_line(&line).unwrap();
            assert_eq!(round.to_line(), line, "roundtrip mismatch for {line}");
        }
    }

    #[test]
    fn sync_carries_game_state() {
        let mut g = Game::new();
        let mv = Move::from_iccs("b2e2").unwrap();
        g.make_move(mv).unwrap();
        let line = (Message::Sync {
            game: Box::new(g.clone()),
        })
        .to_line();
        match Message::from_line(&line).unwrap() {
            Message::Sync { game } => {
                assert_eq!(game.history_len(), 1);
                assert_eq!(game.side_to_move(), chess_core::Color::Black);
            }
            other => panic!("expected Sync, got {other:?}"),
        }
    }
}
