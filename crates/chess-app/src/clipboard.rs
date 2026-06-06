//! Clipboard integration for FEN copy/paste operations.
//!
//! Provides keyboard shortcuts to copy the current board position as FEN
//! and paste a FEN to load a position.

use bevy::prelude::*;
use chess_core::Board;

use crate::app_state::{CoreGame, UiFonts};

/// Copy the current board position to clipboard as FEN.
pub fn copy_fen_to_clipboard(
    keys: Res<ButtonInput<KeyCode>>,
    core: Res<CoreGame>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyC) {
        let fen = core.game.board().to_fen();

        // Use arboard for clipboard access
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if clipboard.set_text(&fen).is_ok() {
                    crate::toast::spawn_toast(
                        &mut commands,
                        &fonts,
                        &format!("已复制FEN: {}", fen),
                    );
                    bevy::log::info!("Copied FEN to clipboard: {}", fen);
                } else {
                    crate::toast::spawn_toast(&mut commands, &fonts, "复制失败");
                    bevy::log::warn!("Failed to copy FEN to clipboard");
                }
            }
            Err(e) => {
                crate::toast::spawn_toast(&mut commands, &fonts, "剪贴板不可用");
                bevy::log::warn!("Clipboard not available: {}", e);
            }
        }
    }
}

/// Paste FEN from clipboard and load the position.
pub fn paste_fen_from_clipboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut core: ResMut<CoreGame>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyV) {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.get_text() {
                    Ok(fen) => {
                        match Board::from_fen(&fen) {
                            Ok(board) => {
                                // Load the position
                                core.game = chess_core::Game::from_board(board);
                                dirty.0 = true;
                                crate::toast::spawn_toast(
                                    &mut commands,
                                    &fonts,
                                    "已从剪贴板加载局面",
                                );
                                bevy::log::info!("Loaded FEN from clipboard: {}", fen);
                            }
                            Err(e) => {
                                crate::toast::spawn_toast(
                                    &mut commands,
                                    &fonts,
                                    &format!("无效FEN: {}", e),
                                );
                                bevy::log::warn!("Invalid FEN from clipboard: {}", fen);
                            }
                        }
                    }
                    Err(e) => {
                        crate::toast::spawn_toast(&mut commands, &fonts, "剪贴板无文本");
                        bevy::log::warn!("No text in clipboard: {}", e);
                    }
                }
            }
            Err(e) => {
                crate::toast::spawn_toast(&mut commands, &fonts, "剪贴板不可用");
                bevy::log::warn!("Clipboard not available: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chess_core::Board;

    #[test]
    fn test_fen_roundtrip() {
        let board = Board::start_position();
        let fen = board.to_fen();
        let parsed = Board::from_fen(&fen).unwrap();
        assert_eq!(board, parsed);
    }

    #[test]
    fn test_custom_fen() {
        let fen = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";
        let board = Board::from_fen(fen).unwrap();
        let exported = board.to_fen();
        assert_eq!(fen, exported);
    }

    #[test]
    fn test_invalid_fen() {
        let invalid_fen = "invalid fen string";
        assert!(Board::from_fen(invalid_fen).is_err());
    }
}
