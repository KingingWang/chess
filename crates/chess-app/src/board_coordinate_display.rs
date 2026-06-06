//! Board coordinate display system.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoordinateStyle {
    Files,
    Ranks,
    Both,
    None,
}

impl CoordinateStyle {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Files => "文件",
            Self::Ranks => "横线",
            Self::Both => "全部",
            Self::None => "无",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Files => Self::Ranks,
            Self::Ranks => Self::Both,
            Self::Both => Self::None,
            Self::None => Self::Files,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct BoardCoordinateDisplay {
    pub style: CoordinateStyle,
    pub show_labels: bool,
}

impl Default for BoardCoordinateDisplay {
    fn default() -> Self {
        Self {
            style: CoordinateStyle::Both,
            show_labels: true,
        }
    }
}

pub fn cycle_coordinate_style(
    keys: Res<ButtonInput<KeyCode>>,
    mut bcd: ResMut<BoardCoordinateDisplay>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyC) {
        bcd.style = bcd.style.next();
        dirty.0 = true;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("坐标显示: {}", bcd.style.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cycle() {
        let mut s = CoordinateStyle::Files;
        s = s.next();
        assert_eq!(s, CoordinateStyle::Ranks);
    }
}
