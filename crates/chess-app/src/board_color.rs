//! Board color customization beyond themes.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardColors {
    pub light_square: [f32; 3],
    pub dark_square: [f32; 3],
    pub border: [f32; 3],
    pub grid_lines: [f32; 3],
}

impl BoardColors {
    pub fn classic() -> Self {
        Self {
            light_square: [0.95, 0.88, 0.70],
            dark_square: [0.72, 0.55, 0.35],
            border: [0.40, 0.30, 0.20],
            grid_lines: [0.20, 0.15, 0.10],
        }
    }
    pub fn modern() -> Self {
        Self {
            light_square: [0.92, 0.92, 0.92],
            dark_square: [0.65, 0.65, 0.65],
            border: [0.30, 0.30, 0.30],
            grid_lines: [0.15, 0.15, 0.15],
        }
    }
    pub fn green() -> Self {
        Self {
            light_square: [0.85, 0.95, 0.85],
            dark_square: [0.55, 0.75, 0.55],
            border: [0.30, 0.50, 0.30],
            grid_lines: [0.15, 0.35, 0.15],
        }
    }
    pub fn blue() -> Self {
        Self {
            light_square: [0.85, 0.90, 0.95],
            dark_square: [0.55, 0.65, 0.75],
            border: [0.30, 0.40, 0.50],
            grid_lines: [0.15, 0.25, 0.35],
        }
    }
    pub fn preset_name(&self) -> &'static str {
        if self.light_square == Self::classic().light_square {
            "经典"
        } else if self.light_square == Self::modern().light_square {
            "现代"
        } else if self.light_square == Self::green().light_square {
            "绿色"
        } else if self.light_square == Self::blue().light_square {
            "蓝色"
        } else {
            "自定义"
        }
    }
}

impl Default for BoardColors {
    fn default() -> Self {
        Self::classic()
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct BoardColorResource {
    pub colors: BoardColors,
}

pub fn cycle_board_colors(
    keys: Res<ButtonInput<KeyCode>>,
    mut bc: ResMut<BoardColorResource>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyB) {
        bc.colors = if bc.colors == BoardColors::classic() {
            BoardColors::modern()
        } else if bc.colors == BoardColors::modern() {
            BoardColors::green()
        } else if bc.colors == BoardColors::green() {
            BoardColors::blue()
        } else {
            BoardColors::classic()
        };
        dirty.0 = true;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("棋盘颜色: {}", bc.colors.preset_name()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_presets() {
        assert_eq!(BoardColors::classic().preset_name(), "经典");
        assert_eq!(BoardColors::modern().preset_name(), "现代");
    }
    #[test]
    fn test_default() {
        let bc = BoardColorResource::default();
        assert_eq!(bc.colors.preset_name(), "经典");
    }
}
