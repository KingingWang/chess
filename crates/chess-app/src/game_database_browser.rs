//! Game database browser for reviewing past games.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct GameRecord {
    pub id: u32,
    pub date: String,
    pub white: String,
    pub black: String,
    pub result: String,
    pub moves: Vec<String>,
    pub opening: String,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct GameDatabaseBrowser {
    pub visible: bool,
    pub games: Vec<GameRecord>,
    pub selected: Option<usize>,
    pub search_query: String,
}

impl GameDatabaseBrowser {
    pub fn add_game(&mut self, game: GameRecord) {
        self.games.push(game);
    }
    pub fn select_game(&mut self, idx: usize) {
        if idx < self.games.len() {
            self.selected = Some(idx);
        }
    }
    pub fn selected_game(&self) -> Option<&GameRecord> {
        self.selected.and_then(|i| self.games.get(i))
    }
    pub fn search(&mut self, query: &str) -> Vec<usize> {
        self.search_query = query.to_string();
        self.games
            .iter()
            .enumerate()
            .filter(|(_, g)| {
                g.white.contains(query)
                    || g.black.contains(query)
                    || g.opening.contains(query)
                    || g.result.contains(query)
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn game_count(&self) -> usize {
        self.games.len()
    }
}

pub fn toggle_database_browser(
    keys: Res<ButtonInput<KeyCode>>,
    mut gdb: ResMut<GameDatabaseBrowser>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyG) {
        gdb.visible = !gdb.visible;
        let msg = if gdb.visible {
            format!("数据库: {}局", gdb.game_count())
        } else {
            "数据库已关闭".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut gdb = GameDatabaseBrowser::default();
        gdb.add_game(GameRecord {
            id: 1,
            date: "2026-01-01".to_string(),
            white: "Red".to_string(),
            black: "Black".to_string(),
            result: "1-0".to_string(),
            moves: vec![],
            opening: "中炮".to_string(),
        });
        assert_eq!(gdb.game_count(), 1);
    }
    #[test]
    fn test_search() {
        let mut gdb = GameDatabaseBrowser::default();
        gdb.add_game(GameRecord {
            id: 1,
            date: "".to_string(),
            white: "张三".to_string(),
            black: "李四".to_string(),
            result: "1-0".to_string(),
            moves: vec![],
            opening: "中炮".to_string(),
        });
        let results = gdb.search("张三");
        assert_eq!(results.len(), 1);
    }
}
