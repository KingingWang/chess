//! Game database for storing and browsing completed games.
//!
//! Allows users to save, load, and search through their game history.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, GameResult, Move};
use std::collections::HashMap;

/// A completed game record.
#[derive(Debug, Clone)]
pub struct GameRecord {
    /// Unique identifier for the game.
    pub id: String,
    /// Date and time when the game was played.
    pub timestamp: String,
    /// Red player name.
    pub red_player: String,
    /// Black player name.
    pub black_player: String,
    /// Game result.
    pub result: Option<GameResult>,
    /// All moves played in the game.
    pub moves: Vec<Move>,
    /// Opening name (if identified).
    pub opening: Option<String>,
    /// Number of moves in the game.
    pub move_count: usize,
    /// Game duration in seconds.
    pub duration_secs: u64,
    /// Tags/metadata for the game.
    pub tags: HashMap<String, String>,
}

impl GameRecord {
    /// Create a new game record.
    pub fn new(
        id: String,
        red_player: String,
        black_player: String,
        moves: Vec<Move>,
        result: Option<GameResult>,
    ) -> Self {
        Self {
            id,
            timestamp: "unknown".to_string(),
            red_player,
            black_player,
            result,
            move_count: moves.len(),
            moves,
            opening: None,
            duration_secs: 0,
            tags: HashMap::new(),
        }
    }

    /// Get the winner's color, if any.
    pub fn winner(&self) -> Option<ChessColor> {
        match self.result {
            Some(GameResult::Win { winner, .. }) => Some(winner),
            _ => None,
        }
    }

    /// Check if the game was a draw.
    pub fn is_draw(&self) -> bool {
        matches!(self.result, Some(GameResult::Draw(_)))
    }

    /// Get a summary string for display.
    pub fn summary(&self) -> String {
        let result_str = match self.result {
            Some(GameResult::Win { winner, .. }) => match winner {
                ChessColor::Red => "1-0",
                ChessColor::Black => "0-1",
            },
            Some(GameResult::Draw(_)) => "1/2-1/2",
            None => "*",
        };

        format!(
            "{} vs {} - {} ({} moves)",
            self.red_player, self.black_player, result_str, self.move_count
        )
    }

    /// Add a tag to the game.
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    /// Get a tag value.
    pub fn get_tag(&self, key: &str) -> Option<&String> {
        self.tags.get(key)
    }
}

/// Resource managing the game database.
#[derive(Resource)]
pub struct GameDatabase {
    /// All saved games.
    pub games: Vec<GameRecord>,
    /// Current search filter.
    pub search_filter: Option<String>,
    /// Selected game index for viewing.
    pub selected_index: Option<usize>,
}

impl Default for GameDatabase {
    fn default() -> Self {
        Self {
            games: Vec::new(),
            search_filter: None,
            selected_index: None,
        }
    }
}

impl GameDatabase {
    /// Add a game to the database.
    pub fn add_game(&mut self, game: GameRecord) {
        self.games.push(game);
    }

    /// Remove a game by index.
    pub fn remove_game(&mut self, index: usize) -> Option<GameRecord> {
        if index < self.games.len() {
            Some(self.games.remove(index))
        } else {
            None
        }
    }

    /// Get a game by index.
    pub fn get_game(&self, index: usize) -> Option<&GameRecord> {
        self.games.get(index)
    }

    /// Get total number of games.
    pub fn game_count(&self) -> usize {
        self.games.len()
    }

    /// Search games by player name.
    pub fn search_by_player(&self, player_name: &str) -> Vec<usize> {
        self.games
            .iter()
            .enumerate()
            .filter(|(_, game)| {
                game.red_player
                    .to_lowercase()
                    .contains(&player_name.to_lowercase())
                    || game
                        .black_player
                        .to_lowercase()
                        .contains(&player_name.to_lowercase())
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Search games by opening name.
    pub fn search_by_opening(&self, opening_name: &str) -> Vec<usize> {
        self.games
            .iter()
            .enumerate()
            .filter(|(_, game)| {
                game.opening
                    .as_ref()
                    .map(|o| o.to_lowercase().contains(&opening_name.to_lowercase()))
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Get games by result (wins for a specific color).
    pub fn get_games_by_result(&self, winner: Option<ChessColor>) -> Vec<usize> {
        self.games
            .iter()
            .enumerate()
            .filter(|(_, game)| game.winner() == winner)
            .map(|(i, _)| i)
            .collect()
    }

    /// Get all draws.
    pub fn get_draws(&self) -> Vec<usize> {
        self.games
            .iter()
            .enumerate()
            .filter(|(_, game)| game.is_draw())
            .map(|(i, _)| i)
            .collect()
    }

    /// Get statistics for a player.
    pub fn player_stats(&self, player_name: &str) -> PlayerStats {
        let mut stats = PlayerStats::default();

        for game in &self.games {
            if game.red_player.to_lowercase() == player_name.to_lowercase() {
                stats.games_as_red += 1;
                match game.winner() {
                    Some(ChessColor::Red) => stats.wins += 1,
                    Some(ChessColor::Black) => stats.losses += 1,
                    None => {
                        if game.is_draw() {
                            stats.draws += 1;
                        }
                    }
                }
            } else if game.black_player.to_lowercase() == player_name.to_lowercase() {
                stats.games_as_black += 1;
                match game.winner() {
                    Some(ChessColor::Black) => stats.wins += 1,
                    Some(ChessColor::Red) => stats.losses += 1,
                    None => {
                        if game.is_draw() {
                            stats.draws += 1;
                        }
                    }
                }
            }
        }

        stats
    }

    /// Clear all games from the database.
    pub fn clear(&mut self) {
        self.games.clear();
        self.selected_index = None;
    }

    /// Select a game for viewing.
    pub fn select_game(&mut self, index: usize) {
        if index < self.games.len() {
            self.selected_index = Some(index);
        }
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selected_index = None;
    }

    /// Get the currently selected game.
    pub fn selected_game(&self) -> Option<&GameRecord> {
        self.selected_index.and_then(|i| self.games.get(i))
    }
}

/// Statistics for a player.
#[derive(Debug, Clone, Default)]
pub struct PlayerStats {
    /// Total games played as Red.
    pub games_as_red: usize,
    /// Total games played as Black.
    pub games_as_black: usize,
    /// Total wins.
    pub wins: usize,
    /// Total losses.
    pub losses: usize,
    /// Total draws.
    pub draws: usize,
}

impl PlayerStats {
    /// Get total games played.
    pub fn total_games(&self) -> usize {
        self.games_as_red + self.games_as_black
    }

    /// Get win rate as a percentage.
    pub fn win_rate(&self) -> f32 {
        let total = self.total_games();
        if total == 0 {
            return 0.0;
        }
        (self.wins as f32 / total as f32) * 100.0
    }

    /// Get draw rate as a percentage.
    pub fn draw_rate(&self) -> f32 {
        let total = self.total_games();
        if total == 0 {
            return 0.0;
        }
        (self.draws as f32 / total as f32) * 100.0
    }
}

/// Component for game database UI elements.
#[derive(Component)]
pub struct GameDatabaseUI;

/// System to toggle game database browser.
pub fn toggle_game_database(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    ui_query: Query<Entity, With<GameDatabaseUI>>,
    db: Res<GameDatabase>,
) {
    if keyboard.just_pressed(KeyCode::KeyG) {
        // Toggle UI
        if let Ok(entity) = ui_query.single() {
            commands.entity(entity).despawn();
        } else {
            spawn_game_database_ui(&mut commands, &db);
        }
    }
}

/// Spawn the game database UI.
fn spawn_game_database_ui(commands: &mut Commands, db: &GameDatabase) {
    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.8),
            custom_size: Some(Vec2::new(400.0, 500.0)),
            ..default()
        },
        Transform::from_xyz(-400.0, 0.0, 10.0),
        GameDatabaseUI,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Square;

    fn create_test_game(
        id: &str,
        red: &str,
        black: &str,
        winner: Option<ChessColor>,
    ) -> GameRecord {
        let from = Square::new(4, 0).unwrap();
        let to = Square::new(4, 1).unwrap();
        let result = winner.map(|w| GameResult::Win {
            winner: w,
            reason: chess_core::WinReason::Checkmate,
        });

        GameRecord::new(
            id.to_string(),
            red.to_string(),
            black.to_string(),
            vec![Move { from, to }],
            result,
        )
    }

    #[test]
    fn test_game_record_creation() {
        let game = create_test_game("test_1", "Alice", "Bob", Some(ChessColor::Red));
        assert_eq!(game.id, "test_1");
        assert_eq!(game.red_player, "Alice");
        assert_eq!(game.black_player, "Bob");
        assert_eq!(game.move_count, 1);
    }

    #[test]
    fn test_game_record_winner() {
        let game = create_test_game("test_1", "Alice", "Bob", Some(ChessColor::Red));
        assert_eq!(game.winner(), Some(ChessColor::Red));

        let draw_game = create_test_game("test_2", "Alice", "Bob", None);
        assert_eq!(draw_game.winner(), None);
    }

    #[test]
    fn test_game_record_summary() {
        let game = create_test_game("test_1", "Alice", "Bob", Some(ChessColor::Red));
        let summary = game.summary();
        assert!(summary.contains("Alice"));
        assert!(summary.contains("Bob"));
        assert!(summary.contains("1-0"));
    }

    #[test]
    fn test_game_database_add() {
        let mut db = GameDatabase::default();
        let game = create_test_game("test_1", "Alice", "Bob", Some(ChessColor::Red));

        db.add_game(game);
        assert_eq!(db.game_count(), 1);
    }

    #[test]
    fn test_game_database_remove() {
        let mut db = GameDatabase::default();
        db.add_game(create_test_game(
            "test_1",
            "Alice",
            "Bob",
            Some(ChessColor::Red),
        ));
        db.add_game(create_test_game(
            "test_2",
            "Charlie",
            "David",
            Some(ChessColor::Black),
        ));

        assert_eq!(db.game_count(), 2);

        let removed = db.remove_game(0);
        assert!(removed.is_some());
        assert_eq!(db.game_count(), 1);
    }

    #[test]
    fn test_game_database_search_by_player() {
        let mut db = GameDatabase::default();
        db.add_game(create_test_game(
            "test_1",
            "Alice",
            "Bob",
            Some(ChessColor::Red),
        ));
        db.add_game(create_test_game(
            "test_2",
            "Charlie",
            "Alice",
            Some(ChessColor::Black),
        ));
        db.add_game(create_test_game(
            "test_3",
            "David",
            "Eve",
            Some(ChessColor::Red),
        ));

        let results = db.search_by_player("Alice");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&0));
        assert!(results.contains(&1));
    }

    #[test]
    fn test_game_database_search_by_player_case_insensitive() {
        let mut db = GameDatabase::default();
        db.add_game(create_test_game(
            "test_1",
            "Alice",
            "Bob",
            Some(ChessColor::Red),
        ));

        let results = db.search_by_player("alice");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_game_database_get_by_result() {
        let mut db = GameDatabase::default();
        db.add_game(create_test_game(
            "test_1",
            "Alice",
            "Bob",
            Some(ChessColor::Red),
        ));
        db.add_game(create_test_game(
            "test_2",
            "Charlie",
            "David",
            Some(ChessColor::Black),
        ));
        db.add_game(create_test_game(
            "test_3",
            "Eve",
            "Frank",
            Some(ChessColor::Red),
        ));

        let red_wins = db.get_games_by_result(Some(ChessColor::Red));
        assert_eq!(red_wins.len(), 2);

        let black_wins = db.get_games_by_result(Some(ChessColor::Black));
        assert_eq!(black_wins.len(), 1);
    }

    #[test]
    fn test_player_stats() {
        let mut db = GameDatabase::default();
        db.add_game(create_test_game(
            "test_1",
            "Alice",
            "Bob",
            Some(ChessColor::Red),
        ));
        db.add_game(create_test_game(
            "test_2",
            "Charlie",
            "Alice",
            Some(ChessColor::Black),
        ));
        db.add_game(create_test_game(
            "test_3",
            "Alice",
            "David",
            Some(ChessColor::Red),
        ));

        let stats = db.player_stats("Alice");
        assert_eq!(stats.total_games(), 3);
        assert_eq!(stats.games_as_red, 2);
        assert_eq!(stats.games_as_black, 1);
        assert_eq!(stats.wins, 3);
        assert_eq!(stats.losses, 0);
    }

    #[test]
    fn test_player_stats_win_rate() {
        let mut db = GameDatabase::default();
        db.add_game(create_test_game(
            "test_1",
            "Alice",
            "Bob",
            Some(ChessColor::Red),
        ));
        db.add_game(create_test_game(
            "test_2",
            "Alice",
            "Charlie",
            Some(ChessColor::Red),
        ));
        db.add_game(create_test_game(
            "test_3",
            "Alice",
            "David",
            Some(ChessColor::Black),
        ));
        db.add_game(create_test_game(
            "test_4",
            "Alice",
            "Eve",
            Some(ChessColor::Black),
        ));

        let stats = db.player_stats("Alice");
        assert!((stats.win_rate() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_game_database_select() {
        let mut db = GameDatabase::default();
        db.add_game(create_test_game(
            "test_1",
            "Alice",
            "Bob",
            Some(ChessColor::Red),
        ));

        db.select_game(0);
        assert_eq!(db.selected_index, Some(0));
        assert!(db.selected_game().is_some());

        db.clear_selection();
        assert_eq!(db.selected_index, None);
        assert!(db.selected_game().is_none());
    }

    #[test]
    fn test_game_database_clear() {
        let mut db = GameDatabase::default();
        db.add_game(create_test_game(
            "test_1",
            "Alice",
            "Bob",
            Some(ChessColor::Red),
        ));
        db.add_game(create_test_game(
            "test_2",
            "Charlie",
            "David",
            Some(ChessColor::Black),
        ));

        db.clear();
        assert_eq!(db.game_count(), 0);
        assert!(db.selected_index.is_none());
    }

    #[test]
    fn test_game_record_tags() {
        let mut game = create_test_game("test_1", "Alice", "Bob", Some(ChessColor::Red));
        game.add_tag("Event".to_string(), "Tournament".to_string());
        game.add_tag("Round".to_string(), "5".to_string());

        assert_eq!(game.get_tag("Event"), Some(&"Tournament".to_string()));
        assert_eq!(game.get_tag("Round"), Some(&"5".to_string()));
        assert_eq!(game.get_tag("NonExistent"), None);
    }
}
