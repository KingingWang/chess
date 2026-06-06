//! Game notation converter between different formats.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotationFormat {
    Iccs,
    Chinese,
    Algebraic,
}

impl NotationFormat {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Iccs => "ICCS",
            Self::Chinese => "中文",
            Self::Algebraic => "代数",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct GameNotationConverter {
    pub format: NotationFormat,
    pub enabled: bool,
}

impl Default for GameNotationConverter {
    fn default() -> Self {
        Self {
            format: NotationFormat::Iccs,
            enabled: true,
        }
    }
}

impl GameNotationConverter {
    pub fn convert(&self, move_str: &str, target: NotationFormat) -> String {
        match target {
            NotationFormat::Iccs => move_str.to_string(),
            NotationFormat::Chinese => self.to_chinese(move_str),
            NotationFormat::Algebraic => self.to_algebraic(move_str),
        }
    }

    fn to_chinese(&self, move_str: &str) -> String {
        if move_str.len() != 4 {
            return move_str.to_string();
        }
        let chars: Vec<char> = move_str.chars().collect();
        format!(
            "{}{}{}{}",
            Self::file_to_chinese(chars[0]),
            chars[1],
            "→",
            format!("{}{}", chars[2], chars[3])
        )
    }

    fn to_algebraic(&self, move_str: &str) -> String {
        if move_str.len() != 4 {
            return move_str.to_string();
        }
        let chars: Vec<char> = move_str.chars().collect();
        format!("{}{}-{}{}", chars[0], chars[1], chars[2], chars[3])
    }

    fn file_to_chinese(file: char) -> &'static str {
        match file {
            'a' => "一",
            'b' => "二",
            'c' => "三",
            'd' => "四",
            'e' => "五",
            'f' => "六",
            'g' => "七",
            'h' => "八",
            'i' => "九",
            _ => "?",
        }
    }
}

pub fn cycle_notation_format(
    keys: Res<ButtonInput<KeyCode>>,
    mut gnc: ResMut<GameNotationConverter>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyN) {
        gnc.format = match gnc.format {
            NotationFormat::Iccs => NotationFormat::Chinese,
            NotationFormat::Chinese => NotationFormat::Algebraic,
            NotationFormat::Algebraic => NotationFormat::Iccs,
        };
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("棋谱格式: {}", gnc.format.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_convert() {
        let gnc = GameNotationConverter::default();
        let chinese = gnc.convert("h2e2", NotationFormat::Chinese);
        assert!(chinese.contains("八"));
    }
    #[test]
    fn test_algebraic() {
        let gnc = GameNotationConverter::default();
        let alg = gnc.convert("h2e2", NotationFormat::Algebraic);
        assert_eq!(alg, "h2-e2");
    }
}
