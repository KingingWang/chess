//! `bevy_ui` menus and the in-game HUD (status text + action buttons).

use bevy::prelude::*;
use chess_core::{Color as ChessColor, GameResult, WinReason};

use crate::ai_bridge::AiTask;
use crate::app_state::{AppState, CoreGame, GameMode, Selection, UiFonts};
use crate::board_view::RenderDirty;
use crate::confirm_resign::ConfirmResignVisible;
use crate::lan_dialog::LanDialog;
use crate::net_bridge::{NetCommand, NetLink};

// --- 国风 palette ---------------------------------------------------------
const BACKDROP: Color = Color::srgb(0.10, 0.08, 0.09); // deep lacquer behind card
const CARD: Color = Color::srgb(0.16, 0.13, 0.13);
const CARD_BORDER: Color = Color::srgb(0.62, 0.45, 0.22); // antique gold
const BTN: Color = Color::srgb(0.55, 0.16, 0.13); // cinnabar red
const BTN_HOVER: Color = Color::srgb(0.72, 0.22, 0.17);
const BTN_BORDER: Color = Color::srgb(0.78, 0.62, 0.32);
const TITLE: Color = Color::srgb(0.93, 0.84, 0.55); // gold ink
const SUBTITLE: Color = Color::srgb(0.78, 0.70, 0.55);
const TEXT: Color = Color::srgb(0.96, 0.93, 0.86);
const PANEL_BG: Color = Color::srgba(0.13, 0.10, 0.10, 0.92);

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

pub fn setup_menu(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    last_result: Res<crate::app_state::LastGameResult>,
    session_stats: Res<crate::app_state::SessionStats>,
    session_play_time: Res<crate::app_state::SessionPlayTime>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BACKDROP),
            MenuRoot,
        ))
        .with_children(|root| {
            // Centered ornamented card.
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(56.0), Val::Px(44.0)),
                    row_gap: Val::Px(14.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(18.0)),
                    ..default()
                },
                BackgroundColor(CARD),
                BorderColor::all(CARD_BORDER),
                BoxShadow::new(
                    Color::srgba(0.0, 0.0, 0.0, 0.55),
                    Val::Px(0.0),
                    Val::Px(10.0),
                    Val::Px(6.0),
                    Val::Px(34.0),
                ),
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new("中 国 象 棋"),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(TITLE),
                ));
                card.spawn((
                    Text::new({
                        let subtitles = [
                            "— 国风对弈 · 楚汉相争 —",
                            "— 棋逢对手 · 将帅之战 —",
                            "— 运筹帷幄 · 决胜千里 —",
                            "— 象棋风云 · 对弈人生 —",
                            "— 纵横九宫 · 驰骋楚河 —",
                            "— 妙手回春 · 攻守兼备 —",
                            "— 胸有成竹 · 步步为营 —",
                            "— 以棋会友 · 乐在其中 —",
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
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(SUBTITLE),
                    Node {
                        margin: UiRect::bottom(Val::Px(22.0)),
                        ..default()
                    },
                    MenuSubtitle,
                ));
                // Last game result summary (if any).
                if let Some(ref result) = last_result.0 {
                    let (emoji, text, color) = match result {
                        chess_core::GameResult::Win { winner, .. } => {
                            if *winner == chess_core::Color::Red {
                                ("「胜」", "红方胜", Color::srgb(0.85, 0.20, 0.15))
                            } else {
                                ("「胜」", "黑方胜", Color::srgb(0.90, 0.87, 0.80))
                            }
                        }
                        chess_core::GameResult::Draw(reason) => {
                            let desc = match reason {
                                chess_core::DrawReason::Agreement => "协议和棋",
                                chess_core::DrawReason::Repetition => "三次重复",
                                chess_core::DrawReason::NoCapture => "无吃子和棋",
                            };
                            ("「和」", desc, Color::srgb(0.80, 0.70, 0.40))
                        }
                    };
                    card.spawn((
                        Text::new(format!("{} 上局: {}", emoji, text)),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(color),
                        Node {
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        },
                    ));
                }
                // Session W/L/D stats.
                if session_stats.total() > 0 {
                    let win_pct = session_stats.wins * 100 / session_stats.total();
                    let stats_text = format!(
                        "「绩」 {}胜 {}负 {}和 ({}%)",
                        session_stats.wins, session_stats.losses, session_stats.draws, win_pct
                    );
                    card.spawn((
                        Text::new(stats_text),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.65, 0.60, 0.50)),
                        Node {
                            margin: UiRect::bottom(Val::Px(6.0)),
                            ..default()
                        },
                    ));
                }
                // Session total play time.
                if session_play_time.0 > 0.0 {
                    let total_secs = session_play_time.0 as u32;
                    let mins = total_secs / 60;
                    let secs = total_secs % 60;
                    let time_str = if mins >= 60 {
                        format!("「时」 本次共对弈 {}h{}分{}秒", mins / 60, mins % 60, secs)
                    } else if mins > 0 {
                        format!("「时」 本次共对弈 {}分{}秒", mins, secs)
                    } else {
                        format!("「时」 本次共对弈 {}秒", secs)
                    };
                    card.spawn((
                        Text::new(time_str),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.55, 0.50, 0.42)),
                        Node {
                            margin: UiRect::bottom(Val::Px(4.0)),
                            ..default()
                        },
                    ));
                }
                for (idx, (label, mode)) in [
                    ("本地双人对弈", GameMode::LocalPvp),
                    ("人机对战", GameMode::VsAi),
                    ("局域网创建房间", GameMode::LanHost),
                    ("局域网加入房间", GameMode::LanJoin),
                    ("互联网创建房间", GameMode::RelayHost),
                    ("互联网加入房间", GameMode::RelayJoin),
                ]
                .into_iter()
                .enumerate()
                {
                    // Divider between local modes (0-1) and network modes (2-5).
                    if idx == 2 {
                        card.spawn((
                            Node {
                                width: Val::Px(180.0),
                                height: Val::Px(1.0),
                                margin: UiRect::axes(Val::ZERO, Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.62, 0.45, 0.22, 0.4)),
                        ));
                    }
                    card.spawn((
                        Button,
                        MenuButton(mode),
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(56.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.5)),
                            border_radius: BorderRadius::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(BTN),
                        BorderColor::all(BTN_BORDER),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new(label),
                            TextFont {
                                font: fonts.bold.clone(),
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(TEXT),
                        ));
                    });
                }
                // Saved game count.
                let save_count = std::fs::read_dir(crate::settings::save_dir())
                    .ok()
                    .map(|rd| {
                        rd.filter_map(|e| e.ok())
                            .filter(|e| e.path().extension().is_some_and(|ext| ext == "pgn"))
                            .count()
                    })
                    .unwrap_or(0);
                if save_count > 0 {
                    card.spawn((
                        Text::new(format!("「档」 {}个存档 · Ctrl+O 加载", save_count)),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.55, 0.50, 0.42)),
                        Node {
                            margin: UiRect::top(Val::Px(10.0)),
                            ..default()
                        },
                    ));
                }
                // Version footer.
                card.spawn((
                    Text::new(format!("v{} · Bevy 0.18 · 2026", env!("CARGO_PKG_VERSION"))),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.55, 0.50, 0.42, 0.6)),
                    Node {
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    },
                ));
                // Keyboard navigation hint.
                card.spawn((
                    Text::new("↑↓ 选择 · Enter 确认"),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.50, 0.45, 0.38)),
                    Node {
                        margin: UiRect::top(Val::Px(2.0)),
                        ..default()
                    },
                ));
            });
        });
}

pub fn menu_interaction(
    mut interactions: Query<
        (&Interaction, &MenuButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut core: ResMut<CoreGame>,
    mut next: ResMut<NextState<AppState>>,
    mut dialog: ResMut<LanDialog>,
    mut diff_state: ResMut<crate::difficulty_dialog::DifficultyDialogState>,
) {
    for (interaction, btn, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match btn.0 {
                // LAN modes open the setup dialog (port / IP / password) first.
                GameMode::LanHost => dialog.open_for(true),
                GameMode::LanJoin => dialog.open_for(false),
                // Relay modes also open the dialog.
                GameMode::RelayHost | GameMode::RelayJoin => {
                    dialog.open_for(btn.0 == GameMode::RelayHost);
                }
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
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN),
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

#[derive(Component)]
pub struct StatusText;

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

pub fn setup_hud(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    core: Res<crate::app_state::CoreGame>,
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
            // Left side panel.
            root.spawn((
                Node {
                    width: Val::Px(232.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(18.0)),
                    row_gap: Val::Px(12.0),
                    border: UiRect::right(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderColor::all(CARD_BORDER),
            ))
            .with_children(|panel| {
                let mode_label = match core.mode {
                    crate::app_state::GameMode::LocalPvp => "「友」 本地对弈",
                    crate::app_state::GameMode::VsAi => "「机」 人机对战",
                    crate::app_state::GameMode::LanHost | crate::app_state::GameMode::LanJoin => {
                        "「网」 局域网对弈"
                    }
                    crate::app_state::GameMode::RelayHost
                    | crate::app_state::GameMode::RelayJoin => "「联」 互联网对弈",
                };
                panel.spawn((
                    Text::new(mode_label),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 26.0,
                        ..default()
                    },
                    TextColor(TITLE),
                    Node {
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    },
                ));
                panel.spawn((
                    Text::new("..."),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(TEXT),
                    Node {
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..default()
                    },
                    StatusText,
                ));
                // Turn indicator dot.
                panel.spawn((
                    Node {
                        width: Val::Px(14.0),
                        height: Val::Px(14.0),
                        margin: UiRect::bottom(Val::Px(4.0)),
                        border_radius: BorderRadius::all(Val::Px(7.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.9, 0.2, 0.1, 0.8)),
                    TurnIndicator,
                ));
                // AI thinking indicator (hidden by default).
                panel.spawn((
                    Text::new("AI思考中…"),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.93, 0.84, 0.55, 0.0)),
                    Node {
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    },
                    AiThinkingText,
                ));
                let mut buttons: Vec<(&str, HudAction)> = vec![("新 对 局", HudAction::NewGame)];
                // Resign visible in VsAi and networked modes (not LocalPvp).
                if core.mode != crate::app_state::GameMode::LocalPvp {
                    buttons.push(("认 输", HudAction::Resign));
                }
                // Draw offer visible in networked and LocalPvp modes.
                if core.mode.is_networked() || core.mode == crate::app_state::GameMode::LocalPvp {
                    buttons.push(("求 和", HudAction::OfferDraw));
                }
                // Undo not in networked (already blocked in handler, but hide button).
                if !core.mode.is_networked() {
                    buttons.push(("悔 棋", HudAction::Undo));
                }
                buttons.push(("返回主菜单", HudAction::BackToMenu));
                for (label, action) in buttons {
                    panel
                        .spawn((
                            Button,
                            action,
                            Node {
                                width: Val::Px(196.0),
                                height: Val::Px(44.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.5)),
                                border_radius: BorderRadius::all(Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(BTN),
                            BorderColor::all(BTN_BORDER),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(label),
                                TextFont {
                                    font: fonts.bold.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(TEXT),
                            ));
                        });
                }
            });
        });
}

#[allow(clippy::too_many_arguments)]
pub fn update_status(
    core: Res<CoreGame>,
    mut q: Query<&mut Text, With<StatusText>>,
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
    let Ok(mut text) = q.single_mut() else {
        return;
    };

    // History view mode takes priority.
    if let Some(ply) = history_view.viewing_ply {
        let total = core.game.history_len();
        let mut s = format!("查看模式 (第{}手 / 共{}手)", ply, total);
        if ply == 0 {
            s.push_str(
                "
起始局面",
            );
        }
        // Show the notation of the move at the current ply.
        if ply > 0 && ply <= total {
            if let Some(board_before) = core.game.board_at_ply(ply - 1) {
                let entry = &core.game.history()[ply - 1];
                let notation = chess_core::move_to_chinese(entry.mv(), &board_before);
                s.push_str(&format!("\n第{}手: {}", ply, notation));
            }
        }
        s.push_str("\n按→或End返回");
        **text = s;
        return;
    }

    if core.peer_disconnected && !core.game.is_over() {
        **text = "对方已断开\n等待重连…".to_string();
        return;
    }
    if core.awaiting_peer {
        **text = match core.mode {
            GameMode::RelayHost => match &core.room_code {
                Some(room) => format!("房间号 {room}\n等待对手加入…"),
                None => "等待对手加入…".to_string(),
            },
            GameMode::LanHost => "等待对手加入…".to_string(),
            GameMode::LanJoin | GameMode::RelayJoin => "正在连接，请稍候…".to_string(),
            _ => "正在连接，请稍候…".to_string(),
        };
        return;
    }

    // Mode header.
    let mode_str = match core.mode {
        GameMode::LocalPvp => "「友」 本地双人".to_string(),
        GameMode::VsAi => format!("「机」 人机对战 · {}", settings.difficulty.label()),
        GameMode::LanHost | GameMode::LanJoin => "「网」 局域网对战".to_string(),
        GameMode::RelayHost | GameMode::RelayJoin => match &core.room_code {
            Some(room) => format!("「联」 联网对战 · 房间 {room}"),
            None => "「联」 联网对战".to_string(),
        },
    };
    let mode_str = if *orient != crate::app_state::BoardOrientation::Red {
        format!("{} ⇅", mode_str)
    } else {
        mode_str
    };
    let mode_str = if volume.level == crate::sound::VolumeLevel::Mute {
        format!("{} 「静」", mode_str)
    } else {
        mode_str
    };

    let status = if let Some(result) = core.game.result() {
        match result {
            GameResult::Win { winner, reason } => {
                let side = match winner {
                    ChessColor::Red => "红方",
                    ChessColor::Black => "黑方",
                };
                let why = match reason {
                    WinReason::Checkmate => "将死",
                    WinReason::Stalemate => "困毙",
                    WinReason::Resignation => "认输",
                    WinReason::PerpetualCheck => "长将判负",
                    WinReason::Timeout => "超时判负",
                };
                format!("{mode_str}\n{side}胜 （{why}）")
            }
            GameResult::Draw(reason) => {
                let (emoji, desc) = match reason {
                    chess_core::DrawReason::Agreement => ("「和」", "协议和棋"),
                    chess_core::DrawReason::Repetition => ("♻", "三次重复"),
                    chess_core::DrawReason::NoCapture => ("「时」", "无吃子和棋"),
                };
                format!("{mode_str}\n{emoji} 和棋 （{desc}）")
            }
        }
    } else {
        let side = match core.game.side_to_move() {
            ChessColor::Red => "红方",
            ChessColor::Black => "黑方",
        };
        let turn = if ai_task.rx.is_some() {
            "AI思考中…"
        } else if core.local_to_move() {
            "轮到你走棋"
        } else {
            "等待对手…"
        };
        let move_num = core.game.history_len() / 2 + 1;
        let elapsed = (time.elapsed_secs() - move_timer.started) as u32;
        let legal_count = core.game.legal_moves().len();
        let clock_str = match &clock_res.clock {
            Some(clock) => {
                let remaining = clock.remaining(core.game.side_to_move());
                let time_str = chess_core::GameClock::format_time(remaining);
                format!(" · 「时」{time_str}")
            }
            None => String::new(),
        };
        let mut s = format!(
            "{mode_str}\n{side}行棋 · 第{move_num}手 · 「时」{elapsed}s{clock_str}\n{turn} (可走{legal_count}步)"
        );
        // Show last move in Chinese notation.
        if core.game.history_len() > 0 {
            let ply = core.game.history_len();
            let entry = &core.game.history()[ply - 1];
            let mv = entry.mv();
            if let Some(board_before) = core.game.board_at_ply(ply - 1) {
                let notation = chess_core::move_to_chinese(mv, &board_before);
                s.push_str(&format!("\n上一步: {}", notation));
            }
        }
        if core.draw_offer_from_peer {
            s.push_str("\n对方提议和棋");
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
            format!(" (红+{})", advantage)
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
        s.push_str(&format!(
            "\n红{} 黑{}{} · {}",
            red_count, black_count, adv_str, phase
        ));
        // Check warning.
        if !core.game.is_over() {
            let stm = core.game.side_to_move();
            if core.game.board().is_in_check(stm) {
                s.push_str("\n⚠ 将军! 请应将!");
            }
            // No-capture draw countdown (120 half-moves = 60 full rounds).
            let hm = core.game.halfmove_clock();
            if hm > 80 {
                let since = core.game.history_len() as u32 - hm;
                s.push_str(&format!(
                    "\n⚠ 无吃子: {}/120 (还剩{}手, 第{}手起)",
                    hm,
                    120 - hm,
                    since
                ));
            }
            // Repetition warning (threefold = auto-draw).
            if core.game.repetition_count() == 2 {
                s.push_str("\n⚠ 二次重复局面! (三次判和)");
            }
        }
        if auto_play.active {
            let interval = auto_play.timer.duration().as_secs_f32();
            s.push_str(&format!("\n▶ 自动播放中 ({:.1}s)", interval));
        }
        if undo_count.0 > 0 {
            s.push_str(&format!("\n「回」 悔棋 {}次", undo_count.0));
        }
        s
    };
    **text = status;
}
#[allow(clippy::too_many_arguments)]
pub fn hud_interaction(
    mut interactions: Query<(&Interaction, &HudAction, &mut BackgroundColor), Changed<Interaction>>,
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
    for (interaction, action, mut bg) in &mut interactions {
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
                        format!(" (弃{}手)", abandoned)
                    } else {
                        String::new()
                    };
                    let msg = format!("「换」 新对局 · {}{}", mode_label, abandon_hint);
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
                    let msg = format!(
                        "悔棋 ({}, 第{}回合, 还剩{}手)",
                        side_label, round, remaining
                    );
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
            Interaction::Hovered => *bg = BackgroundColor(BTN_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN),
        }
    }
}

pub fn teardown_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<NetLink>();
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
                    top: Val::Px(8.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-180.0)),
                    width: Val::Px(360.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: BorderRadius::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.16, 0.13, 0.12)),
                BorderColor::all(CARD_BORDER),
                GlobalZIndex(80),
                DrawOfferPanel,
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("对方请求和棋"),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(TITLE),
                ));
                // Accept button.
                panel
                    .spawn((
                        Button,
                        DrawOfferAction::Accept,
                        Node {
                            width: Val::Px(70.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.18, 0.55, 0.25)),
                        BorderColor::all(Color::srgb(0.30, 0.65, 0.35)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("接受"),
                            TextFont {
                                font: fonts.bold.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(TEXT),
                        ));
                    });
                // Reject button.
                panel
                    .spawn((
                        Button,
                        DrawOfferAction::Reject,
                        Node {
                            width: Val::Px(70.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.55, 0.16, 0.13)),
                        BorderColor::all(Color::srgb(0.70, 0.30, 0.25)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("拒绝"),
                            TextFont {
                                font: fonts.bold.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(TEXT),
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
                let hover_color = match action {
                    DrawOfferAction::Accept => Color::srgb(0.25, 0.65, 0.32),
                    DrawOfferAction::Reject => Color::srgb(0.72, 0.22, 0.17),
                };
                *bg = BackgroundColor(hover_color);
            }
            Interaction::None => {
                let normal_color = match action {
                    DrawOfferAction::Accept => Color::srgb(0.18, 0.55, 0.25),
                    DrawOfferAction::Reject => Color::srgb(0.55, 0.16, 0.13),
                };
                *bg = BackgroundColor(normal_color);
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
            *tc = TextColor(Color::srgba(0.93, 0.84, 0.55, alpha));
        } else {
            *tc = TextColor(Color::srgba(0.93, 0.84, 0.55, 0.0));
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
            chess_core::Color::Red => (0.9, 0.2, 0.1),
            chess_core::Color::Black => (0.15, 0.15, 0.15),
        };
        let alpha = if core.local_to_move() {
            let t = time.elapsed_secs();
            0.5 + 0.5 * (t * 4.0).sin()
        } else {
            0.6
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
    let alpha = 0.5 + 0.5 * (t * 0.8).sin();
    for mut color in &mut query {
        let base = SUBTITLE.to_srgba();
        *color = TextColor(Color::srgba(base.red, base.green, base.blue, alpha));
    }
}

/// Keyboard navigation for the menu: Up/Down to select, Enter to activate.
#[allow(clippy::too_many_arguments)]
pub fn menu_keyboard_nav(
    keys: Res<ButtonInput<KeyCode>>,
    mut sel: ResMut<MenuSelection>,
    mut buttons: Query<(&MenuButton, &mut BackgroundColor)>,
    mut core: ResMut<CoreGame>,
    mut next: ResMut<NextState<AppState>>,
    mut dialog: ResMut<crate::lan_dialog::LanDialog>,
    mut diff_state: ResMut<crate::difficulty_dialog::DifficultyDialogState>,
) {
    const BUTTON_COUNT: usize = 6;

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
    let modes = [
        GameMode::LocalPvp,
        GameMode::VsAi,
        GameMode::LanHost,
        GameMode::LanJoin,
        GameMode::RelayHost,
        GameMode::RelayJoin,
    ];
    for (btn, mut bg) in &mut buttons {
        let idx = modes.iter().position(|m| *m == btn.0);
        if idx == Some(sel.0) {
            *bg = BackgroundColor(BTN_HOVER);
        } else {
            *bg = BackgroundColor(BTN);
        }
    }

    // Enter activates the selected button.
    if keys.just_pressed(KeyCode::Enter) {
        let mode = modes[sel.0];
        match mode {
            GameMode::LanHost => dialog.open_for(true),
            GameMode::LanJoin => dialog.open_for(false),
            GameMode::RelayHost | GameMode::RelayJoin => {
                dialog.open_for(mode == GameMode::RelayHost);
            }
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
