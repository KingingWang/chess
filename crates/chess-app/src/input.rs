//! Mouse input: click a piece to select it, click a destination to move.
//! Moves are validated by the rules engine; in networked games a legal local
//! move is also forwarded to the peer.

use bevy::prelude::*;
use chess_core::Move;

use crate::app_state::{world_to_square, BoardOrientation, CoreGame, GameMode, Selection};
use crate::board_view::RenderDirty;
use crate::net_bridge::{NetCommand, NetLink};

#[allow(clippy::too_many_arguments)]
pub fn handle_click(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut core: ResMut<CoreGame>,
    mut selection: ResMut<Selection>,
    mut dirty: ResMut<RenderDirty>,
    orient: Res<BoardOrientation>,
    net: Option<Res<NetLink>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    // Block board interaction until a networked game is actually connected.
    if core.awaiting_peer {
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
                    return;
                }
            }
            let mv = Move::new(from, clicked);
            if core.game.make_move(mv).is_ok() {
                selection.from = None;
                dirty.0 = true;
                // Forward to peer in networked modes.
                if matches!(core.mode, GameMode::LanHost | GameMode::LanJoin) {
                    if let Some(net) = net {
                        let _ = net.out.send(NetCommand::Move(mv));
                    }
                }
            } else {
                // Illegal target: keep selection so the user can retry.
            }
        }
    }
}
