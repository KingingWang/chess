//! Move tree visualization for exploring variations.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub move_str: String,
    pub comment: Option<String>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

#[derive(Resource, Debug, Clone)]
pub struct MoveTreeView {
    pub enabled: bool,
    pub nodes: Vec<TreeNode>,
    pub current_node: usize,
    pub root: usize,
}

impl Default for MoveTreeView {
    fn default() -> Self {
        Self {
            enabled: false,
            nodes: vec![TreeNode {
                move_str: "start".to_string(),
                comment: None,
                children: Vec::new(),
                parent: None,
            }],
            current_node: 0,
            root: 0,
        }
    }
}

impl MoveTreeView {
    pub fn add_variation(&mut self, parent: usize, move_str: &str) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(TreeNode {
            move_str: move_str.to_string(),
            comment: None,
            children: Vec::new(),
            parent: Some(parent),
        });
        self.nodes[parent].children.push(idx);
        idx
    }

    pub fn navigate_to(&mut self, node_idx: usize) -> bool {
        if node_idx < self.nodes.len() {
            self.current_node = node_idx;
            true
        } else {
            false
        }
    }

    pub fn current_move(&self) -> &str {
        self.nodes
            .get(self.current_node)
            .map(|n| n.move_str.as_str())
            .unwrap_or("")
    }

    pub fn variations_from_current(&self) -> Vec<(usize, &str)> {
        self.nodes
            .get(self.current_node)
            .map(|n| {
                n.children
                    .iter()
                    .map(|&idx| (idx, self.nodes[idx].move_str.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

pub fn toggle_tree_view(
    keys: Res<ButtonInput<KeyCode>>,
    mut tv: ResMut<MoveTreeView>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyT) {
        tv.enabled = !tv.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if tv.enabled {
                "着法树视图已开启"
            } else {
                "着法树视图已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_variation() {
        let mut tv = MoveTreeView::default();
        let idx = tv.add_variation(0, "h2e2");
        assert_eq!(idx, 1);
        assert_eq!(tv.nodes[1].move_str, "h2e2");
    }
    #[test]
    fn test_navigate() {
        let mut tv = MoveTreeView::default();
        tv.add_variation(0, "h2e2");
        assert!(tv.navigate_to(1));
        assert_eq!(tv.current_move(), "h2e2");
    }
}
