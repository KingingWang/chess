//! Move legality visualization showing why illegal moves are rejected.

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub enum IllegalReason {
    NotYourTurn,
    PieceCantMoveThatWay,
    BlockedByPiece,
    WouldBeInCheck,
    InvalidDestination,
    OffBoard,
}

impl IllegalReason {
    pub fn explain_cn(&self) -> &'static str {
        match self {
            Self::NotYourTurn => "不是你的回合",
            Self::PieceCantMoveThatWay => "该棋子不能这样移动",
            Self::BlockedByPiece => "移动路径被阻挡",
            Self::WouldBeInCheck => "走棋后会被将军",
            Self::InvalidDestination => "目标位置无效",
            Self::OffBoard => "超出棋盘范围",
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MoveLegality {
    pub enabled: bool,
    pub last_illegal: Option<(String, IllegalReason)>,
    pub show_hints: bool,
}

impl MoveLegality {
    pub fn record_illegal(&mut self, notation: &str, reason: IllegalReason) {
        self.last_illegal = Some((notation.to_string(), reason));
    }
    pub fn clear(&mut self) {
        self.last_illegal = None;
    }
    pub fn last_explanation(&self) -> Option<String> {
        self.last_illegal
            .as_ref()
            .map(|(n, r)| format!("{}: {}", n, r.explain_cn()))
    }
}

pub fn toggle_legality(
    keys: Res<ButtonInput<KeyCode>>,
    mut ml: ResMut<MoveLegality>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyL) {
        ml.enabled = !ml.enabled;
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            if ml.enabled {
                "合法性提示已开启"
            } else {
                "合法性提示已关闭"
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_reasons() {
        assert_eq!(IllegalReason::NotYourTurn.explain_cn(), "不是你的回合");
    }
    #[test]
    fn test_record() {
        let mut ml = MoveLegality::default();
        ml.record_illegal("h2e2", IllegalReason::BlockedByPiece);
        let exp = ml.last_explanation();
        assert!(exp.is_some());
        assert!(exp.unwrap().contains("阻挡"));
    }
    #[test]
    fn test_clear() {
        let mut ml = MoveLegality::default();
        ml.record_illegal("x", IllegalReason::OffBoard);
        ml.clear();
        assert!(ml.last_illegal.is_none());
    }
}
