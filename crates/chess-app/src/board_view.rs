//! Rendering of the board and pieces in 2D world space.
//!
//! Everything is drawn from primitives (colored quads for the grid, mesh discs
//! plus text labels for pieces) so the build depends on **no external art
//! assets** — original artwork (textures/SVG) can replace these primitives via
//! the documented asset pipeline. Pieces are re-rendered whenever
//! [`RenderDirty`] is set by any move source.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, PieceKind};

use crate::app_state::{square_to_world, CoreGame, Selection, CELL, PIECE_RADIUS};

/// Set to `true` by any system that mutates the game; the render system redraws
/// pieces and clears the flag.
#[derive(Resource, Default)]
pub struct RenderDirty(pub bool);

#[derive(Component)]
pub struct BoardLine;

#[derive(Component)]
pub struct PieceMarker;

#[derive(Component)]
pub struct HighlightMarker;

const LINE_COLOR: Color = Color::srgb(0.25, 0.18, 0.10);
const BOARD_BG: Color = Color::srgb(0.93, 0.84, 0.66);

fn spawn_line(commands: &mut Commands, center: Vec2, size: Vec2) {
    commands.spawn((
        Sprite {
            color: LINE_COLOR,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, 1.0),
        BoardLine,
    ));
}

/// Spawn the camera, board background, and static grid lines.
pub fn setup_board(mut commands: Commands) {
    let w = 8.0 * CELL;
    let h = 9.0 * CELL;

    // Background panel.
    commands.spawn((
        Sprite {
            color: BOARD_BG,
            custom_size: Some(Vec2::new(w + CELL, h + CELL)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        BoardLine,
    ));

    let thick = 2.0;
    // 10 horizontal lines.
    for r in 0..10 {
        let y = (r as f32 - 4.5) * CELL;
        spawn_line(&mut commands, Vec2::new(0.0, y), Vec2::new(w, thick));
    }
    // Vertical lines: outer files full height; inner files split at the river.
    for f in 0..9 {
        let x = (f as f32 - 4.0) * CELL;
        if f == 0 || f == 8 {
            spawn_line(&mut commands, Vec2::new(x, 0.0), Vec2::new(thick, h));
        } else {
            // bottom half: ranks 0..4
            let bottom_center = ((0.0 + 4.0) / 2.0 - 4.5) * CELL;
            spawn_line(
                &mut commands,
                Vec2::new(x, bottom_center),
                Vec2::new(thick, 4.0 * CELL),
            );
            // top half: ranks 5..9
            let top_center = ((5.0 + 9.0) / 2.0 - 4.5) * CELL;
            spawn_line(
                &mut commands,
                Vec2::new(x, top_center),
                Vec2::new(thick, 4.0 * CELL),
            );
        }
    }

    // Palace diagonals (both palaces) as thin rotated quads.
    for &(r0, r1) in &[(0u8, 2u8), (7u8, 9u8)] {
        for &(f_a, f_b) in &[(3u8, 5u8), (5u8, 3u8)] {
            let a = square_to_world(chess_core::Square::new(f_a, r0).unwrap());
            let b = square_to_world(chess_core::Square::new(f_b, r1).unwrap());
            let mid = (a + b) * 0.5;
            let delta = b - a;
            let len = delta.length();
            let angle = delta.y.atan2(delta.x);
            commands.spawn((
                Sprite {
                    color: LINE_COLOR,
                    custom_size: Some(Vec2::new(len, thick)),
                    ..default()
                },
                Transform {
                    translation: Vec3::new(mid.x, mid.y, 1.0),
                    rotation: Quat::from_rotation_z(angle),
                    ..default()
                },
                BoardLine,
            ));
        }
    }
}

fn piece_label(kind: PieceKind) -> &'static str {
    // Latin initials so the bundled default (Latin) font always renders them.
    // A CJK font can be dropped in to show 帅/将 etc. (see assets/README).
    match kind {
        PieceKind::King => "K",
        PieceKind::Advisor => "A",
        PieceKind::Elephant => "E",
        PieceKind::Horse => "H",
        PieceKind::Chariot => "R",
        PieceKind::Cannon => "C",
        PieceKind::Pawn => "P",
    }
}

/// Redraw all pieces + highlights when [`RenderDirty`] is set.
pub fn redraw_pieces(
    mut dirty: ResMut<RenderDirty>,
    mut commands: Commands,
    core: Res<CoreGame>,
    selection: Res<Selection>,
    existing: Query<Entity, Or<(With<PieceMarker>, With<HighlightMarker>)>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !dirty.0 {
        return;
    }
    dirty.0 = false;

    for e in &existing {
        commands.entity(e).despawn();
    }

    // Selection highlight.
    if let Some(from) = selection.from {
        let pos = square_to_world(from);
        commands.spawn((
            Sprite {
                color: Color::srgba(0.1, 0.6, 0.9, 0.45),
                custom_size: Some(Vec2::splat(CELL * 0.9)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 5.0),
            HighlightMarker,
        ));
        // Legal destinations from this square.
        for mv in core.game.legal_moves().into_iter().filter(|m| m.from == from) {
            let p = square_to_world(mv.to);
            commands.spawn((
                Mesh2d(meshes.add(Circle::new(8.0))),
                MeshMaterial2d(materials.add(Color::srgba(0.1, 0.6, 0.2, 0.7))),
                Transform::from_xyz(p.x, p.y, 6.0),
                HighlightMarker,
            ));
        }
    }

    let disc = meshes.add(Circle::new(PIECE_RADIUS));
    for (sq, piece) in core.game.board().pieces() {
        let pos = square_to_world(sq);
        let (bg, fg) = match piece.color {
            ChessColor::Red => (Color::srgb(0.96, 0.93, 0.85), Color::srgb(0.78, 0.12, 0.12)),
            ChessColor::Black => (Color::srgb(0.96, 0.93, 0.85), Color::srgb(0.10, 0.10, 0.12)),
        };
        commands
            .spawn((
                Mesh2d(disc.clone()),
                MeshMaterial2d(materials.add(bg)),
                Transform::from_xyz(pos.x, pos.y, 10.0),
                PieceMarker,
            ))
            .with_children(|parent| {
                // ring
                parent.spawn((
                    Mesh2d(meshes.add(Circle::new(PIECE_RADIUS - 3.0))),
                    MeshMaterial2d(materials.add(fg)),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                ));
                parent.spawn((
                    Mesh2d(disc.clone()),
                    MeshMaterial2d(materials.add(bg)),
                    Transform::from_scale(Vec3::splat(0.78)).with_translation(Vec3::new(0.0, 0.0, 0.2)),
                ));
                parent.spawn((
                    Text2d::new(piece_label(piece.kind)),
                    TextFont {
                        font_size: 26.0,
                        ..default()
                    },
                    TextColor(fg),
                    Transform::from_xyz(0.0, 0.0, 0.3),
                ));
            });
    }
}

/// Mark dirty once when entering the game so the first frame draws pieces.
pub fn mark_dirty_on_enter(mut dirty: ResMut<RenderDirty>) {
    dirty.0 = true;
}

/// Tear down board entities when leaving the game.
pub fn teardown_board(
    mut commands: Commands,
    q: Query<Entity, Or<(With<BoardLine>, With<PieceMarker>, With<HighlightMarker>)>>,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
