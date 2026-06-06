//! Enhanced move history search with filters.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryFilter {
    All,
    RedMoves,
    BlackMoves,
    Captures,
    Checks,
}

impl HistoryFilter {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::RedMoves => "红方",
            Self::BlackMoves => "黑方",
            Self::Captures => "吃子",
            Self::Checks => "将军",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct HistorySearch {
    pub enabled: bool,
    pub filter: HistoryFilter,
    pub query: String,
    pub results: Vec<usize>,
}

impl Default for HistorySearch {
    fn default() -> Self {
        Self {
            enabled: false,
            filter: HistoryFilter::All,
            query: String::new(),
            results: Vec::new(),
        }
    }
}

impl HistorySearch {
    pub fn set_filter(&mut self, filter: HistoryFilter) {
        self.filter = filter;
        self.results.clear();
    }
    pub fn search(&mut self, query: &str) {
        self.query = query.to_string();
    }
    pub fn next_filter(&mut self) {
        self.filter = match self.filter {
            HistoryFilter::All => HistoryFilter::RedMoves,
            HistoryFilter::RedMoves => HistoryFilter::BlackMoves,
            HistoryFilter::BlackMoves => HistoryFilter::Captures,
            HistoryFilter::Captures => HistoryFilter::Checks,
            HistoryFilter::Checks => HistoryFilter::All,
        };
    }
}

pub fn toggle_history_search(
    keys: Res<ButtonInput<KeyCode>>,
    mut hs: ResMut<HistorySearch>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if ctrl && shift && keys.just_pressed(KeyCode::KeyF) {
        hs.next_filter();
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("历史过滤: {}", hs.filter.label_cn()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cycle() {
        let mut hs = HistorySearch::default();
        hs.next_filter();
        assert_eq!(hs.filter, HistoryFilter::RedMoves);
    }
}
