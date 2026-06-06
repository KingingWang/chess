//! Application-level states, modes, and shared resources.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, Game, Square};

/// Standard piece value for material evaluation.
///
/// Shared by the captured tray, status text, resign dialog, and game-over
/// summary to avoid duplicating the 7-arm match across files.
pub fn piece_value(kind: chess_core::PieceKind) -> i32 {
    match kind {
        chess_core::PieceKind::Chariot => 9,
        chess_core::PieceKind::Horse => 4,
        chess_core::PieceKind::Cannon => 4,
        chess_core::PieceKind::Elephant => 2,
        chess_core::PieceKind::Advisor => 2,
        chess_core::PieceKind::Pawn => 1,
        chess_core::PieceKind::King => 0,
    }
}

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

impl GameMode {
    /// True for any mode that involves a remote peer (LAN or relay).
    pub fn is_networked(self) -> bool {
        matches!(
            self,
            GameMode::LanHost | GameMode::LanJoin | GameMode::RelayHost | GameMode::RelayJoin
        )
    }

    /// True for the side that hosts (creates) the room in a networked game.
    /// The host owns the authoritative game state and resynchronises
    /// reconnecting guests.
    pub fn is_net_host(self) -> bool {
        matches!(self, GameMode::LanHost | GameMode::RelayHost)
    }
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
    /// True while a previously connected peer is currently offline and the
    /// host is waiting for them (or a new joiner with the same room/password)
    /// to reconnect. Input is frozen and the HUD shows a reconnect notice.
    pub peer_disconnected: bool,
    /// The last move played (from, to) — used for board highlighting.
    pub last_move: Option<(Square, Square)>,
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
            peer_disconnected: false,
            last_move: None,
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
        self.last_move = None;
    }

    /// Full reset of all network/session fields. Called on return to menu so
    /// that a subsequent local or AI game does not inherit stale `awaiting_peer`,
    /// `connected`, or `peer_disconnected` flags from a previous networked
    /// session (which would cause the HUD to show "正在连接" indefinitely).
    pub fn reset_session(&mut self) {
        self.game = Game::new();
        self.mode = GameMode::LocalPvp;
        self.local_color = ChessColor::Red;
        self.draw_offer_from_peer = false;
        self.room_code = None;
        self.awaiting_peer = false;
        self.connected = false;
        self.peer_disconnected = false;
        self.last_move = None;
    }
}

/// Currently selected source square (for click-to-move).
#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct Selection {
    pub from: Option<chess_core::Square>,
}

/// Whose side faces the bottom of the screen.
///
/// Networked games flip the board for the player controlling Black so their
/// own pieces are always on the near (bottom) side, matching the over-the-board
/// experience. Local PvP / VsAi always keep Red at the bottom.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoardOrientation {
    #[default]
    Red,
    Black,
}

impl BoardOrientation {
    pub fn from_color(color: ChessColor) -> Self {
        match color {
            ChessColor::Red => BoardOrientation::Red,
            ChessColor::Black => BoardOrientation::Black,
        }
    }
}

/// Geometry constants for mapping board coordinates to world space.
pub const CELL: f32 = 64.0;
pub const PIECE_RADIUS: f32 = 27.0;

/// World position (x, y) for a board square under the given orientation.
///
/// Red orientation: rank 0 (Red back-rank) sits at the bottom of the screen.
/// Black orientation: 180°-rotated, so Black's back-rank sits at the bottom.
pub fn square_to_world(sq: chess_core::Square, orient: BoardOrientation) -> Vec2 {
    let (f, r) = match orient {
        BoardOrientation::Red => (sq.file() as f32, sq.rank() as f32),
        BoardOrientation::Black => ((8 - sq.file()) as f32, (9 - sq.rank()) as f32),
    };
    Vec2::new((f - 4.0) * CELL, (r - 4.5) * CELL)
}

/// Nearest board square to a world position, if within half a cell.
pub fn world_to_square(pos: Vec2, orient: BoardOrientation) -> Option<chess_core::Square> {
    let f = (pos.x / CELL + 4.0).round();
    let r = (pos.y / CELL + 4.5).round();
    if !(0.0..=8.0).contains(&f) || !(0.0..=9.0).contains(&r) {
        return None;
    }
    let (sf, sr) = match orient {
        BoardOrientation::Red => (f as u8, r as u8),
        BoardOrientation::Black => (8 - f as u8, 9 - r as u8),
    };
    let sq = chess_core::Square::new(sf, sr)?;
    // Reject clicks too far from the intersection.
    let center = square_to_world(sq, orient);
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

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Square;

    #[test]
    fn red_orientation_puts_rank_zero_at_bottom() {
        let sq = Square::new(4, 0).unwrap(); // Red palace centre
        let p = square_to_world(sq, BoardOrientation::Red);
        assert!(p.y < 0.0, "Red rank 0 must be below center, got y={}", p.y);
    }

    #[test]
    fn black_orientation_puts_rank_zero_at_top() {
        let sq = Square::new(4, 0).unwrap();
        let p = square_to_world(sq, BoardOrientation::Black);
        assert!(
            p.y > 0.0,
            "Black orientation must place Red rank 0 at top, got y={}",
            p.y
        );
    }

    #[test]
    fn black_orientation_puts_black_back_rank_at_bottom() {
        let sq = Square::new(4, 9).unwrap(); // Black general
        let p = square_to_world(sq, BoardOrientation::Black);
        assert!(
            p.y < 0.0,
            "Black rank 9 must be at bottom under Black orientation, got y={}",
            p.y
        );
    }

    #[test]
    fn orientation_is_180_rotation_not_mirror() {
        // A piece on Red's right cannon point (file 7, rank 2) should land,
        // under Black orientation, on the Black side at file 1, rank 7 —
        // i.e. a true 180° rotation, not a horizontal mirror.
        let sq = Square::new(7, 2).unwrap();
        let red_pos = square_to_world(sq, BoardOrientation::Red);
        let black_pos = square_to_world(sq, BoardOrientation::Black);
        assert!((red_pos.x + black_pos.x).abs() < 1e-3);
        assert!((red_pos.y + black_pos.y).abs() < 1e-3);
    }

    #[test]
    fn world_to_square_roundtrip_in_both_orientations() {
        for &orient in &[BoardOrientation::Red, BoardOrientation::Black] {
            for f in 0..9 {
                for r in 0..10 {
                    let sq = Square::new(f, r).unwrap();
                    let world = square_to_world(sq, orient);
                    assert_eq!(
                        world_to_square(world, orient),
                        Some(sq),
                        "roundtrip failed for ({f},{r}) under {orient:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn orientation_from_color_matches_color() {
        assert_eq!(
            BoardOrientation::from_color(ChessColor::Red),
            BoardOrientation::Red
        );
        assert_eq!(
            BoardOrientation::from_color(ChessColor::Black),
            BoardOrientation::Black
        );
    }
}

/// Resource holding the game clock (optional; `None` for unlimited games).
#[derive(Resource, Default)]
#[allow(dead_code)]
pub struct ClockResource {
    pub clock: Option<chess_core::GameClock>,
}

/// Counts consecutive rematches in the current session.
#[derive(Resource, Default)]
pub struct RematchCount(pub u32);
/// Tracks win/loss/draw record for the current play session.
#[derive(Resource, Default)]
pub struct SessionStats {
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}

impl SessionStats {
    /// Total games played this session.
    pub fn total(&self) -> u32 {
        self.wins + self.losses + self.draws
    }
}

/// Timestamp (app-time) when the current in-game session started.
/// Stores the result of the most recent game for display on the menu screen.
#[derive(Resource, Default)]
pub struct LastGameResult(pub Option<chess_core::GameResult>);

/// Used for accurate game duration tracking across multiple games.
#[derive(Resource, Default)]
pub struct GameStartTime(pub f32);
/// Double-press guard for the "返回主菜单" HUD button.
#[derive(Resource, Default)]
pub struct BackToMenuPending(pub Option<f32>);

/// Double-press guard for the local draw offer button.
#[derive(Resource, Default)]
pub struct DrawOfferPending(pub Option<f32>);
/// Cumulative play time across the session (seconds).
#[derive(Resource, Default)]
pub struct SessionPlayTime(pub f32);
/// Tracks how many undos have been performed in the current game.
#[derive(Resource, Default)]
pub struct UndoCount(pub u32);
/// Consecutive win count (VsAi only).
#[derive(Resource, Default)]
pub struct WinStreak(pub u32);

/// System that records the game start time on entering InGame state.
pub fn record_game_start_time(
    time: Res<bevy::prelude::Time>,
    mut start: ResMut<GameStartTime>,
    mut move_times: ResMut<MoveTimeHistory>,
) {
    start.0 = time.elapsed_secs();
    move_times.0.clear();
}

/// Per-move elapsed time history (in seconds).
///
/// Each entry corresponds to the time taken for the move at that ply index.
/// Self-heals via truncation when game state rewinds (undo, restart).
#[derive(Resource, Default)]
pub struct MoveTimeHistory(pub Vec<f32>);

impl MoveTimeHistory {
    /// Format elapsed seconds as a compact string: "5s" or "1:23".
    pub fn format_time(secs: f32) -> String {
        let s = secs as u32;
        if s >= 60 {
            format!("{}:{:02}", s / 60, s % 60)
        } else {
            format!("{}s", s)
        }
    }
}
