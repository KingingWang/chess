//! Application-level states, modes, and shared resources.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, Game};

/// Top-level UI flow.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    InGame,
}

/// How the current game is being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    /// Two humans sharing one screen.
    LocalPvp,
    /// Human (Red) vs. the AI (Black).
    VsAi,
    /// LAN host (authoritative listener).
    LanHost,
    /// LAN guest.
    LanJoin,
    /// Internet host via the relay server (creates a room).
    RelayHost,
    /// Internet guest via the relay server (joins by room number).
    RelayJoin,
}

/// Selected difficulty for [`GameMode::VsAi`].
#[derive(Resource, Debug, Clone)]
pub struct AiSettings {
    pub difficulty: chess_ai::Difficulty,
    /// Optional path to an external UCI engine (Pikafish). `None` => built-in.
    pub engine_path: Option<std::path::PathBuf>,
    /// Optional NNUE file for Pikafish.
    pub eval_file: Option<std::path::PathBuf>,
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            difficulty: chess_ai::Difficulty::Hard,
            engine_path: std::env::var_os("PIKAFISH_PATH").map(Into::into),
            eval_file: std::env::var_os("PIKAFISH_EVAL").map(Into::into),
        }
    }
}

/// The authoritative game state shared by all systems.
#[derive(Resource)]
pub struct CoreGame {
    pub game: Game,
    pub mode: GameMode,
    /// In networked / AI games, the local human's color.
    pub local_color: ChessColor,
    /// True while an unanswered draw offer from the peer is pending.
    pub draw_offer_from_peer: bool,
    /// Relay room number to display (host: assigned; guest: the one joined).
    pub room_code: Option<String>,
    /// True while a networked game is still waiting for the peer to connect.
    pub awaiting_peer: bool,
    /// True once a networked session has actually connected to the peer.
    /// Used to distinguish a failed connect (→ back to menu) from a mid-game drop.
    pub connected: bool,
}

impl Default for CoreGame {
    fn default() -> Self {
        CoreGame {
            game: Game::new(),
            mode: GameMode::LocalPvp,
            local_color: ChessColor::Red,
            draw_offer_from_peer: false,
            room_code: None,
            awaiting_peer: false,
            connected: false,
        }
    }
}

impl CoreGame {
    /// May the local player move the side that is currently to move?
    pub fn local_to_move(&self) -> bool {
        match self.mode {
            GameMode::LocalPvp => true,
            GameMode::VsAi
            | GameMode::LanHost
            | GameMode::LanJoin
            | GameMode::RelayHost
            | GameMode::RelayJoin => self.game.side_to_move() == self.local_color,
        }
    }

    /// Reset to a fresh game keeping the mode/color.
    pub fn restart(&mut self) {
        self.game = Game::new();
    }
}

/// Currently selected source square (for click-to-move).
#[derive(Resource, Default)]
pub struct Selection {
    pub from: Option<chess_core::Square>,
}

/// Geometry constants for mapping board coordinates to world space.
pub const CELL: f32 = 64.0;
pub const PIECE_RADIUS: f32 = 27.0;

/// World position (x, y) for a board square. Red (rank 0) is at the bottom.
pub fn square_to_world(sq: chess_core::Square) -> Vec2 {
    let f = sq.file() as f32;
    let r = sq.rank() as f32;
    Vec2::new((f - 4.0) * CELL, (r - 4.5) * CELL)
}

/// Nearest board square to a world position, if within half a cell.
pub fn world_to_square(pos: Vec2) -> Option<chess_core::Square> {
    let f = (pos.x / CELL + 4.0).round();
    let r = (pos.y / CELL + 4.5).round();
    if !(0.0..=8.0).contains(&f) || !(0.0..=9.0).contains(&r) {
        return None;
    }
    let sq = chess_core::Square::new(f as u8, r as u8)?;
    // Reject clicks too far from the intersection.
    let center = square_to_world(sq);
    if pos.distance(center) <= CELL * 0.5 {
        Some(sq)
    } else {
        None
    }
}

/// Bundled CJK fonts (original artwork-free; OFL/Apache licensed) used for all
/// on-screen text so piece glyphs (帅/将…) and UI render correctly.
#[derive(Resource, Clone)]
pub struct UiFonts {
    pub regular: Handle<Font>,
    pub bold: Handle<Font>,
}
