<div align="center">

# 🐇 claude-cli-rs

**A rabbit-fast Rust reimplementation inspired by Claude Code, with native TUI, deeper tooling, and a cleaner path for terminal-first AI development.**

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

---

## Performance vs Original TypeScript/Bun

| Metric | TypeScript/Bun (v2.1.88) | Rust (claude-cli-rs) | Improvement |
|--------|--------------------------|----------------------|-------------|
| **Cold startup** | ~200 ms | ~15 ms | **13× faster** |
| **Warm startup** | ~120 ms | ~8 ms | **15× faster** |
| **Memory (idle REPL)** | ~150 MB | ~25 MB | **6× less** |
| **Memory (active session)** | ~300 MB | ~60 MB | **5× less** |
| **Binary size** | 22 MB bundle + Bun runtime | ~20 MB single static binary | **No runtime dependency** |
| **File read (10k lines)** | ~12 ms | ~1.5 ms | **8× faster** |
| **Glob (100k files)** | ~800 ms | ~120 ms | **6.7× faster** |
| **Grep (large repo)** | ~600 ms | ~80 ms | **7.5× faster** |
| **Diff generation** | ~15 ms | ~2 ms | **7.5× faster** |
| **SSE parse throughput** | ~40 MB/s | ~200 MB/s | **5× faster** |
| **First token latency** | Network bound | Network bound | ~Same |

> **Note**: numbers are projected estimates based on comparable Rust vs TypeScript benchmarks. Formal benchmarks will be added in a future release.

### Why Rust?

- **Zero-runtime deployment**: single static binary, no Node/Bun/npm required
- **Deterministic memory**: no GC pauses during streaming or tool execution
- **Native async**: Tokio task model vs Node event loop — better parallelism for concurrent tools
- **Startup**: JIT-free cold start means instant `claude --help`, instant REPL

---

## Prerequisites

### Windows

1. **Rust toolchain** (1.85+):
   - Download and run [rustup-init.exe](https://rustup.rs/)
   - During installation, you will be prompted to install **Visual Studio Build Tools** (select the "Desktop development with C++" workload). This is required for compiling Rust projects. If you already have Visual Studio 2019/2022 with the C++ desktop development components installed, you can skip this step.
   - After installation, **restart your terminal** (PowerShell / CMD) and verify:
     ```powershell
     rustc --version   # Should show rustc 1.85.0 or higher
     cargo --version
     ```

2. **Network proxy** (required if crates.io is unreachable):
   If you use a local proxy (e.g. Clash, V2Ray), configure cargo to route through it:
   ```powershell
   # Create or edit ~/.cargo/config.toml (i.e. C:\Users\<USERNAME>\.cargo\config.toml)
   # Add the following (change the port to match your proxy):
   ```
   ```toml
   [http]
   proxy = "http://127.0.0.1:10809"
   [https]
   proxy = "http://127.0.0.1:10809"
   ```
   Alternatively, use a mirror registry (no proxy needed):
   ```toml
   [source.crates-io]
   replace-with = "ustc"
   [source.ustc]
   registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"
   ```

### macOS

```bash
# Xcode command line tools (provides C linker)
xcode-select --install

# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Linux (Ubuntu/Debian)

```bash
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

---

## Installation

### From source

```bash
git clone https://github.com/liberbinjio/claude-cli-rs.git
cd claude-cli-rs
cargo build --release
```

After building, the executable is located at:
- Windows: `target\release\claude.exe`
- macOS/Linux: `target/release/claude`

Optional: install to your system PATH (so you can run `claude` from any directory):
```bash
cargo install --path crates/cli
```

### Verify Installation

```bash
# Run from the project directory (no install needed)
cargo run -- --version
# Output: claude 0.1.0

cargo run -- --help
# Output:
# Claude Code (Rust) — AI coding assistant
#
# Usage: claude.exe [OPTIONS] [PROMPT] [COMMAND]
#
# Commands:
#   self-test  Run internal diagnostics
#   help       Print this message or the help of the given subcommand(s)
#
# Arguments:
#   [PROMPT]  Initial prompt to send (non-interactive when combined with --print)
#
# Options:
#   -p, --print            Print the response and exit (non-interactive mode)
#       --model <MODEL>    Model to use [default: claude-sonnet-4-20250514]
#       --cwd <CWD>        Working directory (defaults to current directory)
#       --resume <RESUME>  Resume a previous session by ID
#   -v, --verbose          Enable verbose/debug logging
#   -h, --help             Print help
#   -V, --version          Print version

# If installed to PATH, you can run directly:
claude --version
claude --help
```

---

## Quick Start

### Set API Key

You must set an Anthropic API key before running:

**Windows PowerShell:**
```powershell
$env:ANTHROPIC_API_KEY = "sk-ant-your-key-here"
```

**Windows CMD:**
```cmd
set ANTHROPIC_API_KEY=sk-ant-your-key-here
```

**macOS / Linux:**
```bash
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

> **Tip**: To persist the key, add the command to your shell profile (PowerShell: `$PROFILE`, Bash: `~/.bashrc`, Zsh: `~/.zshrc`).

### Launch

```bash
# Interactive REPL mode (default)
cargo run

# One-shot mode (print response and exit)
cargo run -- -p "Write a Hello World in Rust"

# Specify a model
cargo run -- --model claude-sonnet-4-20250514 -p "Explain this project architecture"

# Resume a previous session
cargo run -- --resume <session-id>

# If installed to PATH:
claude
claude -p "Explain this code"
```

### Other Authentication Methods

```bash
# AWS Bedrock
# Windows PowerShell:
$env:CLAUDE_CODE_USE_BEDROCK = "1"
$env:AWS_REGION = "us-east-1"

# macOS/Linux:
export CLAUDE_CODE_USE_BEDROCK=1
export AWS_REGION=us-east-1
```

---

## Project Architecture

```
claude-cli-rs/
├── Cargo.toml                     # Workspace root
├── LICENSE                        # MIT
├── README.md                      # This file
├── rustfmt.toml                   # Code formatting
├── clippy.toml                    # Lint config
│
├── crates/
│   ├── cli/                       # Binary entry point
│   │   └── src/
│   │       ├── main.rs            # fn main — startup orchestration
│   │       ├── args.rs            # CLI argument parsing (clap)
│   │       └── setup.rs           # Initialization flow
│   │
│   ├── core/                      # Core types (shared by all crates)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── message.rs         # Message, ContentBlock, UserMessage, AssistantMessage
│   │       ├── tool.rs            # Tool trait, ToolRegistry, ToolUseContext, ToolResult
│   │       ├── permission.rs      # PermissionMode, PermissionResult, rules
│   │       ├── config.rs          # ClaudeConfig, ProjectSettings, load/save
│   │       ├── state.rs           # AppState, AppStateHandle
│   │       └── task.rs            # TaskType, TaskStatus, TaskState
│   │
│   ├── api/                       # Anthropic API client
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs          # ApiClient, provider routing
│   │       ├── streaming.rs       # SSE parser, MessageStream
│   │       ├── errors.rs          # ApiError classification
│   │       ├── retry.rs           # Exponential backoff
│   │       └── normalize.rs       # Message → API request format
│   │
│   ├── tools/                     # 28 built-in tools
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs        # ToolRegistry impl, register_all_tools()
│   │       ├── bash.rs            # BashTool — command execution
│   │       ├── grep.rs            # GrepTool — code search
│   │       ├── file_read.rs       # FileReadTool — read with line ranges
│   │       ├── file_write.rs      # FileWriteTool — atomic write
│   │       ├── file_edit.rs       # FileEditTool — precise string replacement
│   │       ├── glob.rs            # GlobTool — file matching
│   │       ├── web_fetch.rs       # WebFetchTool — HTTP + HTML→Markdown
│   │       ├── web_search.rs      # WebSearchTool — search API
│   │       ├── agent.rs           # AgentTool — sub-agent orchestration
│   │       ├── mcp_tool.rs        # MCPTool — MCP server tool proxy
│   │       ├── todo_write.rs      # TodoWriteTool
│   │       ├── lsp.rs             # LSPTool
│   │       ├── notebook_edit.rs   # NotebookEditTool
│   │       ├── task_create.rs     # TaskCreateTool
│   │       ├── task_get.rs        # TaskGetTool
│   │       ├── task_update.rs     # TaskUpdateTool
│   │       ├── task_list.rs       # TaskListTool
│   │       ├── task_stop.rs       # TaskStopTool
│   │       ├── task_output.rs     # TaskOutputTool
│   │       ├── skill.rs           # SkillTool
│   │       ├── config_tool.rs     # ConfigTool
│   │       ├── team_create.rs     # TeamCreateTool
│   │       ├── team_delete.rs     # TeamDeleteTool
│   │       ├── send_message.rs    # SendMessageTool
│   │       ├── utils.rs           # Internal helpers
│   │       ├── shared/            # Cross-tool shared logic
│   │       │   └── mod.rs
│   │       └── prompts/           # Tool description templates
│   │           ├── file_read.txt
│   │           ├── file_write.txt
│   │           ├── file_edit.txt
│   │           ├── glob.txt
│   │           ├── web_fetch.txt
│   │           ├── web_search.txt
│   │           ├── agent.txt
│   │           └── mcp_tool.txt
│   │
│   ├── query/                     # Conversation engine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── engine.rs          # QueryEngine — multi-turn orchestration
│   │       ├── query_loop.rs      # Single-turn: API→tool→result→continue
│   │       ├── compact.rs         # Context window compaction
│   │       └── system_prompt.rs   # System prompt construction
│   │
│   ├── commands/                  # Slash command system (/help, /compact, ...)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── command.rs         # Command trait, CommandContext, CommandResult
│   │       ├── registry.rs        # CommandRegistry, slash parsing
│   │       └── builtin/           # 20 built-in commands
│   │           ├── mod.rs
│   │           ├── help.rs
│   │           ├── exit.rs
│   │           ├── clear.rs
│   │           ├── compact.rs
│   │           ├── model.rs
│   │           ├── cost.rs
│   │           ├── config.rs
│   │           ├── version.rs
│   │           ├── resume.rs
│   │           ├── session.rs
│   │           ├── permissions.rs
│   │           ├── mcp.rs
│   │           ├── init.rs
│   │           ├── memory.rs
│   │           ├── diff.rs
│   │           ├── commit.rs
│   │           ├── theme.rs
│   │           ├── vim.rs
│   │           ├── status.rs
│   │           └── voice.rs
│   │
│   ├── mcp/                       # MCP (Model Context Protocol) client
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs          # McpClient — single server connection
│   │       ├── transport.rs       # stdio / SSE transport
│   │       ├── types.rs           # MCP protocol types
│   │       └── manager.rs         # McpConnectionManager — multi-server
│   │
│   ├── tui/                       # Terminal UI (ratatui + crossterm)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── terminal.rs        # Terminal init/restore, panic hook
│   │       ├── event.rs           # Event loop (Key/Mouse/Resize/Tick)
│   │       ├── theme.rs           # Color themes (light/dark/auto)
│   │       ├── app.rs             # App struct, main render loop
│   │       ├── repl.rs            # REPL layout (messages + status + input)
│   │       ├── message_view.rs    # Message list rendering
│   │       ├── prompt_input.rs    # Multi-line input widget
│   │       ├── spinner.rs         # Loading animation
│   │       ├── diff_view.rs       # Unified/side-by-side diff
│   │       ├── permission_dialog.rs # Modal permission dialog (y/n/always)
│   │       ├── onboarding.rs      # First-run onboarding flow
│   │       ├── markdown_render.rs # Markdown + syntax highlighting
│   │       ├── status_line.rs     # Bottom bar (model/cost/tokens)
│   │       └── keybindings.rs     # Vim mode (optional)
│   │
│   ├── auth/                      # Authentication
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── oauth.rs           # OAuth 2.0 PKCE flow
│   │       ├── api_key.rs         # API key management
│   │       ├── keychain.rs        # OS keychain (macOS/Windows/Linux)
│   │       └── providers.rs       # Provider routing (Anthropic/Bedrock/Vertex)
│   │
│   ├── services/                  # Application services
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── session.rs         # Session storage (JSONL save/load/list)
│   │       ├── analytics.rs       # Event tracking (no-op by default)
│   │       ├── cost.rs            # Token & cost tracking
│   │       ├── compact.rs         # Compaction strategy
│   │       ├── plugins.rs         # Plugin system skeleton
│   │       └── tips.rs            # Usage tips
│   │
│   ├── bridge/                    # Remote bridge (claude.ai WebSocket relay)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── websocket.rs       # WebSocket connection management
│   │       ├── messaging.rs       # Message protocol serialization
│   │       ├── session.rs         # Remote session creation
│   │       └── auth.rs            # Bridge JWT authentication
│   │
│   └── utils/                     # Shared utilities
│       └── src/
│           ├── lib.rs
│           ├── git.rs             # Git operations (root, diff, log, branch)
│           ├── shell.rs           # Subprocess execution, process tree kill
│           ├── platform.rs        # OS/platform detection
│           ├── fs.rs              # File I/O, binary detection, atomic write
│           ├── diff.rs            # Text diff, string replace & uniqueness
│           ├── tokens.rs          # Token count estimation
│           ├── markdown.rs        # Markdown rendering, HTML→Markdown
│           └── env.rs             # Environment variables, CI detection
│
└── tests/
    └── integration/               # End-to-end integration tests
        ├── mod.rs
        ├── helpers/
        │   ├── mod.rs
        │   └── mock_api.rs        # wiremock-based Anthropic API mock
        ├── test_cli.rs            # CLI argument tests
        ├── test_query_loop.rs     # Query loop cycle tests
        ├── test_tools.rs          # Tool execution tests
        ├── test_commands.rs       # Slash command tests
        ├── test_session.rs        # Session save/restore tests
        └── test_mcp.rs            # MCP client tests
```

---

## Crate Dependency Graph

```
cli ──┬── core
      ├── api ────── core, auth
      ├── query ──── core, api, tools, commands, utils
      ├── commands ─ core, utils
      ├── tools ──── core, utils, mcp
      ├── tui ────── core, query, utils
      ├── auth ───── core
      ├── services ─ core, api, utils
      ├── bridge ─── core, api
      ├── mcp ────── core
      └── utils ──── (standalone)
```

---

## Development

### Command Reference

```bash
cd claude-cli-rs   # All commands must be run from the project root

# === Build ===
cargo check --workspace                  # Quick type-check all crates (no binary output)
cargo build                              # Compile debug build (~7s incremental)
cargo build --release                    # Compile release build (~3 min first time, with LTO)

# === Run ===
cargo run                                # Launch CLI (runs target/debug/claude)
cargo run -- --version                   # Show version
cargo run -- --help                      # Show help
cargo run -- -p "Your question"          # One-shot mode
cargo run -- self-test                   # Run internal diagnostics

# === Test ===
cargo test --workspace                   # Run all unit tests
cargo test --workspace -- --test-threads=1  # Serial execution (avoids env var races)
cargo test -p claude_core                # Run tests for a specific crate
cargo test --test integration            # Run integration tests

# === Code Quality ===
cargo clippy --workspace -- -D warnings  # Lint (zero warnings required)
cargo fmt --all                          # Auto-format
cargo fmt --all -- --check               # Check formatting without modifying

# === Debug ===
cargo run -- -v                          # Verbose mode with detailed logging
RUST_LOG=debug cargo run                 # More detailed logging (macOS/Linux)
$env:RUST_LOG="debug"; cargo run         # More detailed logging (Windows PowerShell)
```

### Project Stats

| Metric | Value |
|--------|-------|
| Rust source files | 121 |
| Lines of code | ~13,000 |
| Unit tests | 523 |
| Crates | 12 |

---

## Troubleshooting

### `cargo build` hangs or times out downloading dependencies

You may need to configure a proxy or a mirror registry. See the **Prerequisites > Network proxy** section above.

### `link.exe not found` on Windows

Install the C++ desktop development component of Visual Studio Build Tools. Run Visual Studio Installer, click Modify, and check "Desktop development with C++".

### `cargo run` reports `error: a bin target must be available`

Ensure your root `Cargo.toml` has `default-members = ["crates/cli"]`. If it does not, run:
```bash
cargo run --bin claude
```
or specify the crate:
```bash
cargo run -p claude_cli
```

### `error[E0658]: edition 2024 is not yet stable`

Your Rust version is below 1.85. Update:
```bash
rustup update stable
```

### TUI displays garbled text or keys repeat

- Ensure your terminal supports UTF-8 (Windows Terminal is recommended; legacy CMD is not)
- Windows users should use **Windows Terminal** or **PowerShell 7**

---

## License

[MIT](LICENSE)
