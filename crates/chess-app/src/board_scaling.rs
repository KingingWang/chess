//! Board scaling system for different screen sizes.
//!
//! Allows users to adjust the board size for better visibility.

use bevy::prelude::*;

/// Preset board sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoardSize {
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
    Custom(u32),
}

impl BoardSize {
    /// Get the pixel size for this board size.
    pub fn pixels(&self) -> u32 {
        match self {
            BoardSize::Small => 400,
            BoardSize::Medium => 600,
            BoardSize::Large => 800,
            BoardSize::ExtraLarge => 1000,
            BoardSize::Custom(size) => *size,
        }
    }

    /// Get the scale factor relative to the default size.
    pub fn scale_factor(&self) -> f32 {
        self.pixels() as f32 / BoardSize::Medium.pixels() as f32
    }

    /// Get the next larger size.
    pub fn next_larger(&self) -> Self {
        match self {
            BoardSize::Small => BoardSize::Medium,
            BoardSize::Medium => BoardSize::Large,
            BoardSize::Large => BoardSize::ExtraLarge,
            BoardSize::ExtraLarge => BoardSize::ExtraLarge,
            BoardSize::Custom(size) => BoardSize::Custom(size + 100),
        }
    }

    /// Get the next smaller size.
    pub fn next_smaller(&self) -> Self {
        match self {
            BoardSize::Small => BoardSize::Small,
            BoardSize::Medium => BoardSize::Small,
            BoardSize::Large => BoardSize::Medium,
            BoardSize::ExtraLarge => BoardSize::Large,
            BoardSize::Custom(size) => BoardSize::Custom(size.saturating_sub(100)),
        }
    }
}

/// Resource managing board scaling.
#[derive(Resource)]
pub struct BoardScaling {
    /// Current board size.
    pub current_size: BoardSize,
    /// Target board size (for smooth transitions).
    pub target_size: BoardSize,
    /// Current scale factor (for smooth transitions).
    pub current_scale: f32,
    /// Target scale factor.
    pub target_scale: f32,
    /// Transition speed (units per second).
    pub transition_speed: f32,
    /// Whether a transition is in progress.
    pub transitioning: bool,
}

impl Default for BoardScaling {
    fn default() -> Self {
        let default_size = BoardSize::default();
        let scale = default_size.scale_factor();
        Self {
            current_size: default_size,
            target_size: default_size,
            current_scale: scale,
            target_scale: scale,
            transition_speed: 2.0,
            transitioning: false,
        }
    }
}

impl BoardScaling {
    /// Set the board size with smooth transition.
    pub fn set_size(&mut self, size: BoardSize) {
        self.target_size = size;
        self.target_scale = size.scale_factor();
        self.transitioning = true;
    }

    /// Set the board size immediately (no transition).
    pub fn set_size_immediate(&mut self, size: BoardSize) {
        self.current_size = size;
        self.target_size = size;
        self.current_scale = size.scale_factor();
        self.target_scale = size.scale_factor();
        self.transitioning = false;
    }

    /// Increase the board size.
    pub fn increase_size(&mut self) {
        self.set_size(self.current_size.next_larger());
    }

    /// Decrease the board size.
    pub fn decrease_size(&mut self) {
        self.set_size(self.current_size.next_smaller());
    }

    /// Update the transition.
    pub fn update_transition(&mut self, delta_time: f32) {
        if !self.transitioning {
            return;
        }

        let diff = self.target_scale - self.current_scale;
        let step = self.transition_speed * delta_time;

        if diff.abs() < step {
            self.current_scale = self.target_scale;
            self.current_size = self.target_size;
            self.transitioning = false;
        } else {
            self.current_scale += diff.signum() * step;
        }
    }

    /// Get the current scale factor.
    pub fn scale(&self) -> f32 {
        self.current_scale
    }

    /// Get the current board size in pixels.
    pub fn size_pixels(&self) -> u32 {
        (BoardSize::Medium.pixels() as f32 * self.current_scale) as u32
    }

    /// Check if a transition is in progress.
    pub fn is_transitioning(&self) -> bool {
        self.transitioning
    }

    /// Set the transition speed.
    pub fn set_transition_speed(&mut self, speed: f32) {
        self.transition_speed = speed;
    }
}

/// System to update board scaling transitions.
pub fn update_board_scaling(mut scaling: ResMut<BoardScaling>, time: Res<Time>) {
    scaling.update_transition(time.delta_secs());
}

/// System to handle board size keyboard shortcuts.
pub fn handle_board_size_input(
    mut scaling: ResMut<BoardScaling>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        scaling.increase_size();
    }
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        scaling.decrease_size();
    }
}

/// System to apply board scaling to the board entity.
pub fn apply_board_scaling(
    scaling: Res<BoardScaling>,
    mut query: Query<&mut Transform, With<crate::board_view::BoardLine>>,
) {
    if scaling.is_changed() {
        for mut transform in query.iter_mut() {
            transform.scale = Vec3::splat(scaling.scale());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_size_pixels() {
        assert_eq!(BoardSize::Small.pixels(), 400);
        assert_eq!(BoardSize::Medium.pixels(), 600);
        assert_eq!(BoardSize::Large.pixels(), 800);
        assert_eq!(BoardSize::ExtraLarge.pixels(), 1000);
        assert_eq!(BoardSize::Custom(500).pixels(), 500);
    }

    #[test]
    fn test_board_size_scale_factor() {
        assert_eq!(BoardSize::Medium.scale_factor(), 1.0);
        assert!(BoardSize::Small.scale_factor() < 1.0);
        assert!(BoardSize::Large.scale_factor() > 1.0);
    }

    #[test]
    fn test_next_larger() {
        assert_eq!(BoardSize::Small.next_larger(), BoardSize::Medium);
        assert_eq!(BoardSize::Medium.next_larger(), BoardSize::Large);
        assert_eq!(BoardSize::Large.next_larger(), BoardSize::ExtraLarge);
        assert_eq!(BoardSize::ExtraLarge.next_larger(), BoardSize::ExtraLarge);
    }

    #[test]
    fn test_next_smaller() {
        assert_eq!(BoardSize::Small.next_smaller(), BoardSize::Small);
        assert_eq!(BoardSize::Medium.next_smaller(), BoardSize::Small);
        assert_eq!(BoardSize::Large.next_smaller(), BoardSize::Medium);
        assert_eq!(BoardSize::ExtraLarge.next_smaller(), BoardSize::Large);
    }

    #[test]
    fn test_board_scaling_default() {
        let scaling = BoardScaling::default();
        assert_eq!(scaling.current_size, BoardSize::Medium);
        assert_eq!(scaling.current_scale, 1.0);
        assert!(!scaling.transitioning);
    }

    #[test]
    fn test_set_size() {
        let mut scaling = BoardScaling::default();
        scaling.set_size(BoardSize::Large);

        assert_eq!(scaling.target_size, BoardSize::Large);
        assert!(scaling.transitioning);
        assert_eq!(scaling.current_size, BoardSize::Medium); // Not yet transitioned
    }

    #[test]
    fn test_set_size_immediate() {
        let mut scaling = BoardScaling::default();
        scaling.set_size_immediate(BoardSize::Large);

        assert_eq!(scaling.current_size, BoardSize::Large);
        assert_eq!(scaling.target_size, BoardSize::Large);
        assert!(!scaling.transitioning);
    }

    #[test]
    fn test_increase_size() {
        let mut scaling = BoardScaling::default();
        scaling.increase_size();

        assert_eq!(scaling.target_size, BoardSize::Large);
    }

    #[test]
    fn test_decrease_size() {
        let mut scaling = BoardScaling::default();
        scaling.decrease_size();

        assert_eq!(scaling.target_size, BoardSize::Small);
    }

    #[test]
    fn test_update_transition() {
        let mut scaling = BoardScaling::default();
        scaling.set_size(BoardSize::Large);

        // Simulate time passing
        scaling.update_transition(0.1);

        // Should have moved towards target
        assert!(scaling.current_scale > 1.0);
        assert!(scaling.current_scale < scaling.target_scale);
    }

    #[test]
    fn test_update_transition_complete() {
        let mut scaling = BoardScaling::default();
        scaling.set_size(BoardSize::Large);

        // Simulate enough time for transition to complete
        scaling.update_transition(10.0);

        assert_eq!(scaling.current_scale, scaling.target_scale);
        assert!(!scaling.transitioning);
    }

    #[test]
    fn test_scale() {
        let mut scaling = BoardScaling::default();
        assert_eq!(scaling.scale(), 1.0);

        scaling.set_size_immediate(BoardSize::Large);
        assert!(scaling.scale() > 1.0);
    }

    #[test]
    fn test_size_pixels() {
        let scaling = BoardScaling::default();
        assert_eq!(scaling.size_pixels(), 600);
    }

    #[test]
    fn test_is_transitioning() {
        let mut scaling = BoardScaling::default();
        assert!(!scaling.is_transitioning());

        scaling.set_size(BoardSize::Large);
        assert!(scaling.is_transitioning());
    }

    #[test]
    fn test_set_transition_speed() {
        let mut scaling = BoardScaling::default();
        scaling.set_transition_speed(5.0);
        assert_eq!(scaling.transition_speed, 5.0);
    }
}
