//! Captured pieces display tray.
//!
//! Lives inside the left sidebar ([`crate::ui::CapturedSlot`]) as a card,
//! grouped by capturing color. Uses the same piece glyphs as the board,
//! rendered smaller. The tray updates whenever the board state changes and
//! collapses entirely while no pieces have been captured.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, Piece, PieceKind};

use crate::app_state::{CoreGame, UiFonts};
use crate::board_view::RenderDirty;

use crate::ui_theme::{CARD as TRAY_BG, HAIRLINE as BORDER, TEXT_FAINT};
const RED_INK: Color = crate::ui_theme::CINNABAR_HOVER;
const BLACK_INK: Color = crate::ui_theme::TEXT;

#[derive(Component)]
pub struct CapturedTrayRoot;

#[derive(Component)]
pub struct CapturedEntry;

fn piece_value(kind: PieceKind) -> i32 {
    crate::app_state::piece_value(kind)
}

/// Rebuild the captured pieces display when the board changes.
#[allow(clippy::too_many_arguments)]
pub fn update_captured_tray(
    dirty: Res<RenderDirty>,
    core: Res<CoreGame>,
    fonts: Res<UiFonts>,
    mut commands: Commands,
    slot_q: Query<Entity, With<crate::ui::CapturedSlot>>,
    mut root_q: Query<(Entity, &mut Node), With<CapturedTrayRoot>>,
    existing: Query<Entity, With<CapturedEntry>>,
    theme: Res<crate::board_theme::BoardTheme>,
) {
    if !dirty.0 {
        return;
    }

    // Lazily attach the tray card to the sidebar slot (spawn order between
    // OnEnter systems is not guaranteed, so we ensure on first update).
    if root_q.is_empty() {
        let Ok(slot) = slot_q.single() else { return };
        commands.entity(slot).with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(TRAY_BG),
                    BorderColor::all(BORDER),
                    CapturedTrayRoot,
                ))
                .with_children(|root| {
                    root.spawn((
                        Text::new("吃子"),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(TEXT_FAINT),
                    ));
                });
        });
        return; // content is filled on the next dirty pass
    }

    // Clear old entries.
    for e in &existing {
        commands.entity(e).despawn();
    }

    let Ok((root, mut root_node)) = root_q.single_mut() else {
        return;
    };
    // Update border to match the current theme.
    commands
        .entity(root)
        .insert(BorderColor::all(theme.palette.disc_border));

    // Collect captured pieces from history.
    let mut red_captured: Vec<PieceKind> = Vec::new(); // Pieces Red captured (Black pieces)
    let mut black_captured: Vec<PieceKind> = Vec::new(); // Pieces Black captured (Red pieces)

    for entry in core.game.history() {
        if let Some(cap) = entry.captured() {
            match cap.color {
                ChessColor::Black => red_captured.push(cap.kind),
                ChessColor::Red => black_captured.push(cap.kind),
            }
        }
    }

    // Collapse the tray while no pieces have been captured.
    if red_captured.is_empty() && black_captured.is_empty() {
        root_node.display = Display::None;
        return;
    }
    root_node.display = Display::Flex;

    // Sort by piece value (most valuable first).
    let sort_key = |k: &PieceKind| -> i32 {
        match k {
            PieceKind::Chariot => 0,
            PieceKind::Cannon => 1,
            PieceKind::Horse => 2,
            PieceKind::Elephant => 3,
            PieceKind::Advisor => 4,
            PieceKind::Pawn => 5,
            PieceKind::King => 6,
        }
    };
    red_captured.sort_by_key(sort_key);
    black_captured.sort_by_key(sort_key);

    // Determine which side made the most recent capture (for "←新" annotation).
    let last_capture_side = if !core.game.is_over() {
        core.game
            .history()
            .last()
            .and_then(|e| e.captured())
            .map(|c| c.color)
    } else {
        None
    };

    commands.entity(root).with_children(|tray| {
        // Red's captures (Black pieces eaten by Red).
        if !red_captured.is_empty() {
            tray.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(2.0),
                    ..default()
                },
                CapturedEntry,
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new("红吃: "),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(RED_INK),
                ));
                for kind in &red_captured {
                    let piece = Piece::new(ChessColor::Black, *kind);
                    row.spawn((
                        Text::new(piece.glyph().to_string()),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(BLACK_INK),
                    ));
                }
                let val: i32 = red_captured.iter().map(|k| piece_value(*k)).sum();
                if val > 0 {
                    row.spawn((
                        Text::new(format!(" ({}, {}子)", val, red_captured.len())),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.55, 0.50, 0.42)),
                    ));
                }
                // Annotate if the most recent capture was by Red.
                if last_capture_side == Some(ChessColor::Black) {
                    row.spawn((
                        Text::new(" ←新"),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 0.65, 0.40)),
                    ));
                }
            });
        }

        // Black's captures (Red pieces eaten by Black).
        if !black_captured.is_empty() {
            tray.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(2.0),
                    ..default()
                },
                CapturedEntry,
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new("黑吃: "),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(BLACK_INK),
                ));
                for kind in &black_captured {
                    let piece = Piece::new(ChessColor::Red, *kind);
                    row.spawn((
                        Text::new(piece.glyph().to_string()),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(RED_INK),
                    ));
                }
                let val: i32 = black_captured.iter().map(|k| piece_value(*k)).sum();
                if val > 0 {
                    row.spawn((
                        Text::new(format!(" ({}, {}子)", val, black_captured.len())),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.55, 0.50, 0.42)),
                    ));
                }
                // Annotate if the most recent capture was by Black.
                if last_capture_side == Some(ChessColor::Red) {
                    row.spawn((
                        Text::new(" ←新"),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 0.65, 0.40)),
                    ));
                }
            });
        }

        // Net material advantage indicator.
        let red_val: i32 = red_captured.iter().map(|k| piece_value(*k)).sum();
        let black_val: i32 = black_captured.iter().map(|k| piece_value(*k)).sum();
        let advantage = red_val - black_val;
        {
            let total_captured = red_captured.len() + black_captured.len();
            let (text, color) = if advantage > 0 {
                let label = if advantage >= 6 {
                    "红方大优"
                } else {
                    "红方优势"
                };
                (
                    format!("{} +{} (吃{}子)", label, advantage, total_captured),
                    Color::srgb(0.80, 0.55, 0.20),
                )
            } else if advantage < 0 {
                let label = if -advantage >= 6 {
                    "黑方大优"
                } else {
                    "黑方优势"
                };
                (
                    format!("{} +{} (吃{}子)", label, -advantage, total_captured),
                    Color::srgb(0.55, 0.52, 0.48),
                )
            } else {
                (
                    format!("「衡」 子力平衡 (吃{}子)", total_captured),
                    Color::srgb(0.60, 0.58, 0.50),
                )
            };
            tray.spawn((
                Text::new(text),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(color),
                CapturedEntry,
            ));
        }
    });
}

/// The tray is a child of the sidebar and dies with it; this only resets
/// state on exit when called (kept for symmetry with other HUD modules).
pub fn teardown_captured_tray(mut commands: Commands, q: Query<Entity, With<CapturedTrayRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
