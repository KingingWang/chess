//! Board perspective lock for viewing from specific sides.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perspective {
    Red,
    Black,
    Dynamic,
}

impl Perspective {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Red => "红方视角",
            Self::Black => "黑方视角",
            Self::Dynamic => "动态视角",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Red => Self::Black,
            Self::Black => Self::Dynamic,
            Self::Dynamic => Self::Red,
        }
    }
}

impl Default for Perspective {
    fn default() -> Self {
        Self::Dynamic
    }
}

#[derive(Resource, Debug, Clone)]
pub struct BoardPerspective {
    pub perspective: Perspective,
    pub locked: bool,
}

impl Default for BoardPerspective {
    fn default() -> Self {
        Self {
            perspective: Perspective::Dynamic,
            locked: false,
        }
    }
}

pub fn cycle_perspective(
    keys: Res<ButtonInput<KeyCode>>,
    mut bp: ResMut<BoardPerspective>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyP) {
        bp.perspective = bp.perspective.next();
        dirty.0 = true;
        crate::toast::spawn_toast(&mut commands, &fonts, bp.perspective.label_cn());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cycle() {
        let mut p = Perspective::Red;
        p = p.next();
        assert_eq!(p, Perspective::Black);
        p = p.next();
        assert_eq!(p, Perspective::Dynamic);
    }
}
