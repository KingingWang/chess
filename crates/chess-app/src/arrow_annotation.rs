//! Arrow annotations for annotating moves and strategies.
//!
//! Allows users to draw arrows on the board to highlight moves or ideas.

use bevy::prelude::*;
use chess_core::Square;

/// An arrow annotation on the board.
#[derive(Debug, Clone)]
pub struct Arrow {
    /// Starting square of the arrow.
    pub from: Square,
    /// Ending square of the arrow.
    pub to: Square,
    /// Color of the arrow.
    pub color: Color,
    /// Width of the arrow line.
    pub width: f32,
    /// Whether the arrow is persistent (saved) or temporary.
    pub persistent: bool,
}

impl Arrow {
    /// Create a new arrow.
    pub fn new(from: Square, to: Square, color: Color) -> Self {
        Self {
            from,
            to,
            color,
            width: 4.0,
            persistent: false,
        }
    }

    /// Create a new persistent arrow.
    pub fn persistent(from: Square, to: Square, color: Color) -> Self {
        Self {
            from,
            to,
            color,
            width: 4.0,
            persistent: true,
        }
    }

    /// Set the arrow width.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

/// Resource managing arrow annotations.
#[derive(Resource, Default)]
pub struct ArrowAnnotations {
    /// List of arrows on the board.
    pub arrows: Vec<Arrow>,
    /// Whether arrow drawing mode is active.
    pub drawing_mode: bool,
    /// Current arrow being drawn (start square).
    pub drawing_start: Option<Square>,
    /// Default color for new arrows.
    pub default_color: Color,
}

impl ArrowAnnotations {
    /// Add an arrow to the board.
    pub fn add_arrow(&mut self, arrow: Arrow) {
        self.arrows.push(arrow);
    }

    /// Remove an arrow by index.
    pub fn remove_arrow(&mut self, index: usize) -> Option<Arrow> {
        if index < self.arrows.len() {
            Some(self.arrows.remove(index))
        } else {
            None
        }
    }

    /// Remove all arrows.
    pub fn clear_all(&mut self) {
        self.arrows.clear();
    }

    /// Remove all temporary arrows.
    pub fn clear_temporary(&mut self) {
        self.arrows.retain(|arrow| arrow.persistent);
    }

    /// Remove arrows starting from a specific square.
    pub fn remove_arrows_from(&mut self, from: Square) {
        self.arrows.retain(|arrow| arrow.from != from);
    }

    /// Remove arrows ending at a specific square.
    pub fn remove_arrows_to(&mut self, to: Square) {
        self.arrows.retain(|arrow| arrow.to != to);
    }

    /// Get the number of arrows.
    pub fn count(&self) -> usize {
        self.arrows.len()
    }

    /// Check if there are any arrows.
    pub fn is_empty(&self) -> bool {
        self.arrows.is_empty()
    }

    /// Enable drawing mode.
    pub fn enable_drawing(&mut self) {
        self.drawing_mode = true;
    }

    /// Disable drawing mode.
    pub fn disable_drawing(&mut self) {
        self.drawing_mode = false;
        self.drawing_start = None;
    }

    /// Toggle drawing mode.
    pub fn toggle_drawing(&mut self) {
        if self.drawing_mode {
            self.disable_drawing();
        } else {
            self.enable_drawing();
        }
    }

    /// Start drawing an arrow from a square.
    pub fn start_drawing(&mut self, from: Square) {
        if self.drawing_mode {
            self.drawing_start = Some(from);
        }
    }

    /// Finish drawing an arrow to a square.
    pub fn finish_drawing(&mut self, to: Square) -> Option<Arrow> {
        if let Some(from) = self.drawing_start {
            if from != to {
                let arrow = Arrow::new(from, to, self.default_color);
                self.add_arrow(arrow.clone());
                self.drawing_start = None;
                return Some(arrow);
            }
        }
        self.drawing_start = None;
        None
    }

    /// Cancel the current drawing.
    pub fn cancel_drawing(&mut self) {
        self.drawing_start = None;
    }

    /// Set the default arrow color.
    pub fn set_default_color(&mut self, color: Color) {
        self.default_color = color;
    }
}

#[derive(Component)]
pub struct ArrowVisual {
    pub arrow_index: usize,
}

/// System to toggle arrow drawing mode.
pub fn toggle_arrow_drawing(
    mut annotations: ResMut<ArrowAnnotations>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyA) {
        annotations.toggle_drawing();
    }
}

/// System to clear all arrows.
pub fn clear_arrows(
    mut annotations: ResMut<ArrowAnnotations>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyC) {
        annotations.clear_all();
    }
}

/// System to update arrow visuals.
pub fn update_arrow_visuals(
    mut commands: Commands,
    annotations: Res<ArrowAnnotations>,
    existing_arrows: Query<(Entity, &ArrowVisual)>,
) {
    // Remove existing arrow visuals
    for (entity, _) in existing_arrows.iter() {
        commands.entity(entity).despawn();
    }

    // Spawn new arrow visuals
    for (index, arrow) in annotations.arrows.iter().enumerate() {
        // TODO: Spawn arrow visual entity
        // This would involve calculating positions and creating line/sprite entities
        let _ = (index, arrow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_square(file: u8, rank: u8) -> Square {
        Square::new(file, rank).unwrap()
    }

    #[test]
    fn test_arrow_creation() {
        let from = test_square(0, 0);
        let to = test_square(1, 1);
        let color = Color::srgb(1.0, 0.0, 0.0);

        let arrow = Arrow::new(from, to, color);
        assert_eq!(arrow.from, from);
        assert_eq!(arrow.to, to);
        assert!(!arrow.persistent);
    }

    #[test]
    fn test_persistent_arrow() {
        let from = test_square(0, 0);
        let to = test_square(1, 1);
        let color = Color::srgb(1.0, 0.0, 0.0);

        let arrow = Arrow::persistent(from, to, color);
        assert!(arrow.persistent);
    }

    #[test]
    fn test_arrow_with_width() {
        let from = test_square(0, 0);
        let to = test_square(1, 1);
        let color = Color::srgb(1.0, 0.0, 0.0);

        let arrow = Arrow::new(from, to, color).with_width(8.0);
        assert_eq!(arrow.width, 8.0);
    }

    #[test]
    fn test_add_arrow() {
        let mut annotations = ArrowAnnotations::default();
        let arrow = Arrow::new(test_square(0, 0), test_square(1, 1), Color::WHITE);

        annotations.add_arrow(arrow);
        assert_eq!(annotations.count(), 1);
    }

    #[test]
    fn test_remove_arrow() {
        let mut annotations = ArrowAnnotations::default();
        let arrow = Arrow::new(test_square(0, 0), test_square(1, 1), Color::WHITE);

        annotations.add_arrow(arrow);
        let removed = annotations.remove_arrow(0);

        assert!(removed.is_some());
        assert_eq!(annotations.count(), 0);
    }

    #[test]
    fn test_clear_all() {
        let mut annotations = ArrowAnnotations::default();
        annotations.add_arrow(Arrow::new(
            test_square(0, 0),
            test_square(1, 1),
            Color::WHITE,
        ));
        annotations.add_arrow(Arrow::new(
            test_square(2, 2),
            test_square(3, 3),
            Color::WHITE,
        ));

        annotations.clear_all();
        assert!(annotations.is_empty());
    }

    #[test]
    fn test_clear_temporary() {
        let mut annotations = ArrowAnnotations::default();
        annotations.add_arrow(Arrow::new(
            test_square(0, 0),
            test_square(1, 1),
            Color::WHITE,
        ));
        annotations.add_arrow(Arrow::persistent(
            test_square(2, 2),
            test_square(3, 3),
            Color::WHITE,
        ));

        annotations.clear_temporary();
        assert_eq!(annotations.count(), 1);
        assert!(annotations.arrows[0].persistent);
    }

    #[test]
    fn test_remove_arrows_from() {
        let mut annotations = ArrowAnnotations::default();
        let sq = test_square(0, 0);
        annotations.add_arrow(Arrow::new(sq, test_square(1, 1), Color::WHITE));
        annotations.add_arrow(Arrow::new(
            test_square(2, 2),
            test_square(3, 3),
            Color::WHITE,
        ));

        annotations.remove_arrows_from(sq);
        assert_eq!(annotations.count(), 1);
    }

    #[test]
    fn test_remove_arrows_to() {
        let mut annotations = ArrowAnnotations::default();
        let sq = test_square(1, 1);
        annotations.add_arrow(Arrow::new(test_square(0, 0), sq, Color::WHITE));
        annotations.add_arrow(Arrow::new(
            test_square(2, 2),
            test_square(3, 3),
            Color::WHITE,
        ));

        annotations.remove_arrows_to(sq);
        assert_eq!(annotations.count(), 1);
    }

    #[test]
    fn test_toggle_drawing() {
        let mut annotations = ArrowAnnotations::default();

        annotations.toggle_drawing();
        assert!(annotations.drawing_mode);

        annotations.toggle_drawing();
        assert!(!annotations.drawing_mode);
    }

    #[test]
    fn test_start_drawing() {
        let mut annotations = ArrowAnnotations::default();
        annotations.enable_drawing();

        let sq = test_square(0, 0);
        annotations.start_drawing(sq);

        assert_eq!(annotations.drawing_start, Some(sq));
    }

    #[test]
    fn test_finish_drawing() {
        let mut annotations = ArrowAnnotations::default();
        annotations.enable_drawing();

        let from = test_square(0, 0);
        let to = test_square(1, 1);

        annotations.start_drawing(from);
        let arrow = annotations.finish_drawing(to);

        assert!(arrow.is_some());
        assert_eq!(annotations.count(), 1);
        assert!(annotations.drawing_start.is_none());
    }

    #[test]
    fn test_finish_drawing_same_square() {
        let mut annotations = ArrowAnnotations::default();
        annotations.enable_drawing();

        let sq = test_square(0, 0);
        annotations.start_drawing(sq);
        let arrow = annotations.finish_drawing(sq);

        assert!(arrow.is_none());
        assert_eq!(annotations.count(), 0);
    }

    #[test]
    fn test_cancel_drawing() {
        let mut annotations = ArrowAnnotations::default();
        annotations.enable_drawing();

        annotations.start_drawing(test_square(0, 0));
        annotations.cancel_drawing();

        assert!(annotations.drawing_start.is_none());
    }

    #[test]
    fn test_set_default_color() {
        let mut annotations = ArrowAnnotations::default();
        let color = Color::srgb(0.0, 1.0, 0.0);

        annotations.set_default_color(color);
        assert_eq!(annotations.default_color, color);
    }
}
