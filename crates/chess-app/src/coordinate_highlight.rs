//! Coordinate highlighting for selected pieces.
//!
//! Highlights the rank and file of the currently selected piece to help
//! users identify square positions.

use bevy::prelude::*;
use chess_core::Square;

/// Resource managing coordinate highlight state.
#[derive(Resource)]
pub struct CoordinateHighlight {
    /// Whether coordinate highlighting is enabled.
    pub enabled: bool,
    /// Currently highlighted square (if any).
    pub highlighted_square: Option<Square>,
    /// Color for highlighted coordinates.
    pub highlight_color: Color,
    /// Color for normal coordinates.
    pub normal_color: Color,
}

impl Default for CoordinateHighlight {
    fn default() -> Self {
        Self {
            enabled: true,
            highlighted_square: None,
            highlight_color: Color::srgb(1.0, 0.8, 0.0), // Gold
            normal_color: Color::srgb(0.3, 0.3, 0.3),    // Dark gray
        }
    }
}

impl CoordinateHighlight {
    /// Enable coordinate highlighting.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable coordinate highlighting.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.highlighted_square = None;
    }

    /// Toggle coordinate highlighting.
    pub fn toggle(&mut self) {
        if self.enabled {
            self.disable();
        } else {
            self.enable();
        }
    }

    /// Set the highlighted square.
    pub fn set_square(&mut self, square: Option<Square>) {
        if self.enabled {
            self.highlighted_square = square;
        }
    }

    /// Clear the highlighted square.
    pub fn clear(&mut self) {
        self.highlighted_square = None;
    }

    /// Check if a file should be highlighted.
    pub fn is_file_highlighted(&self, file: u8) -> bool {
        if let Some(square) = self.highlighted_square {
            square.file() == file
        } else {
            false
        }
    }

    /// Check if a rank should be highlighted.
    pub fn is_rank_highlighted(&self, rank: u8) -> bool {
        if let Some(square) = self.highlighted_square {
            square.rank() == rank
        } else {
            false
        }
    }

    /// Get the color for a file coordinate.
    pub fn file_color(&self, file: u8) -> Color {
        if self.is_file_highlighted(file) {
            self.highlight_color
        } else {
            self.normal_color
        }
    }

    /// Get the color for a rank coordinate.
    pub fn rank_color(&self, rank: u8) -> Color {
        if self.is_rank_highlighted(rank) {
            self.highlight_color
        } else {
            self.normal_color
        }
    }
}

/// Component for file coordinate labels.
#[derive(Component)]
pub struct FileLabel {
    pub file: u8,
}

/// Component for rank coordinate labels.
#[derive(Component)]
pub struct RankLabel {
    pub rank: u8,
}

/// System to update coordinate highlights based on piece selection.
pub fn update_coordinate_highlight(
    mut highlight: ResMut<CoordinateHighlight>,
    selection: Res<crate::app_state::Selection>,
) {
    if selection.from.is_some() {
        highlight.set_square(selection.from);
    } else {
        highlight.clear();
    }
}

/// System to update file label colors.
pub fn update_file_label_colors(
    highlight: Res<CoordinateHighlight>,
    mut query: Query<(&FileLabel, &mut TextColor), Changed<FileLabel>>,
) {
    for (label, mut text_color) in query.iter_mut() {
        let color = highlight.file_color(label.file);
        text_color.0 = color;
    }
}

/// System to update rank label colors.
pub fn update_rank_label_colors(
    highlight: Res<CoordinateHighlight>,
    mut query: Query<(&RankLabel, &mut TextColor), Changed<RankLabel>>,
) {
    for (label, mut text_color) in query.iter_mut() {
        let color = highlight.rank_color(label.rank);
        text_color.0 = color;
    }
}

/// System to toggle coordinate highlighting.
pub fn toggle_coordinate_highlight(
    mut highlight: ResMut<CoordinateHighlight>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyH) {
        highlight.toggle();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_highlight_default() {
        let highlight = CoordinateHighlight::default();
        assert!(highlight.enabled);
        assert!(highlight.highlighted_square.is_none());
    }

    #[test]
    fn test_enable_disable() {
        let mut highlight = CoordinateHighlight::default();

        highlight.disable();
        assert!(!highlight.enabled);

        highlight.enable();
        assert!(highlight.enabled);
    }

    #[test]
    fn test_toggle() {
        let mut highlight = CoordinateHighlight::default();

        highlight.toggle();
        assert!(!highlight.enabled);

        highlight.toggle();
        assert!(highlight.enabled);
    }

    #[test]
    fn test_set_square() {
        let mut highlight = CoordinateHighlight::default();
        let square = Square::new(4, 5).unwrap();

        highlight.set_square(Some(square));
        assert_eq!(highlight.highlighted_square, Some(square));
    }

    #[test]
    fn test_set_square_disabled() {
        let mut highlight = CoordinateHighlight::default();
        highlight.disable();

        let square = Square::new(4, 5).unwrap();
        highlight.set_square(Some(square));
        assert!(highlight.highlighted_square.is_none());
    }

    #[test]
    fn test_clear() {
        let mut highlight = CoordinateHighlight::default();
        let square = Square::new(4, 5).unwrap();

        highlight.set_square(Some(square));
        highlight.clear();

        assert!(highlight.highlighted_square.is_none());
    }

    #[test]
    fn test_is_file_highlighted() {
        let mut highlight = CoordinateHighlight::default();
        let square = Square::new(4, 5).unwrap();

        highlight.set_square(Some(square));

        assert!(highlight.is_file_highlighted(4));
        assert!(!highlight.is_file_highlighted(3));
    }

    #[test]
    fn test_is_rank_highlighted() {
        let mut highlight = CoordinateHighlight::default();
        let square = Square::new(4, 5).unwrap();

        highlight.set_square(Some(square));

        assert!(highlight.is_rank_highlighted(5));
        assert!(!highlight.is_rank_highlighted(4));
    }

    #[test]
    fn test_file_color() {
        let mut highlight = CoordinateHighlight::default();
        let square = Square::new(4, 5).unwrap();

        highlight.set_square(Some(square));

        let highlighted_color = highlight.file_color(4);
        let normal_color = highlight.file_color(3);

        assert_eq!(highlighted_color, highlight.highlight_color);
        assert_eq!(normal_color, highlight.normal_color);
    }

    #[test]
    fn test_rank_color() {
        let mut highlight = CoordinateHighlight::default();
        let square = Square::new(4, 5).unwrap();

        highlight.set_square(Some(square));

        let highlighted_color = highlight.rank_color(5);
        let normal_color = highlight.rank_color(4);

        assert_eq!(highlighted_color, highlight.highlight_color);
        assert_eq!(normal_color, highlight.normal_color);
    }

    #[test]
    fn test_no_highlight_when_no_square() {
        let highlight = CoordinateHighlight::default();

        assert!(!highlight.is_file_highlighted(4));
        assert!(!highlight.is_rank_highlighted(5));
    }
}
