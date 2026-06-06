//! Tournament mode for organizing round-robin or Swiss tournaments.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct TournamentPlayer {
    pub name: String,
    pub score: f32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}

impl TournamentPlayer {
    pub fn games_played(&self) -> u32 {
        self.wins + self.losses + self.draws
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TournamentFormat {
    RoundRobin,
    Swiss,
}

impl TournamentFormat {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::RoundRobin => "循环赛",
            Self::Swiss => "瑞士制",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct Tournament {
    pub active: bool,
    pub format: TournamentFormat,
    pub players: Vec<TournamentPlayer>,
    pub current_round: u32,
    pub total_rounds: u32,
}

impl Default for Tournament {
    fn default() -> Self {
        Self {
            active: false,
            format: TournamentFormat::Swiss,
            players: Vec::new(),
            current_round: 0,
            total_rounds: 5,
        }
    }
}

impl Tournament {
    pub fn add_player(&mut self, name: &str) {
        self.players.push(TournamentPlayer {
            name: name.to_string(),
            score: 0.0,
            wins: 0,
            losses: 0,
            draws: 0,
        });
    }
    pub fn standings(&self) -> Vec<&TournamentPlayer> {
        let mut sorted: Vec<&TournamentPlayer> = self.players.iter().collect();
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        sorted
    }
    pub fn record_result(&mut self, white_idx: usize, black_idx: usize, white_score: f32) {
        if white_idx >= self.players.len() || black_idx >= self.players.len() {
            return;
        }
        self.players[white_idx].score += white_score;
        self.players[black_idx].score += 1.0 - white_score;
        match white_score as u32 {
            1 => {
                self.players[white_idx].wins += 1;
                self.players[black_idx].losses += 1;
            }
            0 => {
                self.players[white_idx].losses += 1;
                self.players[black_idx].wins += 1;
            }
            _ => {
                self.players[white_idx].draws += 1;
                self.players[black_idx].draws += 1;
            }
        }
    }
}

pub fn toggle_tournament(
    keys: Res<ButtonInput<KeyCode>>,
    mut t: ResMut<Tournament>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyM) {
        t.active = !t.active;
        let msg = if t.active {
            format!("锦标赛模式: {}", t.format.label_cn())
        } else {
            "锦标赛已关闭".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_player() {
        let mut t = Tournament::default();
        t.add_player("Alice");
        assert_eq!(t.players.len(), 1);
    }
    #[test]
    fn test_record_win() {
        let mut t = Tournament::default();
        t.add_player("A");
        t.add_player("B");
        t.record_result(0, 1, 1.0);
        assert_eq!(t.players[0].wins, 1);
        assert_eq!(t.players[1].losses, 1);
    }
    #[test]
    fn test_standings() {
        let mut t = Tournament::default();
        t.add_player("A");
        t.add_player("B");
        t.record_result(0, 1, 1.0);
        let s = t.standings();
        assert_eq!(s[0].name, "A");
    }
}
