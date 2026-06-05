//! Captured pieces display tray.
//!
//! Shows captured pieces below each player's timer area, grouped by color.
//! Uses the same piece glyphs as the board, rendered smaller. The tray
//! updates automatically whenever the board state changes.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, Piece, PieceKind};

use crate::app_state::{CoreGame, UiFonts};
use crate::board_view::RenderDirty;

const RED_INK: Color = Color::srgb(0.72, 0.11, 0.11);
const BLACK_INK: Color = Color::srgb(0.30, 0.28, 0.26);
const TRAY_BG: Color = Color::srgba(0.13, 0.10, 0.08, 0.70);
const BORDER: Color = Color::srgb(0.45, 0.35, 0.20);

#[derive(Component)]
pub struct CapturedTrayRoot;

#[derive(Component)]
pub struct CapturedEntry;

pub fn setup_captured_tray(mut commands: Commands, fonts: Res<UiFonts>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                min_width: Val::Px(200.0),
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
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.62, 0.45)),
            ));
        });
}

fn piece_value(kind: PieceKind) -> i32 {
    crate::app_state::piece_value(kind)
}
/// Rebuild the captured pieces display when the board changes.
pub fn update_captured_tray(
    dirty: Res<RenderDirty>,
    core: Res<CoreGame>,
    fonts: Res<UiFonts>,
    mut commands: Commands,
    mut root_q: Query<(Entity, &mut Visibility), With<CapturedTrayRoot>>,
    existing: Query<Entity, With<CapturedEntry>>,
    theme: Res<crate::board_theme::BoardTheme>,
) {
    if !dirty.0 {
        return;
    }

    // Clear old entries.
    for e in &existing {
        commands.entity(e).despawn();
    }

    let Ok((root, mut root_vis)) = root_q.single_mut() else {
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

    // Hide the tray when no pieces have been captured.
    if red_captured.is_empty() && black_captured.is_empty() {
        *root_vis = Visibility::Hidden;
        return;
    }
    *root_vis = Visibility::Inherited;

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

pub fn teardown_captured_tray(mut commands: Commands, q: Query<Entity, With<CapturedTrayRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
