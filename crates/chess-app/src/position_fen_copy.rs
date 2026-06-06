//! Quick FEN position copy/paste system.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct PositionFenCopy {
    pub last_copied: Option<String>,
    pub clipboard_available: bool,
}

impl Default for PositionFenCopy {
    fn default() -> Self {
        Self {
            last_copied: None,
            clipboard_available: true,
        }
    }
}

impl PositionFenCopy {
    pub fn copy_fen(&mut self, fen: &str) -> bool {
        self.last_copied = Some(fen.to_string());
        self.clipboard_available
    }

    pub fn paste_fen(&self) -> Option<&str> {
        self.last_copied.as_deref()
    }
}

pub fn copy_position(
    keys: Res<ButtonInput<KeyCode>>,
    core: Res<crate::app_state::CoreGame>,
    mut fen_copy: ResMut<PositionFenCopy>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyC) && keys.pressed(KeyCode::ShiftLeft) {
        let fen = core.game.board().to_fen();
        fen_copy.copy_fen(&fen);
        crate::toast::spawn_toast(&mut commands, &fonts, "位置FEN已复制");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_copy_paste() {
        let mut fc = PositionFenCopy::default();
        fc.copy_fen("test_fen");
        assert_eq!(fc.paste_fen(), Some("test_fen"));
    }
}
