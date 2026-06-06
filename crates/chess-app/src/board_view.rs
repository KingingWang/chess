//! Rendering of the board and pieces in 2D world space.
//!
//! Everything is drawn from primitives (colored quads for the grid, mesh discs
//! plus CJK text glyphs for pieces) so the build depends on **no external art
//! assets** — original artwork (textures/SVG) can replace these primitives via
//! the documented asset pipeline. Pieces are re-rendered whenever
//! [`RenderDirty`] is set by any move source.

use bevy::prelude::*;
use chess_core::Color as ChessColor;

use crate::animation::{AnimSpeedSetting, AnimateSlide, AnimationPlaying, PendingCapture};
use crate::app_state::{
    square_to_world, BoardOrientation, CoreGame, Selection, UiFonts, CELL, PIECE_RADIUS,
};
use crate::board_theme::BoardTheme;
use crate::drag::Dragging;
use crate::history_view::HistoryView;

/// Set to `true` by any system that mutates the game; the render system redraws
/// pieces and clears the flag.
#[derive(Resource, Default)]
pub struct RenderDirty(pub bool);

/// Whether board coordinate labels are visible (toggled with C key).
#[derive(Resource)]
pub struct ShowCoordinates(pub bool);

impl Default for ShowCoordinates {
    fn default() -> Self {
        Self(true)
    }
}

/// Coordinate display style.
#[derive(
    Resource, Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize,
)]
pub enum CoordinateStyle {
    /// Traditional Xiangqi notation (Chinese numerals for Red, Arabic for Black).
    #[default]
    Traditional,
    /// Algebraic notation (a-i for files, 0-9 for ranks).
    Algebraic,
}

impl CoordinateStyle {
    /// Cycle to next style.
    pub fn next(self) -> Self {
        match self {
            CoordinateStyle::Traditional => CoordinateStyle::Algebraic,
            CoordinateStyle::Algebraic => CoordinateStyle::Traditional,
        }
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            CoordinateStyle::Traditional => "传统",
            CoordinateStyle::Algebraic => "代数",
        }
    }
}

#[derive(Component)]
pub struct BoardLine;

#[derive(Component)]
pub struct PieceMarker;

/// Identifies a persistent piece entity by its board square and piece type.
/// Used for diff-based rendering — pieces are only respawned when the board
/// state actually changes for that square.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceSquare {
    pub sq: chess_core::Square,
    pub piece: chess_core::Piece,
}

#[derive(Component)]
pub struct HighlightMarker;

/// Pulsing red overlay on the checked king's square.
#[derive(Component)]
pub struct CheckHighlight;

/// Marker for the selected-piece golden quad (pulsing glow).
#[derive(Component)]
pub struct SelectionHighlight;

/// Resource controlling the initial scale-in animation when pieces first
/// appear on the board. The animation is decorative and does not block input.
#[derive(Resource)]
pub struct BoardScaleIn {
    pub timer: Timer,
    pub active: bool,
}

impl Default for BoardScaleIn {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
            active: false,
        }
    }
}

/// Marker for file-label entities (九八七…一 / 1-9) so they can be toggled.
#[derive(Component)]
pub struct CoordLabel;

fn spawn_line(commands: &mut Commands, center: Vec2, size: Vec2, line_color: Color) {
    commands.spawn((
        Sprite {
            color: line_color,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, 1.0),
        BoardLine,
    ));
}

/// Traditional L-shaped bracket marks at star-point intersections.
///
/// Interior points get 4 L-marks (all quadrants). Edge points on the
/// leftmost/rightmost files get only 2 inward-facing L-marks to avoid
/// protruding beyond the board boundary.
fn spawn_star(
    commands: &mut Commands,
    sq: chess_core::Square,
    line_color: Color,
    is_edge_left: bool,
    is_edge_right: bool,
) {
    let p = square_to_world(sq, BoardOrientation::Red);
    let arm = 10.0; // length of each arm (world units)
    let thin = 2.0; // thickness of each arm

    // Quadrant offsets: (dx_sign, dy_sign) for the 4 L-marks.
    let mut quadrants: Vec<(f32, f32)> = Vec::new();
    if !is_edge_left {
        quadrants.push((-1.0, -1.0));
        quadrants.push((-1.0, 1.0));
    }
    if !is_edge_right {
        quadrants.push((1.0, -1.0));
        quadrants.push((1.0, 1.0));
    }

    let gap = 4.0; // offset from center
    for (dx, dy) in quadrants {
        // Horizontal arm.
        commands.spawn((
            Sprite {
                color: line_color,
                custom_size: Some(Vec2::new(arm, thin)),
                ..default()
            },
            Transform::from_xyz(p.x + dx * (gap + arm * 0.5), p.y + dy * gap, 1.2),
            BoardLine,
        ));
        // Vertical arm.
        commands.spawn((
            Sprite {
                color: line_color,
                custom_size: Some(Vec2::new(thin, arm)),
                ..default()
            },
            Transform::from_xyz(p.x + dx * gap, p.y + dy * (gap + arm * 0.5), 1.2),
            BoardLine,
        ));
    }
}

/// Spawn the board background, lacquer frame, grid, river text, and star marks.
pub fn setup_board(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    theme: Res<BoardTheme>,
    coord_style: Res<CoordinateStyle>,
) {
    let w = 8.0 * CELL;
    let h = 9.0 * CELL;

    // Outer lacquer frame (largest, darkest).
    commands.spawn((
        Sprite {
            color: theme.palette.frame_dark,
            custom_size: Some(Vec2::new(w + CELL * 2.6, h + CELL * 2.6)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -0.2),
        BoardLine,
    ));
    // Gold-brown rim.
    commands.spawn((
        Sprite {
            color: theme.palette.frame_edge,
            custom_size: Some(Vec2::new(w + CELL * 1.5, h + CELL * 1.5)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -0.1),
        BoardLine,
    ));
    // Wood play field.
    commands.spawn((
        Sprite {
            color: theme.palette.board_bg,
            custom_size: Some(Vec2::new(w + CELL, h + CELL)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        BoardLine,
    ));

    // Inner edge shadows — subtle dark overlays for depth.
    let shadow_width = CELL * 0.12;
    let shadow_color = Color::srgba(0.0, 0.0, 0.0, 0.12);
    // Top shadow
    commands.spawn((
        Sprite {
            color: shadow_color,
            custom_size: Some(Vec2::new(w + CELL, shadow_width)),
            ..default()
        },
        Transform::from_xyz(0.0, h * 0.5 + CELL * 0.5 - shadow_width * 0.5, 0.3),
        BoardLine,
    ));
    // Bottom shadow
    commands.spawn((
        Sprite {
            color: shadow_color,
            custom_size: Some(Vec2::new(w + CELL, shadow_width)),
            ..default()
        },
        Transform::from_xyz(0.0, -h * 0.5 - CELL * 0.5 + shadow_width * 0.5, 0.3),
        BoardLine,
    ));
    // Left shadow
    commands.spawn((
        Sprite {
            color: shadow_color,
            custom_size: Some(Vec2::new(shadow_width, h + CELL)),
            ..default()
        },
        Transform::from_xyz(-w * 0.5 - CELL * 0.5 + shadow_width * 0.5, 0.0, 0.3),
        BoardLine,
    ));
    // Right shadow
    commands.spawn((
        Sprite {
            color: shadow_color,
            custom_size: Some(Vec2::new(shadow_width, h + CELL)),
            ..default()
        },
        Transform::from_xyz(w * 0.5 + CELL * 0.5 - shadow_width * 0.5, 0.0, 0.3),
        BoardLine,
    ));

    let thick = 2.0;
    for r in 0..10 {
        let y = (r as f32 - 4.5) * CELL;
        spawn_line(
            &mut commands,
            Vec2::new(0.0, y),
            Vec2::new(w, thick),
            theme.palette.line_color,
        );
    }
    for f in 0..9 {
        let x = (f as f32 - 4.0) * CELL;
        if f == 0 || f == 8 {
            spawn_line(
                &mut commands,
                Vec2::new(x, 0.0),
                Vec2::new(thick, h),
                theme.palette.line_color,
            );
        } else {
            let bottom_center = ((0.0 + 4.0) / 2.0 - 4.5) * CELL;
            spawn_line(
                &mut commands,
                Vec2::new(x, bottom_center),
                Vec2::new(thick, 4.0 * CELL),
                theme.palette.line_color,
            );
            let top_center = ((5.0 + 9.0) / 2.0 - 4.5) * CELL;
            spawn_line(
                &mut commands,
                Vec2::new(x, top_center),
                Vec2::new(thick, 4.0 * CELL),
                theme.palette.line_color,
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
                    color: theme.palette.line_color,
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
            spawn_star(&mut commands, sq, theme.palette.line_color, f == 0, f == 8);
        }
    }

    // Corner ornaments — small L-shaped marks at the board corners.
    let corner_len = CELL * 0.4;
    let corner_thick = 3.0;
    let corner_offset = 0.5 * CELL; // offset from the board edge
    let bx = 4.0 * CELL + corner_offset;
    let by = 4.5 * CELL + corner_offset;
    let corner_color = theme.palette.line_color;

    for &(sx, sy) in &[(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let cx = sx * bx;
        let cy = sy * by;
        // Horizontal bar.
        commands.spawn((
            Sprite {
                color: corner_color,
                custom_size: Some(Vec2::new(corner_len, corner_thick)),
                ..default()
            },
            Transform::from_xyz(cx - sx * corner_len * 0.5, cy, 1.5),
            BoardLine,
        ));
        // Vertical bar.
        commands.spawn((
            Sprite {
                color: corner_color,
                custom_size: Some(Vec2::new(corner_thick, corner_len)),
                ..default()
            },
            Transform::from_xyz(cx, cy - sy * corner_len * 0.5, 1.5),
            BoardLine,
        ));
    }

    // River calligraphy: 楚河 (left) · 漢界 (right).
    let river_y = -0.5 * CELL + 4.0 * CELL - 4.0 * CELL;
    let _ = river_y;
    let river_srgba = theme.palette.river_color.to_srgba();
    let river_text_color = Color::srgba(river_srgba.red, river_srgba.green, river_srgba.blue, 0.4);
    for (text, x) in [("楚　　河", -2.0 * CELL), ("漢　　界", 2.0 * CELL)] {
        commands.spawn((
            Text2d::new(text),
            TextFont {
                font: fonts.bold.clone(),
                font_size: 48.0,
                ..default()
            },
            TextColor(river_text_color),
            Transform::from_xyz(x, 0.0, 0.5),
            BoardLine,
        ));
    }

    // Decorative horizontal separator line between the river texts.
    commands.spawn((
        Sprite {
            color: river_text_color,
            custom_size: Some(Vec2::new(CELL * 3.0, 1.5)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.4),
        BoardLine,
    ));

    // File labels along bottom and top edges.
    let coord_style = *coord_style;
    let label_offset = CELL * 0.6;

    // File labels (top and bottom)
    for f in 0..9u8 {
        let x = (f as f32 - 4.0) * CELL;

        let (bottom_label, top_label) = match coord_style {
            CoordinateStyle::Traditional => {
                let red_files = ['九', '八', '七', '六', '五', '四', '三', '二', '一'];
                let black_files = ['1', '2', '3', '4', '5', '6', '7', '8', '9'];
                (
                    red_files[f as usize].to_string(),
                    black_files[f as usize].to_string(),
                )
            }
            CoordinateStyle::Algebraic => {
                let files = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
                (files[f as usize].to_string(), files[f as usize].to_string())
            }
        };

        commands.spawn((
            Text2d::new(bottom_label),
            TextFont {
                font: fonts.regular.clone(),
                font_size: 16.0,
                ..default()
            },
            TextColor(theme.palette.river_color),
            Transform::from_xyz(x, -4.5 * CELL - label_offset, 0.5),
            BoardLine,
            CoordLabel,
        ));
        commands.spawn((
            Text2d::new(top_label),
            TextFont {
                font: fonts.regular.clone(),
                font_size: 16.0,
                ..default()
            },
            TextColor(theme.palette.river_color),
            Transform::from_xyz(x, 4.5 * CELL + label_offset, 0.5),
            BoardLine,
            CoordLabel,
        ));
    }

    // Rank labels along the left edge
    for r in 0..10u8 {
        let y = (r as f32 - 4.5) * CELL;

        let rank_label = match coord_style {
            CoordinateStyle::Traditional => r.to_string(),
            CoordinateStyle::Algebraic => r.to_string(),
        };

        commands.spawn((
            Text2d::new(rank_label),
            TextFont {
                font: fonts.regular.clone(),
                font_size: 16.0,
                ..default()
            },
            TextColor(theme.palette.river_color),
            Transform::from_xyz(-4.0 * CELL - label_offset, y, 0.5),
            BoardLine,
            CoordLabel,
        ));
    }
}

/// Redraw all pieces + highlights when [`RenderDirty`] is set.
///
/// Defers the redraw while an animation is playing so that mid-animation
/// `RenderDirty` triggers (e.g. theme change) don't clobber in-flight
/// [`AnimateSlide`] / [`PendingCapture`] entities.
#[allow(clippy::too_many_arguments)]
pub fn redraw_pieces(
    mut dirty: ResMut<RenderDirty>,
    mut commands: Commands,
    core: Res<CoreGame>,
    selection: Res<Selection>,
    fonts: Res<UiFonts>,
    orient: Res<BoardOrientation>,
    theme: Res<BoardTheme>,
    anim_playing: Res<AnimationPlaying>,
    anim_speed: Res<AnimSpeedSetting>,
    history_view: Res<HistoryView>,
    highlight_q: Query<Entity, With<HighlightMarker>>,
    mut piece_q: Query<
        (Entity, &mut PieceSquare, &mut Transform),
        (
            With<PieceMarker>,
            Without<PendingCapture>,
            Without<Dragging>,
        ),
    >,
    capture_q: Query<Entity, With<PendingCapture>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    board_scale_in: Res<BoardScaleIn>,
) {
    let orient = *orient;
    if !dirty.0 {
        return;
    }

    // Defer the dirty redraw while animation is in-flight. The flag stays
    // set so the next frame after animation finishes will process it.
    if anim_playing.0 {
        return;
    }

    dirty.0 = false;

    // Always rebuild highlights (cheap, transient).
    for e in &highlight_q {
        commands.entity(e).despawn();
    }

    // Defensive cleanup: despawn stale PendingCapture entities that survived
    // past the animation window (should not happen, but avoids ghost pieces).
    for e in &capture_q {
        commands.entity(e).despawn();
    }

    // --- Diff-based piece rendering with animation awareness ---
    // In history view, render the historical position.
    let viewing = history_view.viewing_ply;
    let display_board;
    let board_ref = if let Some(ply) = viewing {
        display_board = core
            .game
            .board_at_ply(ply)
            .unwrap_or_else(|| core.game.board().clone());
        &display_board
    } else {
        core.game.board()
    };
    let board_pieces: Vec<(chess_core::Square, chess_core::Piece)> = board_ref.pieces().collect();

    let existing_pieces: Vec<(Entity, PieceSquare)> =
        piece_q.iter().map(|(e, ps, _)| (e, *ps)).collect();

    let mut kept: Vec<bool> = vec![false; existing_pieces.len()];
    let mut board_covered: Vec<bool> = vec![false; board_pieces.len()];

    // Pass 1: exact match (same square, same piece type).
    for (i, (_, ps)) in existing_pieces.iter().enumerate() {
        for (j, (sq, piece)) in board_pieces.iter().enumerate() {
            if !board_covered[j] && ps.sq == *sq && ps.piece == *piece {
                kept[i] = true;
                board_covered[j] = true;
                break;
            }
        }
    }

    // Skip animation detection in history view mode.
    // Pass 2: detect a "moved" piece via last_move and animate it.
    // A piece entity at from_sq whose type matches the piece now at to_sq
    // is the mover — slide it rather than despawn+respawn.
    let mut animated_entity: Option<Entity> = None;
    let suppress_animation = viewing.is_some();
    if !suppress_animation {
        if let Some((from_sq, to_sq)) = core.last_move {
            // Find the uncovered board entry at to_sq.
            let to_idx = board_pieces
                .iter()
                .enumerate()
                .find(|(j, (sq, _))| *sq == to_sq && !board_covered[*j]);

            if let Some((j_to, (_, piece_at_to))) = to_idx {
                // Find the existing entity at from_sq with the same piece type.
                let from_entity_idx = existing_pieces
                    .iter()
                    .enumerate()
                    .find(|(i, (_, ps))| !kept[*i] && ps.sq == from_sq && ps.piece == *piece_at_to);

                if let Some((i_from, (entity, _))) = from_entity_idx {
                    // Mark this entity as the animated mover.
                    kept[i_from] = true;
                    board_covered[j_to] = true;
                    animated_entity = Some(*entity);

                    let from_pos = square_to_world(from_sq, orient);
                    let to_pos = square_to_world(to_sq, orient);
                    let dur = anim_speed.0.duration();
                    commands
                        .entity(*entity)
                        .insert(AnimateSlide::with_duration(from_pos, to_pos, dur));

                    // Check for a captured enemy piece entity at to_sq.
                    let captured_idx = existing_pieces.iter().enumerate().find(|(ci, (_, ps))| {
                        !kept[*ci] && ps.sq == to_sq && ps.piece.color != piece_at_to.color
                    });
                    if let Some((ci, (cap_entity, _))) = captured_idx {
                        kept[ci] = true; // shield from despawn — animate_pieces handles it
                        commands
                            .entity(*cap_entity)
                            .insert(PendingCapture::with_duration(anim_speed.0.duration()));
                    }
                }
            }
        }
    }
    // Remove unmatched entities (no longer on the board).
    for (i, (entity, _)) in existing_pieces.iter().enumerate() {
        if !kept[i] {
            commands.entity(*entity).despawn();
        }
    }

    // Update PieceSquare and transform of kept entities.
    for (entity, mut ps, mut transform) in &mut piece_q {
        if Some(entity) == animated_entity {
            // Update PieceSquare to the destination square.
            if let Some((_, to_sq)) = core.last_move {
                ps.sq = to_sq;
            }
            // Place at FROM position — AnimateSlide will lerp to TO.
            if let Some((from_sq, _)) = core.last_move {
                let from_pos = square_to_world(from_sq, orient);
                transform.translation.x = from_pos.x;
                transform.translation.y = from_pos.y;
            }
        } else {
            let pos = square_to_world(ps.sq, orient);
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }

    // Spawn new pieces for uncovered board positions.
    let disc = meshes.add(Circle::new(PIECE_RADIUS));
    let cream_mat = materials.add(theme.palette.disc_face);
    let shadow_mat = materials.add(Color::srgba(0.0, 0.0, 0.0, 0.28));
    let border_disc = meshes.add(Circle::new(PIECE_RADIUS + 0.5));
    let border_mat = materials.add(theme.palette.disc_border);

    for (j, (sq, piece)) in board_pieces.iter().enumerate() {
        if board_covered[j] {
            continue;
        }
        let pos = square_to_world(*sq, orient);
        let ink = match piece.color {
            ChessColor::Red => theme.palette.red_ink,
            ChessColor::Black => theme.palette.black_ink,
        };
        let ink_mat = materials.add(ink);

        commands
            .spawn((
                Mesh2d(disc.clone()),
                MeshMaterial2d(cream_mat.clone()),
                Transform {
                    translation: Vec3::new(pos.x, pos.y, 10.0),
                    scale: if board_scale_in.active {
                        Vec3::splat(0.0)
                    } else {
                        Vec3::ONE
                    },
                    ..default()
                },
                PieceMarker,
                PieceSquare {
                    sq: *sq,
                    piece: *piece,
                },
            ))
            .with_children(|parent| {
                // Subtle dark border ring for better definition on light themes.
                parent.spawn((
                    Mesh2d(border_disc.clone()),
                    MeshMaterial2d(border_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, -0.1),
                ));
                parent.spawn((
                    Mesh2d(disc.clone()),
                    MeshMaterial2d(shadow_mat.clone()),
                    Transform::from_xyz(2.0, -3.0, -0.5),
                ));
                parent.spawn((
                    Mesh2d(meshes.add(Circle::new(PIECE_RADIUS - 1.5))),
                    MeshMaterial2d(ink_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                ));
                parent.spawn((
                    Mesh2d(meshes.add(Circle::new(PIECE_RADIUS - 4.5))),
                    MeshMaterial2d(cream_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.2),
                ));
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

    // Last-move highlight: jade-green rings around the from/to squares.
    // In history view, highlight the move at the viewed ply.
    let highlight_move = if let Some(ply) = viewing {
        if ply > 0 {
            let entry = &core.game.history()[ply - 1];
            let mv = entry.mv();
            Some((mv.from, mv.to))
        } else {
            None
        }
    } else {
        core.last_move
    };
    if let Some((lm_from, lm_to)) = highlight_move {
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

        // Amber highlight squares behind last-move source and destination.
        for sq in [lm_from, lm_to] {
            let pos = square_to_world(sq, orient);
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.90, 0.78, 0.30, 0.18),
                    custom_size: Some(Vec2::splat(CELL * 0.95)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 2.0),
                HighlightMarker,
            ));
        }
    }

    // Selection highlight + legal destination dots (not shown in history view).
    if viewing.is_none() {
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
                SelectionHighlight,
            ));
            for mv in core
                .game
                .legal_moves()
                .into_iter()
                .filter(|m| m.from == from)
            {
                let p = square_to_world(mv.to, orient);
                let is_capture = board_ref.piece_at(mv.to).is_some();
                if is_capture {
                    // Red-tinted ring around enemy piece to indicate capture.
                    let ring_inner = PIECE_RADIUS + 1.0;
                    let ring_outer = PIECE_RADIUS + 5.0;
                    commands.spawn((
                        Mesh2d(meshes.add(Annulus::new(ring_inner, ring_outer))),
                        MeshMaterial2d(materials.add(Color::srgba(0.85, 0.20, 0.12, 0.70))),
                        Transform::from_xyz(p.x, p.y, 6.0),
                        HighlightMarker,
                    ));
                } else {
                    // Small green dot for empty square moves.
                    commands.spawn((
                        Mesh2d(meshes.add(Circle::new(7.0))),
                        MeshMaterial2d(materials.add(Color::srgba(0.15, 0.55, 0.25, 0.85))),
                        Transform::from_xyz(p.x, p.y, 6.0),
                        HighlightMarker,
                    ));
                }
            }
        }
    }

    // Side-to-move indicator: small colored diamond on the board edge.
    if viewing.is_none() && !core.game.is_over() {
        let stm = core.game.side_to_move();
        let (indicator_y, indicator_color) = match stm {
            ChessColor::Red => (-4.0 * CELL, Color::srgba(0.9, 0.2, 0.1, 0.7)),
            ChessColor::Black => (4.0 * CELL, Color::srgba(0.15, 0.15, 0.15, 0.7)),
        };
        commands.spawn((
            Sprite {
                color: indicator_color,
                custom_size: Some(Vec2::splat(10.0)),
                ..default()
            },
            Transform {
                translation: Vec3::new(4.0 * CELL + CELL * 0.8, indicator_y, 3.0),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                ..default()
            },
            HighlightMarker,
        ));
    }

    // Check warning: red pulse on the checked king's square.
    if viewing.is_none() && !core.game.is_over() {
        let stm = core.game.side_to_move();
        if core.game.board().is_in_check(stm) {
            // Find the king of the side in check.
            for (sq, piece) in core.game.board().pieces() {
                if piece.color == stm && piece.kind == chess_core::PieceKind::King {
                    let pos = square_to_world(sq, orient);
                    commands.spawn((
                        Sprite {
                            color: Color::srgba(1.0, 0.15, 0.1, 0.35),
                            custom_size: Some(Vec2::splat(CELL * 0.92)),
                            ..default()
                        },
                        Transform::from_xyz(pos.x, pos.y, 11.5),
                        HighlightMarker,
                        CheckHighlight,
                    ));
                    break;
                }
            }
        }
    }
}

/// Mark dirty once when entering the game so the first frame draws pieces.
pub fn mark_dirty_on_enter(mut dirty: ResMut<RenderDirty>, mut scale_in: ResMut<BoardScaleIn>) {
    dirty.0 = true;
    scale_in.active = true;
    scale_in.timer.reset();
}

/// Tear down board entities when leaving the game.
pub fn teardown_board(
    mut commands: Commands,
    q: Query<
        Entity,
        Or<(
            With<BoardLine>,
            With<PieceMarker>,
            With<HighlightMarker>,
            With<CoordLabel>,
        )>,
    >,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Pulse the selection highlight alpha for a gentle glow effect.
pub fn animate_selection_glow(
    time: Res<Time>,
    mut query: Query<&mut Sprite, With<SelectionHighlight>>,
) {
    let t = time.elapsed_secs();
    let alpha = 0.2 + 0.15 * (t * 3.0 * std::f32::consts::TAU).sin();
    for mut sprite in &mut query {
        sprite.color = Color::srgba(0.95, 0.78, 0.30, alpha);
    }
}

/// Animate the check warning overlay with a pulsing alpha.
pub fn animate_check_pulse(time: Res<Time>, mut query: Query<&mut Sprite, With<CheckHighlight>>) {
    let t = time.elapsed_secs();
    let alpha = 0.2 + 0.3 * (t * 6.0).sin().abs();
    for mut sprite in &mut query {
        sprite.color = Color::srgba(1.0, 0.15, 0.1, alpha);
    }
}

/// Animate pieces scaling in when the board first appears.
pub fn animate_board_scale_in(
    time: Res<Time>,
    mut scale_in: ResMut<BoardScaleIn>,
    mut pieces: Query<&mut Transform, With<PieceMarker>>,
) {
    if !scale_in.active {
        return;
    }

    scale_in.timer.tick(time.delta());
    let t = scale_in.timer.fraction();
    // Ease-out back: same overshoot easing as piece slide animations.
    let scale = crate::animation::ease_out_back(t);

    for mut tf in &mut pieces {
        tf.scale = Vec3::splat(scale);
    }

    if scale_in.timer.is_finished() {
        scale_in.active = false;
        // Ensure all pieces are exactly at scale 1.0.
        for mut tf in &mut pieces {
            tf.scale = Vec3::ONE;
        }
    }
}
