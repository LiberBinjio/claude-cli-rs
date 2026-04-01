//! Integration tests for the command system (claude_commands).

use claude_commands::builtin::register_builtins;
use claude_commands::{CommandContext, CommandRegistry, CommandResult};

fn setup_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    register_builtins(&mut registry);
    registry
}

fn setup_context() -> CommandContext {
    CommandContext {
        placeholder_state: (),
        event_tx: None,
    }
}

// --- find tests -----------------------------------------------------------

#[test]
fn find_help_by_name() {
    let r = setup_registry();
    let (cmd, _) = r.find("/help").expect("should find help");
    assert_eq!(cmd.name(), "help");
}

#[test]
fn find_help_alias_h() {
    let r = setup_registry();
    assert!(r.find("/h").is_some());
}

#[test]
fn find_help_alias_question_mark() {
    let r = setup_registry();
    assert!(r.find("/?").is_some());
}

#[test]
fn find_no_slash_returns_none() {
    let r = setup_registry();
    assert!(r.find("help").is_none());
}

#[test]
fn find_unknown_command_returns_none() {
    let r = setup_registry();
    assert!(r.find("/nonexistent_command").is_none());
}

#[test]
fn find_case_insensitive() {
    let r = setup_registry();
    assert!(r.find("/HELP").is_some());
    assert!(r.find("/Help").is_some());
}

#[test]
fn find_with_args() {
    let r = setup_registry();
    let (_, args) = r.find("/help version").unwrap();
    assert_eq!(args, "version");
}

// --- edge cases (optimization) -------------------------------------------

#[test]
fn find_empty_string_returns_none() {
    let r = setup_registry();
    assert!(r.find("").is_none());
}

#[test]
fn find_slash_only_returns_none() {
    let r = setup_registry();
    assert!(r.find("/").is_none());
}

#[test]
fn find_very_long_command_returns_none() {
    let r = setup_registry();
    let long = format!("/{}", "a".repeat(5000));
    assert!(r.find(&long).is_none());
}

// --- execute tests --------------------------------------------------------

#[tokio::test]
async fn help_no_args_returns_overview() {
    let r = setup_registry();
    let mut ctx = setup_context();
    let (cmd, args) = r.find("/help").unwrap();
    let result = cmd.execute(&args, &mut ctx).await.unwrap();
    match result {
        CommandResult::Handled(Some(text)) => {
            assert!(text.contains("Available commands"));
            assert!(text.contains("help"));
        }
        _ => panic!("Expected Handled(Some(...))"),
    }
}

#[tokio::test]
async fn help_with_arg_returns_specific_help() {
    let r = setup_registry();
    let mut ctx = setup_context();
    let (cmd, args) = r.find("/help exit").unwrap();
    let result = cmd.execute(&args, &mut ctx).await.unwrap();
    match result {
        CommandResult::Handled(Some(text)) => {
            assert!(text.contains("exit"));
        }
        _ => panic!("Expected Handled(Some(...))"),
    }
}

// --- registry structure ---------------------------------------------------

#[test]
fn all_builtins_have_descriptions() {
    let r = setup_registry();
    for cmd in r.all() {
        assert!(
            !cmd.description().is_empty(),
            "command '{}' has empty description",
            cmd.name()
        );
    }
}

#[test]
fn visible_subset_of_all() {
    let r = setup_registry();
    assert!(r.visible().len() <= r.all().len());
}

#[test]
fn all_registered_commands_have_unique_names() {
    let r = setup_registry();
    let names: Vec<&str> = r.all().iter().map(|c| c.name()).collect();
    let mut deduped = names.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(names.len(), deduped.len(), "duplicate command names found");
}
