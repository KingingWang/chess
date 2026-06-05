//! Mouse hover highlight for own pieces.
//!
//! Shows a subtle amber ring when the cursor is over a piece the player
//! can move. Purely visual — no game state interaction.

use bevy::prelude::*;

use crate::animation::AnimationPlaying;
use crate::app_state::{
    square_to_world, world_to_square, BoardOrientation, CoreGame, PIECE_RADIUS,
};
use crate::drag::DragState;
use crate::history_view::HistoryView;

/// Marker for the hover highlight entity.
#[derive(Component)]
pub struct HoverHighlight;

/// Show a hover ring on own pieces under the cursor.
#[allow(clippy::too_many_arguments)]
pub fn update_hover(
    mut commands: Commands,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    core: Res<CoreGame>,
    orient: Res<BoardOrientation>,
    animation: Res<AnimationPlaying>,
    history_view: Res<HistoryView>,
    drag: Res<DragState>,
    existing: Query<Entity, With<HoverHighlight>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Despawn previous hover.
    for e in &existing {
        commands.entity(e).despawn();
    }

    // Guards: no hover during animation, history view, drag, game over.
    if animation.0 || history_view.is_viewing() || drag.is_dragging() || core.game.is_over() {
        return;
    }
    if !core.local_to_move() {
        return;
    }

    let window = match windows.single() {
        Ok(w) => w,
        Err(_) => return,
    };
    let cursor = match window.cursor_position() {
        Some(c) => c,
        None => return,
    };
    let (camera, cam_tf) = match cameras.single() {
        Ok(c) => c,
        Err(_) => return,
    };
    let world = match camera.viewport_to_world_2d(cam_tf, cursor) {
        Ok(w) => w,
        Err(_) => return,
    };
    let sq = match world_to_square(world, *orient) {
        Some(s) => s,
        None => return,
    };

    let side = core.game.side_to_move();
    if let Some(p) = core.game.board().piece_at(sq) {
        if p.color == side {
            let pos = square_to_world(sq, *orient);
            let ring = meshes.add(Annulus::new(PIECE_RADIUS + 1.0, PIECE_RADIUS + 4.0));
            let mat = materials.add(Color::srgba(0.95, 0.78, 0.30, 0.25));
            commands.spawn((
                Mesh2d(ring),
                MeshMaterial2d(mat),
                Transform::from_xyz(pos.x, pos.y, 4.0),
                HoverHighlight,
            ));
        }
    }
}

/// Tear down hover entities.
pub fn teardown_hover(mut commands: Commands, q: Query<Entity, With<HoverHighlight>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
