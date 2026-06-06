//! Position setup board for custom position creation.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetupMode {
    Edit,
    Play,
}

#[derive(Resource, Debug, Clone)]
pub struct PositionSetupBoard {
    pub active: bool,
    pub mode: SetupMode,
    pub selected_piece: Option<char>,
    pub board: [[Option<char>; 9]; 10],
}

impl Default for PositionSetupBoard {
    fn default() -> Self {
        Self {
            active: false,
            mode: SetupMode::Edit,
            selected_piece: None,
            board: [[None; 9]; 10],
        }
    }
}

impl PositionSetupBoard {
    pub fn place_piece(&mut self, file: u8, rank: u8, piece: char) {
        if (file as usize) < 9 && (rank as usize) < 10 {
            self.board[rank as usize][file as usize] = Some(piece);
        }
    }
    pub fn remove_piece(&mut self, file: u8, rank: u8) {
        if (file as usize) < 9 && (rank as usize) < 10 {
            self.board[rank as usize][file as usize] = None;
        }
    }
    pub fn get_piece(&self, file: u8, rank: u8) -> Option<char> {
        if (file as usize) < 9 && (rank as usize) < 10 {
            self.board[rank as usize][file as usize]
        } else {
            None
        }
    }
    pub fn clear_board(&mut self) {
        self.board = [[None; 9]; 10];
    }
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        for rank in (0..10).rev() {
            let mut empty = 0;
            for file in 0..9 {
                if let Some(piece) = self.board[rank][file] {
                    if empty > 0 {
                        fen.push_str(&empty.to_string());
                        empty = 0;
                    }
                    fen.push(piece);
                } else {
                    empty += 1;
                }
            }
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }
        fen
    }
}

pub fn toggle_setup_board(
    keys: Res<ButtonInput<KeyCode>>,
    mut psb: ResMut<PositionSetupBoard>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyP) {
        psb.active = !psb.active;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if psb.active {
                "位置设置板已打开"
            } else {
                "位置设置板已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_place() {
        let mut psb = PositionSetupBoard::default();
        psb.place_piece(0, 0, 'R');
        assert_eq!(psb.get_piece(0, 0), Some('R'));
    }
    #[test]
    fn test_remove() {
        let mut psb = PositionSetupBoard::default();
        psb.place_piece(0, 0, 'R');
        psb.remove_piece(0, 0);
        assert_eq!(psb.get_piece(0, 0), None);
    }
    #[test]
    fn test_fen() {
        let psb = PositionSetupBoard::default();
        let fen = psb.to_fen();
        assert_eq!(fen, "9/9/9/9/9/9/9/9/9/9");
    }
}
