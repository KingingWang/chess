//! Difficulty selection dialog shown before starting a VsAi game.
//!
//! When the player clicks "人机对战" in the main menu, this dialog appears
//! with 4 difficulty levels (简单/中等/困难/大师). Selecting one sets the
//! `AiSettings` resource and transitions to the game.

use bevy::prelude::*;
use chess_ai::Difficulty;
use chess_core::Color as ChessColor;

use crate::app_state::{AiSettings, AppState, CoreGame, GameMode, UiFonts};

// --- Palette ---
const OVERLAY_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.60);
const CARD_BG: Color = Color::srgb(0.16, 0.13, 0.12);
const CARD_BORDER: Color = Color::srgb(0.62, 0.45, 0.22);
const BTN: Color = Color::srgb(0.55, 0.16, 0.13);
const BTN_HOVER: Color = Color::srgb(0.72, 0.22, 0.17);
const BTN_BORDER: Color = Color::srgb(0.78, 0.62, 0.32);
const TEXT_COLOR: Color = Color::srgb(0.96, 0.93, 0.86);
const TITLE_COLOR: Color = Color::srgb(0.93, 0.84, 0.55);
const DESC_COLOR: Color = Color::srgb(0.65, 0.60, 0.50);

/// Marker for the difficulty dialog root.
#[derive(Component)]
pub struct DifficultyDialogRoot;

/// Button data holding the difficulty level.
#[derive(Component, Clone, Copy)]
pub struct DifficultyButton(pub Difficulty);

/// Resource controlling whether the difficulty dialog is open.
#[derive(Resource, Default)]
pub struct DifficultyDialogState {
    pub open: bool,
}

/// Spawn the difficulty selection overlay.
pub fn spawn_difficulty_dialog(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    state: Res<DifficultyDialogState>,
    existing: Query<Entity, With<DifficultyDialogRoot>>,
    ai_settings: Res<AiSettings>,
) {
    if !state.open || !existing.is_empty() {
        return;
    }

    let levels: [(Difficulty, &str, &str); 4] = [
        (Difficulty::Easy, "① 简  单", "浅搜索，适合初学者 (约800)"),
        (
            Difficulty::Medium,
            "② 中  等",
            "中等深度，有一定棋力 (约1200)",
        ),
        (Difficulty::Hard, "③ 困  难", "深度搜索，较强对手 (约1600)"),
        (
            Difficulty::Master,
            "④ 大  师",
            "最大搜索深度，棋力最强 (约2000)",
        ),
    ];

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
            BackgroundColor(OVERLAY_BG),
            GlobalZIndex(50),
            DifficultyDialogRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(50.0), Val::Px(35.0)),
                        row_gap: Val::Px(12.0),
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
                    // Title
                    card.spawn((
                        Text::new(format!(
                            "选择难度 (上次: {})",
                            ai_settings.difficulty.label()
                        )),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(TITLE_COLOR),
                        Node {
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        },
                    ));

                    // Difficulty buttons
                    for (difficulty, label, desc) in levels {
                        card.spawn((
                            Button,
                            DifficultyButton(difficulty),
                            Node {
                                width: Val::Px(260.0),
                                height: Val::Px(55.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                flex_direction: FlexDirection::Column,
                                border: UiRect::all(Val::Px(1.5)),
                                border_radius: BorderRadius::all(Val::Px(12.0)),
                                row_gap: Val::Px(2.0),
                                ..default()
                            },
                            BackgroundColor(BTN),
                            BorderColor::all(BTN_BORDER),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(label),
                                TextFont {
                                    font: fonts.bold.clone(),
                                    font_size: 22.0,
                                    ..default()
                                },
                                TextColor(TEXT_COLOR),
                            ));
                            btn.spawn((
                                Text::new(desc),
                                TextFont {
                                    font: fonts.regular.clone(),
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(DESC_COLOR),
                            ));
                        });
                    }

                    // Cancel button
                    card.spawn((
                        Button,
                        DifficultyButton(Difficulty::Hard), // placeholder, action is cancel
                        Node {
                            width: Val::Px(260.0),
                            height: Val::Px(38.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(6.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.25, 0.22, 0.20)),
                        BorderColor::all(Color::srgb(0.45, 0.38, 0.30)),
                        CancelButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("返  回"),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.70, 0.65, 0.55)),
                        ));
                    });

                    // Keyboard hint footer.
                    card.spawn((
                        Text::new(format!(
                            "按 1-4 选择 · Esc 返回 · 当前: {}",
                            match ai_settings.difficulty {
                                Difficulty::Easy => "①",
                                Difficulty::Medium => "②",
                                Difficulty::Hard => "③",
                                Difficulty::Master => "④",
                            }
                        )),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.50, 0.45, 0.38)),
                        Node {
                            margin: UiRect::top(Val::Px(6.0)),
                            ..default()
                        },
                    ));
                });
        });
}

/// Marker for the cancel button.
#[derive(Component)]
pub(crate) struct CancelButton;

/// Handle difficulty button clicks.
#[allow(clippy::too_many_arguments)]
pub fn difficulty_dialog_interaction(
    mut interactions: Query<
        (
            &Interaction,
            &DifficultyButton,
            &mut BackgroundColor,
            Option<&CancelButton>,
        ),
        Changed<Interaction>,
    >,
    mut settings: ResMut<AiSettings>,
    mut core: ResMut<CoreGame>,
    mut state: ResMut<DifficultyDialogState>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    dialog_q: Query<Entity, With<DifficultyDialogRoot>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // Keyboard shortcuts: 1-4 select difficulty, Escape cancels.
    if state.open && !dialog_q.is_empty() {
        let kb_difficulty = if keys.just_pressed(KeyCode::Digit1) {
            Some(Difficulty::Easy)
        } else if keys.just_pressed(KeyCode::Digit2) {
            Some(Difficulty::Medium)
        } else if keys.just_pressed(KeyCode::Digit3) {
            Some(Difficulty::Hard)
        } else if keys.just_pressed(KeyCode::Digit4) {
            Some(Difficulty::Master)
        } else {
            None
        };

        if let Some(diff) = kb_difficulty {
            for e in &dialog_q {
                commands.entity(e).despawn();
            }
            state.open = false;
            settings.difficulty = diff;
            crate::settings::save_difficulty(diff);
            core.restart();
            core.mode = GameMode::VsAi;
            core.local_color = ChessColor::Red;
            next.set(AppState::InGame);
            return;
        }

        if keys.just_pressed(KeyCode::Escape) {
            for e in &dialog_q {
                commands.entity(e).despawn();
            }
            state.open = false;
            return;
        }
    }

    for (interaction, diff_btn, mut bg, cancel) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                // Dismiss dialog.
                for e in &dialog_q {
                    commands.entity(e).despawn();
                }
                state.open = false;

                if cancel.is_some() {
                    // Just close, back to menu.
                    return;
                }

                // Set difficulty and start game.
                settings.difficulty = diff_btn.0;
                crate::settings::save_difficulty(diff_btn.0);
                core.restart();
                core.mode = GameMode::VsAi;
                core.local_color = ChessColor::Red;
                next.set(AppState::InGame);
            }
            Interaction::Hovered => {
                *bg = if cancel.is_some() {
                    BackgroundColor(Color::srgb(0.35, 0.30, 0.26))
                } else {
                    BackgroundColor(BTN_HOVER)
                };
            }
            Interaction::None => {
                *bg = if cancel.is_some() {
                    BackgroundColor(Color::srgb(0.25, 0.22, 0.20))
                } else {
                    BackgroundColor(BTN)
                };
            }
        }
    }
}

/// Clean up if we leave Menu state while dialog is open.
pub fn teardown_difficulty_dialog(
    mut commands: Commands,
    mut state: ResMut<DifficultyDialogState>,
    q: Query<Entity, With<DifficultyDialogRoot>>,
) {
    state.open = false;
    for e in &q {
        commands.entity(e).despawn();
    }
}
