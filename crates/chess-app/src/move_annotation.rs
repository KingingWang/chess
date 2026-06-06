//! Move annotation system for game commentary.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Annotation {
    Excellent,
    Good,
    Interesting,
    Dubious,
    Mistake,
    Blunder,
    Novelty,
    WithCompensation,
}

impl Annotation {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Excellent => "!!",
            Self::Good => "!",
            Self::Interesting => "!?",
            Self::Dubious => "?!",
            Self::Mistake => "?",
            Self::Blunder => "??",
            Self::Novelty => "N",
            Self::WithCompensation => "∞",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct MoveAnnotations {
    pub annotations: HashMap<usize, Annotation>,
    pub comments: HashMap<usize, String>,
}

impl Default for MoveAnnotations {
    fn default() -> Self {
        Self {
            annotations: HashMap::new(),
            comments: HashMap::new(),
        }
    }
}

impl MoveAnnotations {
    pub fn annotate(&mut self, move_idx: usize, annotation: Annotation) {
        self.annotations.insert(move_idx, annotation);
    }
    pub fn comment(&mut self, move_idx: usize, text: &str) {
        self.comments.insert(move_idx, text.to_string());
    }
    pub fn get_annotation(&self, move_idx: usize) -> Option<Annotation> {
        self.annotations.get(&move_idx).copied()
    }
    pub fn get_comment(&self, move_idx: usize) -> Option<&str> {
        self.comments.get(&move_idx).map(|s| s.as_str())
    }
}

pub fn toggle_annotations(
    keys: Res<ButtonInput<KeyCode>>,
    mut ma: ResMut<MoveAnnotations>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyK) {
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("标注数: {}", ma.annotations.len()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_annotate() {
        let mut ma = MoveAnnotations::default();
        ma.annotate(0, Annotation::Excellent);
        assert_eq!(ma.get_annotation(0), Some(Annotation::Excellent));
    }
    #[test]
    fn test_symbols() {
        assert_eq!(Annotation::Excellent.symbol(), "!!");
        assert_eq!(Annotation::Blunder.symbol(), "??");
    }
}
