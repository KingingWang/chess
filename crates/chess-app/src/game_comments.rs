//! Game comments and notes system.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct GameComments {
    pub comments: Vec<(usize, String)>,
    pub visible: bool,
}

impl Default for GameComments {
    fn default() -> Self {
        Self {
            comments: Vec::new(),
            visible: false,
        }
    }
}

impl GameComments {
    pub fn add(&mut self, move_idx: usize, comment: String) {
        self.comments.push((move_idx, comment));
    }
    pub fn for_move(&self, move_idx: usize) -> Vec<&str> {
        self.comments
            .iter()
            .filter(|(idx, _)| *idx == move_idx)
            .map(|(_, c)| c.as_str())
            .collect()
    }
    pub fn clear(&mut self) {
        self.comments.clear();
    }
    pub fn count(&self) -> usize {
        self.comments.len()
    }
}

pub fn toggle_comments(
    keys: Res<ButtonInput<KeyCode>>,
    mut gc: ResMut<GameComments>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyC) {
        gc.visible = !gc.visible;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if gc.visible {
                "评论面板已打开"
            } else {
                "评论面板已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut gc = GameComments::default();
        gc.add(0, "好棋".to_string());
        assert_eq!(gc.count(), 1);
    }
    #[test]
    fn test_for_move() {
        let mut gc = GameComments::default();
        gc.add(5, "关键着".to_string());
        assert_eq!(gc.for_move(5).len(), 1);
    }
}
