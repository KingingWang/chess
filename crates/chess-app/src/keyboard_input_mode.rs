//! Enhanced keyboard move input mode.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Click,
    Keyboard,
    Hybrid,
}

impl InputMode {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Click => "鼠标",
            Self::Keyboard => "键盘",
            Self::Hybrid => "混合",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Click => Self::Keyboard,
            Self::Keyboard => Self::Hybrid,
            Self::Hybrid => Self::Click,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct KeyboardInputMode {
    pub mode: InputMode,
    pub buffer: String,
}

impl Default for KeyboardInputMode {
    fn default() -> Self {
        Self {
            mode: InputMode::Click,
            buffer: String::new(),
        }
    }
}

impl KeyboardInputMode {
    pub fn add_char(&mut self, ch: char) {
        if self.buffer.len() < 4 {
            self.buffer.push(ch);
        }
    }
    pub fn backspace(&mut self) {
        self.buffer.pop();
    }
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
    pub fn is_complete(&self) -> bool {
        self.buffer.len() == 4
    }
}

pub fn cycle_input_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut km: ResMut<KeyboardInputMode>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyM) {
        km.mode = km.mode.next();
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("输入模式: {}", km.mode.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_buffer() {
        let mut km = KeyboardInputMode::default();
        km.add_char('h');
        km.add_char('2');
        assert_eq!(km.buffer, "h2");
    }
    #[test]
    fn test_complete() {
        let mut km = KeyboardInputMode::default();
        for c in "h2e2".chars() {
            km.add_char(c);
        }
        assert!(km.is_complete());
    }
    #[test]
    fn test_cycle() {
        let mut m = InputMode::Click;
        m = m.next();
        assert_eq!(m, InputMode::Keyboard);
    }
}
