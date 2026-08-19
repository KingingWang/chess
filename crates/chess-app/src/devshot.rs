//! Dev-only self-screenshot tool.
//!
//! ```bash
//! CHESS_SHOT=/tmp/menu.png  cargo run -p chess-app          # menu
//! CHESS_SHOT=/tmp/game.png CHESS_SCENE=game cargo run -p chess-app  # in-game
//! CHESS_SHOT=/tmp/mv.png CHESS_SCENE=game-move cargo run -p chess-app
//! ```
//!
//! The app renders the requested scene, captures the primary window with
//! Bevy's screenshot API, waits for the PNG to be written, then exits.
//! Used for UI iteration and README artwork without manual interaction.
//!
//! Scenes:
//! - `menu`       — the main menu (default)
//! - `difficulty` — menu with the difficulty dialog open
//! - `game`       — a fresh VsAi game, before any move
//! - `game-move`  — VsAi after 炮二平五, capturing once Pikafish has replied
//!   (verifies the bundled engine end-to-end and shows mid-game UI)
//! - `analysis`   — like `game-move`, with analysis mode (Ctrl+A) enabled so
//!   the sidebar shows the live engine panel
//! - `captures`   — VsAi after a scripted capture line (仙人指路对兵), with
//!   analysis mode on: exercises the sidebar captured-pieces tray

use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};
use bevy::window::PrimaryWindow;

use crate::app_state::{AppState, CoreGame, GameMode};

/// Active screenshot request (inserted only when `CHESS_SHOT` is set).
#[derive(Resource)]
pub struct DevShot {
    pub path: String,
    pub scene: String,
    /// Frames to wait before capturing (lets fonts/pipelines warm up).
    pub warmup: u32,
    /// Frames until a scripted opening move is applied (`game-move` scene).
    pub scripted_move_in: Option<u32>,
    /// Queue of `(frames_from_now, move)` applied by `tick` (captures scene).
    pub scripted_queue: std::collections::VecDeque<(u32, chess_core::Move)>,
    pub captured: bool,
    /// Last observed PNG length — exit once it stabilizes.
    pub last_len: Option<u64>,
    /// Frame counter after capture, for the give-up timeout.
    pub waited: u32,
}

/// Read the env vars; `None` means normal (interactive) startup.
pub fn from_env() -> Option<DevShot> {
    let path = std::env::var("CHESS_SHOT").ok()?;
    let scene = std::env::var("CHESS_SCENE").unwrap_or_else(|_| "menu".to_string());
    // `game-move`/`analysis` must outlast engine startup + the difficulty's
    // think time with margin; at 60 fps 720 frames is ~12 s.
    let warmup = match scene.as_str() {
        "game-move" | "analysis" | "captures" => 720,
        _ => 120,
    };
    Some(DevShot {
        path,
        scene,
        warmup,
        scripted_move_in: None,
        scripted_queue: std::collections::VecDeque::new(),
        captured: false,
        last_len: None,
        waited: 0,
    })
}

/// On startup, jump straight into a game so the screenshot shows the board.
pub fn enter_scene(
    shot: Option<ResMut<DevShot>>,
    mut next: ResMut<NextState<AppState>>,
    mut core: ResMut<CoreGame>,
    mut difficulty: ResMut<crate::difficulty_dialog::DifficultyDialogState>,
    mut analysis: ResMut<crate::analysis_mode::AnalysisMode>,
) {
    let Some(mut shot) = shot else { return };
    let mv = |f1: u8, r1: u8, f2: u8, r2: u8| {
        chess_core::Move::new(
            chess_core::Square::new(f1, r1).expect("from on board"),
            chess_core::Square::new(f2, r2).expect("to on board"),
        )
    };
    match shot.scene.as_str() {
        "game" | "game-move" | "analysis" | "captures" => {
            core.restart();
            // `captures` scripts both sides, so use LocalPvp — otherwise the
            // VsAi bridge would launch the engine on Black's scripted turns.
            core.mode = if shot.scene == "captures" {
                GameMode::LocalPvp
            } else {
                GameMode::VsAi
            };
            core.local_color = chess_core::Color::Red;
            next.set(AppState::InGame);
            if shot.scene == "game-move" || shot.scene == "analysis" {
                // Let the InGame systems (board spawn, camera fit) settle
                // before pushing the scripted move.
                shot.scripted_move_in = Some(90);
            }
            if shot.scene == "analysis" {
                analysis.active = true;
            }
            if shot.scene == "captures" {
                // 仙人指路对兵局: red pawn eats black's centre pawn on move 3.
                analysis.active = true;
                shot.scripted_queue = [
                    (90, mv(4, 3, 4, 4)),  // 兵五进一 (e3e4)
                    (150, mv(4, 6, 4, 5)), // 卒5进1 (e6e5)
                    (210, mv(4, 4, 4, 5)), // 兵五进一 吃卒 (e4e5)
                ]
                .into_iter()
                .collect();
            }
        }
        "difficulty" => {
            difficulty.open = true;
        }
        _ => {}
    }
}

/// Capture after warmup, then exit once the PNG is fully written.
pub fn tick(
    mut commands: Commands,
    shot: Option<ResMut<DevShot>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    mut history_view: ResMut<crate::history_view::HistoryView>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut shot) = shot else { return };

    // Apply queued scripted moves (captures scene).
    if let Some((frames, mv)) = shot.scripted_queue.front().copied() {
        if frames <= 1 {
            shot.scripted_queue.pop_front();
            crate::moves::apply_local_move(&mut core, mv);
            history_view.return_to_live();
            dirty.0 = true;
        } else {
            shot.scripted_queue[0].0 = frames - 1;
        }
    }

    // Play the scripted opening move (炮二平五) once the game has settled.
    // The AI reply then runs during the remaining warmup frames, so the
    // capture shows both sides having moved.
    if let Some(frames) = shot.scripted_move_in.as_mut() {
        *frames = frames.saturating_sub(1);
        if *frames == 0 {
            shot.scripted_move_in = None;
            let from = chess_core::Square::new(7, 2).expect("h2 on board");
            let to = chess_core::Square::new(4, 2).expect("e2 on board");
            let mv = chess_core::Move::new(from, to);
            if crate::moves::apply_local_move(&mut core, mv).is_none() {
                info!("devshot: scripted move {} applied", mv.to_iccs());
            }
            history_view.return_to_live();
            dirty.0 = true;
        }
    }

    if !shot.captured {
        if shot.warmup > 0 {
            shot.warmup -= 1;
            return;
        }
        let Ok(win) = window.single() else { return };
        shot.captured = true;
        info!("devshot: capturing window -> {}", shot.path);
        commands
            .spawn(Screenshot::window(win))
            .observe(save_to_disk(shot.path.clone()));
        return;
    }
    // Wait for the file to appear and stop growing.
    let len = std::fs::metadata(&shot.path).map(|m| m.len()).ok();
    match (len, shot.last_len) {
        (Some(n), Some(prev)) if n == prev && n > 0 => {
            info!("devshot: saved {} bytes; exiting", n);
            exit.write(AppExit::Success);
        }
        _ => {
            shot.last_len = len;
            shot.waited += 1;
            if shot.waited > 600 {
                error!("devshot: timed out waiting for {}", shot.path);
                exit.write(AppExit::from_code(2));
            }
        }
    }
}
