//! Comprehensive keyboard shortcuts help system.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct Shortcut {
    pub key: String,
    pub description: String,
    pub category: String,
}

#[derive(Resource, Debug, Clone)]
pub struct KeyboardShortcutsHelp {
    pub visible: bool,
    pub shortcuts: Vec<Shortcut>,
}

impl Default for KeyboardShortcutsHelp {
    fn default() -> Self {
        Self {
            visible: false,
            shortcuts: vec![
                Shortcut {
                    key: "Ctrl+P".to_string(),
                    description: "切换棋子样式".to_string(),
                    category: "棋盘".to_string(),
                },
                Shortcut {
                    key: "Ctrl+B".to_string(),
                    description: "切换盲棋模式".to_string(),
                    category: "棋盘".to_string(),
                },
                Shortcut {
                    key: "Ctrl+R".to_string(),
                    description: "切换回放模式".to_string(),
                    category: "游戏".to_string(),
                },
                Shortcut {
                    key: "Ctrl+A".to_string(),
                    description: "分析模式".to_string(),
                    category: "分析".to_string(),
                },
                Shortcut {
                    key: "Ctrl+N".to_string(),
                    description: "中文棋谱".to_string(),
                    category: "显示".to_string(),
                },
                Shortcut {
                    key: "Ctrl+O".to_string(),
                    description: "开局练习".to_string(),
                    category: "练习".to_string(),
                },
                Shortcut {
                    key: "Ctrl+H".to_string(),
                    description: "提示系统".to_string(),
                    category: "帮助".to_string(),
                },
                Shortcut {
                    key: "Ctrl+J".to_string(),
                    description: "游戏日记".to_string(),
                    category: "记录".to_string(),
                },
                Shortcut {
                    key: "F1".to_string(),
                    description: "快捷键帮助".to_string(),
                    category: "帮助".to_string(),
                },
                Shortcut {
                    key: "Space".to_string(),
                    description: "播放/暂停回放".to_string(),
                    category: "回放".to_string(),
                },
                Shortcut {
                    key: "←/→".to_string(),
                    description: "前/后一步".to_string(),
                    category: "回放".to_string(),
                },
                Shortcut {
                    key: "Home/End".to_string(),
                    description: "跳到开始/结束".to_string(),
                    category: "回放".to_string(),
                },
            ],
        }
    }
}

impl KeyboardShortcutsHelp {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    pub fn shortcuts_by_category(&self) -> Vec<(String, Vec<&Shortcut>)> {
        let mut categories = std::collections::HashMap::new();
        for s in &self.shortcuts {
            categories
                .entry(s.category.clone())
                .or_insert_with(Vec::new)
                .push(s);
        }
        categories.into_iter().collect()
    }
}

pub fn toggle_help(
    keys: Res<ButtonInput<KeyCode>>,
    mut help: ResMut<KeyboardShortcutsHelp>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    if keys.just_pressed(KeyCode::F1) {
        help.toggle();
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if help.visible {
                "快捷键帮助已打开 (按F1关闭)"
            } else {
                "快捷键帮助已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() {
        let h = KeyboardShortcutsHelp::default();
        assert!(!h.visible);
        assert!(!h.shortcuts.is_empty());
    }
    #[test]
    fn test_toggle() {
        let mut h = KeyboardShortcutsHelp::default();
        h.toggle();
        assert!(h.visible);
    }
    #[test]
    fn test_by_category() {
        let h = KeyboardShortcutsHelp::default();
        let cats = h.shortcuts_by_category();
        assert!(!cats.is_empty());
    }
}
