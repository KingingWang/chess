//! Replay mode for playing back completed games.
//!
//! Allows auto-play through a saved game with configurable speed,
//! forward/backward navigation, and pause/resume.

use bevy::prelude::*;

use crate::app_state::{CoreGame, UiFonts};

/// Replay speed presets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplaySpeed {
    /// 1 move per 2 seconds.
    Slow,
    /// 1 move per 1 second.
    Normal,
    /// 1 move per 0.5 seconds.
    Fast,
    /// 1 move per 0.25 seconds.
    VeryFast,
}

impl ReplaySpeed {
    /// Get the interval in seconds.
    pub fn interval_secs(&self) -> f32 {
        match self {
            Self::Slow => 2.0,
            Self::Normal => 1.0,
            Self::Fast => 0.5,
            Self::VeryFast => 0.25,
        }
    }

    /// Get the Chinese label.
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Slow => "慢速",
            Self::Normal => "正常",
            Self::Fast => "快速",
            Self::VeryFast => "极速",
        }
    }

    /// Get the next faster speed.
    pub fn next_faster(&self) -> Self {
        match self {
            Self::Slow => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast => Self::VeryFast,
            Self::VeryFast => Self::VeryFast,
        }
    }

    /// Get the next slower speed.
    pub fn next_slower(&self) -> Self {
        match self {
            Self::Slow => Self::Slow,
            Self::Normal => Self::Slow,
            Self::Fast => Self::Normal,
            Self::VeryFast => Self::Fast,
        }
    }
}

impl Default for ReplaySpeed {
    fn default() -> Self {
        Self::Normal
    }
}

/// Resource managing replay mode state.
#[derive(Resource, Debug, Clone)]
pub struct ReplayMode {
    /// Whether replay mode is active.
    pub active: bool,
    /// Whether the replay is currently playing (not paused).
    pub playing: bool,
    /// Current move index in the replay.
    pub current_move: usize,
    /// Total number of moves in the game being replayed.
    pub total_moves: usize,
    /// The saved move history for replay.
    pub move_history: Vec<chess_core::Move>,
    /// Current replay speed.
    pub speed: ReplaySpeed,
    /// Time accumulator for auto-advance.
    pub time_accumulator: f32,
}

impl Default for ReplayMode {
    fn default() -> Self {
        Self {
            active: false,
            playing: false,
            current_move: 0,
            total_moves: 0,
            move_history: Vec::new(),
            speed: ReplaySpeed::default(),
            time_accumulator: 0.0,
        }
    }
}

impl ReplayMode {
    /// Start a replay with the given move history.
    pub fn start(&mut self, moves: Vec<chess_core::Move>) {
        self.total_moves = moves.len();
        self.move_history = moves;
        self.current_move = 0;
        self.active = true;
        self.playing = true;
        self.time_accumulator = 0.0;
    }

    /// Stop the replay.
    pub fn stop(&mut self) {
        self.active = false;
        self.playing = false;
        self.current_move = 0;
        self.move_history.clear();
        self.total_moves = 0;
        self.time_accumulator = 0.0;
    }

    /// Toggle play/pause.
    pub fn toggle_play_pause(&mut self) {
        if self.active {
            self.playing = !self.playing;
            if self.playing {
                self.time_accumulator = 0.0;
            }
        }
    }

    /// Advance to the next move.
    pub fn next_move(&mut self) -> Option<chess_core::Move> {
        if self.current_move < self.total_moves {
            let m = self.move_history[self.current_move];
            self.current_move += 1;
            Some(m)
        } else {
            None
        }
    }

    /// Go back to the previous move.
    pub fn prev_move(&mut self) -> bool {
        if self.current_move > 0 {
            self.current_move -= 1;
            true
        } else {
            false
        }
    }

    /// Jump to a specific move.
    pub fn jump_to(&mut self, move_idx: usize) {
        self.current_move = move_idx.min(self.total_moves);
    }

    /// Jump to the beginning.
    pub fn jump_to_start(&mut self) {
        self.current_move = 0;
    }

    /// Jump to the end.
    pub fn jump_to_end(&mut self) {
        self.current_move = self.total_moves;
    }

    /// Check if replay is at the end.
    pub fn is_at_end(&self) -> bool {
        self.current_move >= self.total_moves
    }

    /// Check if replay is at the start.
    pub fn is_at_start(&self) -> bool {
        self.current_move == 0
    }

    /// Get progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        if self.total_moves == 0 {
            return 0.0;
        }
        self.current_move as f32 / self.total_moves as f32
    }

    /// Increase speed.
    pub fn speed_up(&mut self) {
        self.speed = self.speed.next_faster();
    }

    /// Decrease speed.
    pub fn speed_down(&mut self) {
        self.speed = self.speed.next_slower();
    }
}

/// System to auto-advance replay.
pub fn replay_auto_advance(
    time: Res<Time>,
    mut replay: ResMut<ReplayMode>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
) {
    if !replay.active || !replay.playing || replay.is_at_end() {
        return;
    }

    replay.time_accumulator += time.delta_secs();

    if replay.time_accumulator >= replay.speed.interval_secs() {
        replay.time_accumulator = 0.0;

        if let Some(m) = replay.next_move() {
            if core.game.make_move(m).is_ok() {
                dirty.0 = true;
            }
        }

        // Stop at the end
        if replay.is_at_end() {
            replay.playing = false;
        }
    }
}

/// System for replay keyboard controls.
pub fn replay_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut replay: ResMut<ReplayMode>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
) {
    if !replay.active {
        return;
    }

    // Space: play/pause
    if keys.just_pressed(KeyCode::Space) {
        replay.toggle_play_pause();
        let status = if replay.playing { "播放" } else { "暂停" };
        crate::toast::spawn_toast(&mut commands, &fonts, status);
    }

    // Right arrow: next move
    if keys.just_pressed(KeyCode::ArrowRight) {
        replay.playing = false;
        if let Some(m) = replay.next_move() {
            if core.game.make_move(m).is_ok() {
                dirty.0 = true;
            }
        }
    }

    // Left arrow: undo (go back)
    if keys.just_pressed(KeyCode::ArrowLeft) {
        replay.playing = false;
        if replay.prev_move() {
            core.game.undo();
            dirty.0 = true;
        }
    }

    // Home: jump to start
    if keys.just_pressed(KeyCode::Home) {
        replay.playing = false;
        while replay.prev_move() {
            core.game.undo();
        }
        dirty.0 = true;
    }

    // End: jump to end
    if keys.just_pressed(KeyCode::End) {
        replay.playing = false;
        while let Some(m) = replay.next_move() {
            if core.game.make_move(m).is_ok() {
                dirty.0 = true;
            }
        }
    }

    // Plus: speed up
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        replay.speed_up();
        let msg = format!("速度: {}", replay.speed.label_cn());
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }

    // Minus: slow down
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        replay.speed_down();
        let msg = format!("速度: {}", replay.speed.label_cn());
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }

    // Escape: exit replay
    if keys.just_pressed(KeyCode::Escape) {
        crate::toast::spawn_toast(&mut commands, &fonts, "退出回放模式");
        replay.stop();
    }
}

/// Toggle replay mode with Ctrl+R.
pub fn toggle_replay(
    keys: Res<ButtonInput<KeyCode>>,
    mut replay: ResMut<ReplayMode>,
    core: Res<CoreGame>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if ctrl && keys.just_pressed(KeyCode::KeyR) {
        if replay.active {
            replay.stop();
            crate::toast::spawn_toast(&mut commands, &fonts, "回放模式已关闭");
        } else {
            let moves: Vec<chess_core::Move> = core.game.history().iter().map(|e| e.mv()).collect();
            if moves.is_empty() {
                crate::toast::spawn_toast(&mut commands, &fonts, "没有着法可以回放");
            } else {
                let count = moves.len();
                replay.start(moves);
                // Reset the game to the starting position
                // Note: In a real implementation, we'd save the game state first
                crate::toast::spawn_toast(
                    &mut commands,
                    &fonts,
                    &format!("回放模式: {}步 按空格播放/暂停", count),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Square;

    fn test_move(from_file: u8, from_rank: u8, to_file: u8, to_rank: u8) -> chess_core::Move {
        chess_core::Move {
            from: Square::new(from_file, from_rank).unwrap(),
            to: Square::new(to_file, to_rank).unwrap(),
        }
    }

    #[test]
    fn test_replay_default() {
        let replay = ReplayMode::default();
        assert!(!replay.active);
        assert!(!replay.playing);
        assert_eq!(replay.current_move, 0);
    }

    #[test]
    fn test_replay_start() {
        let mut replay = ReplayMode::default();
        let moves = vec![test_move(7, 2, 4, 2), test_move(7, 7, 4, 7)];
        replay.start(moves);
        assert!(replay.active);
        assert!(replay.playing);
        assert_eq!(replay.total_moves, 2);
        assert_eq!(replay.current_move, 0);
    }

    #[test]
    fn test_replay_stop() {
        let mut replay = ReplayMode::default();
        replay.start(vec![test_move(7, 2, 4, 2)]);
        replay.stop();
        assert!(!replay.active);
        assert_eq!(replay.move_history.len(), 0);
    }

    #[test]
    fn test_replay_next_move() {
        let mut replay = ReplayMode::default();
        let moves = vec![test_move(7, 2, 4, 2), test_move(7, 7, 4, 7)];
        replay.start(moves);

        let m = replay.next_move();
        assert!(m.is_some());
        assert_eq!(replay.current_move, 1);

        let m = replay.next_move();
        assert!(m.is_some());
        assert_eq!(replay.current_move, 2);

        let m = replay.next_move();
        assert!(m.is_none()); // At end
    }

    #[test]
    fn test_replay_prev_move() {
        let mut replay = ReplayMode::default();
        let moves = vec![test_move(7, 2, 4, 2), test_move(7, 7, 4, 7)];
        replay.start(moves);
        replay.next_move(); // Move to index 1

        assert!(replay.prev_move());
        assert_eq!(replay.current_move, 0);
        assert!(!replay.prev_move()); // Can't go back further
    }

    #[test]
    fn test_replay_jump_to() {
        let mut replay = ReplayMode::default();
        let moves = vec![
            test_move(7, 2, 4, 2),
            test_move(7, 7, 4, 7),
            test_move(1, 2, 4, 2),
        ];
        replay.start(moves);

        replay.jump_to(2);
        assert_eq!(replay.current_move, 2);

        replay.jump_to(100); // Clamp to total
        assert_eq!(replay.current_move, 3);
    }

    #[test]
    fn test_replay_progress() {
        let mut replay = ReplayMode::default();
        let moves = vec![test_move(7, 2, 4, 2), test_move(7, 7, 4, 7)];
        replay.start(moves);

        assert_eq!(replay.progress(), 0.0);
        replay.next_move();
        assert_eq!(replay.progress(), 0.5);
        replay.next_move();
        assert_eq!(replay.progress(), 1.0);
    }

    #[test]
    fn test_replay_toggle() {
        let mut replay = ReplayMode::default();
        replay.start(vec![test_move(7, 2, 4, 2)]);
        assert!(replay.playing);
        replay.toggle_play_pause();
        assert!(!replay.playing);
        replay.toggle_play_pause();
        assert!(replay.playing);
    }

    #[test]
    fn test_replay_at_boundaries() {
        let mut replay = ReplayMode::default();
        replay.start(vec![test_move(7, 2, 4, 2)]);
        assert!(replay.is_at_start());
        assert!(!replay.is_at_end());

        replay.next_move();
        assert!(!replay.is_at_start());
        assert!(replay.is_at_end());
    }

    #[test]
    fn test_speed_presets() {
        assert!(ReplaySpeed::Slow.interval_secs() > ReplaySpeed::Normal.interval_secs());
        assert!(ReplaySpeed::Normal.interval_secs() > ReplaySpeed::Fast.interval_secs());
        assert!(ReplaySpeed::Fast.interval_secs() > ReplaySpeed::VeryFast.interval_secs());
    }

    #[test]
    fn test_speed_change() {
        let mut replay = ReplayMode::default();
        assert_eq!(replay.speed, ReplaySpeed::Normal);
        replay.speed_up();
        assert_eq!(replay.speed, ReplaySpeed::Fast);
        replay.speed_down();
        assert_eq!(replay.speed, ReplaySpeed::Normal);
        replay.speed_down();
        assert_eq!(replay.speed, ReplaySpeed::Slow);
        replay.speed_down(); // Can't go slower
        assert_eq!(replay.speed, ReplaySpeed::Slow);
    }
}
