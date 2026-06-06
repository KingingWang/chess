//! Game statistics tracking for player performance analysis.
//!
//! Tracks wins, losses, draws, average game length, and favorite openings.

use bevy::prelude::*;
use chess_core::Color as ChessColor;
use std::collections::HashMap;

/// Resource tracking game statistics.
#[derive(Resource, Debug, Clone)]
pub struct GameStatistics {
    /// Total wins as Red.
    pub wins_as_red: u32,
    /// Total wins as Black.
    pub wins_as_black: u32,
    /// Total losses as Red.
    pub losses_as_red: u32,
    /// Total losses as Black.
    pub losses_as_black: u32,
    /// Total draws.
    pub draws: u32,
    /// Total moves played across all games.
    pub total_moves: u64,
    /// Number of games played.
    pub games_played: u32,
    /// Opening move counts (first move of each game).
    pub opening_moves: HashMap<String, u32>,
    /// Longest winning streak.
    pub longest_win_streak: u32,
    /// Current win streak.
    pub current_win_streak: u32,
}

impl Default for GameStatistics {
    fn default() -> Self {
        Self {
            wins_as_red: 0,
            wins_as_black: 0,
            losses_as_red: 0,
            losses_as_black: 0,
            draws: 0,
            total_moves: 0,
            games_played: 0,
            opening_moves: HashMap::new(),
            longest_win_streak: 0,
            current_win_streak: 0,
        }
    }
}

impl GameStatistics {
    /// Record a game result.
    pub fn record_game(
        &mut self,
        player_color: ChessColor,
        result: GameResult,
        move_count: u32,
        first_move: Option<String>,
    ) {
        self.games_played += 1;
        self.total_moves += move_count as u64;

        // Track opening move
        if let Some(opening) = first_move {
            *self.opening_moves.entry(opening).or_insert(0) += 1;
        }

        // Update win/loss/draw counts
        match result {
            GameResult::Win { winner, .. } => {
                if winner == player_color {
                    // Player won
                    match player_color {
                        ChessColor::Red => self.wins_as_red += 1,
                        ChessColor::Black => self.wins_as_black += 1,
                    }
                    self.current_win_streak += 1;
                    if self.current_win_streak > self.longest_win_streak {
                        self.longest_win_streak = self.current_win_streak;
                    }
                } else {
                    // Player lost
                    match player_color {
                        ChessColor::Red => self.losses_as_red += 1,
                        ChessColor::Black => self.losses_as_black += 1,
                    }
                    self.current_win_streak = 0;
                }
            }
            GameResult::Draw(_) => {
                self.draws += 1;
                self.current_win_streak = 0;
            }
        }
    }

    /// Get total wins.
    pub fn total_wins(&self) -> u32 {
        self.wins_as_red + self.wins_as_black
    }

    /// Get total losses.
    pub fn total_losses(&self) -> u32 {
        self.losses_as_red + self.losses_as_black
    }

    /// Get win rate as percentage.
    pub fn win_rate(&self) -> f32 {
        if self.games_played == 0 {
            return 0.0;
        }
        (self.total_wins() as f32 / self.games_played as f32) * 100.0
    }

    /// Get average moves per game.
    pub fn avg_moves_per_game(&self) -> f32 {
        if self.games_played == 0 {
            return 0.0;
        }
        self.total_moves as f32 / self.games_played as f32
    }

    /// Get most played opening.
    pub fn favorite_opening(&self) -> Option<(&String, &u32)> {
        self.opening_moves.iter().max_by_key(|(_, &count)| count)
    }

    /// Reset all statistics.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Game result enum for recording.
#[derive(Debug, Clone, Copy)]
pub enum GameResult {
    Win { winner: ChessColor },
    Draw(DrawReason),
}

/// Draw reason enum.
#[derive(Debug, Clone, Copy)]
pub enum DrawReason {
    Agreement,
    Stalemate,
    Repetition,
    FiftyMove,
}

/// System to display statistics (placeholder for future UI).
pub fn display_stats(stats: Res<GameStatistics>) {
    if stats.games_played == 0 {
        return;
    }

    // This would be integrated with the UI system in the future
    // For now, just log the stats
    bevy::log::info!(
        "Game Stats: {} games, {} wins ({:.1}%), {} losses, {} draws, avg {:.1} moves/game",
        stats.games_played,
        stats.total_wins(),
        stats.win_rate(),
        stats.total_losses(),
        stats.draws,
        stats.avg_moves_per_game()
    );

    if let Some((opening, count)) = stats.favorite_opening() {
        bevy::log::info!("Favorite opening: {} ({} games)", opening, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stats() {
        let stats = GameStatistics::default();
        assert_eq!(stats.games_played, 0);
        assert_eq!(stats.total_wins(), 0);
        assert_eq!(stats.win_rate(), 0.0);
    }

    #[test]
    fn test_record_win() {
        let mut stats = GameStatistics::default();
        stats.record_game(
            ChessColor::Red,
            GameResult::Win {
                winner: ChessColor::Red,
            },
            40,
            Some("h2e2".to_string()),
        );
        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.wins_as_red, 1);
        assert_eq!(stats.total_wins(), 1);
        assert_eq!(stats.win_rate(), 100.0);
        assert_eq!(stats.current_win_streak, 1);
        assert_eq!(stats.longest_win_streak, 1);
    }

    #[test]
    fn test_record_loss() {
        let mut stats = GameStatistics::default();
        stats.record_game(
            ChessColor::Red,
            GameResult::Win {
                winner: ChessColor::Black,
            },
            35,
            None,
        );
        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.losses_as_red, 1);
        assert_eq!(stats.total_losses(), 1);
        assert_eq!(stats.win_rate(), 0.0);
        assert_eq!(stats.current_win_streak, 0);
    }

    #[test]
    fn test_record_draw() {
        let mut stats = GameStatistics::default();
        stats.record_game(
            ChessColor::Black,
            GameResult::Draw(DrawReason::Agreement),
            50,
            None,
        );
        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.draws, 1);
        assert_eq!(stats.win_rate(), 0.0);
    }

    #[test]
    fn test_win_streak() {
        let mut stats = GameStatistics::default();

        // Win 3 games
        for _ in 0..3 {
            stats.record_game(
                ChessColor::Red,
                GameResult::Win {
                    winner: ChessColor::Red,
                },
                30,
                None,
            );
        }
        assert_eq!(stats.current_win_streak, 3);
        assert_eq!(stats.longest_win_streak, 3);

        // Lose 1 game
        stats.record_game(
            ChessColor::Red,
            GameResult::Win {
                winner: ChessColor::Black,
            },
            25,
            None,
        );
        assert_eq!(stats.current_win_streak, 0);
        assert_eq!(stats.longest_win_streak, 3);

        // Win 2 more games
        for _ in 0..2 {
            stats.record_game(
                ChessColor::Red,
                GameResult::Win {
                    winner: ChessColor::Red,
                },
                30,
                None,
            );
        }
        assert_eq!(stats.current_win_streak, 2);
        assert_eq!(stats.longest_win_streak, 3); // Still 3, not 2
    }

    #[test]
    fn test_opening_tracking() {
        let mut stats = GameStatistics::default();

        stats.record_game(
            ChessColor::Red,
            GameResult::Win {
                winner: ChessColor::Red,
            },
            40,
            Some("h2e2".to_string()),
        );
        stats.record_game(
            ChessColor::Red,
            GameResult::Win {
                winner: ChessColor::Red,
            },
            35,
            Some("h2e2".to_string()),
        );
        stats.record_game(
            ChessColor::Red,
            GameResult::Win {
                winner: ChessColor::Black,
            },
            30,
            Some("b0c2".to_string()),
        );

        assert_eq!(stats.opening_moves.get("h2e2"), Some(&2));
        assert_eq!(stats.opening_moves.get("b0c2"), Some(&1));

        let (opening, count) = stats.favorite_opening().unwrap();
        assert_eq!(opening, "h2e2");
        assert_eq!(count, &2);
    }

    #[test]
    fn test_average_moves() {
        let mut stats = GameStatistics::default();

        stats.record_game(
            ChessColor::Red,
            GameResult::Win {
                winner: ChessColor::Red,
            },
            40,
            None,
        );
        stats.record_game(
            ChessColor::Red,
            GameResult::Draw(DrawReason::Stalemate),
            60,
            None,
        );

        assert_eq!(stats.avg_moves_per_game(), 50.0);
    }

    #[test]
    fn test_reset() {
        let mut stats = GameStatistics::default();
        stats.record_game(
            ChessColor::Red,
            GameResult::Win {
                winner: ChessColor::Red,
            },
            40,
            Some("h2e2".to_string()),
        );
        assert_eq!(stats.games_played, 1);

        stats.reset();
        assert_eq!(stats.games_played, 0);
        assert_eq!(stats.total_wins(), 0);
        assert_eq!(stats.opening_moves.len(), 0);
    }
}
