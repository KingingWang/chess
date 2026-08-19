//! Evaluation bar and search information display.
//!
//! Shows a vertical bar indicating the current position evaluation,
//! plus search depth, score, and principal variation when available.

use bevy::prelude::*;

use crate::ai_bridge::SearchInfoResource;
use crate::app_state::{CoreGame, GameMode, UiFonts};

/// Marker for the evaluation bar container.
#[derive(Component)]
pub struct EvalBarContainer;

/// Marker for the red portion of the evaluation bar.
#[derive(Component)]
pub struct EvalBarRed;

/// Marker for the search info text.
#[derive(Component)]
pub struct SearchInfoText;

/// Set up the evaluation bar and search info display.
pub fn setup_eval_bar(mut commands: Commands, fonts: Res<UiFonts>, core: Res<CoreGame>) {
    // Only show in VsAi mode for now
    if core.mode != GameMode::VsAi {
        return;
    }

    // Slim vertical bar docked just left of the right panel, vertically
    // centred. Dark well with a cinnabar fill growing from the bottom.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(248.0),
                top: Val::Percent(50.0),
                margin: UiRect::top(Val::Px(-230.0)),
                width: Val::Px(12.0),
                height: Val::Px(460.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(crate::ui_theme::CARD),
            BorderColor::all(crate::ui_theme::HAIRLINE),
            EvalBarContainer,
        ))
        .with_children(|parent| {
            // Red portion (bottom half when equal)
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(50.0),
                    margin: UiRect::top(Val::Auto),
                    ..default()
                },
                BackgroundColor(crate::ui_theme::CINNABAR),
                EvalBarRed,
            ));
        });

    // Search info text bottom-left of the right panel (depth / score / PV).
    commands.spawn((
        Text::new(""),
        TextFont {
            font: fonts.regular.clone(),
            font_size: 12.0,
            ..default()
        },
        TextColor(crate::ui_theme::TEXT_FAINT),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(248.0),
            bottom: Val::Px(16.0),
            width: Val::Px(210.0),
            ..default()
        },
        SearchInfoText,
    ));
}

/// Update the evaluation bar based on search info.
pub fn update_eval_bar(
    search_info: Res<SearchInfoResource>,
    mut red_bar: Query<&mut Node, (With<EvalBarRed>, Without<SearchInfoText>)>,
    mut info_text: Query<&mut Text, (With<SearchInfoText>, Without<EvalBarRed>)>,
) {
    let Ok(mut red_node) = red_bar.single_mut() else {
        return;
    };
    let Ok(mut text) = info_text.single_mut() else {
        return;
    };

    if let Some(info) = &search_info.latest {
        // Convert centipawn score to bar percentage.
        // `score` is side-to-move perspective; the bar shows Red's.
        let score_cp = info.red_score();

        // Clamp score to reasonable range (-1000 to +1000 centipawns)
        let clamped = score_cp.clamp(-1000, 1000);

        // Convert to percentage (50% = equal, higher = Red advantage)
        // Formula: 50% + (score / 20) to map -1000..+1000 to 0%..100%
        let red_percent = (50.0 + (clamped as f32 / 20.0)).clamp(0.0, 100.0);

        red_node.height = Val::Percent(red_percent);

        // Update search info text
        let score_str = if score_cp.abs() >= 9900 {
            // Mate score
            let moves_to_mate = (10000 - score_cp.abs()) / 2;
            if score_cp > 0 {
                format!("M{} (红胜)", moves_to_mate)
            } else {
                format!("M{} (黑胜)", moves_to_mate)
            }
        } else {
            let pawns = score_cp as f32 / 100.0;
            format!("{:+.2}", pawns)
        };

        let pv_str = if !info.pv.is_empty() {
            let pv_moves: Vec<String> = info
                .pv
                .iter()
                .take(5)
                .map(|m| format!("{}{}", (b'a' + m.from.file()) as char, m.from.rank() + 1))
                .collect();
            pv_moves.join(" ")
        } else {
            String::new()
        };

        **text = format!(
            "深度: {}\n评估: {}\n节点: {}\n{}",
            info.depth,
            score_str,
            info.nodes,
            if pv_str.is_empty() {
                String::new()
            } else {
                format!("PV: {}", pv_str)
            }
        );
    } else {
        // No search info, show neutral
        red_node.height = Val::Percent(50.0);
        **text = String::new();
    }
}

/// Clean up evaluation bar when leaving game.
pub fn teardown_eval_bar(
    mut commands: Commands,
    container: Query<Entity, With<EvalBarContainer>>,
    info_text: Query<Entity, With<SearchInfoText>>,
) {
    for entity in &container {
        commands.entity(entity).despawn();
    }
    for entity in &info_text {
        commands.entity(entity).despawn();
    }
}
