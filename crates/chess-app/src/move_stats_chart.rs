//! Move statistics visualization.
//!
//! Provides data for charting move-by-move statistics:
//! - Evaluation over time
//! - Time spent per move
//! - Move quality distribution

use bevy::prelude::*;

/// Chart data point.
#[derive(Debug, Clone)]
pub struct ChartPoint {
    pub move_number: usize,
    pub value: f32,
    pub label: Option<String>,
}

/// Chart types available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    Evaluation,
    MoveTime,
    Quality,
}

impl ChartType {
    pub fn label_cn(&self) -> &'static str {
        match self {
            Self::Evaluation => "评估曲线",
            Self::MoveTime => "用时分布",
            Self::Quality => "着法质量",
        }
    }
}

/// Resource managing chart data.
#[derive(Resource, Debug, Clone)]
pub struct MoveStatsChart {
    pub visible: bool,
    pub chart_type: ChartType,
    pub eval_data: Vec<ChartPoint>,
    pub time_data: Vec<ChartPoint>,
    pub quality_data: Vec<ChartPoint>,
}

impl Default for MoveStatsChart {
    fn default() -> Self {
        Self {
            visible: false,
            chart_type: ChartType::Evaluation,
            eval_data: Vec::new(),
            time_data: Vec::new(),
            quality_data: Vec::new(),
        }
    }
}

impl MoveStatsChart {
    pub fn add_eval_point(&mut self, move_num: usize, eval: f32) {
        self.eval_data.push(ChartPoint {
            move_number: move_num,
            value: eval,
            label: None,
        });
    }

    pub fn add_time_point(&mut self, move_num: usize, time_secs: f32) {
        self.time_data.push(ChartPoint {
            move_number: move_num,
            value: time_secs,
            label: None,
        });
    }

    pub fn current_data(&self) -> &[ChartPoint] {
        match self.chart_type {
            ChartType::Evaluation => &self.eval_data,
            ChartType::MoveTime => &self.time_data,
            ChartType::Quality => &self.quality_data,
        }
    }

    pub fn max_value(&self) -> f32 {
        self.current_data()
            .iter()
            .map(|p| p.value.abs())
            .fold(0.0f32, f32::max)
    }

    pub fn min_value(&self) -> f32 {
        self.current_data()
            .iter()
            .map(|p| p.value)
            .fold(f32::MAX, f32::min)
    }

    pub fn cycle_chart_type(&mut self) {
        self.chart_type = match self.chart_type {
            ChartType::Evaluation => ChartType::MoveTime,
            ChartType::MoveTime => ChartType::Quality,
            ChartType::Quality => ChartType::Evaluation,
        };
    }

    pub fn clear(&mut self) {
        self.eval_data.clear();
        self.time_data.clear();
        self.quality_data.clear();
    }

    pub fn data_summary(&self) -> String {
        let data = self.current_data();
        if data.is_empty() {
            return "暂无数据".to_string();
        }
        let avg = data.iter().map(|p| p.value).sum::<f32>() / data.len() as f32;
        format!(
            "{}: {}个数据点, 平均{:.1}",
            self.chart_type.label_cn(),
            data.len(),
            avg
        )
    }
}

pub fn toggle_chart(
    keys: Res<ButtonInput<KeyCode>>,
    mut chart: ResMut<MoveStatsChart>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyG) {
        if chart.visible {
            chart.cycle_chart_type();
            let msg = format!("图表: {}", chart.chart_type.label_cn());
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        } else {
            chart.visible = true;
            let msg = format!("图表已打开: {}", chart.chart_type.label_cn());
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        }
    }
    // Shift+G to close
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift && keys.just_pressed(KeyCode::KeyG) {
        chart.visible = false;
        crate::toast::spawn_toast(&mut commands, &fonts, "图表已关闭");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let c = MoveStatsChart::default();
        assert!(!c.visible);
        assert_eq!(c.chart_type, ChartType::Evaluation);
    }

    #[test]
    fn test_add_eval_point() {
        let mut c = MoveStatsChart::default();
        c.add_eval_point(0, 100.0);
        c.add_eval_point(1, -50.0);
        assert_eq!(c.eval_data.len(), 2);
    }

    #[test]
    fn test_max_min_value() {
        let mut c = MoveStatsChart::default();
        c.add_eval_point(0, 100.0);
        c.add_eval_point(1, -200.0);
        c.add_eval_point(2, 50.0);
        assert_eq!(c.max_value(), 200.0);
        assert_eq!(c.min_value(), -200.0);
    }

    #[test]
    fn test_cycle_chart_type() {
        let mut c = MoveStatsChart::default();
        assert_eq!(c.chart_type, ChartType::Evaluation);
        c.cycle_chart_type();
        assert_eq!(c.chart_type, ChartType::MoveTime);
        c.cycle_chart_type();
        assert_eq!(c.chart_type, ChartType::Quality);
        c.cycle_chart_type();
        assert_eq!(c.chart_type, ChartType::Evaluation);
    }

    #[test]
    fn test_clear() {
        let mut c = MoveStatsChart::default();
        c.add_eval_point(0, 100.0);
        c.add_time_point(0, 5.0);
        c.clear();
        assert!(c.eval_data.is_empty());
        assert!(c.time_data.is_empty());
    }

    #[test]
    fn test_data_summary() {
        let mut c = MoveStatsChart::default();
        assert!(c.data_summary().contains("暂无"));
        c.add_eval_point(0, 100.0);
        c.add_eval_point(1, 200.0);
        let summary = c.data_summary();
        assert!(summary.contains("2"));
        assert!(summary.contains("评估"));
    }

    #[test]
    fn test_chart_type_labels() {
        assert_eq!(ChartType::Evaluation.label_cn(), "评估曲线");
        assert_eq!(ChartType::MoveTime.label_cn(), "用时分布");
    }
}
