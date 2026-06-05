//! Drag-and-drop piece movement.
//!
//! Implements a two-phase drag with a distance threshold so that short
//! clicks still work as click-to-select (handled by
//! [`handle_click`](crate::input::handle_click)). Once the cursor moves
//! beyond [`DRAG_THRESHOLD`] pixels while the left button is held, the
//! piece visually follows the cursor and is dropped on release. The drop
//! either applies the move (if legal) or snaps the piece back to its
//! origin square.

use bevy::prelude::*;
use chess_core::{Move, Piece, Square};

use crate::animation::{spawn_invalid_flash, AnimationPlaying};
use crate::app_state::{
    square_to_world, world_to_square, BoardOrientation, CoreGame, Selection, CELL,
};
use crate::board_view::{PieceMarker, PieceSquare, RenderDirty};
use crate::history_view::HistoryView;
use crate::net_bridge::{NetCommand, NetLink};
use crate::sound::{MoveSound, PendingSound};

/// Minimum cursor displacement (in world-space pixels) before a press is
/// promoted from *pending* to an active drag.
const DRAG_THRESHOLD: f32 = CELL * 0.3;

/// Attached to the entity being dragged. The
/// [`redraw_pieces`](crate::board_view::redraw_pieces) query excludes
/// entities with this component so the diff algorithm does not despawn or
/// teleport the piece mid-drag.
#[derive(Component)]
pub struct Dragging;

/// Staging state: left button pressed on own piece, threshold not yet met.
struct DragPending {
    entity: Entity,
    from_sq: Square,
    piece: Piece,
    start_world: Vec2,
}

/// Active drag: threshold exceeded, piece follows cursor.
struct DragActive {
    entity: Entity,
    from_sq: Square,
    piece: Piece,
    offset: Vec2,
}

/// Global drag state resource.
#[derive(Resource, Default)]
pub struct DragState {
    pending: Option<DragPending>,
    active: Option<DragActive>,
}

impl DragState {
    /// An active drag is in progress — [`handle_click`] should early-return.
    #[inline]
    pub fn is_dragging(&self) -> bool {
        self.active.is_some()
    }
}

/// Resolve the cursor to world coordinates.
fn cursor_world(
    windows: &Query<&Window>,
    cameras: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, cam_tf) = cameras.single().ok()?;
    camera.viewport_to_world_2d(cam_tf, cursor).ok()
}

/// Core drag system — runs before `handle_click` in the chain.
#[allow(clippy::too_many_arguments)]
pub fn handle_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
    mut core: ResMut<CoreGame>,
    mut selection: ResMut<Selection>,
    mut dirty: ResMut<RenderDirty>,
    orient: Res<BoardOrientation>,
    net: Option<Res<NetLink>>,
    animation: Res<AnimationPlaying>,
    mut pending_sound: ResMut<PendingSound>,
    mut history_view: ResMut<HistoryView>,
    mut drag: ResMut<DragState>,
    mut piece_q: Query<(Entity, &mut PieceSquare, &mut Transform), With<PieceMarker>>,
) {
    let orient_val = *orient;

    // ── Phase 1: initiate pending on just_pressed ──────────────────
    if buttons.just_pressed(MouseButton::Left) {
        let can_interact = !animation.0
            && !history_view.is_viewing()
            && !core.awaiting_peer
            && !core.peer_disconnected
            && !core.game.is_over()
            && core.local_to_move();

        if can_interact {
            if let Some(world) = cursor_world(&windows, &cameras) {
                if let Some(sq) = world_to_square(world, orient_val) {
                    let side = core.game.side_to_move();
                    if let Some(p) = core.game.board().piece_at(sq) {
                        if p.color == side {
                            // Find the entity for this piece at this square.
                            let found = piece_q
                                .iter()
                                .find(|(_, ps, _)| ps.sq == sq && ps.piece == p);
                            if let Some((entity, _, _)) = found {
                                drag.pending = Some(DragPending {
                                    entity,
                                    from_sq: sq,
                                    piece: p,
                                    start_world: world,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Phase 2: promote pending → active if threshold exceeded ────
    if buttons.pressed(MouseButton::Left) && drag.active.is_none() {
        if let Some(ref pending) = drag.pending {
            if let Some(world) = cursor_world(&windows, &cameras) {
                if world.distance(pending.start_world) > DRAG_THRESHOLD {
                    let entity = pending.entity;
                    let from_sq = pending.from_sq;
                    let piece = pending.piece;

                    // Compute offset so the piece doesn't snap to cursor center.
                    let piece_center = square_to_world(from_sq, orient_val);
                    let offset = piece_center - world;

                    // Raise z and scale up for a "lifted" feel.
                    if let Ok((_, _, mut tf)) = piece_q.get_mut(entity) {
                        tf.translation.z = 20.0;
                        tf.scale = Vec3::splat(1.15);
                    }
                    commands.entity(entity).insert(Dragging);

                    // Set selection so redraw_pieces draws legal destination dots.
                    selection.from = Some(from_sq);
                    dirty.0 = true;

                    drag.active = Some(DragActive {
                        entity,
                        from_sq,
                        piece,
                        offset,
                    });
                    drag.pending = None;
                }
            }
        }
    }

    // ── Phase 3: follow cursor while dragging ──────────────────────
    if let Some(ref active) = drag.active {
        if buttons.pressed(MouseButton::Left) {
            if let Some(world) = cursor_world(&windows, &cameras) {
                let target = world + active.offset;
                if let Ok((_, _, mut tf)) = piece_q.get_mut(active.entity) {
                    tf.translation.x = target.x;
                    tf.translation.y = target.y;
                }
            }
        }
    }

    // ── Phase 4: release handling ──────────────────────────────────
    if buttons.just_released(MouseButton::Left) {
        // Clear pending (sub-threshold release → click already handled).
        drag.pending = None;

        // Resolve active drag drop.
        if let Some(active) = drag.active.take() {
            let world = cursor_world(&windows, &cameras);
            let drop_sq = world.and_then(|w| world_to_square(w, orient_val));

            let mut applied = false;

            if let Some(to_sq) = drop_sq {
                if to_sq != active.from_sq {
                    // Detect capture BEFORE making the move.
                    let is_capture = core.game.board().piece_at(to_sq).is_some();

                    let mv = Move::new(active.from_sq, to_sq);
                    if core.game.make_move(mv).is_ok() {
                        // Detect check AFTER the move.
                        let is_check = core.game.board().is_in_check(core.game.side_to_move());

                        // Update PieceSquare BEFORE removing Dragging so that
                        // redraw_pieces Pass 1 sees an exact match at to_sq
                        // and skips animation (no snap-back).
                        if let Ok((_, mut ps, mut tf)) = piece_q.get_mut(active.entity) {
                            ps.sq = to_sq;
                            let dest = square_to_world(to_sq, orient_val);
                            tf.translation.x = dest.x;
                            tf.translation.y = dest.y;
                            tf.translation.z = 10.0;
                            tf.scale = Vec3::ONE;
                        }

                        // Despawn any captured enemy piece entity at to_sq.
                        for (e, ps, _) in piece_q.iter() {
                            if e != active.entity
                                && ps.sq == to_sq
                                && ps.piece.color != active.piece.color
                            {
                                commands.entity(e).despawn();
                                break;
                            }
                        }

                        selection.from = None;
                        core.last_move = Some((mv.from, mv.to));
                        crate::moves::MOVE_APPLIED_THIS_FRAME
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        dirty.0 = true;

                        history_view.return_to_live();

                        // Queue sound effect with piece kind for pitch variation.
                        let moved_piece = core.game.board().piece_at(mv.to).map(|p| p.kind);
                        pending_sound.sound = Some(if is_check {
                            MoveSound::Check
                        } else if is_capture {
                            MoveSound::Capture
                        } else {
                            MoveSound::Normal
                        });
                        pending_sound.piece = moved_piece;

                        // Forward to peer in networked games.
                        if core.mode.is_networked() {
                            if let Some(ref net) = net {
                                let _ = net.out.send(NetCommand::Move(mv));
                            }
                        }

                        applied = true;
                    }
                }
            }

            if !applied {
                // Invalid drop: flash + buzz feedback.
                if let Some(sq) = drop_sq {
                    if sq != active.from_sq {
                        spawn_invalid_flash(&mut commands, sq, orient_val);
                        pending_sound.sound = Some(MoveSound::Invalid);
                    }
                }
                // Snap piece back to origin.
                if let Ok((_, _, mut tf)) = piece_q.get_mut(active.entity) {
                    let origin = square_to_world(active.from_sq, orient_val);
                    tf.translation.x = origin.x;
                    tf.translation.y = origin.y;
                    tf.translation.z = 10.0;
                    tf.scale = Vec3::ONE;
                }
                selection.from = None;
                dirty.0 = true;
            }

            // Remove Dragging marker.
            commands.entity(active.entity).remove::<Dragging>();
        }
    }
}

/// Clean up drag state when leaving the game.
pub fn teardown_drag(mut drag: ResMut<DragState>) {
    *drag = DragState::default();
}
