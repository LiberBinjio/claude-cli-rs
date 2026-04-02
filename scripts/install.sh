#!/usr/bin/env bash
# Install Rust toolchain and project dependencies for claude-cli-rs.
# Usage: ./scripts/install.sh
# Note: chmod +x scripts/install.sh before first run.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== Claude CLI (Rust) - Installation ==="
echo ""

# 1. Check/install rustup
if ! command -v rustup &>/dev/null; then
    echo "[1/5] Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
    if ! command -v rustup &>/dev/null; then
        echo "  ERROR: rustup not found after install. Source ~/.cargo/env or add ~/.cargo/bin to PATH."
        exit 1
    fi
    echo "  Rust installed successfully."
else
    echo "[1/5] rustup already installed."
fi

# 2. Ensure stable toolchain
echo "[2/5] Ensuring stable toolchain..."
rustup toolchain install stable --no-self-update 2>/dev/null
rustup default stable 2>/dev/null
echo "  Stable toolchain ready."

# 3. Components
echo "[3/5] Adding clippy + rustfmt..."
rustup component add clippy rustfmt 2>/dev/null
echo "  Components added."

# 4. Verify
echo "[4/5] Verifying installation..."
echo ""
echo "  rustc : $(rustc --version)"
echo "  cargo : $(cargo --version)"
echo "  clippy: $(cargo clippy --version 2>&1 || true)"

# 5. Fetch dependencies
echo ""
echo "[5/5] Fetching dependencies..."
cd "$PROJECT_ROOT"
cargo fetch 2>/dev/null
echo "  Dependencies fetched."

echo ""
echo "Installation complete!"
echo "Next step: ./scripts/build.sh"
