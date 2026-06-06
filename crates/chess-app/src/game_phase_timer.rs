//! Game phase timing with different time controls per phase.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamePhaseTime {
    Opening { moves: u32, time_per_move: f32 },
    Middlegame { moves: u32, time_per_move: f32 },
    Endgame { increment: f32 },
}

#[derive(Resource, Debug, Clone)]
pub struct GamePhaseTimer {
    pub enabled: bool,
    pub current_phase: GamePhaseTime,
    pub move_count: u32,
}

impl Default for GamePhaseTimer {
    fn default() -> Self {
        Self {
            enabled: false,
            current_phase: GamePhaseTime::Opening {
                moves: 10,
                time_per_move: 30.0,
            },
            move_count: 0,
        }
    }
}

impl GamePhaseTimer {
    pub fn advance_move(&mut self) {
        self.move_count += 1;
        match self.current_phase {
            GamePhaseTime::Opening { moves, .. } if self.move_count >= moves => {
                self.current_phase = GamePhaseTime::Middlegame {
                    moves: 30,
                    time_per_move: 60.0,
                };
            }
            GamePhaseTime::Middlegame { moves, .. } if self.move_count >= moves => {
                self.current_phase = GamePhaseTime::Endgame { increment: 10.0 };
            }
            _ => {}
        }
    }

    pub fn phase_name(&self) -> &'static str {
        match self.current_phase {
            GamePhaseTime::Opening { .. } => "开局",
            GamePhaseTime::Middlegame { .. } => "中局",
            GamePhaseTime::Endgame { .. } => "残局",
        }
    }
}

pub fn toggle_phase_timer(
    keys: Res<ButtonInput<KeyCode>>,
    mut gpt: ResMut<GamePhaseTimer>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyG) {
        gpt.enabled = !gpt.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if gpt.enabled {
                "阶段计时已开启"
            } else {
                "阶段计时已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_phase_transition() {
        let mut gpt = GamePhaseTimer::default();
        for _ in 0..10 {
            gpt.advance_move();
        }
        assert_eq!(gpt.phase_name(), "中局");
    }
}
