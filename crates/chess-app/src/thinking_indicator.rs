//! Enhanced thinking indicator animations.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingStyle {
    Dots,
    Spinner,
    Pulse,
    Bar,
}

impl ThinkingStyle {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Dots => "点状",
            Self::Spinner => "旋转",
            Self::Pulse => "脉冲",
            Self::Bar => "进度条",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Dots => Self::Spinner,
            Self::Spinner => Self::Pulse,
            Self::Pulse => Self::Bar,
            Self::Bar => Self::Dots,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct ThinkingIndicator {
    pub style: ThinkingStyle,
    pub active: bool,
    pub elapsed: f32,
}

impl Default for ThinkingIndicator {
    fn default() -> Self {
        Self {
            style: ThinkingStyle::Dots,
            active: false,
            elapsed: 0.0,
        }
    }
}

impl ThinkingIndicator {
    pub fn start(&mut self) {
        self.active = true;
        self.elapsed = 0.0;
    }
    pub fn stop(&mut self) {
        self.active = false;
    }
    pub fn update(&mut self, delta: f32) {
        if self.active {
            self.elapsed += delta;
        }
    }
    pub fn dots_text(&self) -> String {
        let n = (self.elapsed * 2.0) as usize % 4;
        format!("思考中{}", ".".repeat(n))
    }
}

pub fn cycle_thinking_style(
    keys: Res<ButtonInput<KeyCode>>,
    mut ti: ResMut<ThinkingIndicator>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyV) {
        ti.style = ti.style.next();
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("思考动画: {}", ti.style.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_dots() {
        let mut ti = ThinkingIndicator::default();
        ti.start();
        ti.update(0.5);
        let text = ti.dots_text();
        assert!(text.starts_with("思考中"));
    }
    #[test]
    fn test_cycle() {
        let mut s = ThinkingStyle::Dots;
        s = s.next();
        assert_eq!(s, ThinkingStyle::Spinner);
    }
}
