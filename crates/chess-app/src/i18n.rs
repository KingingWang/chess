//! Internationalization (i18n) system for multi-language support.
//!
//! Supports Chinese (Simplified/Traditional), English, and Japanese.

use bevy::prelude::*;
use std::collections::HashMap;

/// Supported languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    ChineseSimplified,
    ChineseTraditional,
    English,
    Japanese,
}

impl Language {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ChineseSimplified => "简体中文",
            Self::ChineseTraditional => "繁體中文",
            Self::English => "English",
            Self::Japanese => "日本語",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::ChineseSimplified => Self::ChineseTraditional,
            Self::ChineseTraditional => Self::English,
            Self::English => Self::Japanese,
            Self::Japanese => Self::ChineseSimplified,
        }
    }
}

/// String key for translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I18nKey {
    // Menu
    NewGame,
    VsAi,
    LocalPvp,
    LanGame,
    OnlineGame,
    Settings,
    Quit,
    // In-game
    YourTurn,
    AiThinking,
    Check,
    Checkmate,
    Stalemate,
    Draw,
    Resign,
    Undo,
    NewGameConfirm,
    BackToMenu,
    // Settings
    Difficulty,
    Easy,
    Medium,
    Hard,
    Expert,
    BoardTheme,
    SoundVolume,
    AnimationSpeed,
    // Common
    Yes,
    No,
    Cancel,
    Ok,
    Save,
    Load,
}

/// I18n resource.
#[derive(Resource, Debug, Clone)]
pub struct I18n {
    pub language: Language,
    translations: HashMap<(Language, I18nKey), &'static str>,
}

impl Default for I18n {
    fn default() -> Self {
        let mut translations = HashMap::new();

        // Chinese Simplified
        translations.insert((Language::ChineseSimplified, I18nKey::NewGame), "新对局");
        translations.insert((Language::ChineseSimplified, I18nKey::VsAi), "人机对弈");
        translations.insert((Language::ChineseSimplified, I18nKey::LocalPvp), "本地对弈");
        translations.insert(
            (Language::ChineseSimplified, I18nKey::LanGame),
            "局域网对弈",
        );
        translations.insert(
            (Language::ChineseSimplified, I18nKey::OnlineGame),
            "在线对弈",
        );
        translations.insert((Language::ChineseSimplified, I18nKey::Settings), "设置");
        translations.insert((Language::ChineseSimplified, I18nKey::Quit), "退出");
        translations.insert(
            (Language::ChineseSimplified, I18nKey::YourTurn),
            "轮到你走棋",
        );
        translations.insert(
            (Language::ChineseSimplified, I18nKey::AiThinking),
            "AI思考中...",
        );
        translations.insert((Language::ChineseSimplified, I18nKey::Check), "将军");
        translations.insert((Language::ChineseSimplified, I18nKey::Checkmate), "将杀");
        translations.insert((Language::ChineseSimplified, I18nKey::Stalemate), "逼和");
        translations.insert((Language::ChineseSimplified, I18nKey::Draw), "和棋");
        translations.insert((Language::ChineseSimplified, I18nKey::Resign), "认输");
        translations.insert((Language::ChineseSimplified, I18nKey::Undo), "悔棋");
        translations.insert(
            (Language::ChineseSimplified, I18nKey::BackToMenu),
            "返回菜单",
        );
        translations.insert((Language::ChineseSimplified, I18nKey::Difficulty), "难度");
        translations.insert((Language::ChineseSimplified, I18nKey::Easy), "简单");
        translations.insert((Language::ChineseSimplified, I18nKey::Medium), "中等");
        translations.insert((Language::ChineseSimplified, I18nKey::Hard), "困难");
        translations.insert((Language::ChineseSimplified, I18nKey::Expert), "专家");
        translations.insert(
            (Language::ChineseSimplified, I18nKey::BoardTheme),
            "棋盘主题",
        );
        translations.insert((Language::ChineseSimplified, I18nKey::SoundVolume), "音量");
        translations.insert(
            (Language::ChineseSimplified, I18nKey::AnimationSpeed),
            "动画速度",
        );
        translations.insert((Language::ChineseSimplified, I18nKey::Yes), "是");
        translations.insert((Language::ChineseSimplified, I18nKey::No), "否");
        translations.insert((Language::ChineseSimplified, I18nKey::Cancel), "取消");
        translations.insert((Language::ChineseSimplified, I18nKey::Ok), "确定");
        translations.insert((Language::ChineseSimplified, I18nKey::Save), "保存");
        translations.insert((Language::ChineseSimplified, I18nKey::Load), "加载");

        // English
        translations.insert((Language::English, I18nKey::NewGame), "New Game");
        translations.insert((Language::English, I18nKey::VsAi), "vs AI");
        translations.insert((Language::English, I18nKey::LocalPvp), "Local PvP");
        translations.insert((Language::English, I18nKey::LanGame), "LAN Game");
        translations.insert((Language::English, I18nKey::OnlineGame), "Online Game");
        translations.insert((Language::English, I18nKey::Settings), "Settings");
        translations.insert((Language::English, I18nKey::Quit), "Quit");
        translations.insert((Language::English, I18nKey::YourTurn), "Your turn");
        translations.insert((Language::English, I18nKey::AiThinking), "AI thinking...");
        translations.insert((Language::English, I18nKey::Check), "Check");
        translations.insert((Language::English, I18nKey::Checkmate), "Checkmate");
        translations.insert((Language::English, I18nKey::Stalemate), "Stalemate");
        translations.insert((Language::English, I18nKey::Draw), "Draw");
        translations.insert((Language::English, I18nKey::Resign), "Resign");
        translations.insert((Language::English, I18nKey::Undo), "Undo");
        translations.insert((Language::English, I18nKey::BackToMenu), "Back to Menu");
        translations.insert((Language::English, I18nKey::Difficulty), "Difficulty");
        translations.insert((Language::English, I18nKey::Easy), "Easy");
        translations.insert((Language::English, I18nKey::Medium), "Medium");
        translations.insert((Language::English, I18nKey::Hard), "Hard");
        translations.insert((Language::English, I18nKey::Expert), "Expert");
        translations.insert((Language::English, I18nKey::BoardTheme), "Board Theme");
        translations.insert((Language::English, I18nKey::SoundVolume), "Volume");
        translations.insert(
            (Language::English, I18nKey::AnimationSpeed),
            "Animation Speed",
        );
        translations.insert((Language::English, I18nKey::Yes), "Yes");
        translations.insert((Language::English, I18nKey::No), "No");
        translations.insert((Language::English, I18nKey::Cancel), "Cancel");
        translations.insert((Language::English, I18nKey::Ok), "OK");
        translations.insert((Language::English, I18nKey::Save), "Save");
        translations.insert((Language::English, I18nKey::Load), "Load");

        // Chinese Traditional
        translations.insert((Language::ChineseTraditional, I18nKey::NewGame), "新對局");
        translations.insert((Language::ChineseTraditional, I18nKey::VsAi), "人機對弈");
        translations.insert(
            (Language::ChineseTraditional, I18nKey::LocalPvp),
            "本地對弈",
        );
        translations.insert((Language::ChineseTraditional, I18nKey::Check), "將軍");
        translations.insert((Language::ChineseTraditional, I18nKey::Checkmate), "將殺");
        translations.insert((Language::ChineseTraditional, I18nKey::Resign), "認輸");
        translations.insert((Language::ChineseTraditional, I18nKey::Undo), "悔棋");
        translations.insert((Language::ChineseTraditional, I18nKey::Settings), "設定");
        translations.insert((Language::ChineseTraditional, I18nKey::Quit), "退出");
        translations.insert((Language::ChineseTraditional, I18nKey::Yes), "是");
        translations.insert((Language::ChineseTraditional, I18nKey::No), "否");
        translations.insert((Language::ChineseTraditional, I18nKey::Cancel), "取消");
        translations.insert((Language::ChineseTraditional, I18nKey::Ok), "確定");

        // Japanese (subset)
        translations.insert((Language::Japanese, I18nKey::NewGame), "新しい対局");
        translations.insert((Language::Japanese, I18nKey::VsAi), "AI対戦");
        translations.insert((Language::Japanese, I18nKey::Settings), "設定");
        translations.insert((Language::Japanese, I18nKey::Quit), "終了");
        translations.insert((Language::Japanese, I18nKey::Check), "王手");
        translations.insert((Language::Japanese, I18nKey::Checkmate), "詰み");
        translations.insert((Language::Japanese, I18nKey::Resign), "投了");
        translations.insert((Language::Japanese, I18nKey::Yes), "はい");
        translations.insert((Language::Japanese, I18nKey::No), "いいえ");
        translations.insert((Language::Japanese, I18nKey::Cancel), "キャンセル");
        translations.insert((Language::Japanese, I18nKey::Ok), "OK");

        Self {
            language: Language::default(),
            translations,
        }
    }
}

impl I18n {
    /// Get translated string for a key.
    pub fn t(&self, key: I18nKey) -> &str {
        self.translations
            .get(&(self.language, key))
            .or_else(|| self.translations.get(&(Language::ChineseSimplified, key)))
            .unwrap_or(&"???")
    }

    /// Switch language.
    pub fn set_language(&mut self, lang: Language) {
        self.language = lang;
    }

    /// Get number of translated keys for current language.
    pub fn translation_coverage(&self) -> (usize, usize) {
        let total = self
            .translations
            .iter()
            .filter(|((lang, _), _)| *lang == Language::ChineseSimplified)
            .count();
        let current = self
            .translations
            .iter()
            .filter(|((lang, _), _)| *lang == self.language)
            .count();
        (current, total)
    }
}

pub fn cycle_language(
    keys: Res<ButtonInput<KeyCode>>,
    mut i18n: ResMut<I18n>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyI) {
        i18n.language = i18n.language.next();
        let (current, total) = i18n.translation_coverage();
        let msg = format!("语言: {} ({}/{})", i18n.language.label(), current, total);
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_language() {
        let i = I18n::default();
        assert_eq!(i.language, Language::ChineseSimplified);
    }

    #[test]
    fn test_translate_chinese() {
        let i = I18n::default();
        assert_eq!(i.t(I18nKey::NewGame), "新对局");
        assert_eq!(i.t(I18nKey::Check), "将军");
    }

    #[test]
    fn test_translate_english() {
        let mut i = I18n::default();
        i.set_language(Language::English);
        assert_eq!(i.t(I18nKey::NewGame), "New Game");
        assert_eq!(i.t(I18nKey::Check), "Check");
    }

    #[test]
    fn test_fallback_to_chinese() {
        let mut i = I18n::default();
        i.set_language(Language::Japanese);
        // NewGame is translated in Japanese
        assert_eq!(i.t(I18nKey::NewGame), "新しい対局");
        // VsAi is NOT translated in Japanese, falls back to Chinese
        // VsAi IS in Japanese, test a key that is NOT: LocalPvp
        assert_eq!(i.t(I18nKey::LocalPvp), "本地对弈");
    }

    #[test]
    fn test_language_cycle() {
        let mut lang = Language::ChineseSimplified;
        lang = lang.next();
        assert_eq!(lang, Language::ChineseTraditional);
        lang = lang.next();
        assert_eq!(lang, Language::English);
        lang = lang.next();
        assert_eq!(lang, Language::Japanese);
        lang = lang.next();
        assert_eq!(lang, Language::ChineseSimplified);
    }

    #[test]
    fn test_translation_coverage() {
        let mut i = I18n::default();
        let (current, total) = i.translation_coverage();
        assert_eq!(current, total); // Chinese is complete

        i.set_language(Language::English);
        let (current, total) = i.translation_coverage();
        assert_eq!(current, total); // English is also complete

        i.set_language(Language::Japanese);
        let (current, total) = i.translation_coverage();
        assert!(current < total); // Japanese is partial
    }

    #[test]
    fn test_language_labels() {
        assert_eq!(Language::ChineseSimplified.label(), "简体中文");
        assert_eq!(Language::English.label(), "English");
        assert_eq!(Language::Japanese.label(), "日本語");
    }
}
