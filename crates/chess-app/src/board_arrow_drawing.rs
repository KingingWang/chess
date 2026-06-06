//! Board arrow drawing for annotation and analysis.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct Arrow {
    pub from: (u8, u8),
    pub to: (u8, u8),
    pub color: [f32; 3],
    pub width: f32,
}

#[derive(Resource, Debug, Clone)]
pub struct BoardArrowDrawing {
    pub enabled: bool,
    pub arrows: Vec<Arrow>,
    pub drawing_mode: bool,
    pub current_color: [f32; 3],
    pub start_square: Option<(u8, u8)>,
}

impl Default for BoardArrowDrawing {
    fn default() -> Self {
        Self {
            enabled: true,
            arrows: Vec::new(),
            drawing_mode: false,
            current_color: [0.2, 0.6, 1.0],
            start_square: None,
        }
    }
}

impl BoardArrowDrawing {
    pub fn add_arrow(&mut self, from: (u8, u8), to: (u8, u8)) {
        self.arrows.push(Arrow {
            from,
            to,
            color: self.current_color,
            width: 3.0,
        });
    }
    pub fn clear_arrows(&mut self) {
        self.arrows.clear();
    }
    pub fn set_color(&mut self, color: [f32; 3]) {
        self.current_color = color;
    }
    pub fn start_drawing(&mut self, square: (u8, u8)) {
        self.start_square = Some(square);
        self.drawing_mode = true;
    }
    pub fn finish_drawing(&mut self, square: (u8, u8)) {
        if let Some(from) = self.start_square {
            if from != square {
                self.add_arrow(from, square);
            }
        }
        self.start_square = None;
        self.drawing_mode = false;
    }
}

pub fn toggle_arrow_drawing(
    keys: Res<ButtonInput<KeyCode>>,
    mut bad: ResMut<BoardArrowDrawing>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyA) {
        bad.enabled = !bad.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if bad.enabled {
                "箭头绘制已开启 (拖拽画箭头)"
            } else {
                "箭头绘制已关闭"
            },
        );
    }
}

pub fn clear_arrows_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    mut bad: ResMut<BoardArrowDrawing>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::Delete) {
        bad.clear_arrows();
        crate::toast::spawn_toast(&mut commands, &fonts, "箭头已清除");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_arrow() {
        let mut bad = BoardArrowDrawing::default();
        bad.add_arrow((0, 0), (1, 1));
        assert_eq!(bad.arrows.len(), 1);
    }
    #[test]
    fn test_clear() {
        let mut bad = BoardArrowDrawing::default();
        bad.add_arrow((0, 0), (1, 1));
        bad.clear_arrows();
        assert!(bad.arrows.is_empty());
    }
    #[test]
    fn test_drawing() {
        let mut bad = BoardArrowDrawing::default();
        bad.start_drawing((0, 0));
        bad.finish_drawing((1, 1));
        assert_eq!(bad.arrows.len(), 1);
        assert!(!bad.drawing_mode);
    }
}
