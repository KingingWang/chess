//! Help panel overlay listing keyboard shortcuts.
//!
//! Toggled with the H key. Renders as a semi-transparent dark overlay with
//! white CJK text above all other UI (z=50).

use bevy::prelude::*;

use crate::app_state::UiFonts;

/// Marker for help panel entities.
#[derive(Component)]
pub struct HelpPanelMarker;

/// Whether the help panel is visible.
#[derive(Resource, Default)]
pub struct HelpPanelVisible(pub bool);

const HELP_TEXT: &str = "\
快捷键一览 (共20+个快捷键)\n\
─────────────\n\
Ctrl+Z / U / Backspace  悔棋\n\
Ctrl+N        新局\n\
Ctrl+S        保存棋局\n\
Ctrl+O        加载棋局\n\
Ctrl+E        导出棋谱\n\
Ctrl+D        重置设置\n\
Esc           返回菜单\n\
T             切换主题\n\
F             翻转棋盘\n\
C             坐标显示\n\
A             动画速度\n\
M             音量调节\n\
H             帮助\n\
F11           全屏切换\n\
← →           浏览历史\n\
Home / End    跳转首/末
P / Space     自动播放\n\
R             快速重开\n\
1-4           切换难度\n\
Enter/Esc     对局结束操作\n\
右键          取消选择
拖拽棋子可直接走棋
设置（主题/音量等）自动保存";

/// Spawn the help panel (hidden initially).
pub fn setup_help_panel(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    theme: Res<crate::board_theme::BoardTheme>,
    volume: Res<crate::sound::SoundVolume>,
    anim: Res<crate::animation::AnimSpeedSetting>,
    ai: Res<crate::app_state::AiSettings>,
    session_stats: Res<crate::app_state::SessionStats>,
) {
    let game_count = session_stats.total();
    let session_line = if game_count > 0 {
        format!("\n本次对弈: {}局", game_count)
    } else {
        String::new()
    };
    let settings_block = format!(
        "\n\n当前设置\n─────────────\n主题: {}  音量: {}\n动画: {}  难度: {}{}",
        theme.id.label(),
        volume.level.label(),
        anim.0.label(),
        ai.difficulty.label(),
        session_line
    );
    let full_text = format!("{}{}", HELP_TEXT, settings_block);
    commands
        .spawn((
            Sprite {
                color: Color::srgba(0.0, 0.0, 0.0, 0.82),
                custom_size: Some(Vec2::new(420.0, 600.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 50.0),
            Visibility::Hidden,
            HelpPanelMarker,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text2d::new(full_text),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgba(0.95, 0.95, 0.95, 1.0)),
                Transform::from_xyz(0.0, 0.0, 0.1),
            ));
        });
}

/// Toggle help panel visibility on H key.
pub fn toggle_help_panel(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<HelpPanelVisible>,
    mut query: Query<&mut Visibility, With<HelpPanelMarker>>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    // Escape closes help panel if open (consumes Escape for this frame).
    if keys.just_pressed(KeyCode::Escape) && visible.0 {
        visible.0 = false;
        for mut v in &mut query {
            *v = Visibility::Hidden;
        }
        crate::moves::ESCAPE_CONSUMED.store(true, std::sync::atomic::Ordering::Relaxed);
        return;
    }

    if keys.just_pressed(KeyCode::KeyH) && !ctrl {
        visible.0 = !visible.0;
        let vis = if visible.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for mut v in &mut query {
            *v = vis;
        }
    }
}

/// Tear down help panel entities.
pub fn teardown_help_panel(mut commands: Commands, query: Query<Entity, With<HelpPanelMarker>>) {
    for e in &query {
        commands.entity(e).despawn();
    }
}
