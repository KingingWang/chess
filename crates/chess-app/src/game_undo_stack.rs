//! Enhanced undo stack with redo support.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub move_str: String,
    pub move_idx: usize,
    pub fen: String,
}

#[derive(Resource, Debug, Clone)]
pub struct GameUndoStack {
    pub undo_stack: Vec<UndoEntry>,
    pub redo_stack: Vec<UndoEntry>,
    pub max_size: usize,
}

impl Default for GameUndoStack {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size: 100,
        }
    }
}

impl GameUndoStack {
    pub fn push(&mut self, entry: UndoEntry) {
        self.redo_stack.clear();
        self.undo_stack.push(entry);
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }
    pub fn undo(&mut self) -> Option<UndoEntry> {
        self.undo_stack
            .pop()
            .inspect(|e| self.redo_stack.push(e.clone()))
    }
    pub fn redo(&mut self) -> Option<UndoEntry> {
        self.redo_stack
            .pop()
            .inspect(|e| self.undo_stack.push(e.clone()))
    }
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

pub fn undo_redo_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut stack: ResMut<GameUndoStack>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyZ) && !keys.pressed(KeyCode::ShiftLeft) {
        if stack.can_undo() {
            stack.undo();
            crate::toast::spawn_toast(&mut commands, &fonts, "悔棋");
        }
    }
    if ctrl && keys.just_pressed(KeyCode::KeyY)
        || (ctrl && keys.pressed(KeyCode::ShiftLeft) && keys.just_pressed(KeyCode::KeyZ))
    {
        if stack.can_redo() {
            stack.redo();
            crate::toast::spawn_toast(&mut commands, &fonts, "重做");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_undo_redo() {
        let mut stack = GameUndoStack::default();
        stack.push(UndoEntry {
            move_str: "h2e2".to_string(),
            move_idx: 0,
            fen: "".to_string(),
        });
        assert!(stack.can_undo());
        stack.undo();
        assert!(stack.can_redo());
        stack.redo();
        assert!(stack.can_undo());
    }
}
