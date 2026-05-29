# Assets & Original Art Pipeline

The game renders entirely from **primitives** today (colored quads for the
board, mesh discs + text labels for pieces) so it builds with **zero
third-party art**. This document specifies the original-art deliverables and how
to wire them in, satisfying the "original, no third-party assets" requirement
without fabricating binary art files in source control.

## Required original assets (modern 国风 / minimalist style)

Author these in Figma/PSD and export as listed. Keep the master vector/PSD in
`assets/source/` and exported runtime files in `assets/textures/`.

| Asset                | Format        | Sizes (1080p / 2K / 4K)         |
|----------------------|---------------|---------------------------------|
| Board background     | PNG + SVG     | 720² / 1080² / 2160² intersection grid |
| 7 piece types × 2 colors (14) | PNG (premultiplied) | 64 / 96 / 128 px @1x, plus @2x/@3x |
| Move/selection markers | SVG         | scalable                        |
| Buttons (normal/hover/pressed) | 9-slice PNG | 3 states                  |
| Dialogs/panels       | 9-slice PNG   | —                               |
| App icon             | PNG/ICO/ICNS  | 16…1024                         |

High-DPI: ship `@1x/@2x/@3x` variants; Bevy selects by `UiScale`/window scale
factor. Target color space sRGB.

## Wiring exported textures into Bevy

1. Place exports under `assets/textures/` (Bevy's default asset root is
   `assets/`).
2. Load a piece sprite instead of the primitive disc in
   `crates/chess-app/src/board_view.rs::redraw_pieces`:

   ```rust
   // Replace the Mesh2d disc with:
   let image: Handle<Image> = asset_server.load("textures/pieces/red_chariot.png");
   commands.spawn((
       Sprite { image, custom_size: Some(Vec2::splat(CELL * 0.9)), ..default() },
       Transform::from_xyz(pos.x, pos.y, 10.0),
       PieceMarker,
   ));
   ```

3. Replace the board grid primitives with a single board background sprite, and
   swap button `BackgroundColor` for 9-sliced `ImageNode`s in
   `crates/chess-app/src/ui.rs`.

A `board_view::piece_texture_path(color, kind)` helper is the natural single
point to map a piece to its texture path.

## Chinese piece glyphs (帅/将/车…)

The current build uses Latin initials (K/A/E/H/R/C/P) because Bevy's bundled
default font is Latin-only. To show Chinese glyphs, drop an **OFL-licensed** CJK
font (e.g. an open-source font — fonts under SIL OFL are freely redistributable
and are *not* "art assets") at `assets/fonts/cjk.ttf` and load it via
`asset_server.load("fonts/cjk.ttf")` for the piece `TextFont`. `chess_core::Piece::glyph()`
already returns the correct character for each piece.
