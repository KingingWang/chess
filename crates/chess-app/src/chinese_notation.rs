//! Chinese move notation display (炮二平五, 马八进七).
//!
//! Converts ICCS notation to traditional Chinese chess notation
//! used in Chinese chess literature and broadcasts.

use bevy::prelude::*;

/// Resource managing Chinese notation display.
#[derive(Resource, Debug, Clone, Default)]
pub struct ChineseNotation {
    /// Whether to show Chinese notation in the history panel.
    pub enabled: bool,
}

/// Chinese piece names (Red side).
const RED_PIECE_NAMES: &[(char, &str)] = &[
    ('K', "帅"),
    ('A', "仕"),
    ('E', "相"),
    ('H', "馬"),
    ('R', "車"),
    ('C', "炮"),
    ('P', "兵"),
];

/// Chinese piece names (Black side).
const BLACK_PIECE_NAMES: &[(char, &str)] = &[
    ('K', "将"),
    ('A', "士"),
    ('E', "象"),
    ('H', "馬"),
    ('R', "車"),
    ('C', "砲"),
    ('P', "卒"),
];

/// Chinese numerals for files (1-9, right to left from player's perspective).
const CN_NUMERALS: &[&str] = &["〇", "一", "二", "三", "四", "五", "六", "七", "八", "九"];

/// Convert an ICCS move to Chinese notation.
///
/// `piece_char`: piece type ('K','A','E','H','R','C','P')
/// `is_red`: whether the moving side is Red
/// `from_file`: source file (0-8, a=0 to i=8)
/// `to_file`: destination file
/// `from_rank`: source rank (0-9)
/// `to_rank`: destination rank
pub fn to_chinese_notation(
    piece_char: char,
    is_red: bool,
    from_file: u8,
    to_file: u8,
    from_rank: u8,
    to_rank: u8,
) -> String {
    let piece_name = if is_red {
        RED_PIECE_NAMES
            .iter()
            .find(|(ch, _)| *ch == piece_char)
            .map(|(_, name)| *name)
            .unwrap_or("?")
    } else {
        BLACK_PIECE_NAMES
            .iter()
            .find(|(ch, _)| *ch == piece_char)
            .map(|(_, name)| *name)
            .unwrap_or("?")
    };

    // File number (1-9, from right to left for Red; left to right for Black)
    let file_num = if is_red { 9 - from_file } else { from_file + 1 };
    let file_cn = CN_NUMERALS[file_num as usize];

    // Action: 进(advance), 退(retreat), 平(horizontal move)
    let (action, dest) = if from_rank == to_rank {
        // Horizontal move (平)
        let dest_file = if is_red { 9 - to_file } else { to_file + 1 };
        ("平", CN_NUMERALS[dest_file as usize].to_string())
    } else if (is_red && to_rank > from_rank) || (!is_red && to_rank < from_rank) {
        // Advance (进)
        let steps = if is_red {
            to_rank - from_rank
        } else {
            from_rank - to_rank
        };
        // For horizontal-moving pieces (R, C, K, P), show destination file
        // For diagonal/vertical pieces (A, E, H), show rank change
        match piece_char {
            'A' | 'E' | 'H' => ("进", CN_NUMERALS[steps as usize].to_string()),
            _ => {
                let dest_file = if is_red { 9 - to_file } else { to_file + 1 };
                ("进", CN_NUMERALS[dest_file as usize].to_string())
            }
        }
    } else {
        // Retreat (退)
        let steps = if is_red {
            from_rank - to_rank
        } else {
            to_rank - from_rank
        };
        match piece_char {
            'A' | 'E' | 'H' => ("退", CN_NUMERALS[steps as usize].to_string()),
            _ => {
                let dest_file = if is_red { 9 - to_file } else { to_file + 1 };
                ("退", CN_NUMERALS[dest_file as usize].to_string())
            }
        }
    };

    format!("{}{}{}{}", piece_name, file_cn, action, dest)
}

/// Toggle Chinese notation display.
pub fn toggle_chinese_notation(
    keys: Res<ButtonInput<KeyCode>>,
    mut notation: ResMut<ChineseNotation>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyN) {
        notation.enabled = !notation.enabled;
        let msg = if notation.enabled {
            "已切换中文棋谱"
        } else {
            "已切换西式棋谱"
        };
        crate::toast::spawn_toast(&mut commands, &fonts, msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cannon_central() {
        // 炮二平五 (Cannon from file 2 to file 5, horizontal)
        let result = to_chinese_notation('C', true, 7, 4, 2, 2);
        assert_eq!(result, "炮二平五");
    }

    #[test]
    fn test_horse_advance() {
        // 馬八进七 (Horse from file 8 to file 7, advancing)
        let result = to_chinese_notation('H', true, 1, 2, 0, 2);
        assert_eq!(result, "馬八进二");
    }

    #[test]
    fn test_rook_horizontal() {
        let result = to_chinese_notation('R', true, 8, 0, 3, 3);
        assert!(result.starts_with("車"));
        assert!(result.contains("平"));
    }

    #[test]
    fn test_pawn_advance() {
        let result = to_chinese_notation('P', true, 2, 2, 3, 4);
        assert!(result.starts_with("兵"));
        assert!(result.contains("进"));
    }

    #[test]
    fn test_black_notation() {
        let result = to_chinese_notation('C', false, 7, 4, 7, 7);
        assert!(result.starts_with("砲"));
    }

    #[test]
    fn test_king_retreat() {
        let result = to_chinese_notation('K', true, 4, 4, 9, 8);
        assert!(result.starts_with("帅"));
        assert!(result.contains("退"));
    }

    #[test]
    fn test_notation_default() {
        let n = ChineseNotation::default();
        assert!(!n.enabled);
    }
}
