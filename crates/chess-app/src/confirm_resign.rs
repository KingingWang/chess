//! Confirmation dialog before resignation.
//!
//! Prevents accidental resignation by requiring the player to confirm
//! their intent before the game ends.

use bevy::prelude::*;

use crate::app_state::{CoreGame, UiFonts};
use crate::board_view::RenderDirty;
use crate::net_bridge::{NetCommand, NetLink};

/// Whether the confirm-resign dialog is visible.
#[derive(Resource, Default)]
pub struct ConfirmResignVisible(pub bool);

/// Root entity of the confirm-resign overlay.
#[derive(Component)]
pub struct ConfirmResignRoot;

/// Button action in the confirm-resign dialog.
#[derive(Component, Clone, Copy)]
pub enum ConfirmResignAction {
    Confirm,
    Cancel,
}

const DIALOG_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
const BTN_CONFIRM: Color = Color::srgb(0.75, 0.2, 0.15);
const BTN_CANCEL: Color = Color::srgb(0.3, 0.3, 0.35);
const BTN_HOVER: Color = Color::srgb(0.5, 0.5, 0.55);

/// Spawn or despawn the dialog based on visibility resource.
#[allow(clippy::too_many_arguments)]
pub fn manage_confirm_resign(
    mut commands: Commands,
    mut visible: ResMut<ConfirmResignVisible>,
    existing: Query<Entity, With<ConfirmResignRoot>>,
    fonts: Res<UiFonts>,
    mut core: ResMut<crate::app_state::CoreGame>,
    keys: Res<ButtonInput<KeyCode>>,
    mut dirty: ResMut<RenderDirty>,
    net: Option<Res<NetLink>>,
    time: Res<Time>,
    game_start: Res<crate::app_state::GameStartTime>,
) {
    // Escape closes resign dialog if open (consumes Escape for this frame).
    if keys.just_pressed(KeyCode::Escape) && visible.0 {
        visible.0 = false;
        for e in &existing {
            commands.entity(e).despawn();
        }
        crate::moves::ESCAPE_CONSUMED.store(true, std::sync::atomic::Ordering::Relaxed);
        return;
    }

    // Enter confirms resignation when dialog is visible.
    if keys.just_pressed(KeyCode::Enter) && visible.0 {
        let me = core.local_color;
        core.game.resign(me);
        if let Some(ref net) = net {
            let _ = net.out.send(NetCommand::Resign);
        }
        dirty.0 = true;
        visible.0 = false;
        for e in &existing {
            commands.entity(e).despawn();
        }
        crate::moves::ESCAPE_CONSUMED.store(true, std::sync::atomic::Ordering::Relaxed);
        return;
    }

    if visible.0 && existing.is_empty() {
        let elapsed = (time.elapsed_secs() - game_start.0) as u32;
        spawn_dialog(&mut commands, &fonts, &core, elapsed);
    } else if !visible.0 {
        for e in &existing {
            commands.entity(e).despawn();
        }
    }
}

fn spawn_dialog(
    commands: &mut Commands,
    fonts: &UiFonts,
    core: &crate::app_state::CoreGame,
    elapsed_secs: u32,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(DIALOG_BG),
            GlobalZIndex(10),
            ConfirmResignRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(30.0)),
                        row_gap: Val::Px(20.0),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.12, 0.15)),
                ))
                .with_children(|card| {
                    // Check if player has material advantage (warning).
                    let resign_text = {
                        let moves = core.game.history_len();
                        let (mut red_mat, mut black_mat) = (0i32, 0i32);
                        let (mut red_count, mut black_count) = (0u32, 0u32);
                        for (_, piece) in core.game.board().pieces() {
                            let val = crate::app_state::piece_value(piece.kind);
                            match piece.color {
                                chess_core::Color::Red => {
                                    red_mat += val;
                                    red_count += 1;
                                }
                                chess_core::Color::Black => {
                                    black_mat += val;
                                    black_count += 1;
                                }
                            }
                        }
                        let my_mat = match core.local_color {
                            chess_core::Color::Red => red_mat,
                            chess_core::Color::Black => black_mat,
                        };
                        let opp_mat = match core.local_color {
                            chess_core::Color::Red => black_mat,
                            chess_core::Color::Black => red_mat,
                        };
                        let phase = if moves <= 6 {
                            "开局"
                        } else if red_count + black_count > 18 {
                            "中局"
                        } else {
                            "残局"
                        };
                        let dur = if elapsed_secs >= 60 {
                            format!("{}分{}秒", elapsed_secs / 60, elapsed_secs % 60)
                        } else {
                            format!("{}秒", elapsed_secs)
                        };
                        if my_mat > opp_mat + 3 {
                            format!(
                                "确认认输？\n(已走{}手, {} · {} · 你的子力领先!)",
                                moves, dur, phase
                            )
                        } else if moves < 10 {
                            format!(
                                "确认认输？\n(仅走{}手, {} · {} · 局面尚早!)",
                                moves, dur, phase
                            )
                        } else {
                            format!("确认认输？\n(已走{}手, {} · {})", moves, dur, phase)
                        }
                    };
                    card.spawn((
                        Text::new(resign_text),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(16.0),
                        ..default()
                    })
                    .with_children(|row| {
                        for (label, action, color) in [
                            ("确  认", ConfirmResignAction::Confirm, BTN_CONFIRM),
                            ("取  消", ConfirmResignAction::Cancel, BTN_CANCEL),
                        ] {
                            row.spawn((
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                                    border_radius: BorderRadius::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(color),
                                action,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font: fonts.regular.clone(),
                                        font_size: 20.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        }
                    });

                    // Keyboard hint.
                    card.spawn((
                        Text::new("Enter 确认认输 · Esc 取消"),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.50, 0.48, 0.45)),
                        Node {
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },
                    ));
                });
        });
}

/// Handle button interactions in the confirm-resign dialog.
pub fn confirm_resign_interaction(
    mut interactions: Query<
        (&Interaction, &ConfirmResignAction, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut visible: ResMut<ConfirmResignVisible>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<RenderDirty>,
    net: Option<Res<NetLink>>,
) {
    for (interaction, action, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match action {
                ConfirmResignAction::Confirm => {
                    let me = core.local_color;
                    core.game.resign(me);
                    if let Some(ref net) = net {
                        let _ = net.out.send(NetCommand::Resign);
                    }
                    dirty.0 = true;
                    visible.0 = false;
                }
                ConfirmResignAction::Cancel => {
                    visible.0 = false;
                }
            },
            Interaction::Hovered => {
                *bg = BackgroundColor(BTN_HOVER);
            }
            Interaction::None => {
                let default_color = match action {
                    ConfirmResignAction::Confirm => BTN_CONFIRM,
                    ConfirmResignAction::Cancel => BTN_CANCEL,
                };
                *bg = BackgroundColor(default_color);
            }
        }
    }
}

/// Tear down dialog when leaving game state.
pub fn teardown_confirm_resign(
    mut commands: Commands,
    mut visible: ResMut<ConfirmResignVisible>,
    q: Query<Entity, With<ConfirmResignRoot>>,
) {
    visible.0 = false;
    for e in &q {
        commands.entity(e).despawn();
    }
}
