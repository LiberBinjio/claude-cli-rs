//! Claude Code (Rust) — CLI entry point.

mod args;
mod setup;

use std::time::Duration;

use clap::Parser;
use tokio::sync::{mpsc, oneshot};

use claude_query::engine::QueryEvent;
use claude_tui::app::{App, AppScreen};
use claude_tui::event::{is_quit_key, Event, EventLoop};
use claude_tui::message_view::{DisplayMessage, MessageRole};
use claude_tui::terminal;

use args::{CliArgs, CliCommand};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    // Logging
    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("claude=debug")
            .init();
    }

    // Handle subcommands
    if let Some(CliCommand::SelfTest) = &args.command {
        return run_self_test(&args);
    }

    if args.print {
        return run_print_mode(&args).await;
    }

    run_interactive_mode(&args).await
}

/// Non-interactive mode: send one prompt, stream output to stdout, then exit.
async fn run_print_mode(args: &CliArgs) -> anyhow::Result<()> {
    let prompt = args
        .prompt
        .as_deref()
        .unwrap_or("")
        .to_string();
    if prompt.is_empty() {
        anyhow::bail!("--print mode requires a prompt argument");
    }

    let services = setup::setup(args)?;
    let mut engine = services.engine;
    let (tx, mut rx) = mpsc::channel::<QueryEvent>(256);

    let handle = tokio::spawn(async move {
        let _ = engine.process_user_input(prompt, tx).await;
    });

    while let Some(event) = rx.recv().await {
        match event {
            QueryEvent::StreamDelta { text } => print!("{text}"),
            QueryEvent::ToolStart { tool_name, .. } => {
                eprintln!("\n[tool: {tool_name}]");
            }
            QueryEvent::ToolEnd {
                result, is_error, ..
            } => {
                if is_error {
                    eprintln!("[tool error: {result}]");
                }
            }
            QueryEvent::Error { message } => {
                eprintln!("\nError: {message}");
            }
            QueryEvent::QueryComplete => {
                println!();
                break;
            }
        }
    }

    handle.await?;
    Ok(())
}

/// Interactive TUI mode: launch the full REPL interface.
async fn run_interactive_mode(args: &CliArgs) -> anyhow::Result<()> {
    terminal::install_panic_hook();

    let mut tui = terminal::init()?;
    let mut app = App::new();
    let mut events = EventLoop::new(Duration::from_millis(250));

    // Try to set up services (auth may fail)
    let services = match setup::setup(args) {
        Ok(s) => {
            app.screen = AppScreen::Repl;
            Some(s)
        }
        Err(e) => {
            app.screen = AppScreen::Repl;
            app.repl.messages.push(DisplayMessage {
                role: MessageRole::System,
                text: format!(
                    "Initialization failed: {e}\n\
                     \n\
                     Choose an authentication method:\n\
                     \n\
                     [Option 1] Set ANTHROPIC_API_KEY environment variable\n\
                     [Option 2] Run `claude auth login` for OAuth (browser-based)\n\
                     [Option 3] Use GitHub Copilot: restart with `claude --copilot`\n\
                     \n\
                     Option 3 requires VS Code + Agent Maestro extension running.\n\
                     Set ANTHROPIC_API_KEY or run `claude login`.\n\
                     \n\
                     Set ANTHROPIC_API_KEY and restart."
                ),
                tool_info: None,
                timestamp: 0.0,
            });
            None
        }
    };

    let mut engine = services.map(|s| s.engine);

    // Channel for QueryEngine events
    let (query_tx, mut query_rx) = mpsc::channel::<QueryEvent>(256);

    // Engine return channel (for getting the engine back after spawned query)
    let mut engine_return: Option<oneshot::Receiver<_>> = None;
    let mut query_in_progress = false;

    // If a prompt was provided on the CLI, submit it immediately
    let initial_prompt = args.prompt.clone();
    let mut initial_submitted = false;

    loop {
        // Render
        tui.draw(|frame| app.render(frame))?;

        // Submit initial prompt on first iteration
        if !initial_submitted {
            initial_submitted = true;
            if let Some(prompt) = initial_prompt.clone() {
                if !prompt.trim().is_empty() {
                    if let Some(eng) = engine.take() {
                        app.repl.messages.push(DisplayMessage {
                            role: MessageRole::User,
                            text: prompt.clone(),
                            tool_info: None,
                            timestamp: 0.0,
                        });
                        app.repl.is_loading = true;
                        query_in_progress = true;

                        let tx = query_tx.clone();
                        let (ret_tx, ret_rx) = oneshot::channel();
                        engine_return = Some(ret_rx);

                        tokio::spawn(async move {
                            let mut eng = eng;
                            let _ = eng.process_user_input(prompt, tx).await;
                            let _ = ret_tx.send(eng);
                        });
                    }
                }
            }
        }

        // Check if engine has been returned from a completed query
        if let Some(ref mut rx) = engine_return {
            if let Ok(eng) = rx.try_recv() {
                engine = Some(eng);
                engine_return = None;
            }
        }

        tokio::select! {
            event = events.next() => {
                let Some(event) = event else { break };
                match event {
                    Event::Key(key) => {
                        if is_quit_key(&key) && !query_in_progress {
                            break;
                        }

                        // Let the input widget process the key
                        let submitted = app.repl.input.handle_key(key);

                        if submitted && !query_in_progress {
                            let input = app.repl.input.submit();
                            if !input.trim().is_empty() {
                                if let Some(eng) = engine.take() {
                                    app.repl.messages.push(DisplayMessage {
                                        role: MessageRole::User,
                                        text: input.clone(),
                                        tool_info: None,
                                        timestamp: 0.0,
                                    });
                                    app.repl.is_loading = true;
                                    query_in_progress = true;

                                    let tx = query_tx.clone();
                                    let (ret_tx, ret_rx) = oneshot::channel();
                                    engine_return = Some(ret_rx);

                                    tokio::spawn(async move {
                                        let mut eng = eng;
                                        let _ = eng.process_user_input(input, tx).await;
                                        let _ = ret_tx.send(eng);
                                    });
                                } else {
                                    app.repl.messages.push(DisplayMessage {
                                        role: MessageRole::System,
                                        text: "Engine not available. Please wait or restart.".into(),
                                        tool_info: None,
                                        timestamp: 0.0,
                                    });
                                }
                            }
                        }
                    }
                    Event::Tick => {
                        app.tick();
                    }
                    _ => {}
                }
            }
            // Query events from the engine
            event = query_rx.recv(), if query_in_progress => {
                if let Some(qe) = event {
                    match qe {
                        QueryEvent::StreamDelta { text } => {
                            app.repl.messages.append_streaming(&text);
                        }
                        QueryEvent::ToolStart { tool_name, .. } => {
                            app.repl.messages.push(DisplayMessage {
                                role: MessageRole::System,
                                text: format!("Running tool: {tool_name}"),
                                tool_info: Some(tool_name),
                                timestamp: 0.0,
                            });
                        }
                        QueryEvent::ToolEnd { result, is_error, .. } => {
                            if is_error {
                                app.repl.messages.push(DisplayMessage {
                                    role: MessageRole::ToolResult,
                                    text: format!("Error: {result}"),
                                    tool_info: None,
                                    timestamp: 0.0,
                                });
                            }
                        }
                        QueryEvent::Error { message } => {
                            app.repl.messages.finish_streaming();
                            app.repl.messages.push(DisplayMessage {
                                role: MessageRole::System,
                                text: format!("Error: {message}"),
                                tool_info: None,
                                timestamp: 0.0,
                            });
                            app.repl.is_loading = false;
                            query_in_progress = false;
                        }
                        QueryEvent::QueryComplete => {
                            app.repl.messages.finish_streaming();
                            app.repl.is_loading = false;
                            query_in_progress = false;
                        }
                    }
                }
            }
        }
    }

    terminal::restore()?;
    println!("Goodbye!");
    Ok(())
}

/// Run internal diagnostics: auth, tools, configuration.
fn run_self_test(args: &CliArgs) -> anyhow::Result<()> {
    println!("Claude Code (Rust) — Self-Test Diagnostics");
    println!("==========================================");

    // 1. Auth check
    print!("Auth: ");
    match claude_auth::providers::resolve_api_provider() {
        Ok(provider) => {
            let kind = format!("{provider:?}");
            let label = kind.split('{').next().unwrap_or("Unknown").trim();
            println!("OK ({label})");
        }
        Err(e) => println!("FAIL ({e})"),
    }

    // 2. Config check
    let config = claude_core::config::AppConfig::default();
    println!("Model: {}", config.model);
    println!("Small model: {}", config.small_fast_model);

    // 3. Working directory
    let cwd = match &args.cwd {
        Some(dir) => std::path::PathBuf::from(dir),
        None => std::env::current_dir()?,
    };
    println!("CWD: {}", cwd.display());

    // 4. Tools
    let tool_set = setup::create_tool_set();
    println!("Tools registered: {}", tool_set.len());

    // 5. Commands
    let cmd_registry = setup::create_command_registry();
    println!("Commands registered: {}", cmd_registry.all().len());

    println!("==========================================");
    println!("Self-test complete.");
    Ok(())
}
