#!/usr/bin/env bash
#
# make-icon.sh — Convert a 1024x1024 PNG into AppIcon.icns
#
# Usage:
#   ./macos/make-icon.sh path/to/icon-1024x1024.png
#
# Requires: sips, iconutil (bundled with macOS)

set -euo pipefail

INPUT="${1:?Usage: $0 <1024x1024.png>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ICONSET="$SCRIPT_DIR/AppIcon.iconset"

rm -rf "$ICONSET"
mkdir -p "$ICONSET"

# Generate all required sizes
for SIZE in 16 32 64 128 256 512; do
    sips -z $SIZE $SIZE "$INPUT" --out "$ICONSET/icon_${SIZE}x${SIZE}.png" >/dev/null
done
for SIZE in 32 64 128 256 512 1024; do
    HALF=$((SIZE / 2))
    sips -z $SIZE $SIZE "$INPUT" --out "$ICONSET/icon_${HALF}x${HALF}@2x.png" >/dev/null
done

iconutil --convert icns "$ICONSET" --output "$SCRIPT_DIR/AppIcon.icns"
rm -rf "$ICONSET"

echo "✓ Created $SCRIPT_DIR/AppIcon.icns"
