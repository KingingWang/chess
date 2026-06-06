//! Opening explorer with move statistics.
//!
//! Tracks and displays statistics about opening moves, including frequency
//! and success rates based on game history.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, Move};
use std::collections::HashMap;

/// Statistics for a single move in a position.
#[derive(Debug, Clone, Default)]
pub struct MoveStats {
    /// Number of times this move was played.
    pub games_played: u32,
    /// Number of wins for the side that made this move.
    pub wins: u32,
    /// Number of draws.
    pub draws: u32,
    /// Number of losses for the side that made this move.
    pub losses: u32,
}

impl MoveStats {
    /// Calculate win rate as a percentage.
    pub fn win_rate(&self) -> f32 {
        if self.games_played == 0 {
            return 0.0;
        }
        (self.wins as f32 / self.games_played as f32) * 100.0
    }

    /// Calculate draw rate as a percentage.
    pub fn draw_rate(&self) -> f32 {
        if self.games_played == 0 {
            return 0.0;
        }
        (self.draws as f32 / self.games_played as f32) * 100.0
    }

    /// Calculate score (wins + 0.5 * draws) / total games.
    pub fn score(&self) -> f32 {
        if self.games_played == 0 {
            return 0.0;
        }
        (self.wins as f32 + 0.5 * self.draws as f32) / self.games_played as f32
    }
}

/// Opening database mapping FEN positions to move statistics.
#[derive(Resource, Debug, Clone, Default)]
pub struct OpeningDatabase {
    /// Map from FEN string to move statistics.
    pub positions: HashMap<String, HashMap<Move, MoveStats>>,
    /// Total games recorded.
    pub total_games: u32,
}

impl OpeningDatabase {
    /// Record a game result and update statistics.
    pub fn record_game(&mut self, moves: Vec<Move>, fens: Vec<String>, winner: Option<ChessColor>) {
        self.total_games += 1;

        for (i, fen) in fens.iter().enumerate() {
            if i >= moves.len() {
                break;
            }

            let mv = moves[i];
            let move_stats = self
                .positions
                .entry(fen.clone())
                .or_insert_with(HashMap::new)
                .entry(mv)
                .or_insert_with(MoveStats::default);

            move_stats.games_played += 1;

            // Determine whose move this was (based on FEN side to move)
            // For simplicity, we'll track from the perspective of the player who made the move
            if let Some(winner_color) = winner {
                // We need to know whose move this was
                // Since we don't have easy access to the side to move here,
                // we'll use a simple heuristic: even moves are Red, odd are Black
                let mover_color = if i % 2 == 0 {
                    ChessColor::Red
                } else {
                    ChessColor::Black
                };

                if winner_color == mover_color {
                    move_stats.wins += 1;
                } else {
                    move_stats.losses += 1;
                }
            } else {
                move_stats.draws += 1;
            }
        }
    }

    /// Get statistics for a position.
    pub fn get_position_stats(&self, fen: &str) -> Option<&HashMap<Move, MoveStats>> {
        self.positions.get(fen)
    }

    /// Get statistics for a specific move in a position.
    pub fn get_move_stats(&self, fen: &str, mv: Move) -> Option<&MoveStats> {
        self.positions.get(fen).and_then(|pos| pos.get(&mv))
    }

    /// Get the most popular move for a position.
    pub fn get_most_popular_move(&self, fen: &str) -> Option<Move> {
        self.positions.get(fen).and_then(|pos| {
            pos.iter()
                .max_by_key(|(_, stats)| stats.games_played)
                .map(|(mv, _)| *mv)
        })
    }

    /// Get the best scoring move for a position (minimum games threshold).
    pub fn get_best_scoring_move(&self, fen: &str, min_games: u32) -> Option<Move> {
        self.positions.get(fen).and_then(|pos| {
            pos.iter()
                .filter(|(_, stats)| stats.games_played >= min_games)
                .max_by(|a, b| a.1.score().partial_cmp(&b.1.score()).unwrap())
                .map(|(mv, _)| *mv)
        })
    }

    /// Clear all statistics.
    pub fn clear(&mut self) {
        self.positions.clear();
        self.total_games = 0;
    }

    /// Get total number of unique positions tracked.
    pub fn position_count(&self) -> usize {
        self.positions.len()
    }
}

/// Component for the opening explorer UI panel.
#[derive(Component)]
pub struct OpeningExplorerUI;

/// System to toggle opening explorer visibility.
pub fn toggle_opening_explorer(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    ui_query: Query<Entity, With<OpeningExplorerUI>>,
    db: Res<OpeningDatabase>,
) {
    if keyboard.just_pressed(KeyCode::KeyO) {
        // Toggle UI
        if let Ok(entity) = ui_query.single() {
            commands.entity(entity).despawn();
        } else {
            spawn_opening_explorer_ui(&mut commands, &db);
        }
    }
}

/// Spawn the opening explorer UI.
fn spawn_opening_explorer_ui(commands: &mut Commands, db: &OpeningDatabase) {
    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.8),
            custom_size: Some(Vec2::new(300.0, 400.0)),
            ..default()
        },
        Transform::from_xyz(500.0, 0.0, 10.0),
        OpeningExplorerUI,
    ));
}

/// System to update opening explorer with current position.
pub fn update_opening_explorer(
    core: Res<crate::app_state::CoreGame>,
    db: Res<OpeningDatabase>,
    ui_query: Query<&Transform, With<OpeningExplorerUI>>,
) {
    if ui_query.is_empty() {
        return;
    }

    let fen = core.game.board().to_fen();
    if let Some(stats) = db.get_position_stats(&fen) {
        // Update UI with statistics
        // This would involve updating text components with move statistics
        // For now, we just check if we have data
        let _ = stats;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Square;

    fn create_test_move(from_file: u8, from_rank: u8, to_file: u8, to_rank: u8) -> Move {
        Move {
            from: Square::new(from_file, from_rank).unwrap(),
            to: Square::new(to_file, to_rank).unwrap(),
        }
    }

    #[test]
    fn test_move_stats_win_rate() {
        let stats = MoveStats {
            games_played: 100,
            wins: 60,
            draws: 20,
            losses: 20,
        };
        assert!((stats.win_rate() - 60.0).abs() < 0.001);
        assert!((stats.draw_rate() - 20.0).abs() < 0.001);
        assert!((stats.score() - 0.7).abs() < 0.001); // (60 + 0.5 * 20) / 100
    }

    #[test]
    fn test_move_stats_zero_games() {
        let stats = MoveStats::default();
        assert_eq!(stats.win_rate(), 0.0);
        assert_eq!(stats.draw_rate(), 0.0);
        assert_eq!(stats.score(), 0.0);
    }

    #[test]
    fn test_record_game() {
        let mut db = OpeningDatabase::default();
        let mv = create_test_move(4, 0, 4, 1);
        let fen = "test_fen".to_string();

        db.record_game(vec![mv], vec![fen.clone()], Some(ChessColor::Red));

        assert_eq!(db.total_games, 1);
        let stats = db.get_move_stats(&fen, mv).unwrap();
        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.wins, 1);
    }

    #[test]
    fn test_record_draw() {
        let mut db = OpeningDatabase::default();
        let mv = create_test_move(4, 0, 4, 1);
        let fen = "test_fen".to_string();

        db.record_game(vec![mv], vec![fen.clone()], None);

        let stats = db.get_move_stats(&fen, mv).unwrap();
        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.draws, 1);
        assert_eq!(stats.wins, 0);
        assert_eq!(stats.losses, 0);
    }

    #[test]
    fn test_get_most_popular_move() {
        let mut db = OpeningDatabase::default();
        let fen = "test_fen".to_string();
        let mv1 = create_test_move(4, 0, 4, 1);
        let mv2 = create_test_move(3, 0, 3, 1);

        // Record mv1 three times
        for _ in 0..3 {
            db.record_game(vec![mv1], vec![fen.clone()], Some(ChessColor::Red));
        }
        // Record mv2 once
        db.record_game(vec![mv2], vec![fen.clone()], Some(ChessColor::Red));

        let most_popular = db.get_most_popular_move(&fen).unwrap();
        assert_eq!(most_popular, mv1);
    }

    #[test]
    fn test_get_best_scoring_move() {
        let mut db = OpeningDatabase::default();
        let fen = "test_fen".to_string();
        let mv1 = create_test_move(4, 0, 4, 1);
        let mv2 = create_test_move(3, 0, 3, 1);

        // mv1: 2 wins, 1 loss (score = 0.67)
        db.record_game(vec![mv1], vec![fen.clone()], Some(ChessColor::Red));
        db.record_game(vec![mv1], vec![fen.clone()], Some(ChessColor::Red));
        db.record_game(vec![mv1], vec![fen.clone()], Some(ChessColor::Black));

        // mv2: 1 win, 0 losses (score = 1.0)
        db.record_game(vec![mv2], vec![fen.clone()], Some(ChessColor::Red));

        let best = db.get_best_scoring_move(&fen, 1).unwrap();
        assert_eq!(best, mv2); // mv2 has better score
    }

    #[test]
    fn test_position_count() {
        let mut db = OpeningDatabase::default();
        let fen1 = "fen1".to_string();
        let fen2 = "fen2".to_string();
        let mv = create_test_move(4, 0, 4, 1);

        db.record_game(vec![mv], vec![fen1.clone()], Some(ChessColor::Red));
        db.record_game(vec![mv], vec![fen2.clone()], Some(ChessColor::Red));

        assert_eq!(db.position_count(), 2);
    }

    #[test]
    fn test_clear_database() {
        let mut db = OpeningDatabase::default();
        let fen = "test_fen".to_string();
        let mv = create_test_move(4, 0, 4, 1);

        db.record_game(vec![mv], vec![fen], Some(ChessColor::Red));
        assert_eq!(db.total_games, 1);

        db.clear();
        assert_eq!(db.total_games, 0);
        assert_eq!(db.position_count(), 0);
    }

    #[test]
    fn test_multiple_moves_in_game() {
        let mut db = OpeningDatabase::default();
        let fen1 = "fen1".to_string();
        let fen2 = "fen2".to_string();
        let mv1 = create_test_move(4, 0, 4, 1);
        let mv2 = create_test_move(4, 9, 4, 8);

        db.record_game(
            vec![mv1, mv2],
            vec![fen1.clone(), fen2.clone()],
            Some(ChessColor::Red),
        );

        assert_eq!(db.total_games, 1);
        assert_eq!(db.position_count(), 2);

        let stats1 = db.get_move_stats(&fen1, mv1).unwrap();
        assert_eq!(stats1.games_played, 1);
        assert_eq!(stats1.wins, 1); // Red won

        let stats2 = db.get_move_stats(&fen2, mv2).unwrap();
        assert_eq!(stats2.games_played, 1);
        assert_eq!(stats2.losses, 1); // Black lost
    }

    #[test]
    fn test_nonexistent_position() {
        let db = OpeningDatabase::default();
        assert!(db.get_position_stats("nonexistent").is_none());
        assert!(db.get_most_popular_move("nonexistent").is_none());
        assert!(db.get_best_scoring_move("nonexistent", 1).is_none());
    }
}
