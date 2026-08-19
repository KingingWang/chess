//! Difficulty selection dialog shown before starting a VsAi game.
//!
//! When the player clicks "人机对战" in the main menu, this dialog appears
//! with 4 difficulty levels (简单/中等/困难/大师). Selecting one sets the
//! `AiSettings` resource and transitions to the game. The dialog also shows
//! which engine will play (bundled Pikafish or the built-in fallback).

use bevy::prelude::*;
use chess_ai::Difficulty;
use chess_core::Color as ChessColor;

use crate::app_state::{AiSettings, AppState, CoreGame, GameMode, UiFonts};
use crate::ui_theme::*;

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

/// Marker for the cancel button.
#[derive(Component)]
pub(crate) struct CancelButton;

fn level_desc(d: Difficulty) -> (&'static str, &'static str) {
    match d {
        Difficulty::Easy => ("简单", "每步约 0.2 秒 · 轻快随手，适合热身"),
        Difficulty::Medium => ("中等", "每步约 0.8 秒 · 稳定思考，业余好手"),
        Difficulty::Hard => ("困难", "每步约 2 秒 · 深度计算，棋力强劲"),
        Difficulty::Master => ("大师", "每步约 3 秒 · 全力以赴，大师水准"),
    }
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

    let levels = [
        Difficulty::Easy,
        Difficulty::Medium,
        Difficulty::Hard,
        Difficulty::Master,
    ];

    let engine_line = if ai_settings.engine_path.is_some() {
        "引擎 · Pikafish（强力 NNUE）"
    } else {
        "引擎 · 内置（基础）"
    };

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
            BackgroundColor(SCRIM),
            GlobalZIndex(50),
            DifficultyDialogRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(Val::Px(36.0), Val::Px(30.0)),
                        row_gap: Val::Px(10.0),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(18.0)),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    BorderColor::all(HAIRLINE_STRONG),
                    card_shadow(),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("选择难度"),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 26.0,
                            ..default()
                        },
                        TextColor(GOLD_BRIGHT),
                    ));
                    card.spawn((
                        Text::new(engine_line),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(if ai_settings.engine_path.is_some() {
                            JADE
                        } else {
                            TEXT_FAINT
                        }),
                        Node {
                            margin: UiRect::bottom(Val::Px(14.0)),
                            ..default()
                        },
                    ));

                    for difficulty in levels {
                        let (label, desc) = level_desc(difficulty);
                        let is_current = ai_settings.difficulty == difficulty;
                        card.spawn((
                            Button,
                            DifficultyButton(difficulty),
                            Node {
                                width: Val::Px(380.0),
                                height: Val::Px(56.0),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(18.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(Val::Px(12.0)),
                                ..default()
                            },
                            BackgroundColor(BtnStyle::secondary().bg),
                            BorderColor::all(if is_current {
                                JADE_BORDER
                            } else {
                                BtnStyle::secondary().border
                            }),
                        ))
                        .with_children(|btn| {
                            btn.spawn((Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },))
                                .with_children(|left| {
                                    left.spawn((
                                        Text::new(label),
                                        TextFont {
                                            font: fonts.bold.clone(),
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(TEXT),
                                    ));
                                    if is_current {
                                        left.spawn((
                                            Text::new("当前"),
                                            TextFont {
                                                font: fonts.regular.clone(),
                                                font_size: 11.0,
                                                ..default()
                                            },
                                            TextColor(JADE),
                                        ));
                                    }
                                });
                            btn.spawn((
                                Text::new(desc),
                                TextFont {
                                    font: fonts.regular.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(TEXT_FAINT),
                            ));
                        });
                    }

                    // Cancel button (ghost).
                    card.spawn((
                        Button,
                        DifficultyButton(Difficulty::Hard), // placeholder, action is cancel
                        CancelButton,
                        Node {
                            width: Val::Px(380.0),
                            height: Val::Px(38.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(10.0)),
                            border_radius: BorderRadius::all(Val::Px(10.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("返回"),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(TEXT_DIM),
                        ));
                    });

                    // Keyboard hint footer.
                    card.spawn((
                        Text::new("按 1-4 选择 · Esc 返回"),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(TEXT_FAINT),
                        Node {
                            margin: UiRect::top(Val::Px(10.0)),
                            ..default()
                        },
                    ));
                });
        });
}

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
                    BackgroundColor(Color::srgba(0.85, 0.70, 0.42, 0.10))
                } else {
                    BackgroundColor(CARD_RAISED)
                };
            }
            Interaction::None => {
                *bg = if cancel.is_some() {
                    BackgroundColor(Color::NONE)
                } else {
                    BackgroundColor(BtnStyle::secondary().bg)
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
