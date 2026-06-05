//! Game save/load in a simple ICCS-based text format.
//!
//! The format stores game metadata (players, date, result) plus the sequence
//! of moves in ICCS notation, one pair per line. Example:
//!
//! ```text
//! [Event "Casual Game"]
//! [Date "2026.06.05"]
//! [Red "Player"]
//! [Black "AI (Hard)"]
//! [Result "1-0"]
//! [Mode "VsAi"]
//! [Time "3:25"]
//!
//! 1. h2e2 h9g7
//! 2. b2e2 b9c7
//! ```
//!
//! This is inspired by PGN but uses ICCS coordinates (the standard for
//! Xiangqi software interop). The parser is lenient and skips unknown tags.

use crate::board::Board;
use crate::fen::START_FEN;
use crate::game::Game;
use crate::moves::Move;

/// Metadata for a saved game.
#[derive(Debug, Clone, Default)]
pub struct GameRecord {
    pub event: String,
    pub date: String,
    pub red_player: String,
    pub black_player: String,
    pub result: String,
    pub fen: String,
    /// Game mode identifier (e.g. "VsAi", "LocalPvp").
    pub mode: String,
    /// Game duration formatted as "M:SS".
    pub time: String,
    pub moves: Vec<Move>,
}

impl GameRecord {
    /// Create a record from a live game.
    pub fn from_game(game: &Game) -> Self {
        let result_str = match game.result() {
            Some(crate::game::GameResult::Win { winner, .. }) => match winner {
                crate::piece::Color::Red => "1-0".to_string(),
                crate::piece::Color::Black => "0-1".to_string(),
            },
            Some(crate::game::GameResult::Draw(_)) => "1/2-1/2".to_string(),
            None => "*".to_string(),
        };

        GameRecord {
            event: "Xiangqi Game".to_string(),
            date: String::new(),
            red_player: "Red".to_string(),
            black_player: "Black".to_string(),
            result: result_str,
            fen: START_FEN.to_string(),
            mode: String::new(),
            time: String::new(),
            moves: game.played_moves().collect(),
        }
    }

    /// Serialize to the text format.
    pub fn serialize(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("[Event \"{}\"]\n", self.event));
        if !self.date.is_empty() {
            s.push_str(&format!("[Date \"{}\"]\n", self.date));
        }
        s.push_str(&format!("[Red \"{}\"]\n", self.red_player));
        s.push_str(&format!("[Black \"{}\"]\n", self.black_player));
        s.push_str(&format!("[Result \"{}\"]\n", self.result));
        if self.fen != START_FEN {
            s.push_str(&format!("[FEN \"{}\"]\n", self.fen));
        }
        if !self.mode.is_empty() {
            s.push_str(&format!("[Mode \"{}\"]\n", self.mode));
        }
        if !self.time.is_empty() {
            s.push_str(&format!("[Time \"{}\"]\n", self.time));
        }
        s.push_str(&format!("[PlyCount \"{}\"]\n", self.moves.len()));
        s.push('\n');

        for (i, mv) in self.moves.iter().enumerate() {
            if i % 2 == 0 {
                if i > 0 {
                    s.push('\n');
                }
                s.push_str(&format!("{}. ", i / 2 + 1));
            } else {
                s.push(' ');
            }
            s.push_str(&mv.to_iccs());
        }

        if !self.moves.is_empty() {
            s.push('\n');
        }
        s.push_str(&self.result);
        s.push('\n');
        s
    }

    /// Parse from the text format.
    pub fn parse_record(input: &str) -> Result<Self, String> {
        let mut record = GameRecord {
            fen: START_FEN.to_string(),
            result: "*".to_string(),
            ..GameRecord::default()
        };

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                // Parse tag pair.
                let inner = &trimmed[1..trimmed.len() - 1];
                if let Some((key, value)) = parse_tag(inner) {
                    match key.to_lowercase().as_str() {
                        "event" => record.event = value,
                        "date" => record.date = value,
                        "red" => record.red_player = value,
                        "black" => record.black_player = value,
                        "result" => record.result = value,
                        "fen" => record.fen = value,
                        "mode" => record.mode = value,
                        "time" => record.time = value,
                        _ => {} // Skip unknown tags (e.g. PlyCount).
                    }
                }
            } else if !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !["1-0", "0-1", "1/2-1/2", "*"].contains(&trimmed)
            {
                // Parse move line: "1. h2e2 h9g7"
                for token in trimmed.split_whitespace() {
                    // Skip move numbers like "1." or "12."
                    if token.ends_with('.') {
                        continue;
                    }
                    if let Some(mv) = Move::from_iccs(token) {
                        record.moves.push(mv);
                    }
                }
            }
        }

        Ok(record)
    }

    /// Replay the moves from the record into a Game.
    pub fn to_game(&self) -> Result<Game, String> {
        let board: Board = self.fen.parse().map_err(|e| format!("invalid FEN: {e}"))?;
        let mut game = Game::from_board(board);
        for (i, mv) in self.moves.iter().enumerate() {
            game.make_move(*mv)
                .map_err(|e| format!("illegal move {} at ply {}: {e}", mv.to_iccs(), i))?;
        }
        Ok(game)
    }
}

/// Parse a tag pair like `Event "Casual Game"` into (key, value).
fn parse_tag(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    let space = s.find(' ')?;
    let key = s[..space].to_string();
    let rest = s[space..].trim();
    // Remove surrounding quotes.
    if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
        Some((key, rest[1..rest.len() - 1].to_string()))
    } else {
        Some((key, rest.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Game, Move, START_FEN};

    #[test]
    fn roundtrip_simple_game() {
        let mut g = Game::new();
        g.make_move(Move::from_iccs("h2e2").unwrap()).unwrap();
        g.make_move(Move::from_iccs("h9g7").unwrap()).unwrap();
        g.make_move(Move::from_iccs("b0c2").unwrap()).unwrap();

        let record = GameRecord::from_game(&g);
        let text = record.serialize();

        let parsed = GameRecord::parse_record(&text).unwrap();
        assert_eq!(parsed.moves.len(), 3);
        assert_eq!(parsed.moves[0], Move::from_iccs("h2e2").unwrap());
        assert_eq!(parsed.moves[1], Move::from_iccs("h9g7").unwrap());
        assert_eq!(parsed.moves[2], Move::from_iccs("b0c2").unwrap());
    }

    #[test]
    fn to_game_replays_correctly() {
        let mut g = Game::new();
        g.make_move(Move::from_iccs("h2e2").unwrap()).unwrap();
        g.make_move(Move::from_iccs("h9g7").unwrap()).unwrap();

        let record = GameRecord::from_game(&g);
        let replayed = record.to_game().unwrap();
        assert_eq!(replayed.board().to_fen(), g.board().to_fen());
    }

    #[test]
    fn parse_preserves_metadata() {
        let text = r#"[Event "Tournament Match"]
[Date "2026.06.05"]
[Red "Player1"]
[Black "Player2"]
[Result "1-0"]

1. h2e2 h9g7
1-0
"#;
        let record = GameRecord::parse_record(text).unwrap();
        assert_eq!(record.event, "Tournament Match");
        assert_eq!(record.red_player, "Player1");
        assert_eq!(record.black_player, "Player2");
        assert_eq!(record.result, "1-0");
        assert_eq!(record.moves.len(), 2);
    }

    #[test]
    fn empty_game_serializes() {
        let g = Game::new();
        let record = GameRecord::from_game(&g);
        let text = record.serialize();
        assert!(text.contains("[Event"));
        assert!(text.contains("*")); // ongoing game
    }

    #[test]
    fn mode_and_time_tags_roundtrip() {
        let mut record = GameRecord::default();
        record.event = "人机对战 · 困难".to_string();
        record.mode = "VsAi".to_string();
        record.time = "3:25".to_string();
        record.red_player = "玩家".to_string();
        record.black_player = "AI (困难)".to_string();
        record.moves.push(Move::from_iccs("h2e2").unwrap());

        let text = record.serialize();
        assert!(text.contains("[Mode \"VsAi\"]"));
        assert!(text.contains("[Time \"3:25\"]"));
        assert!(text.contains("[PlyCount \"1\"]"));

        let parsed = GameRecord::parse_record(&text).unwrap();
        assert_eq!(parsed.mode, "VsAi");
        assert_eq!(parsed.time, "3:25");
        assert_eq!(parsed.red_player, "玩家");
        assert_eq!(parsed.black_player, "AI (困难)");
    }

    #[test]
    fn unknown_tags_are_skipped() {
        let text = r#"[Event "Test"]
[Red "R"]
[Black "B"]
[Result "*"]
[UnknownTag "whatever"]
[PlyCount "0"]

*
"#;
        let record = GameRecord::parse_record(text).unwrap();
        assert_eq!(record.event, "Test");
        assert!(record.moves.is_empty());
    }
}
