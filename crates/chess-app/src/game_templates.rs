//! Game templates for preset openings and scenarios.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct GameTemplate {
    pub name_cn: String,
    pub description: String,
    pub fen: String,
    pub category: TemplateCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateCategory {
    Opening,
    Middlegame,
    Endgame,
    Puzzle,
    Famous,
}

impl TemplateCategory {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Opening => "开局",
            Self::Middlegame => "中局",
            Self::Endgame => "残局",
            Self::Puzzle => "题目",
            Self::Famous => "名局",
        }
    }
}

#[derive(Resource, Debug)]
pub struct GameTemplates {
    pub templates: Vec<GameTemplate>,
    pub selected: Option<usize>,
}

impl Default for GameTemplates {
    fn default() -> Self {
        Self {
            templates: vec![
                GameTemplate {
                    name_cn: "初始局面".to_string(),
                    description: "标准开局".to_string(),
                    fen: "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1"
                        .to_string(),
                    category: TemplateCategory::Opening,
                },
                GameTemplate {
                    name_cn: "单车杀王".to_string(),
                    description: "练习单车杀王技巧".to_string(),
                    fen: "4k4/9/9/9/9/9/9/4R4/9/4K4 w - - 0 1".to_string(),
                    category: TemplateCategory::Endgame,
                },
                GameTemplate {
                    name_cn: "双车杀王".to_string(),
                    description: "双车配合杀王".to_string(),
                    fen: "4k4/9/9/9/9/9/9/R8/8R/4K4 w - - 0 1".to_string(),
                    category: TemplateCategory::Endgame,
                },
                GameTemplate {
                    name_cn: "炮仕杀王".to_string(),
                    description: "炮仕配合杀王".to_string(),
                    fen: "4k4/9/9/9/9/9/9/4C4/4A4/4K4 w - - 0 1".to_string(),
                    category: TemplateCategory::Endgame,
                },
            ],
            selected: None,
        }
    }
}

impl GameTemplates {
    pub fn select(&mut self, idx: usize) {
        if idx < self.templates.len() {
            self.selected = Some(idx);
        }
    }
    pub fn selected_template(&self) -> Option<&GameTemplate> {
        self.selected.and_then(|i| self.templates.get(i))
    }
    pub fn templates_by_category(&self) -> Vec<(TemplateCategory, Vec<&GameTemplate>)> {
        let cats = [
            TemplateCategory::Opening,
            TemplateCategory::Middlegame,
            TemplateCategory::Endgame,
            TemplateCategory::Puzzle,
            TemplateCategory::Famous,
        ];
        cats.iter()
            .map(|&c| {
                (
                    c,
                    self.templates
                        .iter()
                        .filter(|t| t.category == c)
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, v)| !v.is_empty())
            .collect()
    }
}

pub fn toggle_templates(
    keys: Res<ButtonInput<KeyCode>>,
    mut t: ResMut<GameTemplates>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyG) {
        t.selected = None;
        let count = t.templates.len();
        crate::toast::spawn_toast(&mut commands, &fonts, &format!("游戏模板: {}个可用", count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() {
        let t = GameTemplates::default();
        assert!(!t.templates.is_empty());
    }
    #[test]
    fn test_select() {
        let mut t = GameTemplates::default();
        t.select(0);
        assert!(t.selected_template().is_some());
    }
    #[test]
    fn test_invalid_select() {
        let mut t = GameTemplates::default();
        t.select(999);
        assert!(t.selected_template().is_none());
    }
    #[test]
    fn test_by_category() {
        let t = GameTemplates::default();
        let cats = t.templates_by_category();
        assert!(!cats.is_empty());
    }
}
