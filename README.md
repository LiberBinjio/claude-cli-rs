<div align="center">

# 🐇 claude-cli-rs

**A rabbit-fast Rust reimplementation inspired by Claude Code, with native TUI, deeper tooling, and a cleaner path for terminal-first AI development.**

[![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)](https://www.rust-lang.org/)
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

> Benchmark methodology: measured on Apple M2 Pro, 16 GB RAM.  
> Tool benchmarks use real-world codebases (Linux kernel tree for glob/grep).  
> "First token latency" is network-bound and equivalent across implementations.

### Why Rust?

- **Zero-runtime deployment**: single static binary, no Node/Bun/npm required
- **Deterministic memory**: no GC pauses during streaming or tool execution
- **Native async**: Tokio task model vs Node event loop — better parallelism for concurrent tools
- **Startup**: JIT-free cold start means instant `claude --help`, instant REPL

---

## Installation

### From source (recommended)

```bash
# Prerequisites: Rust 1.85+ (https://rustup.rs)
git clone https://github.com/<your-username>/claude-cli-rs.git
cd claude-cli-rs
cargo build --release

# Binary at: target/release/claude
# Optionally install to PATH:
cargo install --path crates/cli
```

### Quick install (after first release)

```bash
cargo install claude_cli
```

### Verify installation

```bash
claude --version
# Claude Code v0.1.0

claude --help
# Claude Code - AI coding assistant
# Usage: claude [OPTIONS] [PROMPT]
```

---

## Quick Start

```bash
# Interactive REPL mode (default)
claude

# One-shot mode
claude -p "Explain this Rust project"

# With specific model
claude --model claude-sonnet-4-20250514 -p "Write a test for auth.rs"

# Resume last session
claude --resume
```

### Authentication

```bash
# Option 1: API Key (set env var)
export ANTHROPIC_API_KEY=sk-ant-...

# Option 2: OAuth login (opens browser)
claude auth login

# Option 3: AWS Bedrock
export CLAUDE_CODE_USE_BEDROCK=1
export AWS_REGION=us-east-1
```

### Configuration

```bash
# User config: ~/.claude.json
# Project config: .claude/settings.json

claude /config set theme dark
claude /config set model claude-sonnet-4-20250514
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

```bash
# Check all crates compile
cargo check --workspace

# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# Build release binary
cargo build --release

# Run directly
cargo run -- --help
cargo run -- -p "Hello, Claude"
```

### Feature flags

```toml
[features]
default = []
voice = []          # Voice input mode
kairos = []         # Enterprise features
bridge = []         # claude.ai bridge relay
coordinator = []    # Multi-agent coordinator
```

---

## Contributing

See [tasks/](../tasks/) for the complete development plan:

- [CONTRACT.md](../tasks/CONTRACT.md) — Unified API contract (all type signatures)
- [MASTER-PLAN.md](../tasks/MASTER-PLAN.md) — Task schedule & dev assignments
- [PROGRESS.md](../tasks/PROGRESS.md) — Live progress tracking

---

## License

[MIT](LICENSE)
