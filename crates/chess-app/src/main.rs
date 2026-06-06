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
#![allow(dead_code)]

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

mod achievements;
mod ai_difficulty_scaling;
mod ai_personality;
mod analysis_mode;
mod analytics;
mod arrow_annotation;
mod auto_flip;
mod autosave;
mod blindfold;
mod board_arrow_drawing;
mod board_color;
mod board_coordinate_display;
mod board_editor;
mod board_highlight_last_move;
mod board_perspective;
mod board_rotation_view;
mod board_scaling;
mod board_theme_selector;
mod chinese_notation;
mod clipboard;
mod clock_alarm;
mod comments_panel;
mod coord_style;
mod coordinate_highlight;
mod daily_puzzle;
mod endgame_puzzle_gen;
mod endgame_tablebase2;
mod endgame_training;
mod endgame_training_mode;
mod engine_vs_engine;
mod eval_bar;
mod game_analysis_report;
mod game_bookmarks;
mod game_clock_display;
mod game_collection_manager;
mod game_comments;
mod game_database;
mod game_database_browser;
mod game_evaluation_graph;
mod game_export;
mod game_journal;
mod game_notation_converter;
mod game_pgn_exporter;
mod game_pgn_import;
mod game_phase;
mod game_phase_timer;
mod game_result_tracker;
mod game_speed;
mod game_statistics_dashboard;
mod game_statistics_panel;
mod game_stats;
mod game_tags;
mod game_templates;
mod game_timer_presets;
mod game_undo_stack;
mod game_variations;
mod history_panel;
mod history_search;
mod history_view;
mod hover;
mod i18n;
mod input;
mod keyboard;
mod keyboard_input_mode;
mod keyboard_shortcuts_help;
mod lan_dialog;
mod material_advantage;
mod move_annotation;
mod move_comparison;
mod move_confidence;
mod move_hint;
mod move_history_export;
mod move_input;
mod move_legality;
mod move_legality_checker;
mod move_prediction;
mod move_quality;
mod move_quality_indicator;
mod move_search;
mod move_sound_library;
mod move_sound_player;
mod move_stats_chart;
mod move_time_display;
mod move_time_tracker;
mod move_tree;
mod move_tree_view;
mod movement_trails;
mod moves;
mod notation_autocomplete;
mod notation_validator;
mod opening_book_display;
mod opening_book_manager;
mod opening_explorer;
mod opening_name;
mod opening_practice;
mod opening_repertoire;
mod opening_statistics;
mod opening_tree;
mod piece_animation_custom;
mod piece_mobility;
mod piece_movement_sound;
mod piece_strength;
mod piece_style;
mod piece_threat_display;
mod piece_value_display;
mod position_evaluation_bar;
mod position_evaluator;
mod position_fen_copy;
mod position_hash;
mod position_setup;
mod position_setup_board;
mod position_similarity;
mod puzzle_generator;
mod puzzle_mode;
mod quick_game;
mod replay_mode;
mod session_summary;
mod session_timer;
mod shortcuts_cheatsheet;
mod sound_themes;
mod sound_volume_per_type;
mod spectator;
mod tactical_motifs;
mod tactical_patterns;
mod thinking_indicator;
mod time_bonus_display;
mod time_pressure_display;
mod time_warnings;
mod tournament;

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

/// Reset network/session state on return to menu so the next game starts clean.
fn reset_core_session(mut core: ResMut<app_state::CoreGame>) {
    core.reset_session();
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
        app.insert_resource(board_view::CoordinateStyle::default());
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
        .insert_resource(move_tree::MoveTree::default())
        .insert_resource(opening_explorer::OpeningDatabase::default())
        .insert_resource(puzzle_mode::PuzzleMode::default())
        .insert_resource(game_database::GameDatabase::default())
        .insert_resource(engine_vs_engine::EngineVsEngine::default())
        .insert_resource(position_setup::PositionSetup::default())
        .insert_resource(move_input::MoveInput::default())
        .insert_resource(coordinate_highlight::CoordinateHighlight::default())
        .insert_resource(arrow_annotation::ArrowAnnotations::default())
        .insert_resource(board_scaling::BoardScaling::default())
        .insert_resource(ai_bridge::SearchInfoResource::default())
        .insert_resource(analysis_mode::AnalysisMode::default())
        .insert_resource(game_stats::GameStatistics::default())
        .insert_resource(opening_name::OpeningName::default())
        .insert_resource(move_quality::MoveQualityTracker::default())
        .insert_resource(replay_mode::ReplayMode::default())
        .insert_resource(autosave::AutoSave::default())
        .insert_resource(move_time_display::MoveTimeDisplay::default())
        .insert_resource(piece_style::PieceStyleResource::default())
        .insert_resource(blindfold::BlindfoldMode::default())
        .insert_resource(opening_practice::OpeningPractice::default())
        .insert_resource(move_hint::MoveHint::default())
        .insert_resource(coord_style::CoordStyleResource::default())
        .insert_resource(endgame_tablebase2::EndgameTablebase::default())
        .insert_resource(chinese_notation::ChineseNotation::default())
        .insert_resource(achievements::AchievementTracker::default())
        .insert_resource(daily_puzzle::DailyPuzzleChallenge::default())
        .insert_resource(game_journal::GameJournal::default())
        .insert_resource(analytics::Analytics::default())
        .insert_resource(spectator::SpectatorMode::default())
        .insert_resource(move_stats_chart::MoveStatsChart::default())
        .insert_resource(i18n::I18n::default())
        .insert_resource(board_editor::BoardEditor::default())
        .insert_resource(sound_themes::SoundThemeResource::default())
        .insert_resource(game_export::GameExport::default())
        .insert_resource(shortcuts_cheatsheet::ShortcutsCheatsheet::default())
        .insert_resource(session_timer::SessionTimer::default())
        .insert_resource(move_search::MoveSearch::default())
        .insert_resource(endgame_training::EndgameTraining::default())
        .insert_resource(tournament::Tournament::default())
        .insert_resource(position_evaluator::PositionEvaluator::default())
        .insert_resource(comments_panel::CommentsPanel::default())
        .insert_resource(notation_validator::NotationValidator::default())
        .insert_resource(game_templates::GameTemplates::default())
        .insert_resource(position_hash::PositionHash::default())
        .insert_resource(time_warnings::TimeWarnings::default())
        .insert_resource(game_bookmarks::GameBookmarks::default())
        .insert_resource(ai_personality::AiPersonalityResource::default())
        .insert_resource(move_comparison::MoveComparisonTracker::default())
        .insert_resource(opening_tree::OpeningTree::default())
        .insert_resource(session_summary::SessionSummary::default())
        .insert_resource(move_legality::MoveLegality::default())
        .insert_resource(clock_alarm::ClockAlarm::default())
        .insert_resource(notation_autocomplete::NotationAutocomplete::default())
        .insert_resource(board_perspective::BoardPerspective::default())
        .insert_resource(movement_trails::MovementTrails::default())
        .insert_resource(tactical_patterns::TacticalPatterns::default())
        .insert_resource(game_phase::GamePhaseIndicator::default())
        .insert_resource(material_advantage::MaterialAdvantage::default())
        .insert_resource(quick_game::QuickGame::default())
        .insert_resource(move_confidence::MoveConfidence::default())
        .insert_resource(endgame_puzzle_gen::EndgamePuzzleGen::default())
        .insert_resource(piece_strength::PieceStrength::default())
        .insert_resource(history_search::HistorySearch::default())
        .insert_resource(board_color::BoardColorResource::default())
        .insert_resource(auto_flip::AutoFlip::default())
        .insert_resource(sound_volume_per_type::SoundVolumePerType::default())
        .insert_resource(game_tags::GameTags::default())
        .insert_resource(position_similarity::PositionSimilarity::default())
        .insert_resource(keyboard_input_mode::KeyboardInputMode::default())
        .insert_resource(game_speed::GameSpeedResource::default())
        .insert_resource(thinking_indicator::ThinkingIndicator::default())
        .insert_resource(time_pressure_display::TimePressureDisplay::default())
        .insert_resource(piece_animation_custom::PieceAnimationCustom::default())
        .insert_resource(game_statistics_panel::GameStatisticsPanel::default())
        .insert_resource(opening_book_display::OpeningBookDisplay::default())
        .insert_resource(move_prediction::MovePrediction::default())
        .insert_resource(game_comments::GameComments::default())
        .insert_resource(time_bonus_display::TimeBonusDisplay::default())
        .insert_resource(piece_mobility::PieceMobility::default())
        .insert_resource(game_evaluation_graph::GameEvaluationGraph::default())
        .insert_resource(keyboard_shortcuts_help::KeyboardShortcutsHelp::default())
        .insert_resource(move_tree_view::MoveTreeView::default())
        .insert_resource(position_fen_copy::PositionFenCopy::default())
        .insert_resource(game_phase_timer::GamePhaseTimer::default())
        .insert_resource(piece_threat_display::PieceThreatDisplay::default())
        .insert_resource(move_quality_indicator::MoveQualityIndicator::default())
        .insert_resource(opening_repertoire::OpeningRepertoire::default())
        .insert_resource(tactical_motifs::TacticalMotifsDetector::default())
        .insert_resource(endgame_training_mode::EndgameTrainingMode::default())
        .insert_resource(game_analysis_report::GameAnalysisReport::default())
        .insert_resource(position_evaluation_bar::PositionEvaluationBar::default())
        .insert_resource(game_variations::GameVariations::default())
        .insert_resource(opening_book_manager::OpeningBookManager::default())
        .insert_resource(game_timer_presets::GameTimerPresets::default())
        .insert_resource(move_history_export::MoveHistoryExport::default())
        .insert_resource(piece_value_display::PieceValueDisplay::default())
        .insert_resource(game_pgn_import::GamePgnImport::default())
        .insert_resource(board_highlight_last_move::BoardHighlightLastMove::default())
        .insert_resource(game_undo_stack::GameUndoStack::default())
        .insert_resource(move_annotation::MoveAnnotations::default())
        .insert_resource(game_clock_display::GameClockDisplay::default())
        .insert_resource(board_arrow_drawing::BoardArrowDrawing::default())
        .insert_resource(move_sound_library::MoveSoundLibrary::default())
        .insert_resource(game_database_browser::GameDatabaseBrowser::default())
        .insert_resource(position_setup_board::PositionSetupBoard::default())
        .insert_resource(game_collection_manager::GameCollectionManager::default())
        .insert_resource(ai_difficulty_scaling::AiDifficultyScaling::default())
        .insert_resource(move_legality_checker::MoveLegalityChecker::default())
        .insert_resource(board_rotation_view::BoardRotationView::default())
        .insert_resource(game_notation_converter::GameNotationConverter::default())
        .insert_resource(piece_movement_sound::PieceMovementSound::default())
        .insert_resource(game_statistics_dashboard::GameStatisticsDashboard::default())
        .insert_resource(opening_statistics::OpeningStatistics::default())
        .insert_resource(move_time_tracker::MoveTimeTracker::default())
        .insert_resource(game_result_tracker::GameResultTracker::default())
        .insert_resource(board_theme_selector::BoardThemeSelector::default())
        .insert_resource(puzzle_generator::PuzzleGenerator::default())
        .insert_resource(game_pgn_exporter::GamePgnExporter::default())
        .insert_resource(move_sound_player::MoveSoundPlayer::default())
        .insert_resource(board_coordinate_display::BoardCoordinateDisplay::default())
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
                eval_bar::setup_eval_bar,
                help_panel::setup_help_panel,
                show_first_game_hint,
                app_state::record_game_start_time,
            ),
        )
        .add_systems(
            OnExit(AppState::InGame),
            (
                reset_core_session,
                board_view::teardown_board,
                net_bridge::teardown_net,
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
                eval_bar::teardown_eval_bar,
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
                opening_explorer::toggle_opening_explorer,
                opening_explorer::update_opening_explorer,
                puzzle_mode::toggle_puzzle_mode,
                puzzle_mode::load_sample_puzzles,
                game_database::toggle_game_database,
                engine_vs_engine::toggle_engine_vs_engine,
                engine_vs_engine::make_engine_moves,
                position_setup::toggle_position_setup,
                position_setup::handle_fen_input,
                position_setup::update_fen_input,
                move_input::toggle_move_input,
                move_input::handle_move_input,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                move_input::update_move_input_chars,
                coordinate_highlight::update_coordinate_highlight,
                coordinate_highlight::update_file_label_colors,
                coordinate_highlight::update_rank_label_colors,
                coordinate_highlight::toggle_coordinate_highlight,
                arrow_annotation::toggle_arrow_drawing,
                arrow_annotation::clear_arrows,
                arrow_annotation::update_arrow_visuals,
                board_scaling::update_board_scaling,
                board_scaling::handle_board_size_input,
                board_scaling::apply_board_scaling,
                eval_bar::update_eval_bar,
                analysis_mode::toggle_analysis_mode,
                analysis_mode::update_analysis_from_search_info,
                analysis_mode::update_analysis_ui,
                analysis_mode::despawn_analysis_ui,
                clipboard::copy_fen_to_clipboard,
                clipboard::paste_fen_from_clipboard,
                help_panel::toggle_help_panel,
                window_title::update_window_title,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
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
        .add_systems(
            Update,
            (
                opening_name::detect_opening,
                move_quality::classify_moves,
                move_quality::reset_move_quality,
                move_time_display::track_move_time,
                move_time_display::reset_move_times,
                autosave::periodic_autosave,
                autosave::autosave_on_move,
                replay_mode::toggle_replay,
                replay_mode::replay_keyboard,
                replay_mode::replay_auto_advance,
                piece_style::toggle_piece_style,
                blindfold::toggle_blindfold,
                opening_practice::toggle_opening_practice,
                move_hint::toggle_hints,
                move_hint::update_move_hints,
                move_hint::track_hint_time,
                move_hint::reset_hint_timer,
                coord_style::toggle_coord_style,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                chinese_notation::toggle_chinese_notation,
                achievements::check_achievements_system,
                game_journal::toggle_journal,
                spectator::toggle_spectator,
                move_stats_chart::toggle_chart,
                i18n::cycle_language,
                board_editor::toggle_board_editor,
                sound_themes::cycle_sound_theme,
                game_export::export_game_shortcut,
                shortcuts_cheatsheet::toggle_cheatsheet,
                session_timer::update_session_timer,
                move_search::toggle_move_search,
                endgame_training::toggle_endgame_training,
                tournament::toggle_tournament,
                position_evaluator::toggle_position_eval,
                comments_panel::toggle_comments,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                notation_validator::toggle_validator,
                game_templates::toggle_templates,
                position_hash::toggle_hash_display,
                time_warnings::toggle_time_warnings,
                game_bookmarks::add_bookmark,
                game_bookmarks::toggle_bookmarks,
                ai_personality::cycle_personality,
                move_comparison::toggle_comparison,
                opening_tree::toggle_opening_tree,
                session_summary::show_summary,
                move_legality::toggle_legality,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                clock_alarm::toggle_alarm,
                notation_autocomplete::toggle_autocomplete,
                board_perspective::cycle_perspective,
                movement_trails::toggle_trails,
                tactical_patterns::toggle_patterns,
                game_phase::toggle_phase,
                material_advantage::toggle_material,
                quick_game::toggle_quick_game,
                move_confidence::toggle_confidence,
                endgame_puzzle_gen::toggle_generator,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                piece_strength::toggle_strength,
                history_search::toggle_history_search,
                board_color::cycle_board_colors,
                auto_flip::toggle_auto_flip,
                sound_volume_per_type::cycle_sound_focus,
                game_tags::add_game_tag,
                position_similarity::toggle_similarity,
                keyboard_input_mode::cycle_input_mode,
                game_speed::cycle_game_speed,
                thinking_indicator::cycle_thinking_style,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                time_pressure_display::toggle_pressure_display,
                piece_animation_custom::cycle_animation_speed,
                game_statistics_panel::toggle_stats_panel,
                opening_book_display::toggle_opening_book,
                move_prediction::toggle_prediction,
                game_comments::toggle_comments,
                time_bonus_display::toggle_bonus_display,
                piece_mobility::toggle_mobility,
                game_evaluation_graph::toggle_eval_graph,
                keyboard_shortcuts_help::toggle_help,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                move_tree_view::toggle_tree_view,
                position_fen_copy::copy_position,
                game_phase_timer::toggle_phase_timer,
                piece_threat_display::toggle_threat_display,
                move_quality_indicator::toggle_quality_indicator,
                opening_repertoire::toggle_repertoire,
                tactical_motifs::toggle_motifs_detector,
                endgame_training_mode::toggle_training_mode,
                game_analysis_report::toggle_analysis_report,
                position_evaluation_bar::toggle_eval_bar,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                game_variations::toggle_variations,
                opening_book_manager::toggle_book,
                game_timer_presets::cycle_timer_preset,
                move_history_export::cycle_export_format,
                piece_value_display::toggle_piece_values,
                game_pgn_import::import_pgn_shortcut,
                board_highlight_last_move::toggle_last_move_highlight,
                game_undo_stack::undo_redo_shortcuts,
                move_annotation::toggle_annotations,
                game_clock_display::toggle_clock_display,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                board_arrow_drawing::toggle_arrow_drawing,
                board_arrow_drawing::clear_arrows_shortcut,
                move_sound_library::toggle_sound_library,
                game_database_browser::toggle_database_browser,
                position_setup_board::toggle_setup_board,
                game_collection_manager::toggle_collection_manager,
                ai_difficulty_scaling::cycle_difficulty,
                move_legality_checker::toggle_legality_checker,
                board_rotation_view::rotate_board,
                game_notation_converter::cycle_notation_format,
                piece_movement_sound::toggle_movement_sound,
            )
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            Update,
            (
                game_statistics_dashboard::toggle_stats_dashboard,
                opening_statistics::toggle_opening_stats,
                move_time_tracker::toggle_time_tracker,
                game_result_tracker::show_streak,
                board_theme_selector::cycle_theme,
                endgame_tablebase2::toggle_tablebase,
                puzzle_generator::next_puzzle_shortcut,
                game_pgn_exporter::export_pgn_shortcut,
                move_sound_player::toggle_move_sounds,
                board_coordinate_display::cycle_coordinate_style,
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
