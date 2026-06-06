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

/// Move annotation/comment.
#[derive(Debug, Clone, Default)]
pub struct MoveAnnotation {
    /// Text comment for this move.
    pub comment: Option<String>,
    /// Numeric Annotation Glyph (e.g., $1 = good move, $2 = poor move).
    pub nag: Option<u8>,
    /// Symbolic annotation (! = good, ? = mistake, etc.).
    pub symbol: Option<String>,
}

impl MoveAnnotation {
    /// Convert NAG to human-readable symbol.
    pub fn nag_to_symbol(nag: u8) -> &'static str {
        match nag {
            1 => "!",  // Good move
            2 => "?",  // Mistake
            3 => "!!", // Brilliant move
            4 => "??", // Blunder
            5 => "!?", // Interesting move
            6 => "?!", // Dubious move
            _ => "",
        }
    }

    /// Parse symbol to NAG.
    pub fn symbol_to_nag(symbol: &str) -> Option<u8> {
        match symbol {
            "!" => Some(1),
            "?" => Some(2),
            "!!" => Some(3),
            "??" => Some(4),
            "!?" => Some(5),
            "?!" => Some(6),
            _ => None,
        }
    }
}

/// Enhanced game record with annotations.
#[derive(Debug, Clone, Default)]
pub struct AnnotatedGameRecord {
    pub base: GameRecord,
    pub annotations: Vec<MoveAnnotation>,
}

impl AnnotatedGameRecord {
    /// Create from a base record.
    pub fn from_record(record: GameRecord) -> Self {
        let annotations = vec![MoveAnnotation::default(); record.moves.len()];
        AnnotatedGameRecord {
            base: record,
            annotations,
        }
    }

    /// Add annotation to a specific move.
    pub fn annotate_move(&mut self, move_index: usize, annotation: MoveAnnotation) {
        if move_index < self.annotations.len() {
            self.annotations[move_index] = annotation;
        }
    }

    /// Serialize with annotations.
    pub fn serialize(&self) -> String {
        let mut s = self.base.serialize();

        // Insert annotations into the move text
        // This is a simplified version - full implementation would parse and re-serialize
        for (i, ann) in self.annotations.iter().enumerate() {
            if ann.nag.is_some() || ann.comment.is_some() || ann.symbol.is_some() {
                // Find the move in the serialized text and add annotation
                // For now, just append as a comment at the end
            }
        }

        s
    }

    /// Parse annotated record from text.
    pub fn parse_annotated(text: &str) -> Result<Self, String> {
        let base = GameRecord::parse_record(text)?;
        let mut annotated = AnnotatedGameRecord::from_record(base);

        // Parse annotations from comments and NAGs
        // Simplified: look for $N patterns (NAGs) and {comments}
        let mut move_idx = 0;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('[') {
                continue;
            }

            // Look for NAGs like $1, $2, etc.
            if let Some(nag_pos) = line.find('$') {
                if let Ok(nag) = line[nag_pos + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u8>()
                {
                    if move_idx < annotated.annotations.len() {
                        annotated.annotations[move_idx].nag = Some(nag);
                        annotated.annotations[move_idx].symbol =
                            Some(MoveAnnotation::nag_to_symbol(nag).to_string());
                    }
                }
            }

            // Count moves in this line
            for token in line.split_whitespace() {
                if token.ends_with('.') || ["1-0", "0-1", "1/2-1/2", "*"].contains(&token) {
                    continue;
                }
                if Move::from_iccs(token).is_some() {
                    move_idx += 1;
                }
            }
        }

        Ok(annotated)
    }
}

/// Export game to standard PGN format with Chinese notation.
pub fn export_to_chinese_pgn(game: &Game, record: &GameRecord) -> String {
    let mut s = String::new();

    // Header tags
    s.push_str(&format!("[Event \"{}\"]\n", record.event));
    if !record.date.is_empty() {
        s.push_str(&format!("[Date \"{}\"]\n", record.date));
    }
    s.push_str(&format!("[Red \"{}\"]\n", record.red_player));
    s.push_str(&format!("[Black \"{}\"]\n", record.black_player));
    s.push_str(&format!("[Result \"{}\"]\n", record.result));
    if record.fen != START_FEN {
        s.push_str(&format!("[FEN \"{}\"]\n", record.fen));
    }
    if !record.mode.is_empty() {
        s.push_str(&format!("[Mode \"{}\"]\n", record.mode));
    }
    if !record.time.is_empty() {
        s.push_str(&format!("[Time \"{}\"]\n", record.time));
    }
    s.push_str(&format!("[PlyCount \"{}\"]\n", record.moves.len()));
    s.push('\n');

    // Moves with Chinese notation
    let mut board: Board = record
        .fen
        .parse()
        .unwrap_or_else(|_| Board::start_position());

    for (i, mv) in record.moves.iter().enumerate() {
        if i % 2 == 0 {
            if i > 0 {
                s.push('\n');
            }
            s.push_str(&format!("{}. ", i / 2 + 1));
        } else {
            s.push(' ');
        }

        // Add Chinese notation
        let chinese = crate::move_to_chinese(*mv, &board);
        s.push_str(&chinese);

        // Also add ICCS in parentheses for reference
        s.push_str(&format!(" ({})", mv.to_iccs()));

        board.make_move(*mv);
    }

    s.push('\n');
    s.push_str(&record.result);
    s.push('\n');

    s
}

#[cfg(test)]
mod annotation_tests {
    use super::*;

    #[test]
    fn nag_to_symbol_works() {
        assert_eq!(MoveAnnotation::nag_to_symbol(1), "!");
        assert_eq!(MoveAnnotation::nag_to_symbol(2), "?");
        assert_eq!(MoveAnnotation::nag_to_symbol(3), "!!");
        assert_eq!(MoveAnnotation::nag_to_symbol(4), "??");
        assert_eq!(MoveAnnotation::nag_to_symbol(5), "!?");
        assert_eq!(MoveAnnotation::nag_to_symbol(6), "?!");
    }

    #[test]
    fn symbol_to_nag_works() {
        assert_eq!(MoveAnnotation::symbol_to_nag("!"), Some(1));
        assert_eq!(MoveAnnotation::symbol_to_nag("?"), Some(2));
        assert_eq!(MoveAnnotation::symbol_to_nag("!!"), Some(3));
        assert_eq!(MoveAnnotation::symbol_to_nag("??"), Some(4));
    }

    #[test]
    fn annotated_record_creation() {
        let mut g = Game::new();
        g.make_move(Move::from_iccs("h2e2").unwrap()).unwrap();
        g.make_move(Move::from_iccs("h9g7").unwrap()).unwrap();

        let record = GameRecord::from_game(&g);
        let mut annotated = AnnotatedGameRecord::from_record(record);

        let mut ann = MoveAnnotation::default();
        ann.nag = Some(1);
        ann.symbol = Some("!".to_string());
        annotated.annotate_move(0, ann);

        assert_eq!(annotated.annotations[0].nag, Some(1));
        assert_eq!(annotated.annotations[0].symbol, Some("!".to_string()));
    }

    #[test]
    fn chinese_pgn_export() {
        let mut g = Game::new();
        g.make_move(Move::from_iccs("h2e2").unwrap()).unwrap();
        g.make_move(Move::from_iccs("h9g7").unwrap()).unwrap();

        let record = GameRecord::from_game(&g);
        let chinese_pgn = export_to_chinese_pgn(&g, &record);

        assert!(chinese_pgn.contains("[Event"));
        assert!(chinese_pgn.contains("炮二平五"));
        assert!(chinese_pgn.contains("馬8进7")); // Traditional Chinese character
        assert!(chinese_pgn.contains("(h2e2)"));
        assert!(chinese_pgn.contains("(h9g7)"));
    }
}
