//! Rendering of the board and pieces in 2D world space.
//!
//! Everything is drawn from primitives (colored quads for the grid, mesh discs
//! plus CJK text glyphs for pieces) so the build depends on **no external art
//! assets** — original artwork (textures/SVG) can replace these primitives via
//! the documented asset pipeline. Pieces are re-rendered whenever
//! [`RenderDirty`] is set by any move source.

use bevy::prelude::*;
use chess_core::Color as ChessColor;

use crate::app_state::{
    square_to_world, BoardOrientation, CoreGame, Selection, UiFonts, CELL, PIECE_RADIUS,
};

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

// --- 国风 (national-style) wood palette -----------------------------------
const FRAME_DARK: Color = Color::srgb(0.28, 0.16, 0.09); // outer lacquer frame
const FRAME_EDGE: Color = Color::srgb(0.45, 0.29, 0.16); // inner gold-brown rim
const BOARD_BG: Color = Color::srgb(0.90, 0.79, 0.57); // warm aged-paper wood
const LINE_COLOR: Color = Color::srgb(0.30, 0.19, 0.10); // grid ink
const RIVER_COLOR: Color = Color::srgba(0.30, 0.19, 0.10, 0.55);

// Piece faces.
const DISC_CREAM: Color = Color::srgb(0.97, 0.93, 0.83);
const RED_INK: Color = Color::srgb(0.72, 0.11, 0.11);
const BLACK_INK: Color = Color::srgb(0.12, 0.12, 0.14);

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

/// A small "star point" cross mark at a board intersection.
fn spawn_star(commands: &mut Commands, sq: chess_core::Square) {
    // Star positions are symmetric across both sides of the river, so the
    // orientation does not affect them; we always use Red here.
    let p = square_to_world(sq, BoardOrientation::Red);
    let bar = 10.0;
    let thin = 2.0;
    for (w, h) in [(bar, thin), (thin, bar)] {
        commands.spawn((
            Sprite {
                color: LINE_COLOR,
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            Transform::from_xyz(p.x, p.y, 1.2),
            BoardLine,
        ));
    }
}

/// Spawn the board background, lacquer frame, grid, river text, and star marks.
pub fn setup_board(mut commands: Commands, fonts: Res<UiFonts>) {
    let w = 8.0 * CELL;
    let h = 9.0 * CELL;

    // Outer lacquer frame (largest, darkest).
    commands.spawn((
        Sprite {
            color: FRAME_DARK,
            custom_size: Some(Vec2::new(w + CELL * 2.6, h + CELL * 2.6)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -0.2),
        BoardLine,
    ));
    // Gold-brown rim.
    commands.spawn((
        Sprite {
            color: FRAME_EDGE,
            custom_size: Some(Vec2::new(w + CELL * 1.5, h + CELL * 1.5)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -0.1),
        BoardLine,
    ));
    // Wood play field.
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
            let bottom_center = ((0.0 + 4.0) / 2.0 - 4.5) * CELL;
            spawn_line(
                &mut commands,
                Vec2::new(x, bottom_center),
                Vec2::new(thick, 4.0 * CELL),
            );
            let top_center = ((5.0 + 9.0) / 2.0 - 4.5) * CELL;
            spawn_line(
                &mut commands,
                Vec2::new(x, top_center),
                Vec2::new(thick, 4.0 * CELL),
            );
        }
    }

    // Palace diagonals (both palaces).
    for &(r0, r1) in &[(0u8, 2u8), (7u8, 9u8)] {
        for &(f_a, f_b) in &[(3u8, 5u8), (5u8, 3u8)] {
            let a = square_to_world(
                chess_core::Square::new(f_a, r0).unwrap(),
                BoardOrientation::Red,
            );
            let b = square_to_world(
                chess_core::Square::new(f_b, r1).unwrap(),
                BoardOrientation::Red,
            );
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

    // Star points: cannons and pawn outposts.
    for &(f, r) in &[
        (1u8, 2u8),
        (7, 2),
        (1, 7),
        (7, 7),
        (0, 3),
        (2, 3),
        (4, 3),
        (6, 3),
        (8, 3),
        (0, 6),
        (2, 6),
        (4, 6),
        (6, 6),
        (8, 6),
    ] {
        if let Some(sq) = chess_core::Square::new(f, r) {
            spawn_star(&mut commands, sq);
        }
    }

    // River calligraphy: 楚河 (left) · 漢界 (right).
    let river_y = -0.5 * CELL + 4.0 * CELL - 4.0 * CELL; // == middle row (y=0)
    let _ = river_y;
    for (text, x) in [("楚  河", -2.0 * CELL), ("漢  界", 2.0 * CELL)] {
        commands.spawn((
            Text2d::new(text),
            TextFont {
                font: fonts.bold.clone(),
                font_size: 40.0,
                ..default()
            },
            TextColor(RIVER_COLOR),
            Transform::from_xyz(x, 0.0, 0.5),
            BoardLine,
        ));
    }
}

/// Redraw all pieces + highlights when [`RenderDirty`] is set.
#[allow(clippy::too_many_arguments)]
pub fn redraw_pieces(
    mut dirty: ResMut<RenderDirty>,
    mut commands: Commands,
    core: Res<CoreGame>,
    selection: Res<Selection>,
    fonts: Res<UiFonts>,
    orient: Res<BoardOrientation>,
    existing: Query<Entity, Or<(With<PieceMarker>, With<HighlightMarker>)>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let orient = *orient;
    if !dirty.0 {
        return;
    }
    dirty.0 = false;

    for e in &existing {
        commands.entity(e).despawn();
    }

    // Last-move highlight: jade-green rings around the from/to squares.
    if let Some((lm_from, lm_to)) = core.last_move {
        let ring_inner = PIECE_RADIUS + 1.0;
        let ring_outer = PIECE_RADIUS + 5.0;
        let ring_mesh = meshes.add(Annulus::new(ring_inner, ring_outer));
        let ring_mat = materials.add(Color::srgba(0.18, 0.72, 0.38, 0.90));
        for sq in [lm_from, lm_to] {
            let pos = square_to_world(sq, orient);
            commands.spawn((
                Mesh2d(ring_mesh.clone()),
                MeshMaterial2d(ring_mat.clone()),
                Transform::from_xyz(pos.x, pos.y, 11.0),
                HighlightMarker,
            ));
        }
    }

    // Selection highlight + legal destination dots.
    if let Some(from) = selection.from {
        let pos = square_to_world(from, orient);
        commands.spawn((
            Sprite {
                color: Color::srgba(0.95, 0.78, 0.30, 0.35),
                custom_size: Some(Vec2::splat(CELL * 0.92)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 5.0),
            HighlightMarker,
        ));
        for mv in core
            .game
            .legal_moves()
            .into_iter()
            .filter(|m| m.from == from)
        {
            let p = square_to_world(mv.to, orient);
            commands.spawn((
                Mesh2d(meshes.add(Circle::new(7.0))),
                MeshMaterial2d(materials.add(Color::srgba(0.15, 0.55, 0.25, 0.85))),
                Transform::from_xyz(p.x, p.y, 6.0),
                HighlightMarker,
            ));
        }
    }

    let disc = meshes.add(Circle::new(PIECE_RADIUS));
    let cream_mat = materials.add(DISC_CREAM);
    let shadow_mat = materials.add(Color::srgba(0.0, 0.0, 0.0, 0.28));

    for (sq, piece) in core.game.board().pieces() {
        let pos = square_to_world(sq, orient);
        let ink = match piece.color {
            ChessColor::Red => RED_INK,
            ChessColor::Black => BLACK_INK,
        };
        let ink_mat = materials.add(ink);

        // Soft drop shadow.
        commands.spawn((
            Mesh2d(disc.clone()),
            MeshMaterial2d(shadow_mat.clone()),
            Transform::from_xyz(pos.x + 2.0, pos.y - 3.0, 9.5),
            PieceMarker,
        ));

        commands
            .spawn((
                // Cream wooden disc face.
                Mesh2d(disc.clone()),
                MeshMaterial2d(cream_mat.clone()),
                Transform::from_xyz(pos.x, pos.y, 10.0),
                PieceMarker,
            ))
            .with_children(|parent| {
                // Colored outer ring.
                parent.spawn((
                    Mesh2d(meshes.add(Circle::new(PIECE_RADIUS - 1.5))),
                    MeshMaterial2d(ink_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                ));
                // Inner cream face inside the ring.
                parent.spawn((
                    Mesh2d(meshes.add(Circle::new(PIECE_RADIUS - 4.5))),
                    MeshMaterial2d(cream_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.2),
                ));
                // Engraved glyph.
                parent.spawn((
                    Text2d::new(piece.glyph().to_string()),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 34.0,
                        ..default()
                    },
                    TextColor(ink),
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
