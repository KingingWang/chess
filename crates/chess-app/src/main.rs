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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "中国象棋 Xiangqi".into(),
                resolution: bevy::window::WindowResolution::new(1280, 800),
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .init_resource::<CoreGame>()
        .init_resource::<Selection>()
        .init_resource::<AiSettings>()
        .init_resource::<RenderDirty>()
        .init_resource::<ai_bridge::AiTask>()
        .insert_resource(AsyncRuntime::new())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.07, 0.09)))
        .add_systems(Startup, setup_camera)
        // Menu state.
        .add_systems(OnEnter(AppState::Menu), ui::setup_menu)
        .add_systems(OnExit(AppState::Menu), ui::teardown_menu)
        .add_systems(
            Update,
            ui::menu_interaction.run_if(in_state(AppState::Menu)),
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
