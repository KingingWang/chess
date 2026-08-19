//! `bevy_ui` menus and the in-game HUD.
//!
//! Visual language: 「玄玉」 — warm ink backdrop, brass-gold ornaments, jade
//! for primary actions, cinnabar for danger. Design tokens live in
//! [`crate::ui_theme`]; this file owns layout and interaction for the main
//! menu and the in-game sidebar HUD.

use bevy::prelude::*;
use chess_core::{Color as ChessColor, GameResult, WinReason};

use crate::ai_bridge::AiTask;
use crate::app_state::{AiSettings, AppState, CoreGame, GameMode, Selection, UiFonts};
use crate::board_view::RenderDirty;
use crate::confirm_resign::ConfirmResignVisible;
use crate::lan_dialog::LanDialog;
use crate::net_bridge::{NetCommand, NetLink};
use crate::ui_theme::*;

// --- Menu ----------------------------------------------------------------

#[derive(Component)]
pub struct MenuRoot;

#[derive(Component, Clone, Copy)]
pub struct MenuButton(pub GameMode);

/// Marker for the animated subtitle text in the menu.
#[derive(Component)]
pub struct MenuSubtitle;

/// Tracks the currently keyboard-selected menu button index.
#[derive(Resource, Default)]
pub struct MenuSelection(pub usize);

/// One-line description under each menu button label.
fn menu_caption(mode: GameMode) -> &'static str {
    match mode {
        GameMode::VsAi => "挑战内置的强力引擎",
        GameMode::LocalPvp => "同屏轮流走棋",
        GameMode::LanHost => "局域网开一间棋室",
        GameMode::LanJoin => "加入好友的棋室",
        _ => "",
    }
}

/// The four menu entries in visual (and keyboard-navigation) order.
const MENU_MODES: [GameMode; 4] = [
    GameMode::VsAi,
    GameMode::LocalPvp,
    GameMode::LanHost,
    GameMode::LanJoin,
];

fn menu_btn_style(mode: GameMode) -> BtnStyle {
    match mode {
        GameMode::VsAi => BtnStyle::primary(),
        _ => BtnStyle::secondary(),
    }
}

fn menu_btn_label(mode: GameMode) -> &'static str {
    match mode {
        GameMode::VsAi => "人机对战",
        GameMode::LocalPvp => "本地双人",
        GameMode::LanHost => "创建房间",
        GameMode::LanJoin => "加入房间",
        _ => "开始",
    }
}

/// Spawn one menu button (label + caption, styled by mode).
fn spawn_menu_button(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    mode: GameMode,
    width: f32,
) {
    let style = menu_btn_style(mode);
    let primary = matches!(mode, GameMode::VsAi);
    parent
        .spawn((
            Button,
            MenuButton(mode),
            style,
            Node {
                width: Val::Px(width),
                height: Val::Px(if primary { 62.0 } else { 56.0 }),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(3.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(14.0)),
                ..default()
            },
            BackgroundColor(style.bg),
            BorderColor::all(style.border),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(menu_btn_label(mode)),
                TextFont {
                    font: fonts.bold.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(if primary { JADE_BRIGHT } else { TEXT }),
            ));
            b.spawn((
                Text::new(menu_caption(mode)),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(if primary {
                    Color::srgba(0.85, 0.96, 0.90, 0.70)
                } else {
                    TEXT_FAINT
                }),
            ));
        });
}

/// Small section label like "开 始 对 弈".
fn spawn_section_label(parent: &mut ChildSpawnerCommands, fonts: &UiFonts, text: &str) {
    parent.spawn((
        Text::new(text.to_string()),
        TextFont {
            font: fonts.regular.clone(),
            font_size: 13.0,
            ..default()
        },
        TextColor(TEXT_FAINT),
        Node {
            margin: UiRect {
                left: Val::Px(2.0),
                bottom: Val::Px(10.0),
                top: Val::Px(4.0),
                ..default()
            },
            ..default()
        },
    ));
}

/// Engine status pill shown in the menu: jade dot when the bundled Pikafish
/// is available, gray when only the built-in fallback engine exists.
fn spawn_engine_chip(parent: &mut ChildSpawnerCommands, fonts: &UiFonts, ai: &AiSettings) {
    let (dot, label) = if ai.engine_path.is_some() {
        (JADE, "强力引擎 Pikafish 已就绪")
    } else {
        (TEXT_FAINT, "基础内置引擎")
    };
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(999.0)),
                margin: UiRect::top(Val::Px(26.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.141, 0.129, 0.110, 0.6)),
            BorderColor::all(HAIRLINE),
        ))
        .with_children(|chip| {
            chip.spawn((
                Node {
                    width: Val::Px(7.0),
                    height: Val::Px(7.0),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(dot),
            ));
            chip.spawn((
                Text::new(label),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(TEXT_DIM),
            ));
        });
}

pub fn setup_menu(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    ai_settings: Res<AiSettings>,
    last_result: Res<crate::app_state::LastGameResult>,
    session_stats: Res<crate::app_state::SessionStats>,
    session_play_time: Res<crate::app_state::SessionPlayTime>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(INK),
            MenuRoot,
        ))
        .with_children(|root| {
            // Faint oversized watermark glyph on the right edge.
            root.spawn((
                Text::new("將"),
                TextFont {
                    font: fonts.bold.clone(),
                    font_size: 520.0,
                    ..default()
                },
                TextColor(Color::srgba(0.85, 0.70, 0.42, 0.045)),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(-190.0),
                    top: Val::Percent(50.0),
                    margin: UiRect::top(Val::Px(-240.0)),
                    ..default()
                },
            ));
            root.spawn((
                Text::new("帥"),
                TextFont {
                    font: fonts.bold.clone(),
                    font_size: 300.0,
                    ..default()
                },
                TextColor(Color::srgba(0.68, 0.27, 0.21, 0.05)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-50.0),
                    bottom: Val::Px(-70.0),
                    ..default()
                },
            ));

            // ---- Left column: brand block ---------------------------------
            root.spawn((Node {
                width: Val::Percent(52.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Start,
                padding: UiRect::left(Val::Px(96.0)),
                ..default()
            },))
                .with_children(|col| {
                    // Seal stamp.
                    col.spawn((
                        Node {
                            width: Val::Px(86.0),
                            height: Val::Px(86.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(16.0)),
                            margin: UiRect::bottom(Val::Px(30.0)),
                            ..default()
                        },
                        BackgroundColor(CINNABAR),
                        BorderColor::all(Color::srgba(0.96, 0.90, 0.78, 0.35)),
                        BoxShadow::new(
                            Color::srgba(0.0, 0.0, 0.0, 0.5),
                            Val::Px(0.0),
                            Val::Px(8.0),
                            Val::Px(6.0),
                            Val::Px(24.0),
                        ),
                    ))
                    .with_children(|seal| {
                        seal.spawn((
                            Text::new("將"),
                            TextFont {
                                font: fonts.bold.clone(),
                                font_size: 52.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.96, 0.91, 0.80)),
                        ));
                    });

                    // Title.
                    col.spawn((
                        Text::new("中 国 象 棋"),
                        TextFont {
                            font: fonts.bold.clone(),
                            font_size: 66.0,
                            ..default()
                        },
                        TextColor(GOLD_BRIGHT),
                    ));
                    col.spawn((
                        Text::new("X I A N G Q I"),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(TEXT_FAINT),
                        Node {
                            margin: UiRect::top(Val::Px(10.0)),
                            ..default()
                        },
                    ));
                    // Gold hairline.
                    col.spawn((
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(2.0),
                            margin: UiRect::vertical(Val::Px(24.0)),
                            border_radius: BorderRadius::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(GOLD_DIM),
                    ));
                    // Rotating poetic subtitle.
                    col.spawn((
                        Text::new({
                            let subtitles = [
                                "国风对弈 · 楚汉相争",
                                "棋逢对手 · 将帅之战",
                                "运筹帷幄 · 决胜千里",
                                "象棋风云 · 对弈人生",
                                "纵横九宫 · 驰骋楚河",
                                "妙手回春 · 攻守兼备",
                                "胸有成竹 · 步步为营",
                                "以棋会友 · 乐在其中",
                            ];
                            let idx = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as usize
                                % subtitles.len();
                            subtitles[idx]
                        }),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                        MenuSubtitle,
                    ));

                    // Last game result + session stats (kept from the old menu).
                    if let Some(ref result) = last_result.0 {
                        let (text, color) = match result {
                            chess_core::GameResult::Win { winner, .. } => {
                                if *winner == chess_core::Color::Red {
                                    ("上局 · 红方胜".to_string(), CINNABAR_HOVER)
                                } else {
                                    ("上局 · 黑方胜".to_string(), TEXT)
                                }
                            }
                            chess_core::GameResult::Draw(reason) => {
                                let desc = match reason {
                                    chess_core::DrawReason::Agreement => "协议和棋",
                                    chess_core::DrawReason::Repetition => "三次重复",
                                    chess_core::DrawReason::NoCapture => "无吃子和棋",
                                };
                                (format!("上局 · {desc}"), GOLD)
                            }
                        };
                        col.spawn((
                            Text::new(text),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(color),
                            Node {
                                margin: UiRect::top(Val::Px(18.0)),
                                ..default()
                            },
                        ));
                    }
                    if session_stats.total() > 0 {
                        let win_pct = session_stats.wins * 100 / session_stats.total();
                        col.spawn((
                            Text::new(format!(
                                "战绩 {}胜 {}负 {}和 · 胜率 {}%",
                                session_stats.wins,
                                session_stats.losses,
                                session_stats.draws,
                                win_pct
                            )),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(TEXT_FAINT),
                            Node {
                                margin: UiRect::top(Val::Px(6.0)),
                                ..default()
                            },
                        ));
                    }
                    if session_play_time.0 > 0.0 {
                        let total_secs = session_play_time.0 as u32;
                        let mins = total_secs / 60;
                        let secs = total_secs % 60;
                        let time_str = if mins >= 60 {
                            format!("本次共对弈 {}h{}分", mins / 60, mins % 60)
                        } else if mins > 0 {
                            format!("本次共对弈 {}分{}秒", mins, secs)
                        } else {
                            format!("本次共对弈 {}秒", secs)
                        };
                        col.spawn((
                            Text::new(time_str),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(TEXT_FAINT),
                            Node {
                                margin: UiRect::top(Val::Px(4.0)),
                                ..default()
                            },
                        ));
                    }

                    spawn_engine_chip(col, &fonts, &ai_settings);
                });

            // ---- Right column: actions ------------------------------------
            root.spawn((Node {
                width: Val::Percent(48.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::right(Val::Px(120.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },))
                .with_children(|col| {
                    spawn_section_label(col, &fonts, "开 始 对 弈");
                    spawn_menu_button(col, &fonts, GameMode::VsAi, 380.0);
                    spawn_menu_button(col, &fonts, GameMode::LocalPvp, 380.0);

                    col.spawn(Node {
                        height: Val::Px(16.0),
                        ..default()
                    });
                    spawn_section_label(col, &fonts, "联 机 对 弈");
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_menu_button(row, &fonts, GameMode::LanHost, 184.0);
                        spawn_menu_button(row, &fonts, GameMode::LanJoin, 184.0);
                    });

                    // Saved games + version footer.
                    let save_count = std::fs::read_dir(crate::settings::save_dir())
                        .ok()
                        .map(|rd| {
                            rd.filter_map(|e| e.ok())
                                .filter(|e| e.path().extension().is_some_and(|ext| ext == "pgn"))
                                .count()
                        })
                        .unwrap_or(0);
                    let footer = if save_count > 0 {
                        format!(
                            "{save_count} 个存档 · Ctrl+O 加载 · v{} · ↑↓ 选择 · Enter 确认",
                            env!("CARGO_PKG_VERSION")
                        )
                    } else {
                        format!("v{} · ↑↓ 选择 · Enter 确认", env!("CARGO_PKG_VERSION"))
                    };
                    col.spawn((
                        Text::new(footer),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(TEXT_FAINT),
                        Node {
                            margin: UiRect::top(Val::Px(28.0)),
                            ..default()
                        },
                    ));
                });
        });
}

/// Apply base/hover style to a menu button.
fn set_menu_btn_style(
    mode: GameMode,
    style: BtnStyle,
    active: bool,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    let _ = mode;
    if active {
        *bg = BackgroundColor(style.bg_hover);
        *border = BorderColor::all(style.border_hover);
    } else {
        *bg = BackgroundColor(style.bg);
        *border = BorderColor::all(style.border);
    }
}

pub fn menu_interaction(
    mut interactions: Query<
        (
            &Interaction,
            &MenuButton,
            &BtnStyle,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
    mut core: ResMut<CoreGame>,
    mut next: ResMut<NextState<AppState>>,
    mut dialog: ResMut<LanDialog>,
    mut diff_state: ResMut<crate::difficulty_dialog::DifficultyDialogState>,
) {
    for (interaction, btn, style, mut bg, mut border) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match btn.0 {
                // Network modes open the setup dialog (transport / port / password).
                GameMode::LanHost => dialog.open_for(true),
                GameMode::LanJoin => dialog.open_for(false),
                // AI mode: open difficulty picker first.
                GameMode::VsAi => {
                    diff_state.open = true;
                }
                // Local starts immediately.
                other => {
                    core.restart();
                    core.mode = other;
                    core.local_color = ChessColor::Red;
                    next.set(AppState::InGame);
                }
            },
            Interaction::Hovered => {
                set_menu_btn_style(btn.0, *style, true, &mut bg, &mut border);
            }
            Interaction::None => {
                set_menu_btn_style(btn.0, *style, false, &mut bg, &mut border);
            }
        }
    }
}

pub fn teardown_menu(mut commands: Commands, q: Query<Entity, With<MenuRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// --- In-game HUD ---------------------------------------------------------

#[derive(Component)]
pub struct HudRoot;

/// One-line headline of the status card ("轮到你走棋", "红方胜 · 将死", …).
#[derive(Component)]
pub struct StatusText;

/// Secondary status lines (move number, material, warnings, …).
#[derive(Component)]
pub struct StatusDetail;

/// Pulsing dot showing whose turn it is.
#[derive(Component)]
pub struct TurnIndicator;

/// Text that pulses while AI is thinking.
#[derive(Component)]
pub struct AiThinkingText;

#[derive(Component, Clone, Copy)]
pub enum HudAction {
    NewGame,
    Resign,
    OfferDraw,
    Undo,
    BackToMenu,
}

/// Slot in the left sidebar that hosts the captured-pieces tray
/// (populated by `captured_tray` once any piece has been captured).
#[derive(Component)]
pub struct CapturedSlot;

/// Slot in the left sidebar that hosts the analysis panel
/// (populated by `analysis_mode` while analysis is enabled).
#[derive(Component)]
pub struct AnalysisSlot;

/// Short engine label for the sidebar header chip.
pub fn engine_label(settings: &AiSettings) -> &'static str {
    if settings.engine_path.is_some() {
        "Pikafish"
    } else {
        "内置引擎"
    }
}

fn spawn_hud_button(
    panel: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    label: &str,
    action: HudAction,
    style: BtnStyle,
    text_color: Color,
) {
    panel
        .spawn((
            Button,
            action,
            style,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(42.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(style.bg),
            BorderColor::all(style.border),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font: fonts.bold.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(text_color),
            ));
        });
}

pub fn setup_hud(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    core: Res<crate::app_state::CoreGame>,
    ai_settings: Res<AiSettings>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            HudRoot,
        ))
        .with_children(|root| {
            // ---- Left sidebar ------------------------------------------
            root.spawn((
                Node {
                    width: Val::Px(264.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(22.0)),
                    row_gap: Val::Px(10.0),
                    border: UiRect::right(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(HAIRLINE),
            ))
            .with_children(|panel| {
                // Mode header.
                let mode_label = match core.mode {
                    crate::app_state::GameMode::LocalPvp => "本地双人对弈",
                    crate::app_state::GameMode::VsAi => "人机对战",
                    crate::app_state::GameMode::LanHost | crate::app_state::GameMode::LanJoin => {
                        "局域网对弈"
                    }
                    crate::app_state::GameMode::RelayHost
                    | crate::app_state::GameMode::RelayJoin => "互联网对弈",
                };
                panel.spawn((
                    Text::new(mode_label),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(GOLD_BRIGHT),
                ));
                // Sub chip: engine + difficulty (or mode description).
                let sub = match core.mode {
                    crate::app_state::GameMode::VsAi => format!(
                        "{} · {}",
                        engine_label(&ai_settings),
                        ai_settings.difficulty.label()
                    ),
                    crate::app_state::GameMode::LocalPvp => "同屏轮流走棋".to_string(),
                    _ => "与远方的朋友对弈".to_string(),
                };
                panel
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(7.0),
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    },))
                    .with_children(|chip| {
                        chip.spawn((
                            Node {
                                width: Val::Px(6.0),
                                height: Val::Px(6.0),
                                border_radius: BorderRadius::all(Val::Px(3.0)),
                                ..default()
                            },
                            BackgroundColor(if ai_settings.engine_path.is_some() {
                                JADE
                            } else {
                                TEXT_FAINT
                            }),
                        ));
                        chip.spawn((
                            Text::new(sub),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(TEXT_DIM),
                        ));
                    });

                // ---- Status card ----------------------------------------
                panel
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(6.0),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(CARD),
                        BorderColor::all(HAIRLINE),
                    ))
                    .with_children(|card| {
                        // Turn row: pulsing dot + headline.
                        card.spawn((Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(9.0),
                            ..default()
                        },))
                            .with_children(|row| {
                                row.spawn((
                                    Node {
                                        width: Val::Px(10.0),
                                        height: Val::Px(10.0),
                                        border_radius: BorderRadius::all(Val::Px(5.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.68, 0.26, 0.21, 0.9)),
                                    TurnIndicator,
                                ));
                                row.spawn((
                                    Text::new("…"),
                                    TextFont {
                                        font: fonts.bold.clone(),
                                        font_size: 18.0,
                                        ..default()
                                    },
                                    TextColor(TEXT),
                                    StatusText,
                                ));
                            });
                        card.spawn((
                            Text::new("AI 思考中…"),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.43, 0.78, 0.63, 0.0)),
                            AiThinkingText,
                        ));
                        card.spawn((
                            Text::new(""),
                            TextFont {
                                font: fonts.regular.clone(),
                                font_size: 12.5,
                                ..default()
                            },
                            TextColor(TEXT_DIM),
                            Node {
                                margin: UiRect::top(Val::Px(2.0)),
                                ..default()
                            },
                            StatusDetail,
                        ));
                    });

                // Slots for dynamic cards: captured pieces + analysis panel.
                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    CapturedSlot,
                ));
                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    AnalysisSlot,
                ));

                // Spacer pushes action buttons to the bottom.
                panel.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });

                // ---- Actions ---------------------------------------------
                spawn_hud_button(
                    panel,
                    &fonts,
                    "新对局",
                    HudAction::NewGame,
                    BtnStyle::secondary(),
                    TEXT,
                );
                if !core.mode.is_networked() {
                    spawn_hud_button(
                        panel,
                        &fonts,
                        "悔棋",
                        HudAction::Undo,
                        BtnStyle::secondary(),
                        TEXT,
                    );
                }
                if core.mode.is_networked() || core.mode == crate::app_state::GameMode::LocalPvp {
                    spawn_hud_button(
                        panel,
                        &fonts,
                        "求和",
                        HudAction::OfferDraw,
                        BtnStyle::secondary(),
                        TEXT,
                    );
                }
                if core.mode != crate::app_state::GameMode::LocalPvp {
                    spawn_hud_button(
                        panel,
                        &fonts,
                        "认输",
                        HudAction::Resign,
                        BtnStyle::danger(),
                        CINNABAR_HOVER,
                    );
                }
                // Divider + back-to-menu.
                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(HAIRLINE),
                ));
                spawn_hud_button(
                    panel,
                    &fonts,
                    "返回主菜单",
                    HudAction::BackToMenu,
                    BtnStyle::ghost(),
                    TEXT_DIM,
                );
            });
        });
}

#[allow(clippy::too_many_arguments)]
pub fn update_status(
    core: Res<CoreGame>,
    mut q: Query<&mut Text, With<StatusText>>,
    mut detail_q: Query<&mut Text, (With<StatusDetail>, Without<StatusText>)>,
    history_view: Res<crate::history_view::HistoryView>,
    settings: Res<crate::app_state::AiSettings>,
    ai_task: Res<AiTask>,
    time: Res<Time>,
    move_timer: Res<crate::clock_ui::MoveTimer>,
    clock_res: Res<crate::app_state::ClockResource>,
    auto_play: Res<crate::keyboard::AutoPlayState>,
    orient: Res<crate::app_state::BoardOrientation>,
    volume: Res<crate::sound::SoundVolume>,
    undo_count: Res<crate::app_state::UndoCount>,
) {
    let Ok(mut main_text) = q.single_mut() else {
        return;
    };
    let Ok(mut detail_text) = detail_q.single_mut() else {
        return;
    };

    // History view mode takes priority.
    if let Some(ply) = history_view.viewing_ply {
        let total = core.game.history_len();
        **main_text = format!("回看 · 第 {ply} / {total} 手");
        let mut d = String::new();
        if ply == 0 {
            d.push_str("起始局面\n");
        } else if ply <= total {
            if let Some(board_before) = core.game.board_at_ply(ply - 1) {
                let entry = &core.game.history()[ply - 1];
                let notation = chess_core::move_to_chinese(entry.mv(), &board_before);
                d.push_str(&format!("第{ply}手 · {notation}\n"));
            }
        }
        d.push_str("按 → 或 End 返回对局");
        **detail_text = d;
        return;
    }

    if core.peer_disconnected && !core.game.is_over() {
        **main_text = "对方已断开".to_string();
        **detail_text = "等待重连…".to_string();
        return;
    }
    if core.awaiting_peer {
        **main_text = "等待对手加入…".to_string();
        **detail_text = match core.mode {
            GameMode::RelayHost => match &core.room_code {
                Some(room) => format!("房间号 {room}"),
                None => "把房间号告诉好友".to_string(),
            },
            GameMode::LanHost => "在局域网内等待加入".to_string(),
            _ => "正在连接，请稍候…".to_string(),
        };
        return;
    }

    if let Some(result) = core.game.result() {
        match result {
            GameResult::Win { winner, reason } => {
                let side = match winner {
                    ChessColor::Red => "红方胜",
                    ChessColor::Black => "黑方胜",
                };
                let why = match reason {
                    WinReason::Checkmate => "将死",
                    WinReason::Stalemate => "困毙",
                    WinReason::Resignation => "认输",
                    WinReason::PerpetualCheck => "长将判负",
                    WinReason::Timeout => "超时判负",
                };
                **main_text = format!("{side} · {why}");
            }
            GameResult::Draw(reason) => {
                let desc = match reason {
                    chess_core::DrawReason::Agreement => "协议和棋",
                    chess_core::DrawReason::Repetition => "三次重复",
                    chess_core::DrawReason::NoCapture => "无吃子和棋",
                };
                **main_text = format!("和棋 · {desc}");
            }
        }
        **detail_text = "按 N 开始新对局".to_string();
        return;
    }

    // --- Live game ---------------------------------------------------------
    let side = match core.game.side_to_move() {
        ChessColor::Red => "红方",
        ChessColor::Black => "黑方",
    };
    **main_text = if ai_task.rx.is_some() {
        "对方思考中".to_string()
    } else if core.local_to_move() {
        "轮到你走棋".to_string()
    } else {
        "等待对手…".to_string()
    };

    let move_num = core.game.history_len() / 2 + 1;
    let elapsed = (time.elapsed_secs() - move_timer.started) as u32;
    let legal_count = core.game.legal_moves().len();
    let clock_str = match &clock_res.clock {
        Some(clock) => {
            let remaining = clock.remaining(core.game.side_to_move());
            let time_str = chess_core::GameClock::format_time(remaining);
            format!(" · 剩余 {time_str}")
        }
        None => String::new(),
    };
    let mut d =
        format!("{side}行棋 · 第{move_num}手 · {legal_count}步可行 · 本手{elapsed}s{clock_str}");

    if *orient != crate::app_state::BoardOrientation::Red {
        d.push_str(" · 已翻转");
    }
    if volume.level == crate::sound::VolumeLevel::Mute {
        d.push_str(" · 静音");
    }
    // Last move in Chinese notation.
    if core.game.history_len() > 0 {
        let ply = core.game.history_len();
        let entry = &core.game.history()[ply - 1];
        let mv = entry.mv();
        if let Some(board_before) = core.game.board_at_ply(ply - 1) {
            let notation = chess_core::move_to_chinese(mv, &board_before);
            d.push_str(&format!("\n上一步 · {notation}"));
        }
    }
    if core.draw_offer_from_peer {
        d.push_str("\n对方提议和棋");
    }
    // Material count per side with advantage indicator.
    let (mut red_mat, mut black_mat) = (0i32, 0i32);
    let (mut red_count, mut black_count) = (0u32, 0u32);
    for (_, piece) in core.game.board().pieces() {
        let val = crate::app_state::piece_value(piece.kind);
        match piece.color {
            chess_core::Color::Red => {
                red_mat += val;
                red_count += 1;
            }
            chess_core::Color::Black => {
                black_mat += val;
                black_count += 1;
            }
        }
    }
    let advantage = red_mat - black_mat;
    let adv_str = if advantage > 0 {
        format!(" (红+{advantage})")
    } else if advantage < 0 {
        format!(" (黑+{})", -advantage)
    } else {
        String::new()
    };
    let phase = if core.game.history_len() <= 6 {
        "开局"
    } else if red_count + black_count > 18 {
        "中局"
    } else {
        "残局"
    };
    d.push_str(&format!(
        "\n红{red_count} 黑{black_count}{adv_str} · {phase}"
    ));
    // Warnings.
    let stm = core.game.side_to_move();
    if core.game.board().is_in_check(stm) {
        d.push_str("\n⚠ 将军！请应将");
    }
    let hm = core.game.halfmove_clock();
    if hm > 80 {
        let since = core.game.history_len() as u32 - hm;
        d.push_str(&format!(
            "\n⚠ 无吃子 {hm}/120（还剩{}手，第{since}手起）",
            120 - hm
        ));
    }
    if core.game.repetition_count() == 2 {
        d.push_str("\n⚠ 二次重复局面（三次判和）");
    }
    if auto_play.active {
        let interval = auto_play.timer.duration().as_secs_f32();
        d.push_str(&format!("\n▶ 自动播放中 ({interval:.1}s)"));
    }
    if undo_count.0 > 0 {
        d.push_str(&format!("\n已悔棋 {} 次", undo_count.0));
    }
    let _ = settings; // difficulty shown in the header chip instead
    **detail_text = d;
}

#[allow(clippy::too_many_arguments)]
pub fn hud_interaction(
    mut interactions: Query<
        (
            &Interaction,
            &HudAction,
            &BtnStyle,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
    mut commands: Commands,
    fonts: Res<UiFonts>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<RenderDirty>,
    mut selection: ResMut<Selection>,
    mut ai_task: ResMut<AiTask>,
    mut next: ResMut<NextState<AppState>>,
    net: Option<Res<NetLink>>,
    mut confirm_resign: ResMut<ConfirmResignVisible>,
    time: Res<Time>,
    mut menu_pending: ResMut<crate::app_state::BackToMenuPending>,
    mut draw_pending: ResMut<crate::app_state::DrawOfferPending>,
) {
    for (interaction, action, style, mut bg, mut border) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match action {
                HudAction::NewGame => {
                    let abandoned = core.game.history_len();
                    core.restart();
                    crate::moves::GAME_RESTARTED.store(true, std::sync::atomic::Ordering::Relaxed);
                    selection.from = None;
                    ai_task.rx = None;
                    dirty.0 = true;
                    // Note: auto-play state is reset implicitly because
                    // history_view is cleared and auto_play_history detects
                    // !history_view.is_viewing() → stops auto-play.
                    // Hosts of a networked game must broadcast the reset so
                    // the connected guest also restarts (otherwise the two
                    // sides desync immediately).
                    let mode_label = match core.mode {
                        GameMode::VsAi => "人机",
                        GameMode::LocalPvp => "双人",
                        _ => "联机",
                    };
                    let abandon_hint = if abandoned > 0 {
                        format!(" (弃{abandoned}手)")
                    } else {
                        String::new()
                    };
                    let msg = format!("新对局 · {mode_label}{abandon_hint}");
                    crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                    if core.mode.is_net_host() {
                        if let Some(net) = &net {
                            let _ = net.out.send(NetCommand::Sync(Box::new(core.game.clone())));
                        }
                    }
                }
                HudAction::Resign => {
                    if core.game.history_len() < 3 {
                        let side = match core.game.side_to_move() {
                            ChessColor::Red => "红方",
                            ChessColor::Black => "黑方",
                        };
                        let msg = format!(
                            "至少走3手才能认输 (当前{}手, {}行棋)",
                            core.game.history_len(),
                            side
                        );
                        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                    } else {
                        confirm_resign.0 = true;
                    }
                }
                HudAction::OfferDraw => {
                    if let Some(net) = &net {
                        if core.draw_offer_from_peer {
                            let _ = net.out.send(NetCommand::DrawResponse(true));
                            core.draw_offer_from_peer = false;
                            core.game.agree_draw();
                            dirty.0 = true;
                        } else {
                            let _ = net.out.send(NetCommand::DrawOffer);
                        }
                    } else {
                        // LocalPvp: require double-press to avoid accidental draw.
                        if let Some(ts) = draw_pending.0 {
                            if time.elapsed_secs() - ts < 2.0 {
                                core.game.agree_draw();
                                dirty.0 = true;
                                draw_pending.0 = None;
                            } else {
                                draw_pending.0 = Some(time.elapsed_secs());
                                crate::toast::spawn_toast(
                                    &mut commands,
                                    &fonts,
                                    &format!(
                                        "再次点击确认和棋 (已走{}手)",
                                        core.game.history_len()
                                    ),
                                );
                            }
                        } else {
                            draw_pending.0 = Some(time.elapsed_secs());
                            crate::toast::spawn_toast(
                                &mut commands,
                                &fonts,
                                &format!("再次点击确认和棋 (已走{}手)", core.game.history_len()),
                            );
                        }
                    }
                }
                HudAction::Undo => {
                    // Networked games do not support undo (would require
                    // peer negotiation which is out of scope).
                    if core.mode.is_networked() || core.game.history_len() == 0 {
                        continue;
                    }
                    // Cancel any in-flight AI task so it does not apply a
                    // stale move after we rewind.
                    ai_task.rx = None;
                    // In VsAi, undo two plies (AI + human) so the player
                    // gets to redo their own move. In LocalPvp, undo one.
                    if core.mode == GameMode::VsAi {
                        core.game.undo(); // undo AI's move
                        core.game.undo(); // undo player's move
                    } else {
                        core.game.undo();
                    }
                    selection.from = None;
                    core.last_move = None;
                    dirty.0 = true;
                    // Toast feedback + trigger undo sound (same as keyboard).
                    let remaining = core.game.history_len();
                    let round = remaining / 2 + 1;
                    let side_label = match core.game.side_to_move() {
                        chess_core::Color::Red => "红",
                        chess_core::Color::Black => "黑",
                    };
                    let msg = format!("悔棋 ({side_label}, 第{round}回合, 还剩{remaining}手)");
                    crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                    crate::moves::UNDO_PERFORMED.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                HudAction::BackToMenu => {
                    if core.game.is_over() || core.game.history_len() == 0 {
                        next.set(AppState::Menu);
                    } else if let Some(ts) = menu_pending.0 {
                        if time.elapsed_secs() - ts < 2.0 {
                            next.set(AppState::Menu);
                            menu_pending.0 = None;
                        } else {
                            menu_pending.0 = Some(time.elapsed_secs());
                            let msg =
                                format!("再次点击返回菜单 (已走{}手)", core.game.history_len());
                            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                        }
                    } else {
                        menu_pending.0 = Some(time.elapsed_secs());
                        let msg = format!("再次点击返回菜单 (已走{}手)", core.game.history_len());
                        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
                    }
                }
            },
            Interaction::Hovered => {
                *bg = BackgroundColor(style.bg_hover);
                *border = BorderColor::all(style.border_hover);
            }
            Interaction::None => {
                *bg = BackgroundColor(style.bg);
                *border = BorderColor::all(style.border);
            }
        }
    }
}

pub fn teardown_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// ---------------------------------------------------------------------------
// Draw offer Accept / Reject inline panel
// ---------------------------------------------------------------------------

/// Marker for the draw offer notification panel.
#[derive(Component)]
pub struct DrawOfferPanel;

/// Action for the draw offer buttons.
#[derive(Component, Clone, Copy)]
pub enum DrawOfferAction {
    Accept,
    Reject,
}

/// Spawn or despawn the draw offer panel based on `draw_offer_from_peer`.
pub fn manage_draw_offer(
    mut commands: Commands,
    core: Res<crate::app_state::CoreGame>,
    fonts: Res<crate::app_state::UiFonts>,
    panel_q: Query<Entity, With<DrawOfferPanel>>,
) {
    let panel_exists = !panel_q.is_empty();

    if core.draw_offer_from_peer && !panel_exists {
        // Spawn the accept/reject panel.
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(14.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-190.0)),
                    width: Val::Px(380.0),
                    padding: UiRect::axes(Val::Px(18.0), Val::Px(12.0)),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(14.0)),
                    ..default()
                },
                BackgroundColor(CARD),
                BorderColor::all(HAIRLINE_STRONG),
                BoxShadow::new(
                    Color::srgba(0.0, 0.0, 0.0, 0.5),
                    Val::Px(0.0),
                    Val::Px(10.0),
                    Val::Px(8.0),
                    Val::Px(28.0),
                ),
                GlobalZIndex(80),
                DrawOfferPanel,
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("对方请求和棋"),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 17.0,
                        ..default()
                    },
                    TextColor(TEXT),
                ));
                // Accept button.
                panel
                    .spawn((
                        Button,
                        DrawOfferAction::Accept,
                        Node {
                            width: Val::Px(72.0),
                            height: Val::Px(34.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(9.0)),
                            ..default()
                        },
                        BackgroundColor(JADE_FILL),
                        BorderColor::all(JADE_BORDER),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("接受"),
                            TextFont {
                                font: fonts.bold.clone(),
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(JADE_BRIGHT),
                        ));
                    });
                // Reject button.
                panel
                    .spawn((
                        Button,
                        DrawOfferAction::Reject,
                        Node {
                            width: Val::Px(72.0),
                            height: Val::Px(34.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(9.0)),
                            ..default()
                        },
                        BackgroundColor(CINNABAR_FILL),
                        BorderColor::all(CINNABAR_BORDER),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("拒绝"),
                            TextFont {
                                font: fonts.bold.clone(),
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(CINNABAR_HOVER),
                        ));
                    });
            });
    } else if !core.draw_offer_from_peer && panel_exists {
        // Despawn the panel when the offer is no longer pending.
        for e in &panel_q {
            commands.entity(e).despawn();
        }
    }
}

/// Handle clicks on the draw offer Accept/Reject buttons.
pub fn draw_offer_interaction(
    mut interactions: Query<
        (&Interaction, &DrawOfferAction, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut core: ResMut<crate::app_state::CoreGame>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    net: Option<Res<crate::net_bridge::NetLink>>,
    mut commands: Commands,
    panel_q: Query<Entity, With<DrawOfferPanel>>,
) {
    for (interaction, action, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                match action {
                    DrawOfferAction::Accept => {
                        if let Some(net) = &net {
                            let _ = net
                                .out
                                .send(crate::net_bridge::NetCommand::DrawResponse(true));
                        }
                        core.draw_offer_from_peer = false;
                        core.game.agree_draw();
                        dirty.0 = true;
                    }
                    DrawOfferAction::Reject => {
                        if let Some(net) = &net {
                            let _ = net
                                .out
                                .send(crate::net_bridge::NetCommand::DrawResponse(false));
                        }
                        core.draw_offer_from_peer = false;
                    }
                }
                // Despawn the panel.
                for e in &panel_q {
                    commands.entity(e).despawn();
                }
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(match action {
                    DrawOfferAction::Accept => JADE_FILL_HOVER,
                    DrawOfferAction::Reject => Color::srgba(0.678, 0.263, 0.208, 0.30),
                });
            }
            Interaction::None => {
                *bg = BackgroundColor(match action {
                    DrawOfferAction::Accept => JADE_FILL,
                    DrawOfferAction::Reject => CINNABAR_FILL,
                });
            }
        }
    }
}

/// Clean up draw offer panel on state exit.
pub fn teardown_draw_offer(mut commands: Commands, q: Query<Entity, With<DrawOfferPanel>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// Pulse the AI thinking text visibility.
pub fn pulse_ai_thinking(
    time: Res<Time>,
    ai_task: Res<crate::ai_bridge::AiTask>,
    mut query: Query<&mut TextColor, With<AiThinkingText>>,
) {
    let is_thinking = ai_task.rx.is_some();
    for mut tc in &mut query {
        if is_thinking {
            let alpha = 0.4 + 0.6 * (time.elapsed_secs() * 3.0).sin().abs();
            *tc = TextColor(Color::srgba(0.43, 0.78, 0.63, alpha));
        } else {
            *tc = TextColor(Color::srgba(0.43, 0.78, 0.63, 0.0));
        }
    }
}

/// Pulse the turn indicator dot based on whose turn it is.
pub fn pulse_turn_indicator(
    time: Res<Time>,
    core: Res<CoreGame>,
    mut query: Query<&mut BackgroundColor, With<TurnIndicator>>,
) {
    for mut bg in &mut query {
        if core.game.is_over() {
            *bg = BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.0));
            return;
        }
        let base_color = match core.game.side_to_move() {
            chess_core::Color::Red => (0.68, 0.26, 0.21),
            chess_core::Color::Black => (0.55, 0.52, 0.46),
        };
        let alpha = if core.local_to_move() {
            let t = time.elapsed_secs();
            0.5 + 0.5 * (t * 4.0).sin()
        } else {
            0.75
        };
        *bg = BackgroundColor(Color::srgba(
            base_color.0,
            base_color.1,
            base_color.2,
            alpha,
        ));
    }
}

/// Slow breathing alpha pulse on the menu subtitle text.
pub fn animate_menu_subtitle(
    time: Res<Time>,
    mut query: Query<&mut TextColor, With<MenuSubtitle>>,
) {
    let t = time.elapsed_secs();
    let alpha = 0.55 + 0.45 * (t * 0.8).sin();
    for mut color in &mut query {
        let base = TEXT_DIM.to_srgba();
        *color = TextColor(Color::srgba(base.red, base.green, base.blue, alpha));
    }
}

/// Keyboard navigation for the menu: Up/Down to select, Enter to activate.
#[allow(clippy::too_many_arguments)]
pub fn menu_keyboard_nav(
    keys: Res<ButtonInput<KeyCode>>,
    mut sel: ResMut<MenuSelection>,
    mut buttons: Query<(
        &MenuButton,
        &BtnStyle,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut core: ResMut<CoreGame>,
    mut next: ResMut<NextState<AppState>>,
    mut dialog: ResMut<crate::lan_dialog::LanDialog>,
    mut diff_state: ResMut<crate::difficulty_dialog::DifficultyDialogState>,
) {
    const BUTTON_COUNT: usize = 4;

    if keys.just_pressed(KeyCode::ArrowDown) {
        sel.0 = (sel.0 + 1) % BUTTON_COUNT;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        sel.0 = if sel.0 == 0 {
            BUTTON_COUNT - 1
        } else {
            sel.0 - 1
        };
    }

    // Highlight the selected button.
    for (btn, style, mut bg, mut border) in &mut buttons {
        let idx = MENU_MODES.iter().position(|m| *m == btn.0);
        let active = idx == Some(sel.0);
        set_menu_btn_style(btn.0, *style, active, &mut bg, &mut border);
    }

    // Enter activates the selected button.
    if keys.just_pressed(KeyCode::Enter) {
        let mode = MENU_MODES[sel.0];
        match mode {
            GameMode::LanHost => dialog.open_for(true),
            GameMode::LanJoin => dialog.open_for(false),
            GameMode::VsAi => {
                diff_state.open = true;
            }
            other => {
                core.restart();
                core.mode = other;
                core.local_color = chess_core::Color::Red;
                next.set(AppState::InGame);
            }
        }
    }
}
