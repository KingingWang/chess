//! Session timer for tracking play time.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct SessionTimer {
    pub active: bool,
    pub session_seconds: f64,
    pub game_seconds: f64,
    pub games_played: u32,
}

impl Default for SessionTimer {
    fn default() -> Self {
        Self {
            active: true,
            session_seconds: 0.0,
            game_seconds: 0.0,
            games_played: 0,
        }
    }
}

impl SessionTimer {
    pub fn format_duration(secs: f64) -> String {
        let hours = (secs / 3600.0) as u32;
        let mins = ((secs % 3600.0) / 60.0) as u32;
        let s = (secs % 60.0) as u32;
        if hours > 0 {
            format!("{}时{}分{}秒", hours, mins, s)
        } else if mins > 0 {
            format!("{}分{}秒", mins, s)
        } else {
            format!("{}秒", s)
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "游戏时间: {} | 对局数: {}",
            Self::format_duration(self.session_seconds),
            self.games_played
        )
    }

    pub fn record_game_end(&mut self) {
        self.games_played += 1;
        self.game_seconds = 0.0;
    }
}

pub fn update_session_timer(time: Res<Time>, mut timer: ResMut<SessionTimer>) {
    if timer.active {
        timer.session_seconds += time.delta_secs_f64();
        timer.game_seconds += time.delta_secs_f64();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format() {
        assert_eq!(SessionTimer::format_duration(5.0), "5秒");
        assert_eq!(SessionTimer::format_duration(65.0), "1分5秒");
        assert_eq!(SessionTimer::format_duration(3661.0), "1时1分1秒");
    }
    #[test]
    fn test_record() {
        let mut t = SessionTimer::default();
        t.record_game_end();
        assert_eq!(t.games_played, 1);
    }
}
