//! Opening tree browser for exploring opening variations.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OpeningNode {
    pub move_str: String,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub children: HashMap<String, usize>,
}

impl OpeningNode {
    pub fn new(mv: &str) -> Self {
        Self {
            move_str: mv.to_string(),
            games: 0,
            wins: 0,
            losses: 0,
            draws: 0,
            children: HashMap::new(),
        }
    }
    pub fn win_rate(&self) -> f32 {
        if self.games == 0 {
            0.0
        } else {
            self.wins as f32 / self.games as f32 * 100.0
        }
    }
}

#[derive(Resource, Debug)]
pub struct OpeningTree {
    pub nodes: Vec<OpeningNode>,
    pub current_path: Vec<usize>,
    pub visible: bool,
}

impl Default for OpeningTree {
    fn default() -> Self {
        let mut tree = Self {
            nodes: vec![OpeningNode::new("root")],
            current_path: vec![0],
            visible: false,
        };
        tree.add_move("root", "h2e2");
        tree.add_move("root", "c3c4");
        tree.add_move("root", "b0c2");
        tree.add_move("root", "c0e2");
        tree
    }
}

impl OpeningTree {
    pub fn add_move(&mut self, parent: &str, mv: &str) {
        let parent_idx = self.nodes.iter().position(|n| n.move_str == parent);
        let Some(p_idx) = parent_idx else {
            return;
        };
        if !self.nodes.iter().any(|n| n.move_str == mv) {
            let idx = self.nodes.len();
            self.nodes.push(OpeningNode::new(mv));
            self.nodes[p_idx].children.insert(mv.to_string(), idx);
        }
    }
    pub fn record_game(&mut self, moves: &[&str], result: f32) {
        for (i, &mv) in moves.iter().enumerate() {
            let parent = if i == 0 { "root" } else { moves[i - 1] };
            self.add_move(parent, mv);
            if let Some(node) = self.nodes.iter_mut().find(|n| n.move_str == mv) {
                node.games += 1;
                if result > 0.5 {
                    node.wins += 1;
                } else if result < 0.5 {
                    node.losses += 1;
                } else {
                    node.draws += 1;
                }
            }
        }
    }
    pub fn current_node(&self) -> Option<&OpeningNode> {
        self.current_path.last().and_then(|&i| self.nodes.get(i))
    }
    pub fn children_of_current(&self) -> Vec<&OpeningNode> {
        let Some(node) = self.current_node() else {
            return Vec::new();
        };
        node.children
            .values()
            .filter_map(|&i| self.nodes.get(i))
            .collect()
    }
}

pub fn toggle_opening_tree(
    keys: Res<ButtonInput<KeyCode>>,
    mut ot: ResMut<OpeningTree>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyO) {
        ot.visible = !ot.visible;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if ot.visible {
                "开局树浏览器已打开"
            } else {
                "开局树浏览器已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() {
        let t = OpeningTree::default();
        assert!(t.nodes.len() > 1);
    }
    #[test]
    fn test_record_game() {
        let mut t = OpeningTree::default();
        t.record_game(&["h2e2", "h8g7"], 1.0);
        let node = t.nodes.iter().find(|n| n.move_str == "h2e2").unwrap();
        assert_eq!(node.games, 1);
    }
    #[test]
    fn test_win_rate() {
        let n = OpeningNode {
            move_str: "test".to_string(),
            games: 10,
            wins: 7,
            losses: 2,
            draws: 1,
            children: HashMap::new(),
        };
        assert_eq!(n.win_rate(), 70.0);
    }
}
