//! Dynamic window title showing game state.
//!
//! Updates the window title to reflect the current mode, move count, and
//! game result. Runs as a standalone system (no chain ordering needed).

use bevy::prelude::*;

use crate::app_state::{AiSettings, AppState, CoreGame, GameMode};
use chess_core::game::GameResult;

/// Update window title based on game state.
pub fn update_window_title(
    mut windows: Query<&mut Window>,
    core: Res<CoreGame>,
    state: Res<State<AppState>>,
    ai_settings: Res<AiSettings>,
    theme: Res<crate::board_theme::BoardTheme>,
    session_stats: Res<crate::app_state::SessionStats>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };

    let title = match state.get() {
        AppState::Menu => "中国象棋 Xiangqi".to_string(),
        AppState::InGame => {
            let theme_str = format!(" · {}", theme.id.label());
            let mode_str = match core.mode {
                GameMode::LocalPvp => "双人对战",
                GameMode::VsAi => "人机对战", // difficulty appended below
                GameMode::LanHost | GameMode::LanJoin => "局域网",
                GameMode::RelayHost | GameMode::RelayJoin => "网络对战",
            };
            if let Some(result) = core.game.result() {
                let result_str = match result {
                    GameResult::Win { winner, .. } => match winner {
                        chess_core::Color::Red => "红方胜",
                        chess_core::Color::Black => "黑方胜",
                    },
                    GameResult::Draw(_) => "和棋",
                };
                let difficulty_str = if core.mode == GameMode::VsAi {
                    format!(" · {}", ai_settings.difficulty.label())
                } else {
                    String::new()
                };
                if session_stats.total() > 0 {
                    format!(
                        "中国象棋 · {}{}{} · {} · 共{}手 · {}胜{}负{}",
                        mode_str,
                        difficulty_str,
                        theme_str,
                        result_str,
                        core.game.history_len(),
                        session_stats.wins,
                        session_stats.losses,
                        if session_stats.draws > 0 {
                            format!("{}和", session_stats.draws)
                        } else {
                            String::new()
                        }
                    )
                } else {
                    format!(
                        "中国象棋 · {}{}{} · {} · 共{}手",
                        mode_str,
                        difficulty_str,
                        theme_str,
                        result_str,
                        core.game.history_len()
                    )
                }
            } else {
                let move_num = core.game.history_len() / 2 + 1;
                let color_str = match core.mode {
                    GameMode::VsAi
                    | GameMode::LanHost
                    | GameMode::LanJoin
                    | GameMode::RelayHost
                    | GameMode::RelayJoin => match core.local_color {
                        chess_core::Color::Red => " · 红方",
                        chess_core::Color::Black => " · 黑方",
                    },
                    GameMode::LocalPvp => "",
                };
                let difficulty_str = if core.mode == GameMode::VsAi {
                    format!(" · {}", ai_settings.difficulty.label())
                } else {
                    String::new()
                };
                if session_stats.total() > 0 {
                    format!(
                        "中国象棋 · {}{}{}{} · 第{}手 · {}胜{}负{}",
                        mode_str,
                        difficulty_str,
                        color_str,
                        theme_str,
                        move_num,
                        session_stats.wins,
                        session_stats.losses,
                        if session_stats.draws > 0 {
                            format!("{}和", session_stats.draws)
                        } else {
                            String::new()
                        }
                    )
                } else {
                    format!(
                        "中国象棋 · {}{}{}{} · 第{}手",
                        mode_str, difficulty_str, color_str, theme_str, move_num
                    )
                }
            }
        }
    };

    if window.title != title {
        window.title = title;
    }
}

/// Reset window title when returning to menu.
pub fn reset_window_title(
    mut windows: Query<&mut Window>,
    session_stats: Res<crate::app_state::SessionStats>,
) {
    if let Ok(mut window) = windows.single_mut() {
        window.title = if session_stats.total() > 0 {
            format!(
                "中国象棋 · {}胜{}负{}",
                session_stats.wins,
                session_stats.losses,
                if session_stats.draws > 0 {
                    format!("{}和", session_stats.draws)
                } else {
                    String::new()
                }
            )
        } else {
            "中国象棋 Xiangqi".to_string()
        };
    }
}
