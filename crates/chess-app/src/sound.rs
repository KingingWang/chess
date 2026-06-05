//! Synthesised move / capture / check sound effects.
//!
//! Short WAV clips are generated from sine waves at startup and played via
//! Bevy's audio system. No external sound files are needed — matching the
//! project's self-contained-binary philosophy.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Sound event plumbing
// ---------------------------------------------------------------------------

/// Which sound to play this frame (set by move sources, consumed by
/// [`play_pending_sound`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveSound {
    Normal,
    Capture,
    Check,
    Invalid,
    GameWin,
    GameLose,
    GameDraw,
    Undo,
    UiClick,
}

/// Sound volume level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum VolumeLevel {
    Mute,
    VeryLow,
    Low,
    #[default]
    Normal,
    High,
}

impl VolumeLevel {
    /// Cycle to next level.
    pub fn next(self) -> Self {
        match self {
            VolumeLevel::Mute => VolumeLevel::VeryLow,
            VolumeLevel::VeryLow => VolumeLevel::Low,
            VolumeLevel::Low => VolumeLevel::Normal,
            VolumeLevel::Normal => VolumeLevel::High,
            VolumeLevel::High => VolumeLevel::Mute,
        }
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            VolumeLevel::Mute => "静音",
            VolumeLevel::VeryLow => "很低",
            VolumeLevel::Low => "低",
            VolumeLevel::Normal => "正常",
            VolumeLevel::High => "高",
        }
    }
    /// Emoji icon for this volume level.
    pub fn emoji(self) -> &'static str {
        match self {
            VolumeLevel::High => "「响+」",
            VolumeLevel::Normal => "「响」",
            VolumeLevel::Low => "「轻」",
            VolumeLevel::VeryLow => "「微」",
            VolumeLevel::Mute => "「静」",
        }
    }
}

/// Global sound volume setting.
#[derive(Resource, Default)]
pub struct SoundVolume {
    pub level: VolumeLevel,
}

/// Resource holding the pending sound event for this frame.
/// The optional PieceKind is used for pitch variation on move sounds.
#[derive(Resource, Default)]
pub struct PendingSound {
    pub sound: Option<MoveSound>,
    pub piece: Option<chess_core::PieceKind>,
}

/// Resource holding the pre-generated audio handles.
#[derive(Resource)]
pub struct SoundAssets {
    pub move_sound: Handle<AudioSource>,
    pub capture_sound: Handle<AudioSource>,
    pub check_sound: Handle<AudioSource>,
    pub invalid_sound: Handle<AudioSource>,
    pub win_sound: Handle<AudioSource>,
    pub lose_sound: Handle<AudioSource>,
    pub draw_sound: Handle<AudioSource>,
    pub undo_sound: Handle<AudioSource>,
    pub ui_click: Handle<AudioSource>,
}

// ---------------------------------------------------------------------------
// WAV synthesis
// ---------------------------------------------------------------------------

/// Sample rate for all generated clips.
const SAMPLE_RATE: u32 = 44_100;

/// Build a minimal 16-bit mono WAV from raw `i16` samples.
fn make_wav(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let file_len = 36 + data_len; // RIFF chunk size
    let mut buf = Vec::with_capacity(44 + data_len as usize);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_len.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt sub-chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // sub-chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    buf.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data sub-chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    buf
}

/// Generate a sine-wave burst with amplitude envelope (attack + decay).
fn sine_burst(freq: f32, duration_ms: u32, amplitude: f32) -> Vec<i16> {
    let n = (SAMPLE_RATE as f32 * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(n);
    let attack = (n as f32 * 0.05) as usize; // 5% attack
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let wave = (2.0 * std::f32::consts::PI * freq * t).sin();
        // Envelope: quick attack, exponential decay.
        let env = if i < attack {
            i as f32 / attack as f32
        } else {
            let decay_t = (i - attack) as f32 / (n - attack) as f32;
            (1.0 - decay_t).powi(2)
        };
        let sample = (wave * env * amplitude * 32000.0) as i16;
        samples.push(sample);
    }
    samples
}

/// Wooden "click" — 80 Hz thud with harmonics for fuller sound (65 ms).
fn gen_move_sound() -> Vec<u8> {
    let base = sine_burst(80.0, 65, 0.40);
    let h2 = sine_burst(160.0, 65, 0.10);
    let h3 = sine_burst(240.0, 65, 0.04);
    let samples: Vec<i16> = base
        .iter()
        .zip(h2.iter())
        .zip(h3.iter())
        .map(|((&a, &b), &c)| a.saturating_add(b).saturating_add(c))
        .collect();
    make_wav(&samples)
}

/// Stronger "clack" — 120 Hz + 240 Hz harmonic (80 ms).
fn gen_capture_sound() -> Vec<u8> {
    let base = sine_burst(120.0, 80, 0.55);
    let harmonic = sine_burst(240.0, 80, 0.25);
    let samples: Vec<i16> = base
        .iter()
        .zip(harmonic.iter())
        .map(|(&a, &b)| a.saturating_add(b))
        .collect();
    make_wav(&samples)
}

/// Two-tone rising ping: 440 → 660 Hz (120 ms).
fn gen_check_sound() -> Vec<u8> {
    let n = (SAMPLE_RATE as f32 * 0.12) as usize;
    let mut samples = Vec::with_capacity(n);
    let attack = (n as f32 * 0.05) as usize;
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let frac = i as f32 / n as f32;
        let freq = 440.0 + 220.0 * frac; // sweep 440→660 Hz
        let wave = (2.0 * std::f32::consts::PI * freq * t).sin();
        let env = if i < attack {
            i as f32 / attack as f32
        } else {
            let decay_t = (i - attack) as f32 / (n - attack) as f32;
            (1.0 - decay_t).powi(2)
        };
        samples.push((wave * env * 0.40 * 32000.0) as i16);
    }
    make_wav(&samples)
}

/// Low-frequency buzz for invalid move (100 Hz, 60 ms).
fn gen_invalid_sound() -> Vec<u8> {
    let n = (SAMPLE_RATE as f32 * 0.06) as usize;
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let wave = (2.0 * std::f32::consts::PI * 100.0 * t).sin();
        let env = 1.0 - (i as f32 / n as f32);
        samples.push((wave * env * 0.30 * 32000.0) as i16);
    }
    make_wav(&samples)
}

/// Triumphant rising arpeggio for a win (C5→E5→G5, 330ms) with harmonics.
fn gen_win_sound() -> Vec<u8> {
    let note_len = (SAMPLE_RATE as f32 * 0.11) as usize;
    let mut samples = Vec::with_capacity(note_len * 3);
    for freq in [523.0f32, 659.0, 784.0] {
        for s in 0..note_len {
            let t = s as f32 / SAMPLE_RATE as f32;
            let base = (2.0 * std::f32::consts::PI * freq * t).sin();
            let harmonic = (2.0 * std::f32::consts::PI * freq * 2.0 * t).sin();
            let wave = base + harmonic * 0.2;
            let env = 1.0 - (s as f32 / note_len as f32).powi(2);
            samples.push((wave * env * 0.32 * 32000.0) as i16);
        }
    }
    make_wav(&samples)
}

/// Descending minor for a loss (A4→F4→D4, 330ms) with harmonics.
fn gen_lose_sound() -> Vec<u8> {
    let note_len = (SAMPLE_RATE as f32 * 0.11) as usize;
    let mut samples = Vec::with_capacity(note_len * 3);
    for freq in [440.0f32, 349.0, 293.0] {
        for s in 0..note_len {
            let t = s as f32 / SAMPLE_RATE as f32;
            let base = (2.0 * std::f32::consts::PI * freq * t).sin();
            let harmonic = (2.0 * std::f32::consts::PI * freq * 2.0 * t).sin();
            let wave = base + harmonic * 0.2;
            let env = 1.0 - (s as f32 / note_len as f32).powi(2);
            samples.push((wave * env * 0.32 * 32000.0) as i16);
        }
    }
    make_wav(&samples)
}

/// Two neutral tones for a draw (E4→E4, 280ms) with harmonics.
fn gen_draw_sound() -> Vec<u8> {
    let note_len = (SAMPLE_RATE as f32 * 0.12) as usize;
    let gap = (SAMPLE_RATE as f32 * 0.02) as usize;
    let mut samples = Vec::with_capacity(note_len * 2 + gap);
    for _ in 0..2 {
        for s in 0..note_len {
            let t = s as f32 / SAMPLE_RATE as f32;
            let base = (2.0 * std::f32::consts::PI * 330.0 * t).sin();
            let harmonic = (2.0 * std::f32::consts::PI * 660.0 * t).sin();
            let wave = base + harmonic * 0.15;
            let env = 1.0 - (s as f32 / note_len as f32).powi(2);
            samples.push((wave * env * 0.28 * 32000.0) as i16);
        }
        samples.extend(std::iter::repeat_n(0i16, gap));
    }
    make_wav(&samples)
}

/// Descending tone for undo action (200→100 Hz, 80ms).
fn gen_undo_sound() -> Vec<u8> {
    let n = (SAMPLE_RATE as f32 * 0.08) as usize;
    let mut samples = Vec::with_capacity(n);
    let attack = (n as f32 * 0.05) as usize;
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let frac = i as f32 / n as f32;
        let freq = 200.0 - 100.0 * frac; // sweep 200→100 Hz
        let wave = (2.0 * std::f32::consts::PI * freq * t).sin();
        let env = if i < attack {
            i as f32 / attack as f32
        } else {
            let decay_t = (i - attack) as f32 / (n - attack) as f32;
            (1.0 - decay_t).powi(2)
        };
        samples.push((wave * env * 0.30 * 32000.0) as i16);
    }
    make_wav(&samples)
}

/// Short high-frequency tap for UI button interactions (800 Hz, 25 ms).
fn gen_ui_click() -> Vec<u8> {
    let base = sine_burst(800.0, 25, 0.25);
    let h2 = sine_burst(1600.0, 25, 0.08);
    let samples: Vec<i16> = base
        .iter()
        .zip(h2.iter())
        .map(|(&a, &b)| a.saturating_add(b))
        .collect();
    make_wav(&samples)
}

// ---------------------------------------------------------------------------
// Bevy systems
// ---------------------------------------------------------------------------

/// Startup system: synthesise the three sound clips and store their handles.
pub fn init_sounds(mut commands: Commands, mut audio_assets: ResMut<Assets<AudioSource>>) {
    let move_bytes = gen_move_sound();
    let capture_bytes = gen_capture_sound();
    let check_bytes = gen_check_sound();
    let invalid_bytes = gen_invalid_sound();
    let win_bytes = gen_win_sound();
    let lose_bytes = gen_lose_sound();
    let draw_bytes = gen_draw_sound();
    let undo_bytes = gen_undo_sound();
    let ui_click_bytes = gen_ui_click();

    let move_sound = audio_assets.add(AudioSource {
        bytes: move_bytes.into(),
    });
    let capture_sound = audio_assets.add(AudioSource {
        bytes: capture_bytes.into(),
    });
    let check_sound = audio_assets.add(AudioSource {
        bytes: check_bytes.into(),
    });
    let invalid_sound = audio_assets.add(AudioSource {
        bytes: invalid_bytes.into(),
    });
    let win_sound = audio_assets.add(AudioSource {
        bytes: win_bytes.into(),
    });
    let lose_sound = audio_assets.add(AudioSource {
        bytes: lose_bytes.into(),
    });
    let draw_sound = audio_assets.add(AudioSource {
        bytes: draw_bytes.into(),
    });
    let undo_sound = audio_assets.add(AudioSource {
        bytes: undo_bytes.into(),
    });
    let ui_click = audio_assets.add(AudioSource {
        bytes: ui_click_bytes.into(),
    });

    commands.insert_resource(SoundAssets {
        move_sound,
        capture_sound,
        check_sound,
        invalid_sound,
        win_sound,
        lose_sound,
        draw_sound,
        undo_sound,
        ui_click,
    });
}

/// Consume the pending sound event and spawn an audio playback entity.
pub fn play_pending_sound(
    mut commands: Commands,
    mut pending: ResMut<PendingSound>,
    assets: Option<Res<SoundAssets>>,
    volume: Res<SoundVolume>,
) {
    let Some(sound) = pending.sound.take() else {
        return;
    };
    let piece_kind = pending.piece.take();
    if volume.level == VolumeLevel::Mute {
        return;
    }
    let Some(assets) = assets else {
        return;
    };

    let source = match sound {
        MoveSound::Normal => assets.move_sound.clone(),
        MoveSound::Capture => assets.capture_sound.clone(),
        MoveSound::Check => assets.check_sound.clone(),
        MoveSound::Invalid => assets.invalid_sound.clone(),
        MoveSound::GameWin => assets.win_sound.clone(),
        MoveSound::GameLose => assets.lose_sound.clone(),
        MoveSound::GameDraw => assets.draw_sound.clone(),
        MoveSound::Undo => assets.undo_sound.clone(),
        MoveSound::UiClick => assets.ui_click.clone(),
    };

    // Piece-kind-based pitch variation for move/capture/check sounds.
    let speed = match sound {
        MoveSound::Normal | MoveSound::Capture | MoveSound::Check => match piece_kind {
            Some(chess_core::PieceKind::King) => 0.80,
            Some(chess_core::PieceKind::Chariot) => 0.85,
            Some(chess_core::PieceKind::Cannon) => 0.95,
            Some(chess_core::PieceKind::Elephant) | Some(chess_core::PieceKind::Advisor) => 1.0,
            Some(chess_core::PieceKind::Horse) => 1.10,
            Some(chess_core::PieceKind::Pawn) => 1.20,
            None => 1.0,
        },
        _ => 1.0,
    };

    match volume.level {
        VolumeLevel::High => {
            commands.spawn((
                AudioPlayer::new(source),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Despawn,
                    speed,
                    ..default()
                },
            ));
        }
        VolumeLevel::Normal => {
            commands.spawn((
                AudioPlayer::new(source),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Despawn,
                    volume: bevy::audio::Volume::Linear(0.6),
                    speed,
                    ..default()
                },
            ));
        }
        VolumeLevel::Low => {
            commands.spawn((
                AudioPlayer::new(source),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Despawn,
                    volume: bevy::audio::Volume::Linear(0.3),
                    speed,
                    ..default()
                },
            ));
        }
        VolumeLevel::VeryLow => {
            commands.spawn((
                AudioPlayer::new(source),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Despawn,
                    volume: bevy::audio::Volume::Linear(0.1),
                    speed,
                    ..default()
                },
            ));
        }
        VolumeLevel::Mute => {} // already handled above
    }
}
