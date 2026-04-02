#!/usr/bin/env bash
# Build claude-cli-rs in debug or release mode.
# Usage: ./scripts/build.sh [--release|-r]
# Note: chmod +x scripts/build.sh before first run.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== Claude CLI (Rust) - Build ==="

cd "$PROJECT_ROOT"

RELEASE=false
for arg in "$@"; do
    case "$arg" in
        --release|-r) RELEASE=true ;;
        *) echo "Unknown argument: $arg"; exit 1 ;;
    esac
done

if [ "$RELEASE" = true ]; then
    echo "Building release..."
    cargo build --release
    BIN="$PROJECT_ROOT/target/release/claude"
    ALT_BIN="$PROJECT_ROOT/target/release/claude-cli-rs"
else
    echo "Building debug..."
    cargo build
    BIN="$PROJECT_ROOT/target/debug/claude"
    ALT_BIN="$PROJECT_ROOT/target/debug/claude-cli-rs"
fi

# Find and report binary
if [ -f "$BIN" ]; then
    SIZE=$(du -h "$BIN" | cut -f1)
    echo ""
    echo "  Binary: $BIN"
    echo "  Size:   $SIZE"
elif [ -f "$ALT_BIN" ]; then
    SIZE=$(du -h "$ALT_BIN" | cut -f1)
    echo ""
    echo "  Binary: $ALT_BIN"
    echo "  Size:   $SIZE"
else
    echo "  Warning: Binary not found at expected path."
fi

echo ""
echo "Build complete!"
