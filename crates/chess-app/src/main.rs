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

use app_state::{AppState, AiSettings, CoreGame, Selection};
use async_runtime::AsyncRuntime;
use board_view::RenderDirty;

/// A single persistent 2D camera that renders both the menu UI and the board.
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Resolve the directory that holds the bundled `assets/` folder.
///
/// Order of preference (so the game runs "out of the box" both in development
/// and from a packaged release on any platform):
/// 1. `assets/` sitting next to the executable (the distribution layout).
/// 2. the workspace `assets/` baked in at compile time (development).
/// 3. plain `assets` relative to the current working directory.
fn resolve_asset_root() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("assets");
            if candidate.is_dir() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    let workspace = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");
    if std::path::Path::new(workspace).is_dir() {
        return workspace.to_string();
    }
    "assets".to_string()
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
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "中国象棋 Xiangqi".into(),
                    resolution: bevy::window::WindowResolution::new(1280, 800),
                    ..default()
                }),
                ..default()
            })
            .set(bevy::asset::AssetPlugin {
                file_path: resolve_asset_root(),
                ..default()
            }),
    );

    // Load the bundled CJK fonts up front (order-independent) so the very first
    // menu/board UI renders Chinese text immediately.
    {
        let assets = app.world().resource::<AssetServer>().clone();
        app.insert_resource(app_state::UiFonts {
            regular: assets.load("fonts/cjk.otf"),
            bold: assets.load("fonts/cjk-bold.otf"),
        });
    }

    app.init_state::<AppState>()
        .init_resource::<CoreGame>()
        .init_resource::<Selection>()
        .init_resource::<AiSettings>()
        .init_resource::<RenderDirty>()
        .init_resource::<lan_dialog::LanDialog>()
        .init_resource::<ai_bridge::AiTask>()
        .insert_resource(AsyncRuntime::new())
        .insert_resource(load_relay_config())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.07, 0.09)))
        .add_systems(Startup, setup_camera)
        // Menu state.
        .add_systems(OnEnter(AppState::Menu), ui::setup_menu)
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
