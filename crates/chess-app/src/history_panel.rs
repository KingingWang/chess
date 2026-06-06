//! Right-side move history panel showing all moves in Chinese notation.
//!
//! Displays moves in a scrollable list with move numbers, highlighting the
//! most recent move. The panel updates automatically when [`RenderDirty`] is
//! set (same trigger as the board redraw).

use bevy::prelude::*;
use chess_core::{move_to_chinese, Board, GameResult, WinReason};

use crate::app_state::{CoreGame, UiFonts};
use crate::board_view::RenderDirty;
use crate::history_view::HistoryView;

// --- Palette (matching the HUD style) ---
const PANEL_BG: Color = Color::srgba(0.13, 0.10, 0.10, 0.92);
const CARD_BORDER: Color = Color::srgb(0.62, 0.45, 0.22);
const TITLE_COLOR: Color = Color::srgb(0.93, 0.84, 0.55);
const RED_TEXT: Color = Color::srgb(0.85, 0.20, 0.15);
const BLACK_TEXT: Color = Color::srgb(0.80, 0.78, 0.72);
const MOVE_NUM_COLOR: Color = Color::srgb(0.55, 0.50, 0.42);
const HIGHLIGHT_BG: Color = Color::srgba(0.45, 0.35, 0.15, 0.45);
const CHECK_COLOR: Color = Color::srgb(0.90, 0.65, 0.15);
const ACTIVE_BORDER: Color = Color::srgba(0.78, 0.62, 0.32, 0.60);

/// Marker for the history panel root entity.
#[derive(Component)]
pub struct HistoryPanelRoot;

/// Marker for the scrollable move list container.
#[derive(Component)]
pub struct MoveListContainer;

/// Marker for individual move text entries (used for clearing/rebuilding).
#[derive(Component)]
pub struct MoveEntry;

/// Marker for the opening-pair subtitle (separate from MoveEntry to avoid
/// inflating the entry_count guard and causing per-frame rebuilds).
#[derive(Component)]
pub struct OpeningSubtitle;

/// Badge showing current move number at the top of the history panel.
#[derive(Component)]
pub struct MoveCountBadge;

/// Tracks which ply a history entry represents (for click-to-navigate).
#[derive(Component)]
pub struct MoveEntryPly(pub usize);

/// Set up the right-side history panel (called on entering InGame).
pub fn setup_history_panel(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    core: Res<crate::app_state::CoreGame>,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(220.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::left(Val::Px(2.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::all(CARD_BORDER),
            HistoryPanelRoot,
        ))
        .with_children(|panel| {
            // Title row with move counter badge.
            panel
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new({
                            let (icon, mode) = match core.mode {
                                crate::app_state::GameMode::VsAi => ("「机」", "人机"),
                                crate::app_state::GameMode::LocalPvp => ("「友」", "双人"),
                                _ => ("「网」", "联机"),
                            };
                            format!("{} 棋 谱 · {}", icon, mode)
                        }),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(TITLE_COLOR),
                    ));
                    row.spawn((
                        Text::new("第 0 手"),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(MOVE_NUM_COLOR),
                        MoveCountBadge,
                    ));
                });
            // Scrollable move list.
            panel.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
                MoveListContainer,
            ));
        });
}

/// Rebuild the move history list whenever the board is redrawn.
#[allow(clippy::too_many_arguments)]
pub fn update_history_panel(
    dirty: Res<RenderDirty>,
    core: Res<CoreGame>,
    fonts: Res<UiFonts>,
    mut commands: Commands,
    container_q: Query<Entity, With<MoveListContainer>>,
    existing_entries: Query<Entity, With<MoveEntry>>,
    history_view: Res<HistoryView>,
    mut badge_q: Query<&mut Text, With<MoveCountBadge>>,
    mut move_times: ResMut<crate::app_state::MoveTimeHistory>,
    mut scroll_q: Query<&mut ScrollPosition, With<HistoryPanelRoot>>,
    opening_q: Query<Entity, With<OpeningSubtitle>>,
) {
    // Only rebuild when the board state changed.
    // Note: we check the previous frame's dirty flag (it's cleared by redraw_pieces
    // which runs later in the chain). To avoid missing updates, we detect changes
    // by comparing history length to entry count.
    let history = core.game.history();
    let total_moves = history.len();

    // Self-healing: truncate move times if game was rewound (undo/restart).
    if move_times.0.len() > total_moves {
        move_times.0.truncate(total_moves);
    }

    // Update the move counter badge (do this before early returns).
    for mut badge_text in &mut badge_q {
        let rounds = total_moves.div_ceil(2);
        let label = match history_view.viewing_ply {
            Some(ply) => format!("第 {}/{} 手 ({} 回合)", ply, total_moves, rounds),
            None => format!("第 {} 手 ({} 回合)", total_moves, rounds),
        };
        **badge_text = label;
    }

    let entry_count = existing_entries.iter().count();

    // Only rebuild if history length changed (avoids rebuilding every frame).
    if total_moves == entry_count && !dirty.0 {
        return;
    }

    // Clear existing entries and opening subtitle.
    for entity in &existing_entries {
        commands.entity(entity).despawn();
    }
    for entity in &opening_q {
        commands.entity(entity).despawn();
    }

    let Ok(container) = container_q.single() else {
        return;
    };

    // Replay moves to generate notation strings.
    if history.is_empty() {
        return;
    }

    let total = history.len();
    // Opening pair subtitle (first Red + first Black move).
    if total >= 2 {
        let entry0 = &history[0];
        let board0: Board = entry0
            .fen_before()
            .parse()
            .unwrap_or_else(|_| Board::start_position());
        let mv0 = move_to_chinese(entry0.mv(), &board0);
        let entry1 = &history[1];
        let board1: Board = entry1
            .fen_before()
            .parse()
            .unwrap_or_else(|_| Board::start_position());
        let mv1 = move_to_chinese(entry1.mv(), &board1);
        commands.entity(container).with_children(|c| {
            c.spawn((
                Text::new(format!("开局: {} · {}", mv0, mv1)),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.60, 0.55, 0.45)),
                Node {
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
                OpeningSubtitle,
            ));
        });
    }
    commands.entity(container).with_children(|list| {
        for (i, entry) in history.iter().enumerate() {
            let mv = entry.mv();
            // Get board state before this move to generate notation.
            let board_before: Board = entry
                .fen_before()
                .parse()
                .unwrap_or_else(|_| Board::start_position());
            let notation = move_to_chinese(mv, &board_before);
            // Append capture/check/checkmate indicators.
            let is_checkmate = i == total - 1
                && matches!(
                    core.game.result(),
                    Some(GameResult::Win {
                        reason: WinReason::Checkmate,
                        ..
                    })
                );
            let capture_mark = if entry.captured().is_some() {
                " ×"
            } else {
                ""
            };
            let check_mark = if is_checkmate && entry.gave_check() {
                " ‡"
            } else if entry.gave_check() {
                " †"
            } else {
                ""
            };
            let time_mark = match move_times.0.get(i) {
                Some(&t) if t >= 1.0 => {
                    format!(" ({})", crate::app_state::MoveTimeHistory::format_time(t))
                }
                _ => String::new(),
            };
            let notation = format!("{}{}{}{}", notation, capture_mark, check_mark, time_mark);

            let move_number = i / 2 + 1;
            let is_red = i % 2 == 0;
            let is_highlighted = match history_view.viewing_ply {
                Some(ply) => i + 1 == ply,
                None => i == total - 1,
            };

            let text_color = if entry.gave_check() {
                CHECK_COLOR
            } else if is_red {
                RED_TEXT
            } else {
                BLACK_TEXT
            };

            let prefix = if is_red {
                format!("{:>3}. ", move_number)
            } else {
                "       ".to_string()
            };

            list.spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    border: UiRect::left(Val::Px(if is_highlighted { 3.0 } else { 0.0 })),
                    ..default()
                },
                if is_highlighted {
                    BackgroundColor(HIGHLIGHT_BG)
                } else {
                    BackgroundColor(Color::NONE)
                },
                if is_highlighted {
                    BorderColor::all(ACTIVE_BORDER)
                } else {
                    BorderColor::all(Color::NONE)
                },
                MoveEntry,
                MoveEntryPly(i + 1), // ply = 1-based (position after this move)
            ))
            .with_children(|row| {
                // Move number.
                row.spawn((
                    Text::new(&prefix),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(MOVE_NUM_COLOR),
                ));
                // Move notation.
                row.spawn((
                    Text::new(&notation),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 17.0,
                        ..default()
                    },
                    TextColor(text_color),
                ));
            });
        }

        // Result footer when game is over.
        if let Some(result) = core.game.result() {
            let result_text = match result {
                GameResult::Win { winner, reason } => {
                    let side = match winner {
                        chess_core::Color::Red => "红方胜",
                        chess_core::Color::Black => "黑方胜",
                    };
                    let why = match reason {
                        WinReason::Checkmate => "将死",
                        WinReason::Stalemate => "困毙",
                        WinReason::Resignation => "认输",
                        WinReason::PerpetualCheck => "长将",
                        WinReason::Timeout => "超时",
                    };
                    let rounds = total_moves.div_ceil(2);
                    format!("── {} ({}) · {}回合 ──", side, why, rounds)
                }
                GameResult::Draw(reason) => {
                    let why = match reason {
                        chess_core::DrawReason::Agreement => "协议",
                        chess_core::DrawReason::Repetition => "重复",
                        chess_core::DrawReason::NoCapture => "无吃子",
                    };
                    let rounds = total_moves.div_ceil(2);
                    format!("── 和棋 ({}) · {}回合 ──", why, rounds)
                }
            };
            list.spawn((
                Text::new(result_text),
                TextFont {
                    font: fonts.bold.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.80, 0.45)),
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
                MoveEntry,
            ));
        }
    });
    // Auto-scroll to bottom when in live view (not reviewing history).
    if history_view.viewing_ply.is_none() {
        for mut scroll in &mut scroll_q {
            scroll.0.y = f32::MAX;
        }
    }
}

/// Tear down the history panel when leaving the game.
pub fn teardown_history_panel(mut commands: Commands, q: Query<Entity, With<HistoryPanelRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Handle clicks on history panel move entries to jump to that position.
pub fn history_entry_click(
    interactions: Query<(&Interaction, &MoveEntryPly), (Changed<Interaction>, With<MoveEntry>)>,
    mut history_view: ResMut<HistoryView>,
    mut dirty: ResMut<RenderDirty>,
    core: Res<CoreGame>,
) {
    for (interaction, ply) in &interactions {
        if *interaction == Interaction::Pressed {
            let total = core.game.history_len();
            history_view.set_ply(ply.0, total);
            dirty.0 = true;
        }
    }
}

/// Highlight history panel entries on hover.
pub fn history_entry_hover(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<MoveEntry>),
    >,
) {
    for (interaction, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(0.40, 0.30, 0.15, 0.30));
            }
            Interaction::None => {
                // Return to either highlighted or transparent.
                // Since we rebuild entries each frame anyway, just set to transparent.
                *bg = BackgroundColor(Color::NONE);
            }
            Interaction::Pressed => {} // handled by history_entry_click
        }
    }
}

// ===== Move Annotation System =====

/// Component to store user annotations on moves.
#[derive(Component, Debug, Clone)]
pub struct MoveAnnotation {
    pub ply: usize,
    pub symbol: Option<MoveSymbol>,
    pub comment: Option<String>,
}

/// Move quality symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveSymbol {
    Good,        // !
    Bad,         // ?
    Brilliant,   // !!
    Blunder,     // ??
    Interesting, // !?
    Dubious,     // ?!
}

impl MoveSymbol {
    pub fn as_str(&self) -> &'static str {
        match self {
            MoveSymbol::Good => "!",
            MoveSymbol::Bad => "?",
            MoveSymbol::Brilliant => "!!",
            MoveSymbol::Blunder => "??",
            MoveSymbol::Interesting => "!?",
            MoveSymbol::Dubious => "?!",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            MoveSymbol::Good => Color::srgb(0.3, 0.9, 0.3), // Green
            MoveSymbol::Bad => Color::srgb(0.9, 0.3, 0.3),  // Red
            MoveSymbol::Brilliant => Color::srgb(0.2, 0.8, 1.0), // Cyan
            MoveSymbol::Blunder => Color::srgb(1.0, 0.2, 0.2), // Bright red
            MoveSymbol::Interesting => Color::srgb(0.9, 0.8, 0.2), // Yellow
            MoveSymbol::Dubious => Color::srgb(0.9, 0.6, 0.2), // Orange
        }
    }
}

/// Resource to store all move annotations for the current game.
#[derive(Resource, Debug, Clone, Default)]
pub struct MoveAnnotations {
    pub annotations: Vec<MoveAnnotation>,
}

impl MoveAnnotations {
    /// Add or update an annotation for a specific ply.
    pub fn annotate(&mut self, ply: usize, symbol: Option<MoveSymbol>, comment: Option<String>) {
        // Remove existing annotation for this ply
        self.annotations.retain(|a| a.ply != ply);

        // Add new annotation if there's something to annotate
        if symbol.is_some() || comment.is_some() {
            self.annotations.push(MoveAnnotation {
                ply,
                symbol,
                comment,
            });
        }
    }

    /// Get annotation for a specific ply.
    pub fn get(&self, ply: usize) -> Option<&MoveAnnotation> {
        self.annotations.iter().find(|a| a.ply == ply)
    }

    /// Clear all annotations.
    pub fn clear(&mut self) {
        self.annotations.clear();
    }

    /// Export annotations as text.
    pub fn export(&self, core: &CoreGame) -> String {
        let mut output = String::new();
        for annotation in &self.annotations {
            if let Some(board_before) = core.game.board_at_ply(annotation.ply) {
                let entry = &core.game.history()[annotation.ply];
                let move_notation = chess_core::move_to_chinese(entry.mv(), &board_before);
                let move_num = annotation.ply / 2 + 1;
                let side = if annotation.ply % 2 == 0 {
                    "红"
                } else {
                    "黑"
                };

                output.push_str(&format!("{}.{} {}", move_num, side, move_notation));

                if let Some(symbol) = annotation.symbol {
                    output.push_str(&format!(" {}", symbol.as_str()));
                }

                if let Some(comment) = &annotation.comment {
                    output.push_str(&format!(" {{{}}}", comment));
                }

                output.push('\n');
            }
        }
        output
    }
}

/// Keyboard shortcut to add annotation to the last move.
/// Press ! for good, ? for bad, etc.
pub fn handle_annotation_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut annotations: ResMut<MoveAnnotations>,
    core: Res<CoreGame>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
) {
    if core.game.history().is_empty() {
        return;
    }

    let last_ply = core.game.history().len() - 1;

    // Shift+1 = !, Shift+2 = @, etc. We'll use number keys with Ctrl
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if !ctrl {
        return;
    }

    let symbol = if keys.just_pressed(KeyCode::Digit1) {
        Some(MoveSymbol::Good)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(MoveSymbol::Bad)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(MoveSymbol::Brilliant)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(MoveSymbol::Blunder)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(MoveSymbol::Interesting)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(MoveSymbol::Dubious)
    } else if keys.just_pressed(KeyCode::Digit0) {
        // Clear annotation
        annotations.annotate(last_ply, None, None);
        crate::toast::spawn_toast(&mut commands, &fonts, "已清除标注");
        return;
    } else {
        None
    };

    if let Some(sym) = symbol {
        annotations.annotate(last_ply, Some(sym), None);
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("已添加标注: {}", sym.as_str()),
        );
    }
}

#[cfg(test)]
mod annotation_tests {
    use super::*;

    #[test]
    fn test_move_symbol_as_str() {
        assert_eq!(MoveSymbol::Good.as_str(), "!");
        assert_eq!(MoveSymbol::Bad.as_str(), "?");
        assert_eq!(MoveSymbol::Brilliant.as_str(), "!!");
        assert_eq!(MoveSymbol::Blunder.as_str(), "??");
        assert_eq!(MoveSymbol::Interesting.as_str(), "!?");
        assert_eq!(MoveSymbol::Dubious.as_str(), "?!");
    }

    #[test]
    fn test_annotations_add_and_get() {
        let mut annotations = MoveAnnotations::default();

        annotations.annotate(0, Some(MoveSymbol::Good), Some("Great opening".to_string()));

        let ann = annotations.get(0).unwrap();
        assert_eq!(ann.ply, 0);
        assert_eq!(ann.symbol, Some(MoveSymbol::Good));
        assert_eq!(ann.comment, Some("Great opening".to_string()));
    }

    #[test]
    fn test_annotations_update() {
        let mut annotations = MoveAnnotations::default();

        annotations.annotate(0, Some(MoveSymbol::Good), None);
        annotations.annotate(
            0,
            Some(MoveSymbol::Bad),
            Some("Changed my mind".to_string()),
        );

        let ann = annotations.get(0).unwrap();
        assert_eq!(ann.symbol, Some(MoveSymbol::Bad));
        assert_eq!(ann.comment, Some("Changed my mind".to_string()));
        assert_eq!(annotations.annotations.len(), 1); // Should only have one annotation
    }

    #[test]
    fn test_annotations_clear() {
        let mut annotations = MoveAnnotations::default();

        annotations.annotate(0, Some(MoveSymbol::Good), None);
        annotations.annotate(1, Some(MoveSymbol::Bad), None);

        annotations.clear();

        assert_eq!(annotations.annotations.len(), 0);
    }

    #[test]
    fn test_annotations_remove_specific() {
        let mut annotations = MoveAnnotations::default();

        annotations.annotate(0, Some(MoveSymbol::Good), None);
        annotations.annotate(1, Some(MoveSymbol::Bad), None);

        // Remove annotation by setting to None
        annotations.annotate(0, None, None);

        assert_eq!(annotations.annotations.len(), 1);
        assert!(annotations.get(0).is_none());
        assert!(annotations.get(1).is_some());
    }
}
