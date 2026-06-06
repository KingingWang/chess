//! Highlight the last move on the board.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct BoardHighlightLastMove {
    pub enabled: bool,
    pub from_square: Option<(u8, u8)>,
    pub to_square: Option<(u8, u8)>,
    pub color: [f32; 3],
    pub alpha: f32,
}

impl Default for BoardHighlightLastMove {
    fn default() -> Self {
        Self {
            enabled: true,
            from_square: None,
            to_square: None,
            color: [0.9, 0.9, 0.3],
            alpha: 0.4,
        }
    }
}

impl BoardHighlightLastMove {
    pub fn highlight(&mut self, from: (u8, u8), to: (u8, u8)) {
        self.from_square = Some(from);
        self.to_square = Some(to);
    }
    pub fn clear(&mut self) {
        self.from_square = None;
        self.to_square = None;
    }
}

pub fn toggle_last_move_highlight(
    keys: Res<ButtonInput<KeyCode>>,
    mut bhl: ResMut<BoardHighlightLastMove>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyH) {
        bhl.enabled = !bhl.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if bhl.enabled {
                "最后一步高亮已开启"
            } else {
                "最后一步高亮已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_highlight() {
        let mut bhl = BoardHighlightLastMove::default();
        bhl.highlight((0, 0), (1, 1));
        assert_eq!(bhl.from_square, Some((0, 0)));
    }
}
