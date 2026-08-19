//! Build script for `chess-app`.
//!
//! When the Pikafish engine files for the **target** platform are present in
//! the workspace `engines/` directory, release binaries embed them directly
//! (like the CJK fonts) so the shipped `chess` executable is a single
//! self-contained file with the strong AI baked in. Debug builds skip
//! embedding to keep compile times low — during development the engine is
//! auto-detected from `engines/` on disk instead (see `engine_bundle.rs`).
//!
//! Expected layout (see `engines/README.md`):
//!
//! ```text
//! engines/
//! ├── pikafish.nnue               # shared NNUE weights (all platforms)
//! ├── macos-arm64/pikafish
//! ├── linux-x86_64/pikafish
//! └── windows-x86_64/pikafish.exe
//! ```

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let engines = manifest.join("../../engines");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_default();

    // Map the cargo target to our engines/ subdirectory layout.
    let subdir = match (os.as_str(), arch.as_str()) {
        ("macos", "aarch64") => Some("macos-arm64"),
        ("linux", "x86_64") => Some("linux-x86_64"),
        ("windows", "x86_64") => Some("windows-x86_64"),
        _ => None,
    };

    let Some(dir) = subdir else { return };
    let exe_name = if os == "windows" {
        "pikafish.exe"
    } else {
        "pikafish"
    };
    let bin = engines.join(dir).join(exe_name);
    let nnue = engines.join("pikafish.nnue");

    println!("cargo:rerun-if-changed={}", bin.display());
    println!("cargo:rerun-if-changed={}", nnue.display());

    // Only embed in release builds: the NNUE is ~51 MB, and embedding it into
    // every debug/test binary would needlessly slow the dev loop. Release
    // binaries get the fully self-contained single-file experience.
    if profile == "release" && bin.exists() && nnue.exists() {
        println!("cargo:rustc-cfg=bundled_engine");
        println!("cargo:rustc-env=PIKAFISH_BUNDLED_BIN={}", bin.display());
        println!("cargo:rustc-env=PIKAFISH_BUNDLED_NNUE={}", nnue.display());
        println!("cargo:warning=embedding Pikafish engine ({dir}) + NNUE into release binary");
    }
}
