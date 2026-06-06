//! Piece slide animation and capture fade-out.
//!
//! When a move is applied, [`redraw_pieces`](crate::board_view::redraw_pieces)
//! attaches [`AnimateSlide`] to the moving piece entity instead of
//! teleporting it. A captured piece receives [`PendingCapture`] so it
//! shrinks out over the same duration. The [`animate_pieces`] system
//! runs immediately after redraw to lerp transforms each frame.

use bevy::prelude::*;
use chess_core::Square;

use crate::app_state::{square_to_world, BoardOrientation, CELL, PIECE_RADIUS};

/// Animation speed presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AnimSpeed {
    Fast,
    #[default]
    Normal,
    Slow,
}

impl AnimSpeed {
    pub fn duration(self) -> f32 {
        match self {
            AnimSpeed::Fast => 0.08,
            AnimSpeed::Normal => 0.15,
            AnimSpeed::Slow => 0.30,
        }
    }

    pub fn next(self) -> Self {
        match self {
            AnimSpeed::Fast => AnimSpeed::Normal,
            AnimSpeed::Normal => AnimSpeed::Slow,
            AnimSpeed::Slow => AnimSpeed::Fast,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AnimSpeed::Fast => "快",
            AnimSpeed::Normal => "正常",
            AnimSpeed::Slow => "慢",
        }
    }
    /// Emoji icon for this speed.
    pub fn emoji(self) -> &'static str {
        match self {
            AnimSpeed::Fast => "「疾」",
            AnimSpeed::Normal => "「常」",
            AnimSpeed::Slow => "「缓」",
        }
    }
}

/// Global animation speed setting.
#[derive(Resource, Default)]
pub struct AnimSpeedSetting(pub AnimSpeed);

/// Duration of the slide animation in seconds (default; actual varies by AnimSpeed).
#[allow(dead_code)]
pub const ANIM_DURATION: f32 = 0.15;

/// Ease-out-back easing: smooth deceleration with subtle overshoot.
///
/// Input `t` should be in `0.0..=1.0`; returns a value that overshoots
/// slightly past 1.0 before settling, giving a lively springy feel.
#[inline]
pub fn ease_out_back(t: f32) -> f32 {
    let p = t - 1.0;
    1.0 + p * p * (2.7 * p + 1.7)
}

/// Attached to a piece entity that should slide from one world position to
/// another over [`ANIM_DURATION`] seconds.
#[derive(Component)]
pub struct AnimateSlide {
    pub from: Vec2,
    pub to: Vec2,
    pub timer: Timer,
}

impl AnimateSlide {
    #[allow(dead_code)]
    pub fn new(from: Vec2, to: Vec2) -> Self {
        Self {
            from,
            to,
            timer: Timer::from_seconds(ANIM_DURATION, TimerMode::Once),
        }
    }

    pub fn with_duration(from: Vec2, to: Vec2, duration: f32) -> Self {
        Self {
            from,
            to,
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

/// Scale bounce on the capturing piece after arrival.
#[derive(Component)]
pub struct CaptureBounce {
    pub timer: Timer,
}

impl CaptureBounce {
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(0.1, TimerMode::Once),
        }
    }
}

/// Expanding translucent ring at the destination when a move arrives.
#[derive(Component)]
#[allow(dead_code)]
pub struct ArrivalRing {
    pub timer: Timer,
    pub center: Vec2,
}

impl ArrivalRing {
    pub fn new(center: Vec2) -> Self {
        Self {
            timer: Timer::from_seconds(0.25, TimerMode::Once),
            center,
        }
    }
}

/// Attached to a captured piece entity; it shrinks to zero scale and is
/// despawned when the timer finishes. The timer uses the same duration as
/// the slide animation so the capture disappears right as the attacker
/// arrives.
#[derive(Component)]
pub struct PendingCapture {
    pub timer: Timer,
}

impl PendingCapture {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(ANIM_DURATION, TimerMode::Once),
        }
    }

    pub fn with_duration(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

/// `true` while any [`AnimateSlide`] or [`PendingCapture`] component exists.
/// Read by [`handle_click`](crate::input::handle_click) to block board
/// interaction during animation, and by
/// [`redraw_pieces`](crate::board_view::redraw_pieces) to defer dirty
/// redraws so mid-animation `RenderDirty` triggers don't clobber the
/// animation state.
#[derive(Resource, Default)]
pub struct AnimationPlaying(pub bool);

/// Tick slide and capture animations each frame, then update
/// [`AnimationPlaying`].
#[allow(clippy::too_many_arguments)]
pub fn animate_pieces(
    mut commands: Commands,
    time: Res<Time>,
    mut playing: ResMut<AnimationPlaying>,
    mut slides: Query<(Entity, &mut AnimateSlide, &mut Transform)>,
    mut captures: Query<
        (Entity, &mut PendingCapture, &mut Transform),
        (Without<AnimateSlide>, Without<CaptureBounce>),
    >,
    mut bounces: Query<
        (Entity, &mut CaptureBounce, &mut Transform),
        (Without<AnimateSlide>, Without<PendingCapture>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut any_active = false;

    for (entity, mut slide, mut tf) in &mut slides {
        slide.timer.tick(time.delta());
        let t = slide.timer.fraction(); // 0.0 → 1.0
                                        // Ease-out cubic with subtle overshoot for a lively feel.
        let t_smooth = ease_out_back(t);
        let pos = slide.from.lerp(slide.to, t_smooth);
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;

        if slide.timer.is_finished() {
            commands.entity(entity).remove::<AnimateSlide>();
            // Spawn arrival ripple ring at the destination.
            let ring_inner = PIECE_RADIUS * 0.6;
            let ring_outer = PIECE_RADIUS * 0.8;
            commands.spawn((
                Mesh2d(meshes.add(Annulus::new(ring_inner, ring_outer))),
                MeshMaterial2d(materials.add(Color::srgba(0.30, 0.75, 0.45, 0.4))),
                Transform::from_xyz(slide.to.x, slide.to.y, 9.5),
                ArrivalRing::new(slide.to),
            ));
            // If a PendingCapture exists, this was a capture — add bounce.
            if !captures.is_empty() {
                commands.entity(entity).insert(CaptureBounce::new());
            }
        } else {
            any_active = true;
        }
    }

    for (entity, mut capture, mut tf) in &mut captures {
        capture.timer.tick(time.delta());
        let t = capture.timer.fraction();
        // Shrink captured piece toward zero; scale cascades to children.
        let scale = 1.0 - t;
        tf.scale = Vec3::splat(scale);

        if capture.timer.is_finished() {
            commands.entity(entity).despawn();
        } else {
            any_active = true;
        }
    }

    // Capture bounce: brief scale pulse on the capturing piece.
    for (entity, mut bounce, mut tf) in &mut bounces {
        bounce.timer.tick(time.delta());
        let t = bounce.timer.fraction();
        let scale = 1.0 + 0.1 * (t * std::f32::consts::PI).sin();
        tf.scale = Vec3::splat(scale);

        if bounce.timer.is_finished() {
            tf.scale = Vec3::ONE;
            commands.entity(entity).remove::<CaptureBounce>();
        } else {
            any_active = true;
        }
    }

    playing.0 = any_active;
}

/// Red flash overlay for invalid move attempts.
#[derive(Component)]
pub struct InvalidFlash {
    pub timer: Timer,
}

impl InvalidFlash {
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(0.2, TimerMode::Once),
        }
    }
}

/// Spawn a brief red flash at the given square.
pub fn spawn_invalid_flash(commands: &mut Commands, sq: Square, orient: BoardOrientation) {
    let pos = square_to_world(sq, orient);
    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 0.2, 0.1, 0.5),
            custom_size: Some(Vec2::splat(CELL * 0.92)),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, 12.0),
        InvalidFlash::new(),
    ));
}

/// Fade out and despawn invalid flash overlays.
pub fn animate_invalid_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut InvalidFlash, &mut Sprite)>,
) {
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.timer.tick(time.delta());
        let t = flash.timer.fraction();
        // Fade alpha from 0.5 to 0.0
        sprite.color = Color::srgba(1.0, 0.2, 0.1, 0.5 * (1.0 - t));
        if flash.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Expand and fade the arrival ring, then despawn.
pub fn animate_arrival_ring(
    mut commands: Commands,
    time: Res<Time>,
    mut rings: Query<(
        Entity,
        &mut ArrivalRing,
        &mut Transform,
        &mut MeshMaterial2d<ColorMaterial>,
    )>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, mut ring, mut tf, mat_handle) in &mut rings {
        ring.timer.tick(time.delta());
        let t = ring.timer.fraction();
        // Expand from 1.0× to 1.8× scale.
        let scale = 1.0 + 0.8 * t;
        tf.scale = Vec3::splat(scale);
        // Fade alpha from 0.4 to 0.0.
        let alpha = 0.4 * (1.0 - t);
        if let Some(mat) = materials.get_mut(mat_handle.id()) {
            mat.color = Color::srgba(0.30, 0.75, 0.45, alpha);
        }
        if ring.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Despawn any leftover animation entities when leaving the game state.
/// (Pieces are already caught by `PieceMarker` teardown; this is a
/// safety net for orphaned `PendingCapture` entities.)
pub fn teardown_animations(
    mut commands: Commands,
    q: Query<
        Entity,
        Or<(
            With<AnimateSlide>,
            With<PendingCapture>,
            With<InvalidFlash>,
            With<CaptureBounce>,
            With<ArrivalRing>,
        )>,
    >,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// ===== Check and Checkmate Animations =====

/// Screen shake effect when king is in check.
#[derive(Component)]
pub struct CheckShake {
    pub timer: Timer,
    pub intensity: f32,
}

impl CheckShake {
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
            intensity: 3.0,
        }
    }
}

/// Spawn a check shake effect on the camera.
pub fn spawn_check_shake(commands: &mut Commands, camera_entity: Entity) {
    commands.entity(camera_entity).insert(CheckShake::new());
}

/// Animate the check shake effect.
pub fn animate_check_shake(
    mut commands: Commands,
    time: Res<Time>,
    mut shakes: Query<(Entity, &mut CheckShake, &mut Transform)>,
) {
    for (entity, mut shake, mut transform) in &mut shakes {
        shake.timer.tick(time.delta());

        if shake.timer.is_finished() {
            // Reset position
            transform.translation.x = transform.translation.x.round();
            transform.translation.y = transform.translation.y.round();
            commands.entity(entity).remove::<CheckShake>();
        } else {
            // Apply shake
            let progress = shake.timer.fraction();
            let decay = 1.0 - progress;
            let offset_x = (time.elapsed_secs() * 50.0).sin() * shake.intensity * decay;
            let offset_y = (time.elapsed_secs() * 60.0).cos() * shake.intensity * decay;

            transform.translation.x += offset_x;
            transform.translation.y += offset_y;
        }
    }
}

/// Particle effect for captures.
#[derive(Component)]
pub struct CaptureParticle {
    pub timer: Timer,
    pub velocity: Vec2,
    pub start_pos: Vec2,
}

impl CaptureParticle {
    pub fn new(pos: Vec2, angle: f32, seed: f32) -> Self {
        let speed = 100.0 + seed * 50.0;
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Once),
            velocity: Vec2::new(angle.cos() * speed, angle.sin() * speed),
            start_pos: pos,
        }
    }
}

/// Spawn capture particles at the given position.
pub fn spawn_capture_particles(commands: &mut Commands, pos: Vec2, color: Color) {
    let particle_count = 8;
    for i in 0..particle_count {
        let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
        let seed = (i as f32) / (particle_count as f32);
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(8.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 15.0),
            CaptureParticle::new(pos, angle, seed),
        ));
    }
}

/// Animate capture particles.
pub fn animate_capture_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut CaptureParticle, &mut Transform, &mut Sprite)>,
) {
    for (entity, mut particle, mut transform, mut sprite) in &mut particles {
        particle.timer.tick(time.delta());

        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
        } else {
            let progress = particle.timer.fraction();
            let dt = time.delta_secs();

            // Apply gravity and velocity
            particle.velocity.y -= 300.0 * dt;

            let new_x = transform.translation.x + particle.velocity.x * dt;
            let new_y = transform.translation.y + particle.velocity.y * dt;

            transform.translation.x = new_x;
            transform.translation.y = new_y;

            // Fade out and shrink
            let alpha = 1.0 - progress;
            let scale = 1.0 - progress * 0.5;
            if let Color::Srgba(ref mut rgba) = sprite.color {
                rgba.alpha = alpha;
            }
            transform.scale = Vec3::splat(scale);
        }
    }
}

/// Checkmate celebration effect.
#[derive(Component)]
pub struct CheckmateCelebration {
    pub timer: Timer,
}

impl CheckmateCelebration {
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
        }
    }
}

/// Spawn checkmate celebration particles.
pub fn spawn_checkmate_celebration(commands: &mut Commands, board_center: Vec2) {
    let colors = [
        Color::srgb(1.0, 0.8, 0.2), // Gold
        Color::srgb(1.0, 0.4, 0.2), // Orange
        Color::srgb(1.0, 0.2, 0.3), // Red
    ];

    for i in 0..30 {
        let color = colors[i % colors.len()];
        let angle = (i as f32 / 30.0) * std::f32::consts::TAU;
        let distance = 30.0 + (i as f32 * 3.7);
        let pos = board_center + Vec2::new(angle.cos() * distance, angle.sin() * distance);

        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(12.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 20.0),
            CheckmateCelebration::new(),
        ));
    }
}

/// Animate checkmate celebration.
pub fn animate_checkmate_celebration(
    mut commands: Commands,
    time: Res<Time>,
    mut celebrations: Query<(
        Entity,
        &mut CheckmateCelebration,
        &mut Transform,
        &mut Sprite,
    )>,
) {
    for (entity, mut celebration, mut transform, mut sprite) in &mut celebrations {
        celebration.timer.tick(time.delta());

        if celebration.timer.is_finished() {
            commands.entity(entity).despawn();
        } else {
            let progress = celebration.timer.fraction();

            // Float upward and spin
            transform.translation.y += 30.0 * time.delta_secs();
            transform.rotate_z(2.0 * time.delta_secs());

            // Fade out
            let alpha = 1.0 - progress;
            if let Color::Srgba(ref mut rgba) = sprite.color {
                rgba.alpha = alpha;
            }

            // Pulse scale
            let scale = 1.0 + 0.2 * (progress * std::f32::consts::PI * 4.0).sin();
            transform.scale = Vec3::splat(scale);
        }
    }
}

/// Update teardown to include new animation components.
pub fn teardown_new_animations(
    mut commands: Commands,
    q: Query<
        Entity,
        Or<(
            With<CheckShake>,
            With<CaptureParticle>,
            With<CheckmateCelebration>,
        )>,
    >,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// ===== Animation Trigger Systems =====

/// Camera marker for check shake effect.
#[derive(Component)]
pub struct GameCamera;

/// Consume the CHECK_THIS_FRAME flag and spawn screen shake on the camera.
pub fn trigger_check_animation(mut commands: Commands, cameras: Query<Entity, With<GameCamera>>) {
    if crate::moves::CHECK_THIS_FRAME.swap(false, std::sync::atomic::Ordering::Relaxed) {
        if let Ok(camera) = cameras.single() {
            commands.entity(camera).insert(CheckShake::new());
            bevy::log::info!("Check! Triggering screen shake.");
        }
    }
}

/// Consume the CAPTURE_THIS_FRAME flag and spawn particles at the capture square.
pub fn trigger_capture_animation(
    mut commands: Commands,
    core: Res<crate::app_state::CoreGame>,
    orient: Res<crate::app_state::BoardOrientation>,
    theme: Res<crate::board_theme::BoardTheme>,
) {
    if crate::moves::CAPTURE_THIS_FRAME.swap(false, std::sync::atomic::Ordering::Relaxed) {
        if let Some((_, to)) = core.last_move {
            let pos = crate::app_state::square_to_world(to, *orient);
            // Use the theme's disc border color for particles
            let color = theme.palette.disc_border;
            crate::animation::spawn_capture_particles(&mut commands, pos, color);
            bevy::log::info!("Capture! Spawning particles at {:?}.", to);
        }
    }
}

/// Consume the CHECKMATE_THIS_FRAME flag and spawn celebration particles.
pub fn trigger_checkmate_animation(mut commands: Commands) {
    if crate::moves::CHECKMATE_THIS_FRAME.swap(false, std::sync::atomic::Ordering::Relaxed) {
        // Spawn celebration at board center (0,0)
        crate::animation::spawn_checkmate_celebration(&mut commands, Vec2::ZERO);
        bevy::log::info!("Checkmate! Spawning celebration.");
    }
}
