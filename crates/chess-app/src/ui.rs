//! `bevy_ui` menus and the in-game HUD (status text + action buttons).

use bevy::prelude::*;
use chess_core::{Color as ChessColor, GameResult, WinReason};

use crate::ai_bridge::AiTask;
use crate::app_state::{AppState, CoreGame, GameMode, Selection, UiFonts};
use crate::board_view::RenderDirty;
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

pub fn setup_menu(mut commands: Commands, fonts: Res<UiFonts>) {
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
                    Text::new("— 国风对弈 · 楚汉相争 —"),
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
                ));
                for (label, mode) in [
                    ("本地双人对弈", GameMode::LocalPvp),
                    ("人机对战", GameMode::VsAi),
                    ("创建联机房间", GameMode::LanHost),
                    ("加入联机房间", GameMode::LanJoin),
                ] {
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
) {
    for (interaction, btn, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match btn.0 {
                // LAN modes open the setup dialog (port / IP / password) first.
                GameMode::LanHost => dialog.open_for(true),
                GameMode::LanJoin => dialog.open_for(false),
                // Local / AI start immediately.
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

#[derive(Component, Clone, Copy)]
pub enum HudAction {
    NewGame,
    Resign,
    OfferDraw,
    Undo,
    BackToMenu,
}

pub fn setup_hud(mut commands: Commands, fonts: Res<UiFonts>) {
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
                panel.spawn((
                    Text::new("象棋对弈"),
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
                for (label, action) in [
                    ("新 对 局", HudAction::NewGame),
                    ("认 输", HudAction::Resign),
                    ("求 和", HudAction::OfferDraw),
                    ("悔 棋", HudAction::Undo),
                    ("返回主菜单", HudAction::BackToMenu),
                ] {
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

pub fn update_status(core: Res<CoreGame>, mut q: Query<&mut Text, With<StatusText>>) {
    let Ok(mut text) = q.single_mut() else {
        return;
    };
    if core.peer_disconnected && !core.game.is_over() {
        // Mid-game disconnect: freeze the board and tell the player we're
        // holding for the peer to come back (LAN: same port; relay: same
        // room number).
        **text = "对方已断开
等待重连…"
            .to_string();
        return;
    }
    if core.awaiting_peer {
        **text = match core.mode {
            // Host side: room is open, waiting for a friend to arrive.
            GameMode::RelayHost => match &core.room_code {
                Some(room) => format!("房间号 {room}\n等待对手加入…"),
                None => "等待对手加入…".to_string(),
            },
            GameMode::LanHost => "等待对手加入…".to_string(),
            // Guest side: still establishing/verifying the connection.
            GameMode::LanJoin | GameMode::RelayJoin => "正在连接，请稍候…".to_string(),
            _ => "正在连接，请稍候…".to_string(),
        };
        return;
    }
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
                };
                format!("{side}胜\n（{why}）")
            }
            GameResult::Draw(_) => "和棋".to_string(),
        }
    } else {
        let side = match core.game.side_to_move() {
            ChessColor::Red => "红方",
            ChessColor::Black => "黑方",
        };
        let turn = if core.local_to_move() {
            "轮到你走棋"
        } else {
            "等待对手…"
        };
        let mut s = format!("{side}行棋\n{turn}");
        if core.draw_offer_from_peer {
            s.push_str("\n对方提议和棋");
        }
        s
    };
    **text = status;
}

#[allow(clippy::too_many_arguments)]
pub fn hud_interaction(
    mut interactions: Query<(&Interaction, &HudAction, &mut BackgroundColor), Changed<Interaction>>,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<RenderDirty>,
    mut selection: ResMut<Selection>,
    mut ai_task: ResMut<AiTask>,
    mut next: ResMut<NextState<AppState>>,
    net: Option<Res<NetLink>>,
) {
    for (interaction, action, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match action {
                HudAction::NewGame => {
                    core.restart();
                    selection.from = None;
                    ai_task.rx = None;
                    dirty.0 = true;
                    // Hosts of a networked game must broadcast the reset so
                    // the connected guest also restarts (otherwise the two
                    // sides desync immediately).
                    if core.mode.is_net_host() {
                        if let Some(net) = &net {
                            let _ = net.out.send(NetCommand::Sync(Box::new(core.game.clone())));
                        }
                    }
                }
                HudAction::Resign => {
                    let me = core.local_color;
                    core.game.resign(me);
                    if let Some(net) = &net {
                        let _ = net.out.send(NetCommand::Resign);
                    }
                    dirty.0 = true;
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
                        core.game.agree_draw();
                        dirty.0 = true;
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
                }
                HudAction::BackToMenu => {
                    next.set(AppState::Menu);
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
