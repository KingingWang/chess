#!/usr/bin/env bash
#
# build-dmg.sh — Build a macOS .app bundle and package it as .dmg
#
# Usage:
#   ./macos/build-dmg.sh              # Native build (current arch)
#   ./macos/build-dmg.sh --universal  # Universal binary (arm64 + x86_64)
#
# Prerequisites (run on macOS):
#   - Xcode Command Line Tools
#   - For --universal: rustup target add aarch64-apple-darwin x86_64-apple-darwin

set -euo pipefail

# ─── Configuration ────────────────────────────────────────────────────────────
APP_NAME="Xiangqi"
BIN_NAME="chess"
VERSION="0.1.0"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/macos-bundle"
APP_BUNDLE="$BUILD_DIR/${APP_NAME}.app"
DMG_NAME="${APP_NAME}-${VERSION}-macOS"

# ─── Parse flags ──────────────────────────────────────────────────────────────
UNIVERSAL=false
if [[ "${1:-}" == "--universal" ]]; then
    UNIVERSAL=true
fi

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Building ${APP_NAME}.app → ${DMG_NAME}.dmg"
if $UNIVERSAL; then
echo "║  Mode: Universal binary (arm64 + x86_64)"
else
echo "║  Mode: Native ($(uname -m))"
fi
echo "╚══════════════════════════════════════════════════════════════╝"

# ─── Step 1: Compile ─────────────────────────────────────────────────────────
echo ""
echo "▶ Step 1/4: Compiling release binary..."

cd "$PROJECT_ROOT"
mkdir -p "$BUILD_DIR"

if $UNIVERSAL; then
    cargo build --release -p chess-app --target aarch64-apple-darwin
    cargo build --release -p chess-app --target x86_64-apple-darwin
    lipo -create \
        "target/aarch64-apple-darwin/release/$BIN_NAME" \
        "target/x86_64-apple-darwin/release/$BIN_NAME" \
        -output "$BUILD_DIR/$BIN_NAME"
    BINARY="$BUILD_DIR/$BIN_NAME"
    echo "  ✓ Universal binary created"
else
    cargo build --release -p chess-app
    BINARY="target/release/$BIN_NAME"
    echo "  ✓ Binary compiled"
fi

# ─── Step 2: Assemble .app bundle ────────────────────────────────────────────
echo ""
echo "▶ Step 2/4: Assembling .app bundle..."

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Binary
cp "$BINARY" "$APP_BUNDLE/Contents/MacOS/$BIN_NAME"
chmod +x "$APP_BUNDLE/Contents/MacOS/$BIN_NAME"

# Info.plist
cp "$SCRIPT_DIR/Info.plist" "$APP_BUNDLE/Contents/Info.plist"

# Icon (optional)
if [ -f "$SCRIPT_DIR/AppIcon.icns" ]; then
    cp "$SCRIPT_DIR/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/AppIcon.icns"
    echo "  ✓ App icon bundled"
else
    echo "  ⚠ No AppIcon.icns in macos/ — app will use default icon"
fi

echo "  ✓ ${APP_NAME}.app ready"

# ─── Step 3: Code sign (ad-hoc) ──────────────────────────────────────────────
echo ""
echo "▶ Step 3/4: Code signing..."

if command -v codesign &>/dev/null; then
    codesign --force --deep --sign - "$APP_BUNDLE"
    echo "  ✓ Ad-hoc signed"
else
    echo "  ⚠ codesign not found — skipping"
fi

# ─── Step 4: Create DMG ──────────────────────────────────────────────────────
echo ""
echo "▶ Step 4/4: Creating .dmg..."

DMG_STAGING="$BUILD_DIR/dmg-staging"
DMG_OUTPUT="$BUILD_DIR/${DMG_NAME}.dmg"

rm -rf "$DMG_STAGING" "$DMG_OUTPUT"
mkdir -p "$DMG_STAGING"

cp -R "$APP_BUNDLE" "$DMG_STAGING/"
ln -s /Applications "$DMG_STAGING/Applications"

hdiutil create \
    -volname "$APP_NAME" \
    -srcfolder "$DMG_STAGING" \
    -ov \
    -format UDZO \
    "$DMG_OUTPUT"

rm -rf "$DMG_STAGING"

# ─── Done ─────────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  ✅ Success!"
echo ""
echo "  📦 ${DMG_OUTPUT}"
echo "  📏 $(du -sh "$DMG_OUTPUT" | cut -f1)"
echo ""
echo "  To test: open \"$DMG_OUTPUT\""
echo "════════════════════════════════════════════════════════════════"
