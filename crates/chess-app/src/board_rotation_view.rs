//! Board rotation view for different perspectives.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rotation {
    Normal,
    Rotated90,
    Rotated180,
    Rotated270,
}

impl Rotation {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Normal => "正常",
            Self::Rotated90 => "90°",
            Self::Rotated180 => "180°",
            Self::Rotated270 => "270°",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Normal => Self::Rotated90,
            Self::Rotated90 => Self::Rotated180,
            Self::Rotated180 => Self::Rotated270,
            Self::Rotated270 => Self::Normal,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct BoardRotationView {
    pub rotation: Rotation,
    pub enabled: bool,
}

impl Default for BoardRotationView {
    fn default() -> Self {
        Self {
            rotation: Rotation::Normal,
            enabled: true,
        }
    }
}

impl BoardRotationView {
    pub fn rotate(&mut self) {
        self.rotation = self.rotation.next();
    }
    pub fn transform_coordinates(&self, file: u8, rank: u8) -> (u8, u8) {
        match self.rotation {
            Rotation::Normal => (file, rank),
            Rotation::Rotated90 => (8 - rank, file),
            Rotation::Rotated180 => (8 - file, 9 - rank),
            Rotation::Rotated270 => (rank, 9 - file),
        }
    }
}

pub fn rotate_board(
    keys: Res<ButtonInput<KeyCode>>,
    mut brv: ResMut<BoardRotationView>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyR) {
        brv.rotate();
        dirty.0 = true;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("棋盘旋转: {}", brv.rotation.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_rotate() {
        let mut brv = BoardRotationView::default();
        brv.rotate();
        assert_eq!(brv.rotation, Rotation::Rotated90);
    }
    #[test]
    fn test_transform() {
        let brv = BoardRotationView::default();
        assert_eq!(brv.transform_coordinates(0, 0), (0, 0));
        let mut brv180 = BoardRotationView::default();
        brv180.rotation = Rotation::Rotated180;
        assert_eq!(brv180.transform_coordinates(0, 0), (8, 9));
    }
}
