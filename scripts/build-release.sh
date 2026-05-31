#!/usr/bin/env bash
# Build release artifacts for the Chinese-Chess workspace.
#
# Produces two kinds of binaries:
#
#   * chess-relay  — the public-internet relay server. Built as a fully static
#                    musl ELF (3-4 MB, "statically linked"), drop-in for any
#                    glibc OR musl Linux host with no shared-library deps.
#
#   * chess        — the Bevy desktop client. The Rust standard library and
#                    every Rust dep is linked statically (so users do not need
#                    a Rust toolchain or extra crates). The only dynamic deps
#                    are the host's OpenGL/Vulkan/Wayland/X11/ALSA system
#                    libraries — these MUST be dynamic on every platform
#                    because the graphics and audio backends dlopen them at
#                    runtime; see docs/RELEASE.md for details.
#
# Usage:
#   scripts/build-release.sh            # build everything available for the host
#   scripts/build-release.sh relay      # only the relay
#   scripts/build-release.sh client     # only the desktop client
#
# Optional environment:
#   STRIP=1     run `strip` on the resulting binaries to halve their size
#   TARGET_DIR  override cargo target dir
set -euo pipefail

cd "$(dirname "$0")/.."

TARGETS=${1:-all}
STRIP=${STRIP:-0}

OUT=dist
mkdir -p "$OUT"

build_relay() {
    echo ">>> building chess-relay (x86_64-unknown-linux-musl, fully static)"
    rustup target add x86_64-unknown-linux-musl >/dev/null
    RUSTFLAGS="-C target-feature=+crt-static" \
        CC_x86_64_unknown_linux_musl="${CC_x86_64_unknown_linux_musl:-x86_64-linux-musl-gcc}" \
        cargo build --release -p chess-relay --target x86_64-unknown-linux-musl
    cp target/x86_64-unknown-linux-musl/release/chess-relay "$OUT/chess-relay-linux-x86_64"
    if [ "$STRIP" = "1" ]; then strip "$OUT/chess-relay-linux-x86_64"; fi
    file "$OUT/chess-relay-linux-x86_64"
}

build_client_linux() {
    echo ">>> building chess (Linux glibc, Rust-static + system graphics/audio dynamic)"
    cargo build --release -p chess-app
    cp target/release/chess "$OUT/chess-linux-x86_64"
    if [ "$STRIP" = "1" ]; then strip "$OUT/chess-linux-x86_64"; fi
    file "$OUT/chess-linux-x86_64"
    echo ">>> runtime shared-library dependencies of the client:"
    ldd "$OUT/chess-linux-x86_64" || true
}

build_client_windows() {
    if rustup target list --installed | grep -q '^x86_64-pc-windows-gnu$'; then
        echo ">>> building chess (Windows MinGW cross, Rust-static)"
        cargo build --release -p chess-app --target x86_64-pc-windows-gnu
        cp target/x86_64-pc-windows-gnu/release/chess.exe "$OUT/chess-windows-x86_64.exe"
    else
        echo "(skipping Windows cross: install with \`rustup target add x86_64-pc-windows-gnu\` + MinGW toolchain)"
    fi
}

case "$TARGETS" in
    relay)   build_relay ;;
    client)  build_client_linux; build_client_windows ;;
    all)     build_relay; build_client_linux; build_client_windows ;;
    *)       echo "unknown target: $TARGETS (use relay|client|all)"; exit 2 ;;
esac

echo
echo ">>> artifacts in $OUT/:"
ls -lh "$OUT"
