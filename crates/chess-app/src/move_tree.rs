//! Move tree visualization for variation analysis.
//!
//! Allows users to explore alternative move sequences during analysis mode.

use bevy::prelude::*;
use chess_core::Move;

/// A node in the move tree.
#[derive(Debug, Clone)]
pub struct MoveNode {
    /// The move that led to this position (None for root).
    pub mv: Option<Move>,
    /// The ply number (0 for root).
    pub ply: usize,
    /// Child variations.
    pub children: Vec<MoveNode>,
    /// Comment/annotation for this move.
    pub comment: Option<String>,
    /// Whether this is the main line.
    pub is_main_line: bool,
}

impl MoveNode {
    /// Create a new root node.
    pub fn new_root() -> Self {
        Self {
            mv: None,
            ply: 0,
            children: Vec::new(),
            comment: None,
            is_main_line: true,
        }
    }

    /// Add a child move to this node.
    pub fn add_child(&mut self, mv: Move, is_main_line: bool) -> &mut MoveNode {
        self.children.push(MoveNode {
            mv: Some(mv),
            ply: self.ply + 1,
            children: Vec::new(),
            comment: None,
            is_main_line,
        });
        self.children.last_mut().unwrap()
    }

    /// Find a child node by move.
    pub fn find_child(&self, mv: Move) -> Option<&MoveNode> {
        self.children.iter().find(|child| child.mv == Some(mv))
    }

    /// Find a mutable child node by move.
    pub fn find_child_mut(&mut self, mv: Move) -> Option<&mut MoveNode> {
        self.children.iter_mut().find(|child| child.mv == Some(mv))
    }

    /// Get the total number of nodes in this subtree.
    pub fn count_nodes(&self) -> usize {
        1 + self.children.iter().map(|c| c.count_nodes()).sum::<usize>()
    }

    /// Get the maximum depth of this subtree.
    pub fn max_depth(&self) -> usize {
        if self.children.is_empty() {
            self.ply
        } else {
            self.children
                .iter()
                .map(|c| c.max_depth())
                .max()
                .unwrap_or(self.ply)
        }
    }
}

/// Resource storing the move tree for the current game.
#[derive(Resource)]
pub struct MoveTree {
    /// The root node of the tree.
    pub root: MoveNode,
    /// Current path through the tree (indices into children arrays).
    pub current_path: Vec<usize>,
    /// Whether the tree view is visible.
    pub visible: bool,
}

impl Default for MoveTree {
    fn default() -> Self {
        Self {
            root: MoveNode::new_root(),
            current_path: Vec::new(),
            visible: false,
        }
    }
}

impl MoveTree {
    /// Get the current node based on the current path.
    pub fn current_node(&self) -> &MoveNode {
        let mut node = &self.root;
        for &index in &self.current_path {
            node = &node.children[index];
        }
        node
    }

    /// Get a mutable reference to the current node.
    pub fn current_node_mut(&mut self) -> &mut MoveNode {
        let mut node = &mut self.root;
        for &index in &self.current_path {
            node = &mut node.children[index];
        }
        node
    }

    /// Navigate to a child node.
    pub fn navigate_to_child(&mut self, child_index: usize) -> bool {
        let current = self.current_node();
        if child_index < current.children.len() {
            self.current_path.push(child_index);
            true
        } else {
            false
        }
    }

    /// Navigate back to parent.
    pub fn navigate_to_parent(&mut self) -> bool {
        if !self.current_path.is_empty() {
            self.current_path.pop();
            true
        } else {
            false
        }
    }

    /// Navigate to root.
    pub fn navigate_to_root(&mut self) {
        self.current_path.clear();
    }

    /// Add a move to the current node.
    pub fn add_move(&mut self, mv: Move) -> bool {
        // Check if this move already exists
        let existing_index = {
            let current = self.current_node();
            current.children.iter().position(|c| c.mv == Some(mv))
        };

        if let Some(index) = existing_index {
            // Navigate to existing node
            self.current_path.push(index);
            return true;
        }

        // Add as new child
        let (_is_main_line, new_index) = {
            let current = self.current_node_mut();
            let is_main_line = current.children.is_empty();
            current.add_child(mv, is_main_line);
            (is_main_line, current.children.len() - 1)
        };

        self.current_path.push(new_index);
        true
    }

    /// Get the moves in the current path.
    pub fn get_current_line(&self) -> Vec<Move> {
        let mut moves = Vec::new();
        let mut node = &self.root;
        for &index in &self.current_path {
            if let Some(mv) = node.children[index].mv {
                moves.push(mv);
            }
            node = &node.children[index];
        }
        moves
    }

    /// Toggle tree visibility.
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    /// Clear the tree.
    pub fn clear(&mut self) {
        self.root = MoveNode::new_root();
        self.current_path.clear();
    }
}

/// Marker component for move tree UI elements.
#[derive(Component)]
pub struct MoveTreeUI;

/// Component for a move node button in the UI.
#[derive(Component)]
pub struct MoveNodeButton {
    pub path: Vec<usize>,
}

/// System to toggle move tree visibility.
pub fn toggle_move_tree_visibility(
    mut move_tree: ResMut<MoveTree>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyV) {
        move_tree.toggle_visibility();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Square;

    fn create_test_move() -> Move {
        Move {
            from: Square::new(0, 0).unwrap(),
            to: Square::new(0, 1).unwrap(),
        }
    }

    #[test]
    fn test_new_root() {
        let root = MoveNode::new_root();
        assert_eq!(root.ply, 0);
        assert!(root.mv.is_none());
        assert!(root.children.is_empty());
        assert!(root.is_main_line);
    }

    #[test]
    fn test_add_child() {
        let mut root = MoveNode::new_root();
        let mv = create_test_move();

        root.add_child(mv, true);

        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].ply, 1);
        assert_eq!(root.children[0].mv, Some(mv));
    }

    #[test]
    fn test_find_child() {
        let mut root = MoveNode::new_root();
        let mv1 = create_test_move();
        let mv2 = Move {
            from: Square::new(1, 0).unwrap(),
            to: Square::new(1, 1).unwrap(),
        };

        root.add_child(mv1, true);
        root.add_child(mv2, false);

        assert!(root.find_child(mv1).is_some());
        assert!(root.find_child(mv2).is_some());
    }

    #[test]
    fn test_count_nodes() {
        let mut root = MoveNode::new_root();
        let mv1 = create_test_move();

        root.add_child(mv1, true);
        root.children[0].add_child(mv1, true);

        assert_eq!(root.count_nodes(), 3); // root + 2 children
    }

    #[test]
    fn test_max_depth() {
        let mut root = MoveNode::new_root();
        let mv1 = create_test_move();

        root.add_child(mv1, true);
        root.children[0].add_child(mv1, true);
        root.children[0].children[0].add_child(mv1, true);

        assert_eq!(root.max_depth(), 3);
    }

    #[test]
    fn test_move_tree_navigation() {
        let mut tree = MoveTree::default();
        let mv1 = create_test_move();

        tree.add_move(mv1);
        assert_eq!(tree.current_path, vec![0]);

        tree.navigate_to_parent();
        assert!(tree.current_path.is_empty());

        tree.navigate_to_child(0);
        assert_eq!(tree.current_path, vec![0]);

        tree.navigate_to_root();
        assert!(tree.current_path.is_empty());
    }

    #[test]
    fn test_get_current_line() {
        let mut tree = MoveTree::default();
        let mv1 = create_test_move();
        let mv2 = Move {
            from: Square::new(1, 0).unwrap(),
            to: Square::new(1, 1).unwrap(),
        };

        tree.add_move(mv1);
        tree.add_move(mv2);

        let line = tree.get_current_line();
        assert_eq!(line.len(), 2);
        assert_eq!(line[0], mv1);
        assert_eq!(line[1], mv2);
    }

    #[test]
    fn test_add_duplicate_move() {
        let mut tree = MoveTree::default();
        let mv1 = create_test_move();

        tree.add_move(mv1);
        tree.navigate_to_parent();
        tree.add_move(mv1); // Add same move again

        // Should navigate to existing node, not create duplicate
        assert_eq!(tree.root.children.len(), 1);
        assert_eq!(tree.current_path, vec![0]);
    }

    #[test]
    fn test_clear_tree() {
        let mut tree = MoveTree::default();
        let mv1 = create_test_move();

        tree.add_move(mv1);
        tree.clear();

        assert!(tree.root.children.is_empty());
        assert!(tree.current_path.is_empty());
    }

    #[test]
    fn test_toggle_visibility() {
        let mut tree = MoveTree::default();
        assert!(!tree.visible);

        tree.toggle_visibility();
        assert!(tree.visible);

        tree.toggle_visibility();
        assert!(!tree.visible);
    }
}
