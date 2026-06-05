//! Game-over overlay dialog with result display and action buttons.
//!
//! When the game ends (checkmate, stalemate, resignation, timeout, draw),
//! a centered overlay appears showing the result and offering "再来一局"
//! (rematch) and "返回主菜单" (back to menu) options.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, DrawReason, GameClock, GameResult, WinReason};

use crate::app_state::{AppState, ClockResource, CoreGame, Selection, UiFonts};
use crate::board_view::RenderDirty;

// --- Palette ---
const OVERLAY_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.65);
const CARD_BG: Color = Color::srgb(0.16, 0.13, 0.13);
const CARD_BORDER: Color = Color::srgb(0.62, 0.45, 0.22);
const RED_COLOR: Color = Color::srgb(0.85, 0.20, 0.15);
const BLACK_COLOR: Color = Color::srgb(0.90, 0.87, 0.80);
const DRAW_COLOR: Color = Color::srgb(0.80, 0.70, 0.40);
const BTN_COLOR: Color = Color::srgb(0.55, 0.16, 0.13);
const BTN_HOVER: Color = Color::srgb(0.72, 0.22, 0.17);
const BTN_BORDER: Color = Color::srgb(0.78, 0.62, 0.32);
const TEXT_COLOR: Color = Color::srgb(0.96, 0.93, 0.86);

#[derive(Component)]
pub struct GameOverRoot;

#[derive(Component, Clone, Copy)]
pub enum GameOverAction {
    Rematch,
    BackToMenu,
}

/// Tracks whether the overlay has been shown for the current game result.
#[derive(Resource, Default)]
pub struct GameOverShown(pub bool);
/// Timer for the fade-in animation when the overlay appears.
#[derive(Component)]
pub struct GameOverFadeIn {
    pub timer: Timer,
}

impl Default for GameOverFadeIn {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
        }
    }
}

/// Check if the game just ended and spawn the overlay.
#[allow(clippy::too_many_arguments)]
pub fn check_game_over(
    mut commands: Commands,
    core: Res<CoreGame>,
    fonts: Res<UiFonts>,
    mut shown: ResMut<GameOverShown>,
    existing: Query<Entity, With<GameOverRoot>>,
    mut pending_sound: ResMut<crate::sound::PendingSound>,
    ai_settings: Res<crate::app_state::AiSettings>,
    time: Res<Time>,
    rematch_count: Res<crate::app_state::RematchCount>,
    mut session_stats: ResMut<crate::app_state::SessionStats>,
    game_start: Res<crate::app_state::GameStartTime>,
    mut last_result: ResMut<crate::app_state::LastGameResult>,
    move_times: Res<crate::app_state::MoveTimeHistory>,
    mut session_play_time: ResMut<crate::app_state::SessionPlayTime>,
    undo_count: Res<crate::app_state::UndoCount>,
    mut win_streak: ResMut<crate::app_state::WinStreak>,
) {
    // Only show once per game result.
    if shown.0 || !core.game.is_over() {
        return;
    }
    // Don't show if already displayed.
    if !existing.is_empty() {
        return;
    }

    let result = core.game.result().unwrap();
    shown.0 = true;
    last_result.0 = Some(result);

    // Update session W/L/D stats.
    match result {
        GameResult::Win { winner, .. } => {
            if winner == core.local_color {
                session_stats.wins += 1;
            } else {
                session_stats.losses += 1;
            }
        }
        GameResult::Draw(_) => {
            session_stats.draws += 1;
        }
    }

    // Update VsAi win streak.
    if core.mode == crate::app_state::GameMode::VsAi {
        match result {
            GameResult::Win { winner, .. } if winner == core.local_color => {
                win_streak.0 += 1;
            }
            _ => win_streak.0 = 0,
        }
    }
    let game_dur = (time.elapsed_secs() - game_start.0) as u32;
    session_play_time.0 += time.elapsed_secs() - game_start.0;
    if auto_save_game(&core, Some(&ai_settings), Some(game_dur)) {
        let auto_msg = format!("✓ 已自动保存 (共{}手)", core.game.history_len());
        crate::toast::spawn_toast(&mut commands, &fonts, &auto_msg);
    } else if core.game.history_len() > 0 {
        let fail_msg = format!("⚠ 保存失败 (共{}手未保存)", core.game.history_len());
        crate::toast::spawn_toast(&mut commands, &fonts, &fail_msg);
    }

    // Queue game-over sound effect (plays next frame since play_pending_sound
    // runs earlier in the chain — imperceptible 1-frame delay).
    let game_sound = match result {
        GameResult::Win { winner, .. } => {
            if winner == core.local_color {
                crate::sound::MoveSound::GameWin
            } else {
                crate::sound::MoveSound::GameLose
            }
        }
        GameResult::Draw(_) => crate::sound::MoveSound::GameDraw,
    };
    pending_sound.sound = Some(game_sound);
    // Quick toast announcing the result.
    let total_moves = core.game.history_len();
    let result_toast = match result {
        GameResult::Win { winner, .. } => match winner {
            ChessColor::Red => format!("红方胜! 🏆 (共{}手)", total_moves),
            ChessColor::Black => format!("黑方胜! 🏆 (共{}手)", total_moves),
        },
        GameResult::Draw(_) => format!("和棋 🤝 (共{}手)", total_moves),
    };
    crate::toast::spawn_toast(&mut commands, &fonts, &result_toast);
    let (title, subtitle, title_color) = match result {
        GameResult::Win { winner, reason } => {
            let side = match winner {
                ChessColor::Red => "红 方 胜",
                ChessColor::Black => "黑 方 胜",
            };
            let why = match reason {
                WinReason::Checkmate => "♟ 将死对方",
                WinReason::Stalemate => "🚫 困毙对方",
                WinReason::Resignation => "🏳️ 对方认输",
                WinReason::PerpetualCheck => "♻️ 长将判负",
                WinReason::Timeout => "⏰ 对方超时",
            };
            let color = match winner {
                ChessColor::Red => RED_COLOR,
                ChessColor::Black => BLACK_COLOR,
            };
            (side, why, color)
        }
        GameResult::Draw(reason) => {
            let why = match reason {
                DrawReason::Agreement => "双方同意和棋",
                DrawReason::Repetition => "三次重复局面",
                DrawReason::NoCapture => "六十手无吃子",
            };
            ("和 棋", why, DRAW_COLOR)
        }
    };

    // Prepend mode indicator for VsAi.
    let title = if core.mode == crate::app_state::GameMode::VsAi {
        format!("🤖 {}", title)
    } else {
        title.to_string()
    };
    // Full-screen overlay.
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            // High z-index to appear above everything.
            GlobalZIndex(100),
            GameOverRoot,
            GameOverFadeIn::default(),
        ))
        .with_children(|overlay| {
            // Centered card.
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(60.0), Val::Px(40.0)),
                        row_gap: Val::Px(16.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(18.0)),
                        ..default()
                    },
                    BackgroundColor(CARD_BG),
                    BorderColor::all(CARD_BORDER),
                    BoxShadow::new(
                        Color::srgba(0.0, 0.0, 0.0, 0.6),
                        Val::Px(0.0),
                        Val::Px(12.0),
                        Val::Px(8.0),
                        Val::Px(40.0),
                    ),
                ))
                .with_children(|card| {
                    // Title.
                    card.spawn((
                        Text::new(title),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 48.0,
                            ..default()
                        },
                        TextColor(title_color),
                    ));
                    // Subtitle.
                    card.spawn((
                        Text::new(subtitle),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.70, 0.65, 0.55)),
                        Node {
                            margin: UiRect::bottom(Val::Px(4.0)),
                            ..default()
                        },
                    ));
                    // Personalized result text for AI games.
                    if core.mode == crate::app_state::GameMode::VsAi {
                        if let GameResult::Win { winner, .. } = result {
                            let personal = if winner == core.local_color {
                                "恭喜获胜! 🎉"
                            } else {
                                "再接再厉 · 下次加油 💪"
                            };
                            card.spawn((
                                Text::new(personal),
                                TextFont {
                                    font: fonts.regular.clone(),
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.85, 0.75, 0.50)),
                                Node {
                                    margin: UiRect::bottom(Val::Px(2.0)),
                                    ..default()
                                },
                            ));
                        }
                    } else if core.mode == crate::app_state::GameMode::LocalPvp {
                        let personal = match result {
                            GameResult::Win { winner, .. } => {
                                let side = match winner {
                                    ChessColor::Red => "红方",
                                    ChessColor::Black => "黑方",
                                };
                                format!("{}恭喜! 🎊", side)
                            }
                            GameResult::Draw(_) => "握手言和 🤝".to_string(),
                        };
                        card.spawn((
                            Text::new(personal),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.85, 0.75, 0.50)),
                            Node {
                                margin: UiRect::bottom(Val::Px(2.0)),
                                ..default()
                            },
                        ));
                    }
                    // Difficulty info for AI games.
                    if core.mode == crate::app_state::GameMode::VsAi {
                        let diff_label = ai_settings.difficulty.label();
                        card.spawn((
                            Text::new(format!(
                                "难度: {} {}",
                                ai_settings.difficulty.emoji(),
                                diff_label
                            )),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.50, 0.45)),
                            Node {
                                margin: UiRect::bottom(Val::Px(8.0)),
                                ..default()
                            },
                        ));
                    }

                    // Rematch count (shown if > 0).
                    if rematch_count.0 > 0 {
                        card.spawn((
                            Text::new({
                                let n = rematch_count.0 + 1;
                                if n >= 10 {
                                    format!("🔥 第{}局!", n)
                                } else {
                                    format!("🎮 第{}局", n)
                                }
                            }),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.50, 0.45)),
                            Node {
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                        ));
                    }

                    // Session W/L/D stats (shown when at least 1 game played).
                    if session_stats.total() > 0 {
                        card.spawn((
                            Text::new(format!(
                                "第{}局 · 战绩 {}胜 {}负 {}和 ({}%)",
                                session_stats.total(),
                                session_stats.wins,
                                session_stats.losses,
                                session_stats.draws,
                                session_stats.wins * 100 / session_stats.total()
                            )),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.70, 0.60, 0.35)),
                            Node {
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                        ));
                    }

                    // Move count + game duration.
                    if core.game.history_len() > 0 {
                        let moves = core.game.history_len();
                        let rounds = moves.div_ceil(2);
                        card.spawn((
                            Text::new(format!("共 {} 手 ({} 回合)", moves, rounds)),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.50, 0.45)),
                            Node {
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                        ));
                        // Show elapsed game duration.
                        let elapsed_secs = (time.elapsed_secs() - game_start.0) as u32;
                        let mins = elapsed_secs / 60;
                        let secs = elapsed_secs % 60;
                        let comment = if elapsed_secs >= 3600 {
                            " · 持久战!"
                        } else if elapsed_secs <= 30 {
                            " · 速战速决!"
                        } else {
                            ""
                        };
                        let duration_str = if mins >= 60 {
                            format!("用时 {}h{}分{}秒{}", mins / 60, mins % 60, secs, comment)
                        } else if mins > 0 {
                            format!("用时 {}分{}秒{}", mins, secs, comment)
                        } else {
                            format!("用时 {}秒{}", secs, comment)
                        };
                        card.spawn((
                            Text::new(duration_str),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.50, 0.45)),
                            Node {
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                        ));
                        // Average game duration (when multiple games played).
                        if session_stats.total() > 1 {
                            let avg_secs =
                                (session_play_time.0 / session_stats.total() as f32) as u32;
                            let avg_m = avg_secs / 60;
                            let avg_s = avg_secs % 60;
                            let avg_str = if avg_m > 0 {
                                format!("平均每局 {}分{}秒", avg_m, avg_s)
                            } else {
                                format!("平均每局 {}秒", avg_s)
                            };
                            card.spawn((
                                Text::new(avg_str),
                                TextFont {
                                    font: fonts.regular.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.55, 0.50, 0.45)),
                                Node {
                                    margin: UiRect::bottom(Val::Px(12.0)),
                                    ..default()
                                },
                            ));
                        }
                    }

                    // Move time statistics.
                    if move_times.0.len() >= 2 {
                        let sum: f32 = move_times.0.iter().sum();
                        let avg = sum / move_times.0.len() as f32;
                        let max = move_times.0.iter().cloned().fold(0.0f32, f32::max);
                        card.spawn((
                            Text::new(format!("平均每手 {:.0}s · 最慢 {:.0}s", avg, max)),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.50, 0.45)),
                            Node {
                                margin: UiRect::bottom(Val::Px(8.0)),
                                ..default()
                            },
                        ));
                    }

                    // Win streak (VsAi only).
                    if core.mode == crate::app_state::GameMode::VsAi && win_streak.0 > 1 {
                        card.spawn((
                            Text::new(format!("🔥 连胜 {}局!", win_streak.0)),
                            TextFont {
                                font: fonts.bold.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.95, 0.55, 0.10)),
                            Node {
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                        ));
                    }
                    // Undo count (if any).
                    if undo_count.0 > 0 {
                        card.spawn((
                            Text::new(format!("↩ 悔棋 {}次", undo_count.0)),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.50, 0.45)),
                            Node {
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                        ));
                    }
                    // Remaining piece counts.
                    {
                        let (mut red_c, mut black_c) = (0u32, 0u32);
                        for (_, piece) in core.game.board().pieces() {
                            match piece.color {
                                chess_core::Color::Red => red_c += 1,
                                chess_core::Color::Black => black_c += 1,
                            }
                        }
                        let captured = 32u32.saturating_sub(red_c + black_c);
                        let capture_str = if captured > 0 {
                            format!(" · 共吃{}子", captured)
                        } else {
                            String::new()
                        };
                        let phase = if total_moves <= 6 {
                            "开局"
                        } else if red_c + black_c > 18 {
                            "中局"
                        } else {
                            "残局"
                        };
                        card.spawn((
                            Text::new(format!(
                                "红方 {} 子 · 黑方 {} 子{} · {}",
                                red_c, black_c, capture_str, phase
                            )),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.50, 0.45)),
                            Node {
                                margin: UiRect::bottom(Val::Px(4.0)),
                                ..default()
                            },
                        ));
                    }

                    // Per-side capture exchange breakdown.
                    {
                        let (mut red_caps, mut black_caps) = (0u32, 0u32);
                        for entry in core.game.history() {
                            if let Some(cap) = entry.captured() {
                                match cap.color {
                                    ChessColor::Black => red_caps += 1,
                                    ChessColor::Red => black_caps += 1,
                                }
                            }
                        }
                        if red_caps + black_caps > 0 {
                            card.spawn((
                                Text::new(format!("红吃{}子 · 黑吃{}子", red_caps, black_caps)),
                                TextFont {
                                    font: fonts.regular.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.55, 0.50, 0.45)),
                                Node {
                                    margin: UiRect::bottom(Val::Px(8.0)),
                                    ..default()
                                },
                            ));
                        }
                    }

                    // Action buttons.
                    for (label, action) in [
                        ("再来一局", GameOverAction::Rematch),
                        ("返回主菜单", GameOverAction::BackToMenu),
                    ] {
                        card.spawn((
                            Button,
                            action,
                            Node {
                                width: Val::Px(220.0),
                                height: Val::Px(50.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.5)),
                                border_radius: BorderRadius::all(Val::Px(12.0)),
                                ..default()
                            },
                            BackgroundColor(BTN_COLOR),
                            BorderColor::all(BTN_BORDER),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(label),
                                TextFont {
                                    font: fonts.bold.clone(),
                                    font_size: 22.0,
                                    ..default()
                                },
                                TextColor(TEXT_COLOR),
                            ));
                        });
                    }

                    // Keyboard hint footer.
                    card.spawn((
                        Text::new(if core.mode == crate::app_state::GameMode::VsAi {
                            "Enter 再来 · Esc 返回 · 1-4难度"
                        } else {
                            "Enter 再来 · Esc 返回"
                        }),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.50, 0.46, 0.40)),
                        Node {
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },
                    ));
                });
        });
}

/// Handle game-over dialog button interactions.
#[allow(clippy::too_many_arguments)]
pub fn game_over_interaction(
    mut interactions: Query<
        (&Interaction, &GameOverAction, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<RenderDirty>,
    mut selection: ResMut<Selection>,
    mut shown: ResMut<GameOverShown>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    dialog_q: Query<Entity, With<GameOverRoot>>,
    mut ai_task: Option<ResMut<crate::ai_bridge::AiTask>>,
    mut clock_res: ResMut<ClockResource>,
    mut rematch_count: ResMut<crate::app_state::RematchCount>,
    mut session_stats: ResMut<crate::app_state::SessionStats>,
    mut pending_sound: ResMut<crate::sound::PendingSound>,
    fonts: Res<UiFonts>,
    ai_settings: Res<crate::app_state::AiSettings>,
) {
    for (interaction, action, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                pending_sound.sound = Some(crate::sound::MoveSound::UiClick);
                // Dismiss the dialog.
                for e in &dialog_q {
                    commands.entity(e).despawn();
                }
                shown.0 = false;

                match action {
                    GameOverAction::Rematch => {
                        rematch_count.0 += 1;
                        core.restart();
                        crate::moves::GAME_RESTARTED
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        selection.from = None;
                        if let Some(ref mut ai) = ai_task {
                            ai.rx = None;
                        }
                        // Reset the clock for the new game.
                        if let Some(ref mut clock) = clock_res.clock {
                            let tc = clock.time_control;
                            *clock = GameClock::new(tc);
                            clock.start(ChessColor::Red);
                        }
                        dirty.0 = true;
                        if core.mode == crate::app_state::GameMode::VsAi {
                            let msg = format!(
                                "🔄 再来一局 · {} (第{}局)",
                                ai_settings.difficulty.label(),
                                rematch_count.0
                            );
                            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                        }
                    }
                    GameOverAction::BackToMenu => {
                        rematch_count.0 = 0;
                        session_stats.wins = 0;
                        session_stats.losses = 0;
                        session_stats.draws = 0;
                        next.set(AppState::Menu);
                    }
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_COLOR),
        }
    }
}

/// Clean up on leaving the game state.
pub fn teardown_game_over(
    mut commands: Commands,
    mut shown: ResMut<GameOverShown>,
    q: Query<Entity, With<GameOverRoot>>,
) {
    shown.0 = false;
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Animate the game-over overlay fade-in (alpha 0 → OVERLAY_BG over 300ms).
pub fn animate_game_over_fadein(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut GameOverFadeIn, &mut BackgroundColor), With<GameOverRoot>>,
) {
    for (entity, mut fade, mut bg) in &mut q {
        fade.timer.tick(time.delta());
        let t = fade.timer.fraction();
        // Smooth ease-in: cubic ramp from 0 to target alpha.
        let alpha = OVERLAY_BG.alpha() * (t * t * (3.0 - 2.0 * t));
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, alpha));
        if fade.timer.is_finished() {
            *bg = BackgroundColor(OVERLAY_BG);
            commands.entity(entity).remove::<GameOverFadeIn>();
        }
    }
}

/// Keyboard shortcuts for the game-over dialog: Enter → rematch, Escape → menu.
#[allow(clippy::too_many_arguments)]
pub fn game_over_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<RenderDirty>,
    mut selection: ResMut<Selection>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    dialog_q: Query<Entity, With<GameOverRoot>>,
    mut ai_task: Option<ResMut<crate::ai_bridge::AiTask>>,
    mut clock_res: ResMut<ClockResource>,
    mut rematch_count: ResMut<crate::app_state::RematchCount>,
    mut shown: ResMut<GameOverShown>,
    mut session_stats: ResMut<crate::app_state::SessionStats>,
    mut pending_sound: ResMut<crate::sound::PendingSound>,
    mut move_times: ResMut<crate::app_state::MoveTimeHistory>,
    fonts: Res<UiFonts>,
    ai_settings: Res<crate::app_state::AiSettings>,
) {
    if !shown.0 {
        return;
    }

    let enter = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter);
    let escape = keys.just_pressed(KeyCode::Escape);

    if !enter && !escape {
        return;
    }

    // Dismiss the dialog.
    for e in &dialog_q {
        commands.entity(e).despawn();
    }
    shown.0 = false;
    pending_sound.sound = Some(crate::sound::MoveSound::UiClick);

    if escape {
        // Prevent keyboard_shortcuts from also handling Escape.
        crate::moves::ESCAPE_CONSUMED.store(true, std::sync::atomic::Ordering::Relaxed);
        rematch_count.0 = 0;
        session_stats.wins = 0;
        session_stats.losses = 0;
        session_stats.draws = 0;
        next.set(AppState::Menu);
    } else {
        // Enter → rematch
        rematch_count.0 += 1;
        core.restart();
        crate::moves::GAME_RESTARTED.store(true, std::sync::atomic::Ordering::Relaxed);
        move_times.0.clear();
        selection.from = None;
        if let Some(ref mut ai) = ai_task {
            ai.rx = None;
        }
        // Reset the clock for the new game.
        if let Some(ref mut clock) = clock_res.clock {
            let tc = clock.time_control;
            *clock = GameClock::new(tc);
            clock.start(ChessColor::Red);
        }
        dirty.0 = true;
        if core.mode == crate::app_state::GameMode::VsAi {
            let msg = format!(
                "🔄 再来一局 · {} (第{}局)",
                ai_settings.difficulty.label(),
                rematch_count.0
            );
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        }
    }
}

/// Silently save the game to disk on game over or before restart.
///
/// Enriches the PGN record with date, player names, mode, difficulty, and
/// optional game duration. Called from \, \,
/// and \ (Ctrl+N).
pub(crate) fn auto_save_game(
    core: &crate::app_state::CoreGame,
    ai_settings: Option<&crate::app_state::AiSettings>,
    game_duration_secs: Option<u32>,
) -> bool {
    if core.game.history_len() < 4 {
        return false;
    }

    let mut record = chess_core::GameRecord::from_game(&core.game);

    // Enrich metadata.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    record.date = crate::keyboard::format_date(secs);

    // Player names — include difficulty for AI games.
    let diff_label = ai_settings.map(|s| s.difficulty.label());
    record.red_player = match core.mode {
        crate::app_state::GameMode::VsAi => "玩家".to_string(),
        _ => "红方".to_string(),
    };
    record.black_player = match core.mode {
        crate::app_state::GameMode::VsAi => match diff_label {
            Some(label) => format!("AI ({})", label),
            None => "AI".to_string(),
        },
        _ => "黑方".to_string(),
    };

    // Mode and event tags.
    record.mode = match core.mode {
        crate::app_state::GameMode::VsAi => "VsAi".to_string(),
        crate::app_state::GameMode::LocalPvp => "LocalPvp".to_string(),
        crate::app_state::GameMode::LanHost | crate::app_state::GameMode::LanJoin => {
            "Lan".to_string()
        }
        crate::app_state::GameMode::RelayHost | crate::app_state::GameMode::RelayJoin => {
            "Relay".to_string()
        }
    };
    record.event = match core.mode {
        crate::app_state::GameMode::VsAi => match diff_label {
            Some(label) => format!("人机对战 · {} · {}手", label, core.game.history_len()),
            None => format!("人机对战 · {}手", core.game.history_len()),
        },
        crate::app_state::GameMode::LocalPvp => "双人对弈".to_string(),
        _ => "联网对弈".to_string(),
    };

    // Game duration tag.
    if let Some(dur) = game_duration_secs {
        let mins = dur / 60;
        let s = dur % 60;
        record.time = if mins >= 60 {
            format!("{}:{:02}:{:02}", mins / 60, mins % 60, s)
        } else {
            format!("{}:{:02}", mins, s)
        };
    }

    let content = record.serialize();
    let dir = crate::settings::save_dir();
    let filename = format!("game_{}_auto.pgn", secs);
    let path = dir.join(&filename);

    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &content)) {
        Ok(()) => {
            bevy::log::info!(path = %path.display(), "game auto-saved");
            true
        }
        Err(e) => {
            bevy::log::warn!(error = %e, "failed to auto-save game");
            false
        }
    }
}
