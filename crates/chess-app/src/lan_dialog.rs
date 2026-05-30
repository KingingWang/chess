//! Modal dialog for configuring a LAN game before it starts.
//!
//! * **Host** sets the listening **port** and a room **password**.
//! * **Guest** sets the host **IP**, **port**, and the same **password**.
//!
//! The password keys the symmetric (AEAD) encryption of the link, so a wrong
//! password simply fails to connect. Text entry is a small self-contained
//! `bevy_ui` widget (click a field to focus it, then type).

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use chess_core::Color as ChessColor;
use chess_net::Role;

use crate::app_state::{AppState, CoreGame, GameMode, UiFonts};
use crate::async_runtime::AsyncRuntime;
use crate::net_bridge::start_net;

// --- palette -------------------------------------------------------------
const OVERLAY: Color = Color::srgba(0.0, 0.0, 0.0, 0.62);
const CARD: Color = Color::srgb(0.16, 0.13, 0.13);
const CARD_BORDER: Color = Color::srgb(0.62, 0.45, 0.22);
const TITLE: Color = Color::srgb(0.93, 0.84, 0.55);
const LABEL: Color = Color::srgb(0.82, 0.74, 0.58);
const TEXT: Color = Color::srgb(0.97, 0.94, 0.87);
const FIELD_BG: Color = Color::srgb(0.10, 0.09, 0.10);
const FIELD_BG_FOCUS: Color = Color::srgb(0.19, 0.16, 0.12);
const FIELD_BORDER: Color = Color::srgb(0.45, 0.38, 0.25);
const FIELD_BORDER_FOCUS: Color = Color::srgb(0.88, 0.70, 0.34);
const BTN_OK: Color = Color::srgb(0.20, 0.45, 0.22);
const BTN_OK_HOVER: Color = Color::srgb(0.27, 0.58, 0.29);
const BTN_CANCEL: Color = Color::srgb(0.45, 0.22, 0.20);
const BTN_CANCEL_HOVER: Color = Color::srgb(0.60, 0.29, 0.26);
const ERROR_COLOR: Color = Color::srgb(0.92, 0.45, 0.40);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LanField {
    #[default]
    None,
    Ip,
    Port,
    Password,
}

/// State of the LAN setup dialog. `open` drives spawning/despawning.
#[derive(Resource, Default)]
pub struct LanDialog {
    pub open: bool,
    pub is_host: bool,
    pub ip: String,
    pub port: String,
    pub password: String,
    pub focus: LanField,
    pub submit: bool,
    pub error: Option<String>,
}

impl LanDialog {
    /// Open the dialog for the given LAN mode with sensible defaults.
    pub fn open_for(&mut self, is_host: bool) {
        self.open = true;
        self.is_host = is_host;
        self.ip = "192.168.1.100".to_string();
        self.port = "9696".to_string();
        self.password = String::new();
        self.focus = if is_host { LanField::Port } else { LanField::Ip };
        self.submit = false;
        self.error = None;
    }

    fn focused_value_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            LanField::Ip => Some(&mut self.ip),
            LanField::Port => Some(&mut self.port),
            LanField::Password => Some(&mut self.password),
            LanField::None => None,
        }
    }
}

#[derive(Component)]
pub struct LanDialogRoot;

#[derive(Component, Clone, Copy)]
pub struct LanFieldButton(LanField);

#[derive(Component, Clone, Copy)]
pub struct LanFieldText(LanField);

#[derive(Component)]
pub struct LanErrorText;

#[derive(Component, Clone, Copy)]
pub enum LanAction {
    Confirm,
    Cancel,
}

// --- spawn / despawn -----------------------------------------------------

/// Spawn the overlay when the dialog opens; despawn it when it closes.
pub fn manage_lan_dialog(
    mut commands: Commands,
    dialog: Res<LanDialog>,
    fonts: Res<UiFonts>,
    existing: Query<Entity, With<LanDialogRoot>>,
) {
    let root = existing.iter().next();
    if dialog.open && root.is_none() {
        build_dialog(&mut commands, &fonts, &dialog);
    } else if !dialog.open {
        if let Some(e) = root {
            commands.entity(e).despawn();
        }
    }
}

fn build_dialog(commands: &mut Commands, fonts: &UiFonts, dialog: &LanDialog) {
    let title = if dialog.is_host {
        "创建联机房间"
    } else {
        "加入联机房间"
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(OVERLAY),
            GlobalZIndex(100),
            LanDialogRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(44.0), Val::Px(34.0)),
                    row_gap: Val::Px(14.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(16.0)),
                    min_width: Val::Px(420.0),
                    ..default()
                },
                BackgroundColor(CARD),
                BorderColor::all(CARD_BORDER),
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new(title),
                    TextFont {
                        font: fonts.bold.clone(),
                        font_size: 30.0,
                        ..default()
                    },
                    TextColor(TITLE),
                    Node {
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..default()
                    },
                ));

                if !dialog.is_host {
                    spawn_field(card, fonts, "主机 IP", LanField::Ip);
                }
                spawn_field(card, fonts, "端口", LanField::Port);
                spawn_field(card, fonts, "房间密码", LanField::Password);

                // Error line (filled in by the render system).
                card.spawn((
                    Text::new(""),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(ERROR_COLOR),
                    LanErrorText,
                ));

                // Buttons row.
                card.spawn((Node {
                    column_gap: Val::Px(16.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },))
                    .with_children(|row| {
                        spawn_button(row, fonts, "确定", LanAction::Confirm, BTN_OK);
                        spawn_button(row, fonts, "取消", LanAction::Cancel, BTN_CANCEL);
                    });

            });
        });
}

fn spawn_field(
    card: &mut bevy::ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>,
    fonts: &UiFonts,
    label: &str,
    field: LanField,
) {
    card.spawn((Node {
        width: Val::Px(340.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    },))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(LABEL),
            ));
            row.spawn((
                Button,
                LanFieldButton(field),
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(FIELD_BG),
                BorderColor::all(FIELD_BORDER),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new(""),
                    TextFont {
                        font: fonts.regular.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(TEXT),
                    LanFieldText(field),
                ));
            });
        });
}

fn spawn_button(
    row: &mut bevy::ecs::relationship::RelatedSpawnerCommands<'_, ChildOf>,
    fonts: &UiFonts,
    label: &str,
    action: LanAction,
    color: Color,
) {
    row.spawn((
        Button,
        action,
        Node {
            width: Val::Px(120.0),
            height: Val::Px(44.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.5)),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(color),
        BorderColor::all(CARD_BORDER),
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

// --- interaction ---------------------------------------------------------

/// Click a field to focus it; hover/click the action buttons.
#[allow(clippy::type_complexity)]
pub fn lan_dialog_buttons(
    mut dialog: ResMut<LanDialog>,
    mut fields: Query<
        (&Interaction, &LanFieldButton),
        (Changed<Interaction>, Without<LanAction>),
    >,
    mut actions: Query<
        (&Interaction, &LanAction, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, field) in &mut fields {
        if *interaction == Interaction::Pressed {
            dialog.focus = field.0;
        }
    }
    for (interaction, action, mut bg) in &mut actions {
        let (base, hover) = match action {
            LanAction::Confirm => (BTN_OK, BTN_OK_HOVER),
            LanAction::Cancel => (BTN_CANCEL, BTN_CANCEL_HOVER),
        };
        match *interaction {
            Interaction::Pressed => match action {
                LanAction::Confirm => dialog.submit = true,
                LanAction::Cancel => dialog.open = false,
            },
            Interaction::Hovered => *bg = BackgroundColor(hover),
            Interaction::None => *bg = BackgroundColor(base),
        }
    }
}

/// Handle typing into the focused field (and Enter/Esc shortcuts).
pub fn lan_dialog_keyboard(
    mut dialog: ResMut<LanDialog>,
    mut events: MessageReader<KeyboardInput>,
) {
    if !dialog.open {
        events.clear();
        return;
    }
    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match &ev.logical_key {
            Key::Escape => {
                dialog.open = false;
                return;
            }
            Key::Enter => {
                dialog.submit = true;
            }
            Key::Backspace => {
                if let Some(s) = dialog.focused_value_mut() {
                    s.pop();
                }
            }
            Key::Space if dialog.focus == LanField::Password && dialog.password.len() < 64 => {
                dialog.password.push(' ');
            }
            Key::Character(text) => {
                let field = dialog.focus;
                if let Some(s) = dialog.focused_value_mut() {
                    for ch in text.chars() {
                        let len = s.chars().count();
                        if accept_char(field, len, ch) {
                            s.push(ch);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn accept_char(field: LanField, len: usize, ch: char) -> bool {
    match field {
        LanField::Port => ch.is_ascii_digit() && len < 5,
        LanField::Ip => (ch.is_ascii_alphanumeric() || matches!(ch, '.' | ':' | '-')) && len < 45,
        LanField::Password => !ch.is_control() && len < 64,
        LanField::None => false,
    }
}

/// Refresh field texts (mask the password), highlight the focused field, and
/// show any error.
pub fn lan_dialog_render(
    dialog: Res<LanDialog>,
    mut texts: Query<(&LanFieldText, &mut Text), Without<LanErrorText>>,
    mut field_styles: Query<(&LanFieldButton, &mut BackgroundColor, &mut BorderColor)>,
    mut err: Query<&mut Text, With<LanErrorText>>,
) {
    if !dialog.open {
        return;
    }
    for (tag, mut text) in &mut texts {
        let (raw, focused) = match tag.0 {
            LanField::Ip => (dialog.ip.clone(), dialog.focus == LanField::Ip),
            LanField::Port => (dialog.port.clone(), dialog.focus == LanField::Port),
            LanField::Password => (
                "*".repeat(dialog.password.chars().count()),
                dialog.focus == LanField::Password,
            ),
            LanField::None => (String::new(), false),
        };
        let shown = if focused { format!("{raw}|") } else { raw };
        **text = shown;
    }
    for (tag, mut bg, mut border) in &mut field_styles {
        let focused = tag.0 == dialog.focus;
        *bg = BackgroundColor(if focused { FIELD_BG_FOCUS } else { FIELD_BG });
        *border = BorderColor::all(if focused {
            FIELD_BORDER_FOCUS
        } else {
            FIELD_BORDER
        });
    }
    if let Ok(mut t) = err.single_mut() {
        **t = dialog.error.clone().unwrap_or_default();
    }
}

/// Act on a submit (Confirm button or Enter): validate, start the network
/// session, and enter the game.
pub fn lan_dialog_submit(
    mut dialog: ResMut<LanDialog>,
    mut core: ResMut<CoreGame>,
    mut next: ResMut<NextState<AppState>>,
    runtime: Res<AsyncRuntime>,
    mut commands: Commands,
) {
    if !dialog.open || !dialog.submit {
        return;
    }
    dialog.submit = false;

    let port: u16 = match dialog.port.trim().parse() {
        Ok(p) if p > 0 => p,
        _ => {
            dialog.error = Some("端口无效（应为 1–65535）".into());
            return;
        }
    };

    let (role, addr, mode) = if dialog.is_host {
        (
            Role::Host,
            format!("0.0.0.0:{port}"),
            GameMode::LanHost,
        )
    } else {
        let ip = dialog.ip.trim();
        if ip.is_empty() {
            dialog.error = Some("请填写主机 IP".into());
            return;
        }
        (Role::Guest, format!("{ip}:{port}"), GameMode::LanJoin)
    };

    core.restart();
    core.mode = mode;
    core.local_color = ChessColor::Red; // guest gets corrected on Connected

    let link = start_net(
        &runtime.0,
        role,
        addr,
        ChessColor::Red,
        if dialog.is_host { "host" } else { "guest" }.to_string(),
        dialog.password.clone(),
    );
    commands.insert_resource(link);

    dialog.open = false;
    next.set(AppState::InGame);
}

/// Close the dialog (and clear focus) when leaving the menu, and despawn the
/// full-screen overlay so it cannot linger over the game and swallow clicks.
pub fn teardown_lan_dialog(
    mut commands: Commands,
    mut dialog: ResMut<LanDialog>,
    roots: Query<Entity, With<LanDialogRoot>>,
) {
    dialog.open = false;
    dialog.focus = LanField::None;
    for e in &roots {
        commands.entity(e).despawn();
    }
}
