//! Clock display and lifecycle integration for timed games.
//!
//! This module:
//! - Renders two countdown timers (one per player) in the HUD.
//! - Ticks down each frame while the game is running.
//! - Triggers game-over with `WinReason::Timeout` when a player flags.
//! - Flashes red when remaining time is below 30 seconds.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, GameClock, GameResult, TimeControl, WinReason};

use crate::app_state::{ClockResource, CoreGame, GameMode, UiFonts};
use crate::board_view::RenderDirty;

// --- Palette ---
const TIMER_NORMAL: Color = Color::srgb(0.90, 0.87, 0.80);
const TIMER_LOW: Color = Color::srgb(0.95, 0.25, 0.15);
const TIMER_BG: Color = Color::srgba(0.10, 0.08, 0.06, 0.85);
const TIMER_ACTIVE_BG: Color = Color::srgba(0.25, 0.18, 0.08, 0.90);
const BORDER_COLOR: Color = Color::srgb(0.62, 0.45, 0.22);

/// Low time threshold (seconds) — below this the timer flashes.
const LOW_TIME_SECS: u64 = 30;

#[derive(Component)]
pub struct ClockRoot;

#[derive(Component)]
pub struct RedTimerText;

#[derive(Component)]
pub struct BlackTimerText;

#[derive(Component)]
pub struct RedTimerBg;

#[derive(Component)]
pub struct BlackTimerBg;

/// Initialize the clock from the game mode's time control.
pub fn init_clock(mut clock_res: ResMut<ClockResource>, core: Res<CoreGame>) {
    // Only create a clock for timed modes.
    // For now, all games get a Rapid 10+5 clock unless in unlimited mode.
    let tc = match core.mode {
        GameMode::LocalPvp | GameMode::VsAi => TimeControl::RAPID_10_5,
        _ => TimeControl::Unlimited, // Network games handle clock separately
    };

    if tc == TimeControl::Unlimited {
        clock_res.clock = None;
    } else {
        let mut clock = GameClock::new(tc);
        clock.start(ChessColor::Red);
        clock_res.clock = Some(clock);
    }
}

/// Spawn the timer UI elements (called on entering InGame).
pub fn setup_clock_ui(mut commands: Commands, fonts: Res<UiFonts>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(230.0), // Right of history panel
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            ClockRoot,
        ))
        .with_children(|root| {
            // Black timer (top = opponent for Red).
            root.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    min_width: Val::Px(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(TIMER_BG),
                BorderColor::all(BORDER_COLOR),
                BlackTimerBg,
            ))
            .with_children(|bg| {
                bg.spawn((
                    Text::new("⚫"),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(TIMER_NORMAL),
                    Node {
                        margin: UiRect::right(Val::Px(6.0)),
                        ..default()
                    },
                ));
                bg.spawn((
                    Text::new("10:00"),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(TIMER_NORMAL),
                    BlackTimerText,
                ));
            });

            // Red timer (bottom = local player).
            root.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    min_width: Val::Px(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(TIMER_BG),
                BorderColor::all(BORDER_COLOR),
                RedTimerBg,
            ))
            .with_children(|bg| {
                bg.spawn((
                    Text::new("🔴"),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(TIMER_NORMAL),
                    Node {
                        margin: UiRect::right(Val::Px(6.0)),
                        ..default()
                    },
                ));
                bg.spawn((
                    Text::new("10:00"),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(TIMER_NORMAL),
                    RedTimerText,
                ));
            });
        });
}

/// Update the timer display each frame and check for time-out.
#[allow(clippy::too_many_arguments)]
pub fn tick_clock(
    mut clock_res: ResMut<ClockResource>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<RenderDirty>,
    mut red_text: Query<(&mut Text, &mut TextColor), (With<RedTimerText>, Without<BlackTimerText>)>,
    mut black_text: Query<
        (&mut Text, &mut TextColor),
        (With<BlackTimerText>, Without<RedTimerText>),
    >,
    mut red_bg: Query<&mut BackgroundColor, (With<RedTimerBg>, Without<BlackTimerBg>)>,
    mut black_bg: Query<&mut BackgroundColor, (With<BlackTimerBg>, Without<RedTimerBg>)>,
    mut root_vis: Query<&mut Visibility, With<ClockRoot>>,
) {
    // Hide clock UI for unlimited time control games.
    if let Ok(mut vis) = root_vis.single_mut() {
        *vis = if clock_res.clock.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let Some(clock) = &clock_res.clock else {
        return;
    };

    if core.game.is_over() {
        // Stop the clock cleanly when the game ends by any means.
        if let Some(clock) = &mut clock_res.clock {
            clock.stop_current();
        }
        return;
    }

    // Update display.
    let red_remaining = clock.remaining(ChessColor::Red);
    let black_remaining = clock.remaining(ChessColor::Black);

    if let Ok((mut text, mut color)) = red_text.single_mut() {
        **text = GameClock::format_time(red_remaining);
        *color = if red_remaining.as_secs() < LOW_TIME_SECS {
            TextColor(TIMER_LOW)
        } else {
            TextColor(TIMER_NORMAL)
        };
    }

    if let Ok((mut text, mut color)) = black_text.single_mut() {
        **text = GameClock::format_time(black_remaining);
        *color = if black_remaining.as_secs() < LOW_TIME_SECS {
            TextColor(TIMER_LOW)
        } else {
            TextColor(TIMER_NORMAL)
        };
    }

    // Highlight active timer background.
    let active = clock.active_side();
    if let Ok(mut bg) = red_bg.single_mut() {
        *bg = if active == Some(ChessColor::Red) {
            BackgroundColor(TIMER_ACTIVE_BG)
        } else {
            BackgroundColor(TIMER_BG)
        };
    }
    if let Ok(mut bg) = black_bg.single_mut() {
        *bg = if active == Some(ChessColor::Black) {
            BackgroundColor(TIMER_ACTIVE_BG)
        } else {
            BackgroundColor(TIMER_BG)
        };
    }

    // Check for time-out.
    if clock.is_flagged(ChessColor::Red) {
        core.game.force_result(GameResult::Win {
            winner: ChessColor::Black,
            reason: WinReason::Timeout,
        });
        dirty.0 = true;
    } else if clock.is_flagged(ChessColor::Black) {
        core.game.force_result(GameResult::Win {
            winner: ChessColor::Red,
            reason: WinReason::Timeout,
        });
        dirty.0 = true;
    }
}

/// Handle clock updates when a move is made. Checks the atomic flag set by
/// `apply_local_move` to detect moves from any source (input, AI, network).
/// Elapsed time since the last move was made.
#[derive(Resource)]
pub struct MoveTimer {
    pub started: f32,
}

impl Default for MoveTimer {
    fn default() -> Self {
        Self { started: 0.0 }
    }
}

pub fn clock_on_move(
    mut clock_res: ResMut<ClockResource>,
    core: Res<CoreGame>,
    time: Res<Time>,
    mut move_timer: ResMut<MoveTimer>,
    mut move_times: ResMut<crate::app_state::MoveTimeHistory>,
) {
    use crate::moves::MOVE_APPLIED_THIS_FRAME;
    use std::sync::atomic::Ordering;

    if !MOVE_APPLIED_THIS_FRAME.swap(false, Ordering::Relaxed) {
        return;
    }

    // Record elapsed time for this move (before clock guard so unlimited
    // games also track move times).
    let elapsed = time.elapsed_secs() - move_timer.started;
    move_times.0.push(elapsed);

    // Reset move timer on every move.
    move_timer.started = time.elapsed_secs();

    let Some(clock) = &mut clock_res.clock else {
        return;
    };
    if clock.is_running() {
        let mover = core.game.side_to_move().opponent();
        clock.move_made(mover);
    }
}

/// Tear down clock UI.
pub fn teardown_clock_ui(mut commands: Commands, q: Query<Entity, With<ClockRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Play a soft tick sound every second when the local player's clock
/// drops below 10 seconds, creating a sense of urgency.
pub fn clock_warning_sound(
    clock_res: Res<ClockResource>,
    core: Res<CoreGame>,
    mut pending_sound: ResMut<crate::sound::PendingSound>,
    mut last_sec: Local<u32>,
) {
    let Some(clock) = &clock_res.clock else {
        return;
    };
    if core.game.is_over() {
        return;
    }

    // Only tick for the local player's clock.
    let active = clock.active_side();
    if active != Some(core.local_color) {
        *last_sec = u32::MAX;
        return;
    }

    let remaining = clock.remaining(core.local_color);
    let secs = remaining.as_secs() as u32;

    if secs < 10 && secs != *last_sec {
        *last_sec = secs;
        // Reuse the wooden click sound for the tick.
        pending_sound.sound = Some(crate::sound::MoveSound::Normal);
    } else if secs >= 10 {
        *last_sec = u32::MAX;
    }
}
