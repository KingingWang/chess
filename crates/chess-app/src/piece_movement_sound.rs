//! Piece movement sound system with per-piece sounds.

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceType {
    King,
    Advisor,
    Elephant,
    Horse,
    Rook,
    Cannon,
    Pawn,
}

impl PieceType {
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'K' => Some(Self::King),
            'A' => Some(Self::Advisor),
            'E' => Some(Self::Elephant),
            'H' => Some(Self::Horse),
            'R' => Some(Self::Rook),
            'C' => Some(Self::Cannon),
            'P' => Some(Self::Pawn),
            _ => None,
        }
    }
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::King => "帅/将",
            Self::Advisor => "仕/士",
            Self::Elephant => "相/象",
            Self::Horse => "马",
            Self::Rook => "车",
            Self::Cannon => "炮",
            Self::Pawn => "兵/卒",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct PieceMovementSound {
    pub enabled: bool,
    pub volumes: HashMap<PieceType, f32>,
    pub master_volume: f32,
}

impl Default for PieceMovementSound {
    fn default() -> Self {
        let mut volumes = HashMap::new();
        for piece in [
            PieceType::King,
            PieceType::Advisor,
            PieceType::Elephant,
            PieceType::Horse,
            PieceType::Rook,
            PieceType::Cannon,
            PieceType::Pawn,
        ] {
            volumes.insert(piece, 1.0);
        }
        Self {
            enabled: true,
            volumes,
            master_volume: 0.8,
        }
    }
}

impl PieceMovementSound {
    pub fn get_volume(&self, piece: PieceType) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        self.volumes.get(&piece).copied().unwrap_or(1.0) * self.master_volume
    }
    pub fn set_volume(&mut self, piece: PieceType, volume: f32) {
        self.volumes.insert(piece, volume.clamp(0.0, 1.0));
    }
}

pub fn toggle_movement_sound(
    keys: Res<ButtonInput<KeyCode>>,
    mut pms: ResMut<PieceMovementSound>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyM) {
        pms.enabled = !pms.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if pms.enabled {
                "棋子移动音效已开启"
            } else {
                "棋子移动音效已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_from_char() {
        assert_eq!(PieceType::from_char('R'), Some(PieceType::Rook));
        assert_eq!(PieceType::from_char('x'), None);
    }
    #[test]
    fn test_volume() {
        let mut pms = PieceMovementSound::default();
        pms.set_volume(PieceType::Rook, 0.5);
        assert!((pms.get_volume(PieceType::Rook) - 0.4).abs() < 0.01);
    }
    #[test]
    fn test_disabled() {
        let mut pms = PieceMovementSound::default();
        pms.enabled = false;
        assert_eq!(pms.get_volume(PieceType::King), 0.0);
    }
}
