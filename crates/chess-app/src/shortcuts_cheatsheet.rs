//! Keyboard shortcuts cheatsheet overlay.

use bevy::prelude::*;

/// A single shortcut entry.
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub keys: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

/// Cheatsheet resource.
#[derive(Resource, Debug, Clone)]
pub struct ShortcutsCheatsheet {
    pub visible: bool,
    shortcuts: Vec<Shortcut>,
}

impl Default for ShortcutsCheatsheet {
    fn default() -> Self {
        Self {
            visible: false,
            shortcuts: vec![
                // Movement
                Shortcut {
                    keys: "Space",
                    description: "播放/暂停回放",
                    category: "回放",
                },
                Shortcut {
                    keys: "←/→",
                    description: "后退/前进一步",
                    category: "回放",
                },
                Shortcut {
                    keys: "Home/End",
                    description: "跳到开始/结束",
                    category: "回放",
                },
                // Board
                Shortcut {
                    keys: "Ctrl+P",
                    description: "切换棋子样式",
                    category: "棋盘",
                },
                Shortcut {
                    keys: "Ctrl+B",
                    description: "切换盲棋模式",
                    category: "棋盘",
                },
                Shortcut {
                    keys: "Ctrl+L",
                    description: "切换坐标样式",
                    category: "棋盘",
                },
                Shortcut {
                    keys: "+/-",
                    description: "放大/缩小平盘",
                    category: "棋盘",
                },
                // Game
                Shortcut {
                    keys: "Ctrl+R",
                    description: "切换回放模式",
                    category: "游戏",
                },
                Shortcut {
                    keys: "Ctrl+O",
                    description: "开局练习",
                    category: "游戏",
                },
                Shortcut {
                    keys: "Ctrl+N",
                    description: "中文棋谱",
                    category: "游戏",
                },
                Shortcut {
                    keys: "Ctrl+H",
                    description: "提示系统",
                    category: "游戏",
                },
                Shortcut {
                    keys: "Ctrl+J",
                    description: "游戏日记",
                    category: "游戏",
                },
                // Analysis
                Shortcut {
                    keys: "Ctrl+A",
                    description: "分析模式",
                    category: "分析",
                },
                Shortcut {
                    keys: "Ctrl+G",
                    description: "统计图表",
                    category: "分析",
                },
                // Export
                Shortcut {
                    keys: "Ctrl+Shift+X",
                    description: "导出游戏",
                    category: "导出",
                },
                Shortcut {
                    keys: "Ctrl+C",
                    description: "复制FEN",
                    category: "剪贴板",
                },
                Shortcut {
                    keys: "Ctrl+V",
                    description: "粘贴FEN",
                    category: "剪贴板",
                },
                // Other
                Shortcut {
                    keys: "Ctrl+W",
                    description: "观战模式",
                    category: "其他",
                },
                Shortcut {
                    keys: "Ctrl+Shift+E",
                    description: "棋盘编辑器",
                    category: "其他",
                },
                Shortcut {
                    keys: "Ctrl+Shift+I",
                    description: "切换语言",
                    category: "其他",
                },
                Shortcut {
                    keys: "Ctrl+Shift+S",
                    description: "音效主题",
                    category: "其他",
                },
            ],
        }
    }
}

impl ShortcutsCheatsheet {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn shortcuts(&self) -> &[Shortcut] {
        &self.shortcuts
    }

    pub fn shortcuts_by_category(&self) -> Vec<(&str, Vec<&Shortcut>)> {
        let mut categories: Vec<&str> = self.shortcuts.iter().map(|s| s.category).collect();
        categories.dedup();

        categories
            .iter()
            .map(|cat| {
                let shorts: Vec<&Shortcut> = self
                    .shortcuts
                    .iter()
                    .filter(|s| s.category == *cat)
                    .collect();
                (*cat, shorts)
            })
            .collect()
    }
}

pub fn toggle_cheatsheet(
    keys: Res<ButtonInput<KeyCode>>,
    mut cheatsheet: ResMut<ShortcutsCheatsheet>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    if keys.just_pressed(KeyCode::F1) {
        cheatsheet.toggle();
        let msg = if cheatsheet.visible {
            "快捷键帮助 (F1关闭)"
        } else {
            "快捷键帮助已关闭"
        };
        crate::toast::spawn_toast(&mut commands, &fonts, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let c = ShortcutsCheatsheet::default();
        assert!(!c.visible);
        assert!(!c.shortcuts().is_empty());
    }

    #[test]
    fn test_toggle() {
        let mut c = ShortcutsCheatsheet::default();
        c.toggle();
        assert!(c.visible);
        c.toggle();
        assert!(!c.visible);
    }

    #[test]
    fn test_categories() {
        let c = ShortcutsCheatsheet::default();
        let cats = c.shortcuts_by_category();
        assert!(!cats.is_empty());
        // All shortcuts should be categorized
        let total: usize = cats.iter().map(|(_, v)| v.len()).sum();
        assert_eq!(total, c.shortcuts().len());
    }
}
