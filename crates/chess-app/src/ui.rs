//! `bevy_ui` menus and the in-game HUD (status text + action buttons).

use bevy::prelude::*;
use chess_core::{Color as ChessColor, GameResult, WinReason};
use chess_net::Role;

use crate::app_state::{AppState, CoreGame, GameMode};
use crate::board_view::RenderDirty;
use crate::net_bridge::{start_net, NetCommand, NetLink};

const PANEL: Color = Color::srgb(0.12, 0.12, 0.14);
const BTN: Color = Color::srgb(0.20, 0.22, 0.28);
const BTN_HOVER: Color = Color::srgb(0.28, 0.32, 0.42);
const TEXT: Color = Color::srgb(0.92, 0.92, 0.95);

fn lan_addr_host() -> String {
    std::env::var("CHESS_BIND").unwrap_or_else(|_| "0.0.0.0:9696".to_string())
}
fn lan_addr_join() -> String {
    std::env::var("CHESS_ADDR").unwrap_or_else(|_| "127.0.0.1:9696".to_string())
}

// --- Menu ----------------------------------------------------------------

#[derive(Component)]
pub struct MenuRoot;

#[derive(Component, Clone, Copy)]
pub struct MenuButton(pub GameMode);

pub fn setup_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(PANEL),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("中国象棋  Xiangqi"),
                TextFont {
                    font_size: 44.0,
                    ..default()
                },
                TextColor(TEXT),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));
            for (label, mode) in [
                ("Local 2-Player", GameMode::LocalPvp),
                ("Vs Computer (AI)", GameMode::VsAi),
                ("Host LAN Game", GameMode::LanHost),
                ("Join LAN Game", GameMode::LanJoin),
            ] {
                root.spawn((
                    Button,
                    MenuButton(mode),
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Px(52.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BTN),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(TEXT),
                    ));
                });
            }
        });
}

pub fn menu_interaction(
    mut interactions: Query<
        (&Interaction, &MenuButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut core: ResMut<CoreGame>,
    mut next: ResMut<NextState<AppState>>,
    runtime: Res<crate::async_runtime::AsyncRuntime>,
    mut commands: Commands,
) {
    for (interaction, btn, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                core.restart();
                core.mode = btn.0;
                core.local_color = ChessColor::Red;

                match btn.0 {
                    GameMode::LanHost => {
                        let link = start_net(
                            &runtime.0,
                            Role::Host,
                            lan_addr_host(),
                            ChessColor::Red,
                            "host".into(),
                        );
                        core.local_color = ChessColor::Red;
                        commands.insert_resource(link);
                    }
                    GameMode::LanJoin => {
                        let link = start_net(
                            &runtime.0,
                            Role::Guest,
                            lan_addr_join(),
                            ChessColor::Red,
                            "guest".into(),
                        );
                        // local_color updated when NetEvent::Connected arrives.
                        commands.insert_resource(link);
                    }
                    _ => {}
                }
                next.set(AppState::InGame);
            }
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
    BackToMenu,
}

pub fn setup_hud(mut commands: Commands) {
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
                    width: Val::Px(220.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.10, 0.10, 0.12, 0.85)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("..."),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(TEXT),
                    StatusText,
                ));
                for (label, action) in [
                    ("New Game", HudAction::NewGame),
                    ("Resign", HudAction::Resign),
                    ("Offer Draw", HudAction::OfferDraw),
                    ("Main Menu", HudAction::BackToMenu),
                ] {
                    panel
                        .spawn((
                            Button,
                            action,
                            Node {
                                width: Val::Px(180.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(BTN),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(label),
                                TextFont {
                                    font_size: 18.0,
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
    let status = if let Some(result) = core.game.result() {
        match result {
            GameResult::Win { winner, reason } => {
                let side = match winner {
                    ChessColor::Red => "Red",
                    ChessColor::Black => "Black",
                };
                let why = match reason {
                    WinReason::Checkmate => "checkmate",
                    WinReason::Stalemate => "stalemate",
                    WinReason::Resignation => "resignation",
                    WinReason::PerpetualCheck => "perpetual check",
                };
                format!("{side} wins\n({why})")
            }
            GameResult::Draw(_) => "Draw".to_string(),
        }
    } else {
        let side = match core.game.side_to_move() {
            ChessColor::Red => "Red",
            ChessColor::Black => "Black",
        };
        let turn = if core.local_to_move() {
            "your move"
        } else {
            "waiting..."
        };
        format!("{side} to move\n({turn})")
    };
    **text = status;
}

pub fn hud_interaction(
    mut interactions: Query<
        (&Interaction, &HudAction, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut core: ResMut<CoreGame>,
    mut dirty: ResMut<RenderDirty>,
    mut next: ResMut<NextState<AppState>>,
    net: Option<Res<NetLink>>,
) {
    for (interaction, action, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match action {
                HudAction::NewGame => {
                    core.restart();
                    dirty.0 = true;
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
                            // Accept the peer's pending offer.
                            let _ = net.out.send(NetCommand::DrawResponse(true));
                            core.draw_offer_from_peer = false;
                            core.game.agree_draw();
                            dirty.0 = true;
                        } else {
                            let _ = net.out.send(NetCommand::DrawOffer);
                        }
                    } else {
                        // Local game: accept immediately.
                        core.game.agree_draw();
                        dirty.0 = true;
                    }
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
