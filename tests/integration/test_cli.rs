//! Integration tests for the CLI binary (claude_cli).
//!
//! These tests invoke the compiled binary via `cargo run -p claude_cli`
//! and verify flag parsing, output format, and exit codes.

use std::process::Command;

/// Run the CLI with given args and return (stdout, stderr, success).
fn run_cli(args: &[&str]) -> (String, String, bool) {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-q", "-p", "claude_cli", "--"]);
    cmd.args(args);
    // Set cwd to workspace root
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));

    let output = cmd.output().expect("Failed to execute cargo run");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

#[test]
fn cli_version_flag() {
    let (stdout, _stderr, success) = run_cli(&["--version"]);
    assert!(success, "--version should exit successfully");
    assert!(
        stdout.contains("0.1.0"),
        "version output should contain 0.1.0, got: {stdout}"
    );
}

#[test]
fn cli_help_flag() {
    let (stdout, _stderr, success) = run_cli(&["--help"]);
    assert!(success, "--help should exit successfully");
    // clap outputs the about text and the arguments
    assert!(
        stdout.contains("claude") || stdout.contains("Claude"),
        "help should mention claude: {stdout}"
    );
    assert!(
        stdout.contains("--print"),
        "help should list --print flag: {stdout}"
    );
    assert!(
        stdout.contains("--model"),
        "help should list --model flag: {stdout}"
    );
}

#[test]
fn cli_help_contains_subcommands() {
    let (stdout, _stderr, _) = run_cli(&["--help"]);
    assert!(
        stdout.contains("self-test"),
        "help should list self-test subcommand: {stdout}"
    );
}

#[test]
fn cli_print_mode_without_prompt_fails() {
    let (_stdout, stderr, success) = run_cli(&["--print"]);
    // --print without a prompt should bail
    assert!(
        !success,
        "--print without prompt should fail"
    );
    assert!(
        stderr.contains("prompt") || stderr.contains("requires"),
        "error should mention prompt requirement: {stderr}"
    );
}

#[test]
fn cli_unknown_flag_fails() {
    let (_stdout, stderr, success) = run_cli(&["--nonexistent-flag"]);
    assert!(!success, "unknown flag should fail");
    assert!(
        stderr.contains("unexpected") || stderr.contains("error") || stderr.contains("unrecognized"),
        "error should indicate unknown flag: {stderr}"
    );
}

#[test]
fn cli_self_test_runs() {
    let (stdout, stderr, _success) = run_cli(&["self-test"]);
    let combined = format!("{stdout}{stderr}");
    // self-test should output diagnostic info (Tools registered, Commands registered, etc.)
    assert!(
        combined.contains("Tools") || combined.contains("tool") || combined.contains("Self-test"),
        "self-test should output diagnostic info: {combined}"
    );
}
