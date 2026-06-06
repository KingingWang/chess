//! Tactical motif detection and highlighting.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TacticalMotif {
    Fork,
    Pin,
    Skewer,
    DiscoveredAttack,
    DoubleAttack,
    BackRankMate,
}

impl TacticalMotif {
    pub fn name_cn(&self) -> &'static str {
        match self {
            Self::Fork => "捉双",
            Self::Pin => "牵制",
            Self::Skewer => "串打",
            Self::DiscoveredAttack => "闪击",
            Self::DoubleAttack => "双攻",
            Self::BackRankMate => "底线杀",
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct TacticalMotifsDetector {
    pub enabled: bool,
    pub detected: Vec<TacticalMotif>,
}

pub fn toggle_motifs_detector(
    keys: Res<ButtonInput<KeyCode>>,
    mut tmd: ResMut<TacticalMotifsDetector>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyM) {
        tmd.enabled = !tmd.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if tmd.enabled {
                "战术模式检测已开启"
            } else {
                "战术模式检测已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_names() {
        assert_eq!(TacticalMotif::Fork.name_cn(), "捉双");
        assert_eq!(TacticalMotif::Pin.name_cn(), "牵制");
    }
}
