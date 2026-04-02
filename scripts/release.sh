#!/usr/bin/env bash
# Build an optimized release binary and package it for distribution.
# Usage: ./scripts/release.sh
# Note: chmod +x scripts/release.sh if needed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_ROOT/dist"

echo "=== Claude CLI (Rust) — Release Build ==="

# 1. Build release
echo "[1/4] Building release binary..."
cd "$PROJECT_ROOT"
cargo build --release

# Locate binary
BIN_SRC="$PROJECT_ROOT/target/release/claude"
if [ ! -f "$BIN_SRC" ]; then
    BIN_SRC="$PROJECT_ROOT/target/release/claude-cli-rs"
fi
if [ ! -f "$BIN_SRC" ]; then
    echo "ERROR: Release binary not found in target/release/"
    echo "  Looked for: claude, claude-cli-rs"
    exit 1
fi

# 2. Strip binary (Linux/macOS only)
echo "[2/4] Stripping binary..."
if command -v strip &>/dev/null; then
    strip "$BIN_SRC"
    echo "  Stripped successfully"
else
    echo "  strip not available, skipping"
fi

# 3. Prepare dist
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp "$BIN_SRC" "$DIST_DIR/claude"
chmod +x "$DIST_DIR/claude"

SIZE=$(du -h "$DIST_DIR/claude" | cut -f1)
echo "  Binary: $DIST_DIR/claude ($SIZE)"

# 4. Archive
echo "[3/4] Creating archive..."
VERSION="0.1.0"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
ARCHIVE_NAME="claude-cli-rs-v${VERSION}-${OS}-${ARCH}.tar.gz"

cd "$DIST_DIR"
tar czf "$ARCHIVE_NAME" claude
ARCHIVE_SIZE=$(du -h "$ARCHIVE_NAME" | cut -f1)
echo "  Archive: $DIST_DIR/$ARCHIVE_NAME ($ARCHIVE_SIZE)"

# SHA256
echo "[4/4] Computing checksum..."
if command -v sha256sum &>/dev/null; then
    HASH=$(sha256sum "$ARCHIVE_NAME" | cut -d' ' -f1)
elif command -v shasum &>/dev/null; then
    HASH=$(shasum -a 256 "$ARCHIVE_NAME" | cut -d' ' -f1)
else
    HASH="(sha256sum not available)"
fi
echo "$HASH  $ARCHIVE_NAME" > SHA256SUMS.txt
echo "  SHA256: $HASH"

echo ""
echo "Release artifacts in: $DIST_DIR"
ls -lh "$DIST_DIR"
echo ""
echo "Done!"
