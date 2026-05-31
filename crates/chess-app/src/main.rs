//! Xiangqi (Chinese Chess) — Bevy front-end.
//!
//! ECS architecture that cleanly decouples the four concerns required by the
//! spec:
//!
//! * **Rules / logic** — [`chess_core`] owns the authoritative [`CoreGame`].
//! * **Rendering** — [`board_view`] + [`ui`] draw the board, pieces, and HUD
//!   from primitives (no third-party art).
//! * **AI** — [`ai_bridge`] runs [`chess_ai`] (Pikafish via UCI, or the
//!   built-in fallback) on a Tokio runtime, polled non-blockingly each frame.
//! * **Networking** — [`net_bridge`] runs [`chess_net`] LAN sessions on the
//!   same runtime, also bridged via lock-free channels.
//!
//! The AI and network never run on the render thread, so the frame loop stays
//! smooth regardless of search time or socket latency.

// Bevy query tuples legitimately exceed clippy's type-complexity threshold.
#![allow(clippy::type_complexity)]

mod ai_bridge;
mod app_state;
mod async_runtime;
mod board_view;
mod input;
mod lan_dialog;
mod moves;
mod net_bridge;
mod ui;

use bevy::prelude::*;
use bevy::text::Font;

use app_state::{AiSettings, AppState, BoardOrientation, CoreGame, Selection};
use async_runtime::AsyncRuntime;
use board_view::RenderDirty;

/// CJK fonts embedded directly into the binary via `include_bytes!` so the
/// game runs from a single self-contained executable (no external `assets/`
/// directory required at runtime).
const CJK_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/cjk.otf");
const CJK_BOLD: &[u8] = include_bytes!("../../../assets/fonts/cjk-bold.otf");


/// Reset the board orientation back to Red when returning to the menu so that
/// a subsequent local / VsAi game does not inherit a flipped board from a
/// previous networked session where the local player was Black.
fn reset_board_orientation(mut orient: ResMut<BoardOrientation>) {
    *orient = BoardOrientation::Red;
}

/// A single persistent 2D camera that renders both the menu UI and the board.
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Parse the embedded font bytes into a [`Font`] asset and return its handle.
///
/// Panics with a clear message if the bytes are not a valid font; this is a
/// build-time invariant (the bytes are baked into the binary), so a panic
/// here means the source asset itself is broken.
fn embed_font(world: &mut World, bytes: &'static [u8], label: &str) -> Handle<Font> {
    let font = Font::try_from_bytes(bytes.to_vec())
        .unwrap_or_else(|e| panic!("embedded CJK font `{label}` failed to parse: {e:?}"));
    world.resource_mut::<Assets<Font>>().add(font)
}

/// Load the relay client configuration with precedence
/// **config file > environment > compiled default**. An optional
/// `--config <path>` / `--config=<path>` CLI argument selects the file.
fn load_relay_config() -> net_bridge::RelayConfig {
    let mut path: Option<std::path::PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" || arg == "-c" {
            path = args.next().map(std::path::PathBuf::from);
        } else if let Some(rest) = arg.strip_prefix("--config=") {
            path = Some(std::path::PathBuf::from(rest));
        }
    }
    net_bridge::RelayConfig(chess_net::RelayClientConfig::load(path.as_deref()))
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "中国象棋 Xiangqi".into(),
            resolution: bevy::window::WindowResolution::new(1280, 800),
            ..default()
        }),
        ..default()
    }));

    // Register the embedded CJK fonts so the very first menu/board UI renders
    // Chinese text immediately, with no dependency on an external assets dir.
    {
        let world = app.world_mut();
        let regular = embed_font(world, CJK_REGULAR, "cjk.otf");
        let bold = embed_font(world, CJK_BOLD, "cjk-bold.otf");
        app.insert_resource(app_state::UiFonts { regular, bold });
    }

    app.init_state::<AppState>()
        .init_resource::<CoreGame>()
        .init_resource::<Selection>()
        .init_resource::<AiSettings>()
        .init_resource::<RenderDirty>()
        .init_resource::<BoardOrientation>()
        .init_resource::<lan_dialog::LanDialog>()
        .init_resource::<ai_bridge::AiTask>()
        .insert_resource(AsyncRuntime::new())
        .insert_resource(load_relay_config())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.07, 0.09)))
        .add_systems(Startup, setup_camera)
        // Menu state.
        .add_systems(OnEnter(AppState::Menu), (ui::setup_menu, reset_board_orientation))
        .add_systems(
            OnExit(AppState::Menu),
            (ui::teardown_menu, lan_dialog::teardown_lan_dialog),
        )
        .add_systems(
            Update,
            (
                ui::menu_interaction,
                lan_dialog::manage_lan_dialog,
                lan_dialog::lan_dialog_buttons,
                lan_dialog::lan_dialog_keyboard,
                lan_dialog::lan_dialog_ime,
                lan_dialog::lan_dialog_sync_ime,
                lan_dialog::lan_dialog_submit,
                lan_dialog::lan_dialog_render,
            )
                .run_if(in_state(AppState::Menu)),
        )
        // In-game state.
        .add_systems(
            OnEnter(AppState::InGame),
            (
                board_view::setup_board,
                ui::setup_hud,
                board_view::mark_dirty_on_enter,
            ),
        )
        .add_systems(
            OnExit(AppState::InGame),
            (board_view::teardown_board, ui::teardown_hud),
        )
        .add_systems(
            Update,
            (
                input::handle_click,
                ai_bridge::request_ai_move,
                ai_bridge::poll_ai_move,
                net_bridge::poll_net_events,
                ui::hud_interaction,
                ui::update_status,
                board_view::redraw_pieces,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_fonts_are_present() {
        assert!(!CJK_REGULAR.is_empty(), "regular font bytes empty");
        assert!(!CJK_BOLD.is_empty(), "bold font bytes empty");
    }

    #[test]
    fn embedded_fonts_parse() {
        Font::try_from_bytes(CJK_REGULAR.to_vec())
            .expect("embedded CJK regular font failed to parse");
        Font::try_from_bytes(CJK_BOLD.to_vec())
            .expect("embedded CJK bold font failed to parse");
    }
}
