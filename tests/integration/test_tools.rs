//! Integration tests for tool execution (claude_tools).
//!
//! Uses real tool implementations with a temp directory to verify
//! tool registration, lookup, and execution end-to-end.

use crate::helpers;
use claude_tools::ToolRegistry;

fn setup_full_registry() -> ToolRegistry {
    claude_tools::create_default_registry()
}

// --- Registration tests ---------------------------------------------------

#[test]
fn default_registry_has_p0_tools() {
    let r = setup_full_registry();
    assert!(r.find("Bash").is_some(), "Bash should be registered");
    assert!(r.find("Grep").is_some(), "Grep should be registered");
}

#[test]
fn default_registry_has_p0b_tools() {
    let r = setup_full_registry();
    assert!(r.find("FileRead").is_some(), "FileRead should be registered");
    assert!(r.find("FileWrite").is_some(), "FileWrite should be registered");
    assert!(r.find("FileEdit").is_some(), "FileEdit should be registered");
    assert!(r.find("Glob").is_some(), "Glob should be registered");
}

#[test]
fn default_registry_has_at_least_20_tools() {
    let r = setup_full_registry();
    assert!(
        r.all().len() >= 20,
        "expected at least 20 tools, got {}",
        r.all().len()
    );
}

#[test]
fn all_tools_have_schemas() {
    let r = setup_full_registry();
    for tool in r.all() {
        let schema = tool.input_schema();
        assert!(
            schema.is_object(),
            "tool '{}' should have object schema, got: {}",
            tool.name(),
            schema
        );
    }
}

// --- Execution tests (real tools, temp dir) --------------------------------

#[tokio::test]
async fn bash_tool_echo() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = helpers::create_tool_context(dir.path());
    let r = setup_full_registry();
    let bash = r.find("Bash").expect("Bash tool");
    let input = serde_json::json!({ "command": "echo hello" });
    let result = bash.call(input, &mut ctx).await.unwrap();
    assert!(!result.is_error, "echo should succeed");
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("hello"),
        "echo output should contain 'hello': {text}"
    );
}

#[tokio::test]
async fn file_write_and_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = helpers::create_tool_context(dir.path());
    let r = setup_full_registry();

    // Write
    let write_tool = r.find("FileWrite").expect("FileWrite tool");
    let file_path = dir.path().join("test_roundtrip.txt");
    let input = serde_json::json!({
        "path": file_path.to_str().unwrap(),
        "content": "Hello, integration test!\nLine two."
    });
    let result = write_tool.call(input, &mut ctx).await.unwrap();
    assert!(!result.is_error, "write should succeed");

    // Read
    let read_tool = r.find("FileRead").expect("FileRead tool");
    let input = serde_json::json!({ "path": file_path.to_str().unwrap() });
    let result = read_tool.call(input, &mut ctx).await.unwrap();
    assert!(!result.is_error, "read should succeed");
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("Hello, integration test!"),
        "read should return written content: {text}"
    );
}

#[tokio::test]
async fn glob_tool_finds_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = helpers::create_tool_context(dir.path());
    // Create some files
    std::fs::write(dir.path().join("a.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn test() {}").unwrap();
    std::fs::write(dir.path().join("c.txt"), "not rust").unwrap();

    let r = setup_full_registry();
    let glob_tool = r.find("Glob").expect("Glob tool");
    let input = serde_json::json!({ "pattern": "*.rs" });
    let result = glob_tool.call(input, &mut ctx).await.unwrap();
    assert!(!result.is_error);
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(text.contains("a.rs"), "should find a.rs: {text}");
    assert!(text.contains("b.rs"), "should find b.rs: {text}");
    assert!(!text.contains("c.txt"), "should not find c.txt: {text}");
}

#[tokio::test]
async fn file_read_nonexistent_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = helpers::create_tool_context(dir.path());
    let r = setup_full_registry();
    let read_tool = r.find("FileRead").expect("FileRead tool");
    let input = serde_json::json!({ "path": "/tmp/nonexistent_file_12345.txt" });
    let result = read_tool.call(input, &mut ctx).await.unwrap();
    assert!(result.is_error, "reading nonexistent file should error");
}

#[tokio::test]
#[ignore] // Requires rg (ripgrep); findstr fallback has Windows quoting issues
async fn grep_tool_finds_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = helpers::create_tool_context(dir.path());
    // Create a file with known content
    std::fs::write(dir.path().join("search_target.txt"), "line one\nfind_me_here\nline three\n")
        .unwrap();

    let r = setup_full_registry();
    let grep_tool = r.find("Grep").expect("Grep tool");
    let input = serde_json::json!({
        "pattern": "find_me_here",
        "path": dir.path().to_str().unwrap()
    });
    let result = grep_tool.call(input, &mut ctx).await.unwrap();
    assert!(!result.is_error, "grep should succeed");
    let text = result.content[0].text.as_deref().unwrap_or("");
    assert!(
        text.contains("find_me_here"),
        "grep should find the pattern: {text}"
    );
}

#[test]
fn grep_tool_has_valid_schema() {
    let r = setup_full_registry();
    let grep = r.find("Grep").expect("Grep tool");
    let schema = grep.input_schema();
    let props = schema.get("properties").expect("schema must have properties");
    assert!(props.get("pattern").is_some(), "schema needs 'pattern' property");
}
