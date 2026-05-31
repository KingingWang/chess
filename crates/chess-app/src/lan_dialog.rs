//! Modal dialog for configuring an online game before it starts.
//!
//! A **联机方式 (transport)** toggle switches between:
//! * **局域网 (LAN)** — direct peer-to-peer over the local network (existing).
//!   Host sets **port** + **password**; guest sets host **IP**, **port**, and
//!   the same **password**.
//! * **服务器 (Server)** — relayed over the internet via `chess-relay`. Host
//!   sets only a **password** and is shown a generated **room number**; guest
//!   enters the **room number** + **password**.
//!
//! The password keys the end-to-end (AEAD) encryption of the link, so a wrong
//! password simply fails to connect — and the relay server never sees it.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::window::Ime;
use chess_core::Color as ChessColor;
use chess_net::Role;

use crate::app_state::{AppState, CoreGame, GameMode, UiFonts};
use crate::async_runtime::AsyncRuntime;
use crate::net_bridge::{start_net, NetTarget, RelayConfig};

// --- palette -------------------------------------------------------------
const OVERLAY: Color = Color::srgba(0.0, 0.0, 0.0, 0.62);
const CARD: Color = Color::srgb(0.16, 0.13, 0.13);
const CARD_BORDER: Color = Color::srgb(0.62, 0.45, 0.22);
const TITLE: Color = Color::srgb(0.93, 0.84, 0.55);
const LABEL: Color = Color::srgb(0.82, 0.74, 0.58);
const TEXT: Color = Color::srgb(0.97, 0.94, 0.87);
const HINT: Color = Color::srgb(0.66, 0.72, 0.55);
const FIELD_BG: Color = Color::srgb(0.10, 0.09, 0.10);
const FIELD_BG_FOCUS: Color = Color::srgb(0.19, 0.16, 0.12);
const FIELD_BORDER: Color = Color::srgb(0.45, 0.38, 0.25);
const FIELD_BORDER_FOCUS: Color = Color::srgb(0.88, 0.70, 0.34);
const BTN_OK: Color = Color::srgb(0.20, 0.45, 0.22);
const BTN_OK_HOVER: Color = Color::srgb(0.27, 0.58, 0.29);
const BTN_CANCEL: Color = Color::srgb(0.45, 0.22, 0.20);
const BTN_CANCEL_HOVER: Color = Color::srgb(0.60, 0.29, 0.26);
const BTN_TOGGLE: Color = Color::srgb(0.22, 0.28, 0.42);
const BTN_TOGGLE_HOVER: Color = Color::srgb(0.29, 0.36, 0.54);
const ERROR_COLOR: Color = Color::srgb(0.92, 0.45, 0.40);

/// Which transport the dialog is configuring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Transport {
    #[default]
    Lan,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LanField {
    #[default]
    None,
    Ip,
    Port,
    Room,
    Password,
}

/// State of the online setup dialog. `open` drives spawning/despawning.
#[derive(Resource, Default)]
pub struct LanDialog {
    pub open: bool,
    pub is_host: bool,
    pub transport: Transport,
    pub ip: String,
    pub port: String,
    pub room: String,
    pub password: String,
    pub focus: LanField,
    pub submit: bool,
    pub error: Option<String>,
    /// Set when the field layout changed (e.g. transport toggled) so the
    /// overlay is despawned and rebuilt.
    pub rebuild: bool,
}

impl LanDialog {
    /// Open the dialog for host/guest with sensible defaults (LAN transport).
    pub fn open_for(&mut self, is_host: bool) {
        self.open = true;
        self.is_host = is_host;
        self.transport = Transport::Lan;
        self.ip = "192.168.1.100".to_string();
        self.port = "9696".to_string();
        self.room = String::new();
        self.password = String::new();
        self.focus = self.default_focus();
        self.submit = false;
        self.error = None;
        self.rebuild = false;
    }

    /// The fields shown for the current transport + role, in display order.
    fn fields(&self) -> Vec<(&'static str, LanField)> {
        match (self.transport, self.is_host) {
            (Transport::Lan, true) => vec![("端口", LanField::Port), ("房间密码", LanField::Password)],
            (Transport::Lan, false) => vec![
                ("主机 IP", LanField::Ip),
                ("端口", LanField::Port),
                ("房间密码", LanField::Password),
            ],
            (Transport::Server, true) => vec![("房间密码", LanField::Password)],
            (Transport::Server, false) => {
                vec![("房间号", LanField::Room), ("房间密码", LanField::Password)]
            }
        }
    }

    fn default_focus(&self) -> LanField {
        self.fields().first().map(|f| f.1).unwrap_or(LanField::None)
    }

    fn focused_value_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            LanField::Ip => Some(&mut self.ip),
            LanField::Port => Some(&mut self.port),
            LanField::Room => Some(&mut self.room),
            LanField::Password => Some(&mut self.password),
            LanField::None => None,
        }
    }
}

impl LanDialog {
    /// Append accepted characters of `text` into the focused field, applying the
    /// per-field validation rules. Shared by the keyboard and IME handlers.
    fn insert_text(&mut self, text: &str) {
        let field = self.focus;
        if let Some(s) = self.focused_value_mut() {
            for ch in text.chars() {
                let len = s.chars().count();
                if accept_char(field, len, ch) {
                    s.push(ch);
                }
            }
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

#[derive(Component)]
pub struct LanTransportToggle;

#[derive(Component)]
pub struct LanTransportText;

#[derive(Component, Clone, Copy)]
pub enum LanAction {
    Confirm,
    Cancel,
}

// --- spawn / despawn -----------------------------------------------------

/// Spawn the overlay when the dialog opens, rebuild it when the field layout
/// changes, and despawn it when it closes.
pub fn manage_lan_dialog(
    mut commands: Commands,
    mut dialog: ResMut<LanDialog>,
    fonts: Res<UiFonts>,
    existing: Query<Entity, With<LanDialogRoot>>,
) {
    let root = existing.iter().next();
    if dialog.open {
        if root.is_none() || dialog.rebuild {
            if let Some(e) = root {
                commands.entity(e).despawn();
            }
            build_dialog(&mut commands, &fonts, &dialog);
            dialog.rebuild = false;
        }
    } else if let Some(e) = root {
        commands.entity(e).despawn();
    }
}

fn transport_label(t: Transport) -> &'static str {
    match t {
        Transport::Lan => "联机方式：局域网（点此切换）",
        Transport::Server => "联机方式：服务器（点此切换）",
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
            FocusPolicy::Block,
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
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    },
                ));

                // Transport toggle (局域网 / 服务器).
                card.spawn((
                    Button,
                    LanTransportToggle,
                    Node {
                        width: Val::Px(340.0),
                        height: Val::Px(38.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.5)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(BTN_TOGGLE),
                    BorderColor::all(CARD_BORDER),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new(transport_label(dialog.transport)),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(TEXT),
                        LanTransportText,
                    ));
                });

                for (label, field) in dialog.fields() {
                    spawn_field(card, fonts, label, field);
                }

                // Hint for the server host (room number is generated later).
                if dialog.transport == Transport::Server && dialog.is_host {
                    card.spawn((
                        Text::new("确定后将生成房间号，并在棋盘界面等待对手加入"),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(HINT),
                    ));
                }

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

/// Click a field to focus it; toggle the transport; hover/click the actions.
#[allow(clippy::type_complexity)]
pub fn lan_dialog_buttons(
    mut dialog: ResMut<LanDialog>,
    mut fields: Query<(&Interaction, &LanFieldButton), (Changed<Interaction>, Without<LanAction>)>,
    mut toggle: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<LanTransportToggle>, Without<LanAction>),
    >,
    mut actions: Query<
        (&Interaction, &LanAction, &mut BackgroundColor),
        (Changed<Interaction>, Without<LanTransportToggle>),
    >,
) {
    for (interaction, field) in &mut fields {
        if *interaction == Interaction::Pressed {
            dialog.focus = field.0;
        }
    }
    for (interaction, mut bg) in &mut toggle {
        match *interaction {
            Interaction::Pressed => {
                dialog.transport = match dialog.transport {
                    Transport::Lan => Transport::Server,
                    Transport::Server => Transport::Lan,
                };
                dialog.focus = dialog.default_focus();
                dialog.error = None;
                dialog.rebuild = true;
            }
            Interaction::Hovered => *bg = BackgroundColor(BTN_TOGGLE_HOVER),
            Interaction::None => *bg = BackgroundColor(BTN_TOGGLE),
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
                // When the OS IME is active the same text also arrives via
                // `Ime::Commit`; with `ime_enabled` set, winit suppresses these
                // character events so there is no double entry.
                dialog.insert_text(text.as_str());
            }
            _ => {}
        }
    }
}

fn accept_char(field: LanField, len: usize, ch: char) -> bool {
    match field {
        LanField::Port => ch.is_ascii_digit() && len < 5,
        LanField::Room => ch.is_ascii_digit() && len < 8,
        LanField::Ip => (ch.is_ascii_alphanumeric() || matches!(ch, '.' | ':' | '-')) && len < 45,
        LanField::Password => !ch.is_control() && len < 64,
        LanField::None => false,
    }
}

/// Refresh field texts (mask the password), highlight the focused field, update
/// the transport label, and show any error.
pub fn lan_dialog_render(
    dialog: Res<LanDialog>,
    mut texts: Query<(&LanFieldText, &mut Text), (Without<LanErrorText>, Without<LanTransportText>)>,
    mut field_styles: Query<(&LanFieldButton, &mut BackgroundColor, &mut BorderColor)>,
    mut toggle_text: Query<&mut Text, (With<LanTransportText>, Without<LanErrorText>)>,
    mut err: Query<&mut Text, With<LanErrorText>>,
) {
    if !dialog.open {
        return;
    }
    for (tag, mut text) in &mut texts {
        let (raw, focused) = match tag.0 {
            LanField::Ip => (dialog.ip.clone(), dialog.focus == LanField::Ip),
            LanField::Port => (dialog.port.clone(), dialog.focus == LanField::Port),
            LanField::Room => (dialog.room.clone(), dialog.focus == LanField::Room),
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
    if let Ok(mut t) = toggle_text.single_mut() {
        **t = transport_label(dialog.transport).to_string();
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
    relay: Res<RelayConfig>,
    mut commands: Commands,
) {
    if !dialog.open || !dialog.submit {
        return;
    }
    dialog.submit = false;

    match dialog.transport {
        Transport::Lan => submit_lan(&mut dialog, &mut core, &mut next, &runtime, &mut commands),
        Transport::Server => submit_server(
            &mut dialog,
            &mut core,
            &mut next,
            &runtime,
            &relay,
            &mut commands,
        ),
    }
}

fn submit_lan(
    dialog: &mut LanDialog,
    core: &mut CoreGame,
    next: &mut NextState<AppState>,
    runtime: &AsyncRuntime,
    commands: &mut Commands,
) {
    let port: u16 = match dialog.port.trim().parse() {
        Ok(p) if p > 0 => p,
        _ => {
            dialog.error = Some("端口无效（应为 1–65535）".into());
            return;
        }
    };

    let (target, mode) = if dialog.is_host {
        (
            NetTarget::Lan {
                role: Role::Host,
                addr: format!("0.0.0.0:{port}"),
            },
            GameMode::LanHost,
        )
    } else {
        let ip = dialog.ip.trim();
        if ip.is_empty() {
            dialog.error = Some("请填写主机 IP".into());
            return;
        }
        (
            NetTarget::Lan {
                role: Role::Guest,
                addr: format!("{ip}:{port}"),
            },
            GameMode::LanJoin,
        )
    };

    start_game(core, next, GameSetup { mode, room_code: None });
    let link = start_net(
        &runtime.0,
        target,
        ChessColor::Red,
        if dialog.is_host { "host" } else { "guest" }.to_string(),
        dialog.password.clone(),
    );
    commands.insert_resource(link);
    dialog.open = false;
}

fn submit_server(
    dialog: &mut LanDialog,
    core: &mut CoreGame,
    next: &mut NextState<AppState>,
    runtime: &AsyncRuntime,
    relay: &RelayConfig,
    commands: &mut Commands,
) {
    let cfg = relay.0.clone();
    let (target, mode, room_code) = if dialog.is_host {
        (NetTarget::RelayHost { cfg }, GameMode::RelayHost, None)
    } else {
        let room = dialog.room.trim().to_string();
        if room.len() != 8 || !room.bytes().all(|b| b.is_ascii_digit()) {
            dialog.error = Some("房间号应为 8 位数字".into());
            return;
        }
        (
            NetTarget::RelayJoin {
                cfg,
                room: room.clone(),
            },
            GameMode::RelayJoin,
            Some(room),
        )
    };

    start_game(core, next, GameSetup { mode, room_code });
    let link = start_net(
        &runtime.0,
        target,
        ChessColor::Red,
        if dialog.is_host { "host" } else { "guest" }.to_string(),
        dialog.password.clone(),
    );
    commands.insert_resource(link);
    dialog.open = false;
}

struct GameSetup {
    mode: GameMode,
    room_code: Option<String>,
}

fn start_game(core: &mut CoreGame, next: &mut NextState<AppState>, setup: GameSetup) {
    core.restart();
    core.mode = setup.mode;
    core.local_color = ChessColor::Red; // guest gets corrected on Connected
    core.room_code = setup.room_code;
    // Networked games are not playable until the peer is actually connected and
    // the password-keyed handshake has succeeded. Stay in a "connecting / waiting"
    // state (board input disabled) until a `Connected` event arrives; a failure
    // bounces back to the menu with an error instead of looking joined.
    core.awaiting_peer = matches!(
        setup.mode,
        GameMode::LanHost | GameMode::LanJoin | GameMode::RelayHost | GameMode::RelayJoin
    );
    core.connected = false;
    core.draw_offer_from_peer = false;
    next.set(AppState::InGame);
}

/// Close the dialog (and clear focus) when leaving the menu, and despawn the
/// full-screen overlay so it cannot linger over the game and swallow clicks.
pub fn teardown_lan_dialog(
    mut commands: Commands,
    mut dialog: ResMut<LanDialog>,
    roots: Query<Entity, With<LanDialogRoot>>,
    mut windows: Query<&mut Window>,
) {
    dialog.open = false;
    dialog.focus = LanField::None;
    if let Ok(mut window) = windows.single_mut() {
        window.ime_enabled = false;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
}

/// Insert IME-committed text (CJK, or any text when an OS IME is active) into
/// the focused field. Complements [`lan_dialog_keyboard`], which still handles
/// the control keys (Esc/Enter/Backspace) that arrive as `KeyboardInput`.
pub fn lan_dialog_ime(mut dialog: ResMut<LanDialog>, mut events: MessageReader<Ime>) {
    if !dialog.open {
        events.clear();
        return;
    }
    for ev in events.read() {
        if let Ime::Commit { value, .. } = ev {
            let text = value.clone();
            dialog.insert_text(&text);
        }
    }
}

/// Enable the window IME only while the dialog is open. Without this, on Linux
/// (X11/Wayland) an active system input method (fcitx/ibus/搜狗) swallows key
/// presses and the text fields appear unresponsive. Enabling IME routes the
/// committed text to us as [`Ime`] events instead.
pub fn lan_dialog_sync_ime(dialog: Res<LanDialog>, mut windows: Query<&mut Window>) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    if window.ime_enabled != dialog.open {
        window.ime_enabled = dialog.open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input::ButtonState;

    fn key(ch: &str, code: KeyCode) -> KeyboardInput {
        KeyboardInput {
            key_code: code,
            logical_key: Key::Character(ch.into()),
            state: ButtonState::Pressed,
            text: Some(ch.into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn keyboard_types_into_default_focused_field() {
        let mut app = App::new();
        app.add_message::<KeyboardInput>();
        app.add_systems(Update, lan_dialog_keyboard);

        let mut d = LanDialog::default();
        d.open_for(true); // host + LAN => first field is Port (default "9696")
        assert_eq!(d.focus, LanField::Port);
        app.insert_resource(d);

        app.world_mut().write_message(key("5", KeyCode::Digit5));
        app.update();

        assert_eq!(app.world().resource::<LanDialog>().port, "96965");
    }

    #[test]
    fn clicking_a_field_button_focuses_it() {
        let mut app = App::new();
        app.add_systems(Update, lan_dialog_buttons);

        let mut d = LanDialog::default();
        d.open_for(false); // guest + LAN => fields Ip, Port, Password; focus = Ip
        assert_eq!(d.focus, LanField::Ip);
        app.insert_resource(d);

        app.world_mut()
            .spawn((Interaction::Pressed, LanFieldButton(LanField::Password)));
        app.update();

        assert_eq!(app.world().resource::<LanDialog>().focus, LanField::Password);
    }

    #[test]
    fn dialog_root_is_stable_across_frames() {
        let mut app = App::new();
        app.insert_resource(UiFonts {
            regular: Handle::default(),
            bold: Handle::default(),
        });
        let mut d = LanDialog::default();
        d.open_for(true);
        app.insert_resource(d);
        app.add_systems(Update, manage_lan_dialog);

        app.update(); // first build
        let root1 = app
            .world_mut()
            .query_filtered::<Entity, With<LanDialogRoot>>()
            .iter(app.world())
            .next()
            .expect("dialog root spawned");

        app.update();
        app.update();

        let roots: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<LanDialogRoot>>()
            .iter(app.world())
            .collect();
        assert_eq!(roots.len(), 1, "exactly one dialog root expected");
        assert_eq!(
            roots[0], root1,
            "dialog must not be despawned/rebuilt every frame"
        );
    }

    #[test]
    fn ime_commit_inserts_into_focused_field() {
        let mut app = App::new();
        app.add_message::<Ime>();
        app.add_systems(Update, lan_dialog_ime);

        let mut d = LanDialog::default();
        d.open_for(true);
        d.focus = LanField::Password;
        app.insert_resource(d);

        app.world_mut().write_message(Ime::Commit {
            window: Entity::PLACEHOLDER,
            value: "你好".to_string(),
        });
        app.update();

        assert_eq!(app.world().resource::<LanDialog>().password, "你好");
    }

    #[test]
    fn ime_commit_respects_numeric_field_rules() {
        let mut app = App::new();
        app.add_message::<Ime>();
        app.add_systems(Update, lan_dialog_ime);

        let mut d = LanDialog::default();
        d.open_for(false); // guest + server? no, LAN. set room manually
        d.transport = Transport::Server;
        d.focus = LanField::Room;
        d.room.clear();
        app.insert_resource(d);

        // Non-digits rejected; digits accepted up to 8.
        app.world_mut().write_message(Ime::Commit {
            window: Entity::PLACEHOLDER,
            value: "12ab34".to_string(),
        });
        app.update();

        assert_eq!(app.world().resource::<LanDialog>().room, "1234");
    }

    #[test]
    fn typing_into_password_after_focus() {
        let mut app = App::new();
        app.add_message::<KeyboardInput>();
        app.add_systems(Update, lan_dialog_keyboard);

        let mut d = LanDialog::default();
        d.open_for(true);
        d.focus = LanField::Password;
        app.insert_resource(d);

        app.world_mut().write_message(key("a", KeyCode::KeyA));
        app.update();

        assert_eq!(app.world().resource::<LanDialog>().password, "a");
    }
}
