//! Move notation validator for checking input correctness.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationFormat {
    Iccs,
    Chinese,
    Uci,
}

impl NotationFormat {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Iccs => "ICCS",
            Self::Chinese => "中文",
            Self::Uci => "UCI",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct NotationValidator {
    pub enabled: bool,
    pub strict_mode: bool,
    pub last_error: Option<String>,
}

impl Default for NotationValidator {
    fn default() -> Self {
        Self {
            enabled: true,
            strict_mode: false,
            last_error: None,
        }
    }
}

impl NotationValidator {
    pub fn validate_iccs(&mut self, input: &str) -> bool {
        self.last_error = None;
        if input.len() != 4 {
            self.last_error = Some("ICCS格式应为4个字符 (如h2e2)".to_string());
            return false;
        }
        let chars: Vec<char> = input.chars().collect();
        if !('a'..='i').contains(&chars[0]) {
            self.last_error = Some("文件字符无效 (a-i)".to_string());
            return false;
        }
        if !('0'..='9').contains(&chars[1]) {
            self.last_error = Some("横线数字无效 (0-9)".to_string());
            return false;
        }
        if !('a'..='i').contains(&chars[2]) {
            self.last_error = Some("目标文件无效".to_string());
            return false;
        }
        if !('0'..='9').contains(&chars[3]) {
            self.last_error = Some("目标横线无效".to_string());
            return false;
        }
        true
    }

    pub fn validate_chinese(&mut self, input: &str) -> bool {
        self.last_error = None;
        let chars: Vec<char> = input.chars().collect();
        if chars.len() < 4 {
            self.last_error = Some("中文棋谱至少需要4个字符".to_string());
            return false;
        }
        let piece_chars = "帅将仕士相象馬马车車炮兵卒";
        if !piece_chars.contains(chars[0]) {
            self.last_error = Some("棋子名称无效".to_string());
            return false;
        }
        let actions = "进退平";
        if chars.len() >= 3 && !actions.contains(chars[2]) {
            self.last_error = Some("动作无效 (进/退/平)".to_string());
            return false;
        }
        true
    }

    pub fn detect_format(input: &str) -> NotationFormat {
        if input.len() == 4 && input.chars().all(|c| c.is_ascii_alphanumeric()) {
            NotationFormat::Iccs
        } else {
            NotationFormat::Chinese
        }
    }
}

pub fn toggle_validator(
    keys: Res<ButtonInput<KeyCode>>,
    mut v: ResMut<NotationValidator>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyN) {
        v.enabled = !v.enabled;
        let msg = if v.enabled {
            "着法验证已开启"
        } else {
            "着法验证已关闭"
        };
        crate::toast::spawn_toast(&mut commands, &fonts, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_valid_iccs() {
        let mut v = NotationValidator::default();
        assert!(v.validate_iccs("h2e2"));
    }
    #[test]
    fn test_invalid_iccs_length() {
        let mut v = NotationValidator::default();
        assert!(!v.validate_iccs("h2e"));
    }
    #[test]
    fn test_invalid_file() {
        let mut v = NotationValidator::default();
        assert!(!v.validate_iccs("z2e2"));
    }
    #[test]
    fn test_valid_chinese() {
        let mut v = NotationValidator::default();
        assert!(v.validate_chinese("炮二平五"));
    }
    #[test]
    fn test_invalid_piece() {
        let mut v = NotationValidator::default();
        assert!(!v.validate_chinese("X二平五"));
    }
    #[test]
    fn test_detect_format() {
        assert_eq!(
            NotationValidator::detect_format("h2e2"),
            NotationFormat::Iccs
        );
        assert_eq!(
            NotationValidator::detect_format("炮二平五"),
            NotationFormat::Chinese
        );
    }
}
