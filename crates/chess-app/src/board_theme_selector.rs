//! Board theme selector with preset themes.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoardTheme {
    Classic,
    Modern,
    Wood,
    Green,
    Blue,
    Dark,
    HighContrast,
}

impl BoardTheme {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Classic => "经典",
            Self::Modern => "现代",
            Self::Wood => "木纹",
            Self::Green => "绿色",
            Self::Blue => "蓝色",
            Self::Dark => "暗色",
            Self::HighContrast => "高对比",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Classic => Self::Modern,
            Self::Modern => Self::Wood,
            Self::Wood => Self::Green,
            Self::Green => Self::Blue,
            Self::Blue => Self::Dark,
            Self::Dark => Self::HighContrast,
            Self::HighContrast => Self::Classic,
        }
    }
    pub fn colors(&self) -> ([f32; 3], [f32; 3]) {
        match self {
            Self::Classic => ([0.95, 0.88, 0.70], [0.72, 0.55, 0.35]),
            Self::Modern => ([0.92, 0.92, 0.92], [0.65, 0.65, 0.65]),
            Self::Wood => ([0.85, 0.75, 0.55], [0.60, 0.45, 0.30]),
            Self::Green => ([0.85, 0.95, 0.85], [0.55, 0.75, 0.55]),
            Self::Blue => ([0.85, 0.90, 0.95], [0.55, 0.65, 0.75]),
            Self::Dark => ([0.35, 0.35, 0.35], [0.20, 0.20, 0.20]),
            Self::HighContrast => ([1.0, 1.0, 1.0], [0.0, 0.0, 0.0]),
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct BoardThemeSelector {
    pub theme: BoardTheme,
}

impl Default for BoardThemeSelector {
    fn default() -> Self {
        Self {
            theme: BoardTheme::Classic,
        }
    }
}

pub fn cycle_theme(
    keys: Res<ButtonInput<KeyCode>>,
    mut bts: ResMut<BoardThemeSelector>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyB) {
        bts.theme = bts.theme.next();
        dirty.0 = true;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("棋盘主题: {}", bts.theme.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cycle() {
        let mut t = BoardTheme::Classic;
        t = t.next();
        assert_eq!(t, BoardTheme::Modern);
    }
    #[test]
    fn test_colors() {
        let t = BoardTheme::Classic;
        let (light, dark) = t.colors();
        assert!(light[0] > dark[0]);
    }
}
