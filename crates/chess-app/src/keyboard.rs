//! Keyboard shortcuts for in-game actions.
//!
//! - `Ctrl+Z` / `U`: Undo move
//! - `Ctrl+N`: New game
//! - `Escape`: Back to menu
//! - `T`: Cycle board theme
//! - `F`: Flip board orientation
//! - `Ctrl+S`: Save game to file
//! - `←` / `→`: Step through move history (view-only)

use bevy::prelude::*;

use crate::animation::{AnimSpeedSetting, AnimationPlaying};
use crate::app_state::{AppState, BoardOrientation, CoreGame, GameMode, Selection, UiFonts};
use crate::board_theme::BoardTheme;
use crate::board_view::{CoordLabel, RenderDirty, ShowCoordinates};
use crate::history_view::HistoryView;
use crate::sound::SoundVolume;

/// Handle keyboard shortcuts during gameplay.
#[allow(clippy::too_many_arguments)]
pub fn keyboard_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut core: ResMut<CoreGame>,
    mut selection: ResMut<Selection>,
    mut dirty: ResMut<RenderDirty>,
    mut next: ResMut<NextState<AppState>>,
    mut ai_task: ResMut<crate::ai_bridge::AiTask>,
    mut theme: ResMut<BoardTheme>,
    mut orient: ResMut<BoardOrientation>,
    mut history_view: ResMut<HistoryView>,
    fonts: Res<UiFonts>,
    animation: Res<AnimationPlaying>,
    mut show_coords: ResMut<ShowCoordinates>,
    mut sound_volume: ResMut<SoundVolume>,
    mut coord_q: Query<&mut Visibility, With<CoordLabel>>,
    mut anim_speed: ResMut<AnimSpeedSetting>,
) {
    // Escape → back to menu (always allowed, even during animation).
    // But skip if a panel consumed Escape this frame.
    if keys.just_pressed(KeyCode::Escape) {
        if !crate::moves::ESCAPE_CONSUMED.swap(false, std::sync::atomic::Ordering::Relaxed) {
            next.set(AppState::Menu);
        }
        return;
    }

    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    // Ctrl+Z or U → undo (blocked during AI thinking, animation, and history view).
    if ((ctrl && keys.just_pressed(KeyCode::KeyZ))
        || keys.just_pressed(KeyCode::KeyU)
        || keys.just_pressed(KeyCode::Backspace))
        && !core.mode.is_networked()
        && core.game.history_len() > 0
    {
        if animation.0 {
            crate::toast::spawn_toast(&mut commands, &fonts, "动画播放中… (等待完成)");
        } else if history_view.viewing_ply.is_some() {
            crate::toast::spawn_toast(&mut commands, &fonts, "查看模式中… (按→或End返回)");
        } else if core.mode == GameMode::VsAi && ai_task.rx.is_some() {
            crate::toast::spawn_toast(&mut commands, &fonts, "AI思考中… (请稍候)");
        } else {
            let old_selection = selection.from;
            // Capture notation of the move being undone for the toast.
            let undo_notation = {
                let ply = core.game.history_len();
                let target_ply = if core.mode == GameMode::VsAi && ply >= 2 {
                    ply - 2 // Player's move (before AI's response)
                } else {
                    ply - 1 // Last move
                };
                if let Some(board_before) = core.game.board_at_ply(target_ply) {
                    let entry = &core.game.history()[target_ply];
                    chess_core::move_to_chinese(entry.mv(), &board_before)
                } else {
                    String::new()
                }
            };
            if core.mode == GameMode::VsAi {
                core.game.undo(); // AI's move
                core.game.undo(); // Player's move
            } else {
                core.game.undo();
            }
            // Keep selection if a piece of the current side still exists there.
            let keep = match old_selection {
                Some(sq) => {
                    let stm = core.game.side_to_move();
                    core.game
                        .board()
                        .piece_at(sq)
                        .is_some_and(|p| p.color == stm)
                }
                None => false,
            };
            if !keep {
                selection.from = None;
            }
            core.last_move = None;
            dirty.0 = true;
            let remaining = core.game.history_len();
            let side_label = match core.game.side_to_move() {
                chess_core::Color::Red => "红",
                chess_core::Color::Black => "黑",
            };
            let msg = if remaining > 0 {
                if undo_notation.is_empty() {
                    format!("悔棋 ({}, 还剩{}手)", side_label, remaining)
                } else {
                    format!(
                        "悔棋 ({}): {} (还剩{}手)",
                        side_label, undo_notation, remaining
                    )
                }
            } else {
                if undo_notation.is_empty() {
                    "悔棋 (回到起始)".to_string()
                } else {
                    format!("悔棋: {} (回到起始)", undo_notation)
                }
            };
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
            crate::moves::UNDO_PERFORMED.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    // T → cycle board theme.
    if keys.just_pressed(KeyCode::KeyT) && !ctrl {
        theme.cycle();
        bevy::log::info!(theme = theme.id.label(), "theme changed");
        dirty.0 = true;
        let theme_num = match theme.id {
            crate::board_theme::ThemeId::Classic => 1,
            crate::board_theme::ThemeId::Dark => 2,
            crate::board_theme::ThemeId::Paper => 3,
            crate::board_theme::ThemeId::Rosewood => 4,
            crate::board_theme::ThemeId::Jade => 5,
            crate::board_theme::ThemeId::Bamboo => 6,
            crate::board_theme::ThemeId::Imperial => 7,
            crate::board_theme::ThemeId::Midnight => 8,
        };
        let label = format!(
            "{} 主题: {} ({}/8)",
            theme.id.emoji(),
            theme.id.label(),
            theme_num
        );
        crate::toast::spawn_toast(&mut commands, &fonts, &label);
        crate::settings::save_settings(&crate::settings::collect_settings(
            &theme,
            &sound_volume,
            &anim_speed,
            &show_coords,
        ));
    }

    // F → flip board orientation.
    if keys.just_pressed(KeyCode::KeyF) && !ctrl {
        if animation.0 {
            return; // don't flip mid-animation
        }
        *orient = match *orient {
            BoardOrientation::Red => BoardOrientation::Black,
            BoardOrientation::Black => BoardOrientation::Red,
        };
        dirty.0 = true;
        let orient_label = if *orient == BoardOrientation::Red {
            "红方"
        } else {
            "黑方"
        };
        let turn_hint = if core.game.is_over() {
            ""
        } else if core.local_to_move() {
            " · 轮到你走"
        } else {
            " · 等待对手"
        };
        let moves = core.game.history_len();
        let round_hint = if !core.game.is_over() && moves > 0 {
            format!(" · 第{}回合", moves / 2 + 1)
        } else {
            String::new()
        };
        crate::toast::spawn_toast(
            &mut commands,
            &fonts,
            &format!("「换」 视角: {}{}{}", orient_label, round_hint, turn_hint),
        );
    }

    // Ctrl+N → new game (auto-save if ≥4 moves).
    if ctrl && keys.just_pressed(KeyCode::KeyN) {
        let saved = if core.game.history_len() >= 4 && !core.game.is_over() {
            crate::game_over_dialog::auto_save_game(&core, None, None)
        } else {
            false
        };
        let abandoned = core.game.history_len();
        core.restart();
        crate::moves::GAME_RESTARTED.store(true, std::sync::atomic::Ordering::Relaxed);
        selection.from = None;
        ai_task.rx = None;
        history_view.viewing_ply = None;
        dirty.0 = true;
        let mode_label = match core.mode {
            crate::app_state::GameMode::VsAi => "人机",
            crate::app_state::GameMode::LocalPvp => "双人",
            crate::app_state::GameMode::LanHost | crate::app_state::GameMode::LanJoin => "局域网",
            _ => "联机",
        };
        let abandon_hint = if abandoned > 0 {
            format!(" (弃{}手)", abandoned)
        } else {
            String::new()
        };
        let msg = if saved {
            format!(
                "「✓」 已保存 · 「换」 新局 · {}{}",
                mode_label, abandon_hint
            )
        } else {
            format!("「换」 新局 · {}{}", mode_label, abandon_hint)
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }

    // Ctrl+S → save game.
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        save_game(&mut commands, &core, &fonts);
    }

    // Ctrl+O → load most recent saved game.
    if ctrl && keys.just_pressed(KeyCode::KeyO) {
        load_game(
            &mut commands,
            &mut core,
            &mut selection,
            &mut ai_task,
            &mut history_view,
            &mut dirty,
            &fonts,
        );
    }

    // C → toggle coordinate labels.
    if keys.just_pressed(KeyCode::KeyC) && !ctrl {
        show_coords.0 = !show_coords.0;
        let vis = if show_coords.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for mut v in &mut coord_q {
            *v = vis;
        }
        let coord_count = coord_q.iter().count();
        let label = if show_coords.0 {
            format!("「标」 坐标: 显示 ({}个)", coord_count)
        } else {
            "「标」 坐标: 隐藏".to_string()
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &label);
        crate::settings::save_settings(&crate::settings::collect_settings(
            &theme,
            &sound_volume,
            &anim_speed,
            &show_coords,
        ));
    }

    // M → cycle sound volume.
    if keys.just_pressed(KeyCode::KeyM) && !ctrl {
        sound_volume.level = sound_volume.level.next();
        let vol_num = match sound_volume.level {
            crate::sound::VolumeLevel::Mute => 1,
            crate::sound::VolumeLevel::VeryLow => 2,
            crate::sound::VolumeLevel::Low => 3,
            crate::sound::VolumeLevel::Normal => 4,
            crate::sound::VolumeLevel::High => 5,
        };
        let msg = format!(
            "{} 音量: {} ({}/5)",
            sound_volume.level.emoji(),
            sound_volume.level.label(),
            vol_num
        );
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        crate::settings::save_settings(&crate::settings::collect_settings(
            &theme,
            &sound_volume,
            &anim_speed,
            &show_coords,
        ));
    }

    // A → cycle animation speed.
    if keys.just_pressed(KeyCode::KeyA) && !ctrl {
        anim_speed.0 = anim_speed.0.next();
        let ms = (anim_speed.0.duration() * 1000.0) as u32;
        let msg = format!(
            "{} 动画速度: {} ({}ms)",
            anim_speed.0.emoji(),
            anim_speed.0.label(),
            ms
        );
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        crate::settings::save_settings(&crate::settings::collect_settings(
            &theme,
            &sound_volume,
            &anim_speed,
            &show_coords,
        ));
    }

    // ← → arrow key history navigation.
    if keys.just_pressed(KeyCode::ArrowLeft) && !ctrl {
        let total = core.game.history_len();
        if total == 0 {
            return;
        }
        let current = history_view.viewing_ply.unwrap_or(total);
        if current > 0 {
            history_view.viewing_ply = Some(current - 1);
            dirty.0 = true;
        }
    }

    if keys.just_pressed(KeyCode::ArrowRight) && !ctrl {
        if let Some(ply) = history_view.viewing_ply {
            let total = core.game.history_len();
            history_view.set_ply(ply + 1, total);
            dirty.0 = true;
        }
    }

    // Home → jump to starting position (ply 0).
    if keys.just_pressed(KeyCode::Home) && !ctrl && core.game.history_len() > 0 {
        history_view.viewing_ply = Some(0);
        dirty.0 = true;
    }

    // End → return to live view.
    if keys.just_pressed(KeyCode::End) && !ctrl && history_view.viewing_ply.is_some() {
        history_view.viewing_ply = None;
        dirty.0 = true;
    }
}

/// Save the current game to a PGN-like file.
fn save_game(commands: &mut Commands, core: &CoreGame, fonts: &UiFonts) {
    use chess_core::GameRecord;
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut record = GameRecord::from_game(&core.game);

    // Populate metadata.
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    record.date = format_date(secs);
    record.red_player = match core.mode {
        GameMode::VsAi => "玩家".to_string(),
        _ => "红方".to_string(),
    };
    record.black_player = match core.mode {
        GameMode::VsAi => "AI".to_string(),
        _ => "黑方".to_string(),
    };
    record.mode = match core.mode {
        GameMode::VsAi => "VsAi".to_string(),
        GameMode::LocalPvp => "LocalPvp".to_string(),
        GameMode::LanHost | GameMode::LanJoin => "Lan".to_string(),
        GameMode::RelayHost | GameMode::RelayJoin => "Relay".to_string(),
    };

    let content = record.serialize();

    // Determine save directory: $HOME/xiangqi_saves/ or ./saves/
    let dir = crate::settings::save_dir();

    let mode_tag = match core.mode {
        crate::app_state::GameMode::VsAi => "ai",
        crate::app_state::GameMode::LocalPvp => "pvp",
        crate::app_state::GameMode::LanHost | crate::app_state::GameMode::LanJoin => "lan",
        _ => "net",
    };
    let filename = format!("game_{}_{}.pgn", mode_tag, secs);
    let path = dir.join(&filename);

    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &content)) {
        Ok(()) => {
            let mode_str = match core.mode {
                crate::app_state::GameMode::VsAi => "人机",
                crate::app_state::GameMode::LocalPvp => "双人",
                _ => "联机",
            };
            let msg = format!(
                "棋局已保存 ({}, 共{}手, {}): {}",
                mode_str,
                core.game.history_len(),
                &record.date,
                filename
            );
            crate::toast::spawn_toast(commands, fonts, &msg);
            bevy::log::info!(path = %path.display(), "game saved");
        }
        Err(e) => {
            let fail_msg = format!("保存失败 (共{}手)", core.game.history_len());
            crate::toast::spawn_toast(commands, fonts, &fail_msg);
            bevy::log::warn!(error = %e, "failed to save game");
        }
    }
}

/// Format a unix timestamp as YYYY.MM.DD.
pub(crate) fn format_date(secs: u64) -> String {
    // Simple date calculation from unix timestamp.
    let days = (secs / 86400) as i64;
    let (year, month, day) = days_to_ymd(days + 719_468); // days since 0000-03-01
    format!("{:04}.{:02}.{:02}", year, month, day)
}

/// Civil date from day count (algorithm from Howard Hinnant).
pub(crate) fn days_to_ymd(g: i64) -> (i64, u32, u32) {
    let era = if g >= 0 { g } else { g - 146_096 } / 146_097;
    let doe = (g - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Load the most recently saved game from the save directory.
fn load_game(
    commands: &mut Commands,
    core: &mut CoreGame,
    selection: &mut Selection,
    ai_task: &mut crate::ai_bridge::AiTask,
    history_view: &mut HistoryView,
    dirty: &mut RenderDirty,
    fonts: &UiFonts,
) {
    // Block in networked modes.
    if core.mode.is_networked() {
        let mode_label = match core.mode {
            crate::app_state::GameMode::LanHost | crate::app_state::GameMode::LanJoin => "局域网",
            _ => "网络",
        };
        crate::toast::spawn_toast(commands, fonts, &format!("联机中无法加载 ({})", mode_label));
        return;
    }

    let dir = crate::settings::save_dir();

    let latest = match std::fs::read_dir(&dir) {
        Ok(entries) => {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "pgn"))
                .collect();
            files.sort_by_key(|e| e.file_name());
            files.last().map(|e| e.path())
        }
        Err(_) => None,
    };

    let path = match latest {
        Some(p) => p,
        None => {
            crate::toast::spawn_toast(commands, fonts, "无存档 · Ctrl+S 可保存");
            return;
        }
    };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            bevy::log::warn!(error = %e, "failed to read save file");
            let fname = path.file_name().unwrap_or_default().to_string_lossy();
            crate::toast::spawn_toast(commands, fonts, &format!("读取失败: {}", fname));
            return;
        }
    };

    let record = match chess_core::GameRecord::parse_record(&content) {
        Ok(r) => r,
        Err(e) => {
            bevy::log::warn!(error = %e, "failed to parse save file");
            let fname = path.file_name().unwrap_or_default().to_string_lossy();
            crate::toast::spawn_toast(commands, fonts, &format!("解析失败: {}", fname));
            return;
        }
    };

    match record.to_game() {
        Ok(game) => {
            core.game = game;
            crate::moves::GAME_RESTARTED.store(true, std::sync::atomic::Ordering::Relaxed);
            core.last_move = None;
            selection.from = None;
            ai_task.rx = None;
            history_view.viewing_ply = None;
            dirty.0 = true;

            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            let mode_hint = if record.mode.is_empty() {
                String::new()
            } else {
                format!("{}, ", record.mode)
            };
            let date_hint = if record.date.is_empty() {
                String::new()
            } else {
                format!(", {}", record.date)
            };
            let msg = format!(
                "已加载 ({}共{}手{}): {}",
                mode_hint,
                core.game.history_len(),
                date_hint,
                filename
            );
            crate::toast::spawn_toast(commands, fonts, &msg);
            bevy::log::info!(path = %path.display(), "game loaded");
        }
        Err(e) => {
            bevy::log::warn!(error = %e, "failed to replay loaded game");
            let fname = path.file_name().unwrap_or_default().to_string_lossy();
            crate::toast::spawn_toast(commands, fonts, &format!("棋谱回放失败: {}", fname));
        }
    }
}

/// Toggle fullscreen with F11 (standalone system — separated to keep
/// `keyboard_shortcuts` within Bevy's 16-param limit).
pub fn toggle_fullscreen(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    fonts: Res<UiFonts>,
    mut windows: Query<&mut Window>,
) {
    if keys.just_pressed(KeyCode::F11) {
        if let Ok(mut window) = windows.single_mut() {
            match window.mode {
                bevy::window::WindowMode::Windowed => {
                    window.mode = bevy::window::WindowMode::BorderlessFullscreen(
                        bevy::window::MonitorSelection::Current,
                    );
                    crate::toast::spawn_toast(&mut commands, &fonts, "「屏」 全屏模式");
                }
                _ => {
                    window.mode = bevy::window::WindowMode::Windowed;
                    crate::toast::spawn_toast(&mut commands, &fonts, "「窗」 窗口模式");
                }
            }
        }
    }
}

/// Export move history as Chinese notation text (Ctrl+E).
/// Standalone system (separated from `keyboard_shortcuts` to stay within
/// Bevy's 16-param limit).
pub fn export_history(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    core: Res<CoreGame>,
    fonts: Res<UiFonts>,
    ai_settings: Res<crate::app_state::AiSettings>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if !(ctrl && keys.just_pressed(KeyCode::KeyE)) {
        return;
    }

    let total = core.game.history_len();
    if total == 0 {
        crate::toast::spawn_toast(&mut commands, &fonts, "无棋谱可导出 · 至少走1手");
        return;
    }

    // Metadata header.
    let mut lines = Vec::new();
    let secs_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let date_str = format_date(secs_now);
    let mode_str = match core.mode {
        crate::app_state::GameMode::VsAi => {
            format!("人机对战 · {}", ai_settings.difficulty.label())
        }
        crate::app_state::GameMode::LocalPvp => "本地双人".to_string(),
        _ => "联机对战".to_string(),
    };
    lines.push("# 中国象棋 棋谱".to_string());
    lines.push(format!(
        "# 日期: {}  模式: {}  共{}手",
        date_str, mode_str, total
    ));
    let (red_name, black_name) = match core.mode {
        crate::app_state::GameMode::VsAi => {
            ("玩家", format!("AI ({})", ai_settings.difficulty.label()))
        }
        _ => ("红方", "黑方".to_string()),
    };
    lines.push(format!("# 红方: {}  黑方: {}", red_name, black_name));
    if let Some(result) = core.game.result() {
        let result_str = match result {
            chess_core::GameResult::Win { winner, .. } => match winner {
                chess_core::Color::Red => "红方胜",
                chess_core::Color::Black => "黑方胜",
            },
            chess_core::GameResult::Draw(_) => "和棋",
        };
        lines.push(format!("# 结果: {}", result_str));
    }
    lines.push(String::new());
    for i in 0..total {
        let entry = &core.game.history()[i];
        let mv = entry.mv();
        let board_before = match core.game.board_at_ply(i) {
            Some(b) => b,
            None => continue,
        };
        let notation = chess_core::move_to_chinese(mv, &board_before);

        if i % 2 == 0 {
            // Red's move — start a new line with move number.
            let move_num = i / 2 + 1;
            lines.push(format!("{:>3}. {}", move_num, notation));
        } else {
            // Black's move — append to existing line.
            if let Some(last) = lines.last_mut() {
                last.push_str(&format!("  {}", notation));
            }
        }
    }

    lines.push(String::new());
    lines.push("# — 棋谱结束 —".to_string());
    let content = lines.join(
        "
",
    );

    let dir = crate::settings::save_dir();

    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mode_tag = match core.mode {
        crate::app_state::GameMode::VsAi => "ai",
        crate::app_state::GameMode::LocalPvp => "pvp",
        crate::app_state::GameMode::LanHost | crate::app_state::GameMode::LanJoin => "lan",
        _ => "net",
    };
    let filename = format!("history_{}_{}.txt", mode_tag, secs);
    let path = dir.join(&filename);

    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &content)) {
        Ok(()) => {
            let result_hint = match core.game.result() {
                Some(chess_core::GameResult::Win { winner, .. }) => match winner {
                    chess_core::Color::Red => ", 红方胜",
                    chess_core::Color::Black => ", 黑方胜",
                },
                Some(chess_core::GameResult::Draw(_)) => ", 和棋",
                None => "",
            };
            let kb = content.len() as f32 / 1024.0;
            let msg = format!(
                "棋谱已导出 ({}行, {:.1}KB{}): {}",
                lines.len(),
                kb,
                result_hint,
                filename
            );
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
            bevy::log::info!(path = %path.display(), "history exported");
        }
        Err(e) => {
            let fail_msg = format!("导出失败 (共{}手未导出)", total);
            crate::toast::spawn_toast(&mut commands, &fonts, &fail_msg);
            bevy::log::warn!(error = %e, "failed to export history");
        }
    }
}

/// Play a soft click when the history view position changes (arrow keys,
/// Home/End). Uses `Local` to track the previous frame's ply, accepting a
/// 1-frame sound delay (imperceptible) to avoid ordering hazards with the
/// chained `keyboard_shortcuts` system.
pub fn history_nav_sound(
    history_view: Res<crate::history_view::HistoryView>,
    mut pending_sound: ResMut<crate::sound::PendingSound>,
    mut prev_ply: Local<Option<usize>>,
) {
    let current = history_view.viewing_ply;
    if current != *prev_ply {
        // Only play sound when entering or changing history view, not when
        // returning to live view (current == None).
        if current.is_some() {
            pending_sound.sound = Some(crate::sound::MoveSound::Normal);
        }
        *prev_ply = current;
    }
}

/// State for auto-play through move history.
#[derive(Resource)]
pub struct AutoPlayState {
    pub active: bool,
    pub timer: bevy::time::Timer,
}

impl Default for AutoPlayState {
    fn default() -> Self {
        Self {
            active: false,
            timer: bevy::time::Timer::from_seconds(0.5, bevy::time::TimerMode::Repeating),
        }
    }
}

/// Toggle auto-play with P key and advance history on timer tick.
/// Standalone system — cannot be in `keyboard_shortcuts` (16-param limit).
#[allow(clippy::too_many_arguments)]
pub fn auto_play_history(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    core: Res<crate::app_state::CoreGame>,
    mut auto_play: ResMut<AutoPlayState>,
    mut history_view: ResMut<crate::history_view::HistoryView>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    anim_speed: Res<crate::animation::AnimSpeedSetting>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let total = core.game.history_len();

    // P toggles auto-play.
    if (keys.just_pressed(KeyCode::KeyP) || keys.just_pressed(KeyCode::Space)) && !ctrl {
        if auto_play.active {
            auto_play.active = false;
            let progress = history_view.viewing_ply.unwrap_or(total);
            let side = if progress.is_multiple_of(2) {
                "红方"
            } else {
                "黑方"
            };
            let msg = format!(
                "自动播放: 停止 (已播{}/{}手, {}行棋)",
                progress, total, side
            );
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        } else if total > 0 {
            auto_play.active = true;
            // Set timer interval based on animation speed.
            let interval = anim_speed.0.duration() + 0.4;
            auto_play
                .timer
                .set_duration(std::time::Duration::from_secs_f32(interval));
            auto_play.timer.reset();
            // Start from beginning if in live view.
            if history_view.viewing_ply.is_none() {
                history_view.viewing_ply = Some(0);
                dirty.0 = true;
            }
            let msg = format!("自动播放: 开始 ({:.1}秒/步, 共{}手)", interval, total);
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        }
    }

    // Arrow keys stop auto-play.
    if auto_play.active
        && (keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::ArrowRight))
    {
        auto_play.active = false;
    }

    // Stop auto-play if view was externally changed to live.
    if auto_play.active && !history_view.is_viewing() {
        auto_play.active = false;
    }

    // Tick timer and advance.
    if auto_play.active {
        auto_play.timer.tick(time.delta());
        if auto_play.timer.just_finished() {
            let current = history_view.viewing_ply.unwrap_or(0);
            if current < total {
                history_view.set_ply(current + 1, total);
                dirty.0 = true;
            }
            // Stop at the end of history.
            if current + 1 >= total {
                auto_play.active = false;
                history_view.return_to_live();
                dirty.0 = true;
                let done_msg = format!("播放完毕 (共{}手)", total);
                crate::toast::spawn_toast(&mut commands, &fonts, &done_msg);
            }
        }
    }
}

/// Quick restart with R key (standalone system — keyboard_shortcuts is at 16 params).
/// Auto-saves the current game if it has ≥4 moves.
#[allow(clippy::too_many_arguments)]
pub fn quick_restart(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    mut core: ResMut<crate::app_state::CoreGame>,
    mut selection: ResMut<crate::app_state::Selection>,
    mut ai_task: ResMut<crate::ai_bridge::AiTask>,
    mut history_view: ResMut<crate::history_view::HistoryView>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    ai_settings: Res<crate::app_state::AiSettings>,
    mut move_times: ResMut<crate::app_state::MoveTimeHistory>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if keys.just_pressed(KeyCode::KeyR) && !ctrl && !core.mode.is_networked() {
        // Auto-save before restart if the game has enough moves.
        let saved = if core.game.history_len() >= 4 && !core.game.is_over() {
            crate::game_over_dialog::auto_save_game(&core, Some(&ai_settings), None)
        } else {
            false
        };
        let abandoned = core.game.history_len();
        core.restart();
        crate::moves::GAME_RESTARTED.store(true, std::sync::atomic::Ordering::Relaxed);
        selection.from = None;
        ai_task.rx = None;
        history_view.viewing_ply = None;
        move_times.0.clear();
        dirty.0 = true;
        let mode_label = match core.mode {
            crate::app_state::GameMode::VsAi => {
                format!("人机 ({})", ai_settings.difficulty.label())
            }
            crate::app_state::GameMode::LocalPvp => "双人".to_string(),
            _ => "对弈".to_string(),
        };
        let abandon_hint = if abandoned > 0 {
            format!(" (弃{}手)", abandoned)
        } else {
            String::new()
        };
        let msg = if saved {
            format!(
                "「✓」 已保存 · 「换」 新对局 · {}{}",
                mode_label, abandon_hint
            )
        } else {
            format!("「换」 新对局 · {}{}", mode_label, abandon_hint)
        };
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

/// Quick difficulty switch with number keys 1-4 (VsAi mode only).
/// 1=Easy, 2=Medium, 3=Hard, 4=Expert
pub fn quick_difficulty(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    core: Res<crate::app_state::CoreGame>,
    mut settings: ResMut<crate::app_state::AiSettings>,
) {
    if core.mode != crate::app_state::GameMode::VsAi {
        return;
    }
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl {
        return;
    }

    let new_diff = if keys.just_pressed(KeyCode::Digit1) {
        Some(chess_ai::Difficulty::Easy)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(chess_ai::Difficulty::Medium)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(chess_ai::Difficulty::Hard)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(chess_ai::Difficulty::Master)
    } else {
        None
    };

    if let Some(diff) = new_diff {
        if diff != settings.difficulty {
            let old_label = settings.difficulty.label();
            settings.difficulty = diff;
            let diff_num = match diff {
                chess_ai::Difficulty::Easy => 1,
                chess_ai::Difficulty::Medium => 2,
                chess_ai::Difficulty::Hard => 3,
                chess_ai::Difficulty::Master => 4,
            };
            let msg = format!(
                "{} 难度: {} → {} ({}/4)",
                diff.emoji(),
                old_label,
                diff.label(),
                diff_num
            );
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
            crate::settings::save_difficulty(diff);
        }
    }
}

/// Play the undo sound effect when the undo flag is set.
/// Standalone system (1-frame delay, imperceptible) to avoid adding a
/// parameter to the already-full `keyboard_shortcuts` function.
pub fn undo_sound_trigger(
    mut pending_sound: ResMut<crate::sound::PendingSound>,
    time: Res<Time>,
    mut move_timer: ResMut<crate::clock_ui::MoveTimer>,
    mut undo_count: ResMut<crate::app_state::UndoCount>,
) {
    if crate::moves::UNDO_PERFORMED.swap(false, std::sync::atomic::Ordering::Relaxed) {
        pending_sound.sound = Some(crate::sound::MoveSound::Undo);
        move_timer.started = time.elapsed_secs();
        undo_count.0 += 1;
    }
    // Reset undo count when game is restarted/loaded/rematched.
    if crate::moves::GAME_RESTARTED.swap(false, std::sync::atomic::Ordering::Relaxed) {
        undo_count.0 = 0;
    }
}

/// Reset all user settings to defaults (Ctrl+D).
#[allow(clippy::too_many_arguments)]
pub fn reset_settings(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    fonts: Res<crate::app_state::UiFonts>,
    mut theme: ResMut<crate::board_theme::BoardTheme>,
    mut volume: ResMut<crate::sound::SoundVolume>,
    mut anim_speed: ResMut<crate::animation::AnimSpeedSetting>,
    mut show_coords: ResMut<crate::board_view::ShowCoordinates>,
    mut dirty: ResMut<crate::board_view::RenderDirty>,
    mut coord_q: Query<&mut Visibility, With<crate::board_view::CoordLabel>>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if !(ctrl && keys.just_pressed(KeyCode::KeyD)) {
        return;
    }

    // Ctrl+Shift+D: delete all saved games (.pgn only, preserves settings.json).
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift {
        let dir = crate::settings::save_dir();
        let mut deleted = 0u32;
        let mut total_bytes = 0u64;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "pgn") {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    if std::fs::remove_file(&path).is_ok() {
                        deleted += 1;
                        total_bytes += size;
                    }
                }
            }
        }
        if deleted > 0 {
            let kb = total_bytes as f32 / 1024.0;
            let msg = format!("「删」 已清除{}个存档 ({:.1}KB)", deleted, kb);
            crate::toast::spawn_toast(&mut commands, &fonts, &msg);
        } else {
            crate::toast::spawn_toast(&mut commands, &fonts, "无存档可清除 · Ctrl+S 保存棋局");
        }
        return;
    }

    // Reset to defaults.
    let old_theme = theme.id.label();
    let old_volume = volume.level;
    let old_anim = anim_speed.0;
    let old_coords = show_coords.0;
    *theme = crate::board_theme::BoardTheme::default();
    volume.level = crate::sound::VolumeLevel::default();
    anim_speed.0 = crate::animation::AnimSpeed::default();
    show_coords.0 = true;

    // Update coordinate label visibility.
    for mut v in &mut coord_q {
        *v = Visibility::Inherited;
    }

    dirty.0 = true;
    crate::settings::save_settings(&crate::settings::collect_settings(
        &theme,
        &volume,
        &anim_speed,
        &show_coords,
    ));
    let already_default = old_theme == "经典"
        && old_volume == crate::sound::VolumeLevel::default()
        && old_anim == crate::animation::AnimSpeed::default()
        && old_coords;
    if already_default {
        crate::toast::spawn_toast(&mut commands, &fonts, "「设」 已是默认设置");
    } else {
        let msg = format!("「换」 设置已重置 (从{})", old_theme);
        crate::toast::spawn_toast(&mut commands, &fonts, &msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_date_unix_epoch() {
        assert_eq!(format_date(0), "1970.01.01");
    }

    #[test]
    fn format_date_y2000() {
        // 2000-01-01 00:00:00 UTC = 946684800 seconds since epoch.
        assert_eq!(format_date(946_684_800), "2000.01.01");
    }

    #[test]
    fn format_date_y2024() {
        // 2024-01-01 00:00:00 UTC = 1704067200 seconds since epoch.
        assert_eq!(format_date(1_704_067_200), "2024.01.01");
    }

    #[test]
    fn format_date_leap_day() {
        // 2024-02-29 00:00:00 UTC = 1709164800 seconds since epoch.
        assert_eq!(format_date(1_709_164_800), "2024.02.29");
    }

    #[test]
    fn format_date_end_of_year() {
        // 2023-12-31 00:00:00 UTC = 1703980800 seconds since epoch.
        assert_eq!(format_date(1_703_980_800), "2023.12.31");
    }
}
