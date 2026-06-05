//! Ephemeral toast notification system.
//!
//! Spawns a centred text label at the top of the viewport that fades out
//! over a configurable duration. Used for undo confirmation, theme change,
//! and other transient feedback.

use bevy::prelude::*;

use crate::app_state::UiFonts;

/// Maximum number of toasts visible at once.
const MAX_TOASTS: usize = 3;

/// Marker component for toast entities.
#[derive(Component)]
pub struct Toast;

/// Timer controlling toast fade-out.
#[derive(Component)]
pub struct ToastTimer {
    timer: Timer,
}

/// Internal helper: spawn a toast with the given duration in seconds.
fn spawn_toast_inner(commands: &mut Commands, fonts: &UiFonts, text: &str, duration: f32) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(60.0),
                left: Val::Percent(50.0),
                // Center horizontally by translating back 50%.
                margin: UiRect::left(Val::Px(-130.0)),
                width: Val::Px(260.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.08, 0.06, 0.85)),
            GlobalZIndex(110),
            Toast,
            ToastTimer {
                timer: Timer::from_seconds(duration, TimerMode::Once),
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font: fonts.bold.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgba(0.93, 0.84, 0.55, 1.0)),
            ));
        });
}

/// Spawn a toast notification with the given text. Auto-despawns after fading.
pub fn spawn_toast(commands: &mut Commands, fonts: &UiFonts, text: &str) {
    spawn_toast_inner(commands, fonts, text, 1.5);
}

/// Spawn a toast with extended display duration (3 seconds).
/// Used for hints that need more reading time.
pub fn spawn_toast_long(commands: &mut Commands, fonts: &UiFonts, text: &str) {
    spawn_toast_inner(commands, fonts, text, 3.0);
}

/// Tick toast timers, reposition for stacking, fade out (background + text),
/// cap visible count, and despawn finished toasts.
pub fn update_toasts(
    mut commands: Commands,
    time: Res<Time>,
    mut toasts: Query<
        (
            Entity,
            &mut ToastTimer,
            &mut BackgroundColor,
            &mut Node,
            &Children,
        ),
        With<Toast>,
    >,
    mut text_colors: Query<&mut TextColor>,
) {
    // Collect and sort by remaining time (oldest first = most faded first).
    let mut items: Vec<_> = toasts.iter_mut().collect();
    // Sort by timer fraction descending so the newest (lowest fraction) is on top.
    items.sort_by(|a, b| {
        a.1.timer
            .fraction()
            .partial_cmp(&b.1.timer.fraction())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Enforce max-count: despawn oldest toasts beyond the limit.
    while items.len() > MAX_TOASTS {
        if let Some((entity, _, _, _, _)) = items.pop() {
            commands.entity(entity).despawn();
        }
    }

    for (slot, (entity, ref mut tt, ref mut bg, ref mut node, children)) in
        items.iter_mut().enumerate()
    {
        tt.timer.tick(time.delta());
        let frac = tt.timer.fraction();
        // Reposition: stack vertically with 50px spacing.
        node.top = Val::Px(60.0 + slot as f32 * 50.0);
        // Fade out using cubic easing.
        let alpha = 0.85 * (1.0 - frac.powi(3));
        // Fade background.
        **bg = BackgroundColor(Color::srgba(0.10, 0.08, 0.06, alpha));
        // Fade child text colour to match.
        let text_alpha = 1.0 - frac.powi(3);
        for child in children.iter() {
            if let Ok(mut tc) = text_colors.get_mut(child) {
                *tc = TextColor(Color::srgba(0.93, 0.84, 0.55, text_alpha));
            }
        }
        if tt.timer.is_finished() {
            commands.entity(*entity).despawn();
        }
    }
}

/// Despawn all toasts (called on state exit).
pub fn teardown_toasts(mut commands: Commands, q: Query<Entity, With<Toast>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
