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
mod animation;
mod app_state;
mod async_runtime;
mod board_theme;
mod board_view;
mod captured_tray;
mod clock_ui;
mod confirm_resign;
#[allow(dead_code)]
mod difficulty_dialog;
mod drag;
mod game_over_dialog;
mod help_panel;

mod history_panel;
mod history_view;
mod hover;
mod input;
mod keyboard;
mod lan_dialog;
mod moves;
mod net_bridge;
mod opening_hints;
mod settings;
mod sound;
mod toast;
mod ui;
mod window_title;

use bevy::prelude::*;
use bevy::text::Font;

use app_state::{AiSettings, AppState, BoardOrientation, CoreGame, Selection};
use async_runtime::AsyncRuntime;
use board_view::RenderDirty;

/// Tracks whether the first-game keyboard shortcut hint has been shown.
#[derive(Resource, Default)]
struct FirstGameHintShown(bool);

/// CJK fonts embedded directly into the binary via `include_bytes!` so the
/// game runs from a single self-contained executable (no external `assets/`
/// directory required at runtime).
const CJK_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/cjk.otf");
const CJK_BOLD: &[u8] = include_bytes!("../../../assets/fonts/cjk-bold.otf");

/// Reset the board orientation back to Red when returning to the menu so that
/// a subsequent local / VsAi game does not inherit a flipped board from a
/// previous networked session where the local player was Black.
/// Show a one-time hint about the help panel on first game entry.
fn show_first_game_hint(
    mut commands: Commands,
    fonts: Res<app_state::UiFonts>,
    mut shown: ResMut<FirstGameHintShown>,
) {
    if !shown.0 {
        shown.0 = true;
        let save_count = std::fs::read_dir(settings::save_dir())
            .ok()
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "pgn"))
                    .count()
            })
            .unwrap_or(0);
        if save_count > 0 {
            let msg = format!("按 H 查看快捷键 · {}个存档 · Ctrl+O 加载", save_count);
            toast::spawn_toast(&mut commands, &fonts, &msg);
        } else {
            toast::spawn_toast(&mut commands, &fonts, "按 H 查看快捷键");
        }
    }
}

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

    // Load persisted user settings and apply to resources.
    let saved_difficulty;
    {
        let user_settings = settings::load_settings();
        app.insert_resource(board_theme::BoardTheme {
            id: user_settings.theme,
            palette: user_settings.theme.palette(),
        });
        app.insert_resource(sound::SoundVolume {
            level: user_settings.volume,
        });
        app.insert_resource(animation::AnimSpeedSetting(user_settings.anim_speed));
        app.insert_resource(board_view::ShowCoordinates(user_settings.show_coordinates));
        saved_difficulty = user_settings.difficulty;
    }

    app.init_state::<AppState>()
        .init_resource::<CoreGame>()
        .init_resource::<Selection>()
        .insert_resource(AiSettings {
            difficulty: saved_difficulty.unwrap_or(chess_ai::Difficulty::Hard),
            ..AiSettings::default()
        })
        .init_resource::<RenderDirty>()
        .init_resource::<game_over_dialog::GameOverShown>()
        .init_resource::<difficulty_dialog::DifficultyDialogState>()
        .init_resource::<app_state::ClockResource>()
        .init_resource::<clock_ui::MoveTimer>()
        .init_resource::<BoardOrientation>()
        .init_resource::<lan_dialog::LanDialog>()
        .init_resource::<ai_bridge::AiTask>()
        .init_resource::<animation::AnimationPlaying>()
        .init_resource::<drag::DragState>()
        .init_resource::<help_panel::HelpPanelVisible>()
        .init_resource::<confirm_resign::ConfirmResignVisible>()
        .init_resource::<history_view::HistoryView>()
        .init_resource::<sound::PendingSound>()
        .init_resource::<FirstGameHintShown>()
        .init_resource::<keyboard::AutoPlayState>()
        .init_resource::<opening_hints::OpeningHintShown>()
        .init_resource::<app_state::RematchCount>()
        .init_resource::<app_state::SessionStats>()
        .init_resource::<app_state::LastGameResult>()
        .init_resource::<app_state::BackToMenuPending>()
        .init_resource::<app_state::DrawOfferPending>()
        .init_resource::<app_state::SessionPlayTime>()
        .init_resource::<app_state::UndoCount>()
        .init_resource::<app_state::WinStreak>()
        .init_resource::<app_state::GameStartTime>()
        .init_resource::<app_state::MoveTimeHistory>()
        .init_resource::<ui::MenuSelection>()
        .init_resource::<board_view::BoardScaleIn>()
        .insert_resource(AsyncRuntime::new())
        .insert_resource(load_relay_config())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.07, 0.09)))
        .add_systems(Startup, (setup_camera, sound::init_sounds))
        // Menu state.
        .add_systems(
            OnEnter(AppState::Menu),
            (
                ui::setup_menu,
                reset_board_orientation,
                window_title::reset_window_title,
            ),
        )
        .add_systems(
            OnExit(AppState::Menu),
            (
                ui::teardown_menu,
                lan_dialog::teardown_lan_dialog,
                difficulty_dialog::teardown_difficulty_dialog,
            ),
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
                ui::animate_menu_subtitle,
                ui::menu_keyboard_nav,
                difficulty_dialog::spawn_difficulty_dialog,
                difficulty_dialog::difficulty_dialog_interaction,
            )
                .run_if(in_state(AppState::Menu)),
        )
        // In-game state.
        .add_systems(
            OnEnter(AppState::InGame),
            (
                board_view::setup_board,
                ui::setup_hud,
                history_panel::setup_history_panel,
                clock_ui::setup_clock_ui,
                clock_ui::init_clock,
                captured_tray::setup_captured_tray,
                board_view::mark_dirty_on_enter,
                help_panel::setup_help_panel,
                show_first_game_hint,
                app_state::record_game_start_time,
            ),
        )
        .add_systems(
            OnExit(AppState::InGame),
            (
                board_view::teardown_board,
                ui::teardown_hud,
                history_panel::teardown_history_panel,
                clock_ui::teardown_clock_ui,
                captured_tray::teardown_captured_tray,
                game_over_dialog::teardown_game_over,
                toast::teardown_toasts,
                ui::teardown_draw_offer,
                animation::teardown_animations,
                drag::teardown_drag,
                help_panel::teardown_help_panel,
                hover::teardown_hover,
                confirm_resign::teardown_confirm_resign,
            ),
        )
        .add_systems(
            Update,
            (
                drag::handle_drag,
                input::handle_click,
                ai_bridge::request_ai_move,
                ai_bridge::poll_ai_move,
                net_bridge::poll_net_events,
                sound::play_pending_sound,
                ui::hud_interaction,
                ui::manage_draw_offer,
                ui::draw_offer_interaction,
                ui::update_status,
                clock_ui::clock_on_move,
                clock_ui::tick_clock,
                captured_tray::update_captured_tray,
                history_panel::update_history_panel,
                board_view::redraw_pieces,
                animation::animate_pieces,
                game_over_dialog::check_game_over,
                game_over_dialog::game_over_interaction,
                keyboard::keyboard_shortcuts,
                toast::update_toasts,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                help_panel::toggle_help_panel,
                window_title::update_window_title,
                animation::animate_invalid_flash,
                ui::pulse_turn_indicator,
                hover::update_hover,
                confirm_resign::manage_confirm_resign,
                confirm_resign::confirm_resign_interaction,
                keyboard::toggle_fullscreen,
                keyboard::export_history,
                board_view::animate_check_pulse,
                animation::animate_arrival_ring,
                keyboard::history_nav_sound,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                keyboard::auto_play_history,
                board_view::animate_selection_glow,
                history_panel::history_entry_click,
                ui::pulse_ai_thinking,
                history_panel::history_entry_hover,
                keyboard::quick_restart,
                keyboard::quick_difficulty,
                board_view::animate_board_scale_in,
                keyboard::undo_sound_trigger,
                opening_hints::show_opening_hint,
                keyboard::reset_settings,
                clock_ui::clock_warning_sound,
                game_over_dialog::animate_game_over_fadein,
                game_over_dialog::game_over_keyboard,
            )
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
        Font::try_from_bytes(CJK_BOLD.to_vec()).expect("embedded CJK bold font failed to parse");
    }
}
