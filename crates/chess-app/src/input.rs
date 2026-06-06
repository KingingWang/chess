//! Mouse input: click a piece to select it, click a destination to move.
//! Right-click deselects. Moves are validated by the rules engine; in
//! networked games a legal local move is also forwarded to the peer.

use bevy::prelude::*;
use chess_core::Move;

use crate::animation::{spawn_invalid_flash, AnimationPlaying};
use crate::app_state::{world_to_square, BoardOrientation, CoreGame, Selection};
use crate::board_view::RenderDirty;
use crate::drag::DragState;
use crate::history_view::HistoryView;
use crate::net_bridge::{NetCommand, NetLink};
use crate::sound::{MoveSound, PendingSound};

#[allow(clippy::too_many_arguments)]
pub fn handle_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut core: ResMut<CoreGame>,
    mut selection: ResMut<Selection>,
    mut dirty: ResMut<RenderDirty>,
    orient: Res<BoardOrientation>,
    net: Option<Res<NetLink>>,
    animation: Res<AnimationPlaying>,
    mut pending_sound: ResMut<PendingSound>,
    mut history_view: ResMut<HistoryView>,
    drag: Res<DragState>,
) {
    // Block clicks during an active drag.
    if drag.is_dragging() {
        return;
    }

    // Right-click: clear selection, or return to live view from history.
    if buttons.just_pressed(MouseButton::Right) {
        if history_view.is_viewing() {
            history_view.return_to_live();
            dirty.0 = true;
        } else if selection.from.is_some() {
            selection.from = None;
            dirty.0 = true;
        }
        return;
    }

    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    // Block input during animation.
    if animation.0 {
        return;
    }
    // Block input during history review mode.
    if history_view.is_viewing() {
        return;
    }
    // Block board interaction until a networked game is actually connected,
    // and while the peer is offline (host waiting for a reconnect).
    if core.awaiting_peer || core.peer_disconnected {
        return;
    }
    if core.game.is_over() || !core.local_to_move() {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };
    let Some(clicked) = world_to_square(world, *orient) else {
        return;
    };

    let side = core.game.side_to_move();
    match selection.from {
        None => {
            // Select only own pieces.
            if let Some(p) = core.game.board().piece_at(clicked) {
                if p.color == side {
                    selection.from = Some(clicked);
                    dirty.0 = true;
                    pending_sound.sound = Some(MoveSound::PieceSelect);
                    pending_sound.piece = Some(p.kind);
                }
            }
        }
        Some(from) => {
            if clicked == from {
                selection.from = None; // deselect
                dirty.0 = true;
                return;
            }
            // Reselect if clicking another own piece.
            if let Some(p) = core.game.board().piece_at(clicked) {
                if p.color == side {
                    selection.from = Some(clicked);
                    dirty.0 = true;
                    pending_sound.sound = Some(MoveSound::PieceSelect);
                    pending_sound.piece = Some(p.kind);
                    return;
                }
            }
            // Detect capture BEFORE making the move.
            let is_capture = core.game.board().piece_at(clicked).is_some();

            let mv = Move::new(from, clicked);
            if core.game.make_move(mv).is_ok() {
                // Detect check AFTER the move.
                let is_check = core.game.board().is_in_check(core.game.side_to_move());

                selection.from = None;
                core.last_move = Some((mv.from, mv.to));
                crate::moves::MOVE_APPLIED_THIS_FRAME
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                dirty.0 = true;

                // Auto-return to live view if in history review.
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

                // Forward to peer in any networked mode.
                if core.mode.is_networked() {
                    if let Some(net) = net {
                        let _ = net.out.send(NetCommand::Move(mv));
                    }
                }
            } else {
                // Invalid move: visual + audio feedback.
                spawn_invalid_flash(&mut commands, clicked, *orient);
                pending_sound.sound = Some(MoveSound::Invalid);
            }
        }
    }
}
