//! Bundled Pikafish engine detection and extraction.
//!
//! The strong AI (Pikafish + NNUE, ~51 MB) ships **inside** release binaries:
//! `build.rs` embeds the engine for the target platform via `include_bytes!`
//! when the files are present under `engines/` (see `engines/README.md`).
//! On first launch the bytes are extracted once to a per-user cache dir and
//! executed from there (Pikafish is a separate UCI *process*, so it must live
//! on the filesystem — it cannot be run from memory).
//!
//! Detection order (first hit wins):
//!
//! 1. `PIKAFISH_PATH` / `PIKAFISH_EVAL` environment variables (explicit).
//! 2. `engines/<platform>/pikafish` next to the executable (dist layout).
//! 3. `../Resources/engines/<platform>/pikafish` relative to the executable
//!    (macOS `.app` bundle layout).
//! 4. `engines/<platform>/pikafish` under the current working directory
//!    (running from the repo during development).
//! 5. `../../engines/<platform>/pikafish` relative to the executable
//!    (`target/{debug,release}/chess` → workspace root).
//! 6. The embedded copy compiled into the binary (release builds only).
//!
//! Returns `None` when nothing is found — the caller then uses the built-in
//! fallback engine, so the game is always playable.

use std::path::{Path, PathBuf};

/// Resolved engine + NNUE paths.
#[derive(Debug, Clone)]
pub struct EnginePaths {
    pub engine: PathBuf,
    pub nnue: Option<PathBuf>,
}

/// Per-platform subdirectory of `engines/` holding the binary for this target.
fn platform_dir() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "macos-arm64",
        ("linux", "x86_64") => "linux-x86_64",
        ("windows", "x86_64") => "windows-x86_64",
        // Other platforms: look in the flat layout only.
        _ => "",
    }
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "pikafish.exe"
    } else {
        "pikafish"
    }
}

/// Look for `pikafish[.exe]` under `<base>/<platform-dir>` or directly under
/// `<base>`, with the NNUE either beside the binary or one level up (our
/// `engines/` layout shares one `pikafish.nnue` across platforms).
fn probe(base: &Path) -> Option<EnginePaths> {
    let platform = platform_dir();
    let candidates: Vec<PathBuf> = if platform.is_empty() {
        vec![base.join(exe_name())]
    } else {
        vec![base.join(platform).join(exe_name()), base.join(exe_name())]
    };
    for engine in candidates {
        if !engine.is_file() {
            continue;
        }
        let dir = engine.parent()?;
        let nnue_candidates = [
            dir.join("pikafish.nnue"),
            dir.join("..").join("pikafish.nnue"),
        ];
        let nnue = nnue_candidates.iter().find(|p| p.is_file()).cloned();
        return Some(EnginePaths { engine, nnue });
    }
    None
}

/// Per-user cache directory used to hold the extracted embedded engine.
fn cache_root() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| Path::new(&h).join("Library/Caches/chess"))
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))
            .map(|p| p.join("chess"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("chess"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// Extract the embedded engine + NNUE into the cache directory (once, keyed
/// by content length) and return the on-disk paths. Only available in
/// release builds that had the engine files present at compile time.
#[cfg(bundled_engine)]
fn extract_embedded() -> Option<EnginePaths> {
    const ENGINE_BYTES: &[u8] = include_bytes!(env!("PIKAFISH_BUNDLED_BIN"));
    const NNUE_BYTES: &[u8] = include_bytes!(env!("PIKAFISH_BUNDLED_NNUE"));

    // Fingerprint: content lengths are enough in practice — the cache is
    // per-user and we always re-verify the files exist before launching.
    let tag = format!("pikafish-{}-{}", ENGINE_BYTES.len(), NNUE_BYTES.len());
    let dir = cache_root()?.join("engines").join(tag);
    let engine = dir.join(exe_name());
    let nnue = dir.join("pikafish.nnue");

    if engine.is_file() && nnue.is_file() {
        return Some(EnginePaths {
            engine,
            nnue: Some(nnue),
        });
    }

    // Write to a temp sibling dir first, then rename, so a crash mid-write
    // never leaves a half-extracted engine behind.
    let tmp = dir.with_extension("tmp");
    std::fs::create_dir_all(&tmp).ok()?;
    std::fs::write(tmp.join(exe_name()), ENGINE_BYTES).ok()?;
    std::fs::write(tmp.join("pikafish.nnue"), NNUE_BYTES).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ =
            std::fs::set_permissions(tmp.join(exe_name()), std::fs::Permissions::from_mode(0o755));
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&tmp, &dir).ok()?;
    tracing::info!(path = %engine.display(), "extracted embedded Pikafish engine");
    Some(EnginePaths {
        engine,
        nnue: Some(nnue),
    })
}

#[cfg(not(bundled_engine))]
fn extract_embedded() -> Option<EnginePaths> {
    None
}

/// Find the best available Pikafish installation; see module docs for the
/// search order. Environment variables always win.
pub fn detect() -> Option<EnginePaths> {
    // 1. Explicit environment override.
    if let Some(path) = std::env::var_os("PIKAFISH_PATH") {
        let engine = PathBuf::from(path);
        let nnue = std::env::var_os("PIKAFISH_EVAL")
            .map(PathBuf::from)
            .or_else(|| {
                engine
                    .parent()
                    .map(|d| d.join("pikafish.nnue"))
                    .filter(|p| p.is_file())
            });
        return Some(EnginePaths { engine, nnue });
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));

    // 2/3. Next to the executable (dist + .app bundle layouts).
    if let Some(dir) = &exe_dir {
        for base in [
            dir.join("engines"),
            dir.join("..").join("Resources").join("engines"),
            dir.join("..").join("..").join("engines"),
        ] {
            if let Some(paths) = probe(&base) {
                tracing::info!(path = %paths.engine.display(), "found bundled Pikafish");
                return Some(paths);
            }
        }
    }

    // 4. Development: engines/ under the working directory.
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(paths) = probe(&cwd.join("engines")) {
            tracing::info!(path = %paths.engine.display(), "found Pikafish in ./engines");
            return Some(paths);
        }
    }

    // 5. Embedded into the binary (release builds).
    extract_embedded()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_finds_platform_layout() {
        let tmp = std::env::temp_dir().join(format!("chess-probe-{}", std::process::id()));
        let plat = tmp.join(platform_dir());
        std::fs::create_dir_all(&plat).unwrap();
        std::fs::write(plat.join(exe_name()), b"fake-engine").unwrap();
        std::fs::write(tmp.join("pikafish.nnue"), b"fake-nnue").unwrap();
        let found = probe(&tmp).expect("should find engine");
        assert!(found.engine.ends_with(exe_name()));
        // NNUE lives one level up from the platform dir.
        assert!(found.nnue.is_some());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn probe_finds_flat_layout() {
        // Some users drop a lone `pikafish` + nnue straight into a folder.
        let tmp = std::env::temp_dir().join(format!("chess-probe-flat-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join(exe_name()), b"fake-engine").unwrap();
        std::fs::write(tmp.join("pikafish.nnue"), b"fake-nnue").unwrap();
        let found = probe(&tmp).expect("flat layout should be found");
        assert!(found.engine.ends_with(exe_name()));
        assert!(found.nnue.unwrap().ends_with("pikafish.nnue"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn probe_returns_none_when_missing() {
        let tmp = std::env::temp_dir().join(format!("chess-probe-none-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(probe(&tmp).is_none());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
