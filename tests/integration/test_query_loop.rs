//! Integration tests for `QueryEngine` (claude_query).
//!
//! Uses the mock API from helpers to create a fully wired engine and test
//! slash-command dispatch, message management, and engine lifecycle.

use crate::helpers;
use claude_query::engine::QueryEvent;
use tokio::sync::mpsc;

#[tokio::test]
async fn engine_creation_succeeds() {
    let mock = helpers::mock_api::MockAnthropicApi::new();
    let dir = tempfile::tempdir().unwrap();
    let engine = helpers::create_test_engine(&mock, dir.path().to_path_buf()).await;
    assert!(
        engine.messages().is_empty(),
        "new engine should have no messages"
    );
}

#[tokio::test]
async fn engine_clear_messages() {
    let mock = helpers::mock_api::MockAnthropicApi::new();
    let dir = tempfile::tempdir().unwrap();
    let mut engine = helpers::create_test_engine(&mock, dir.path().to_path_buf()).await;
    engine.clear_messages();
    assert!(engine.messages().is_empty());
}

#[tokio::test]
async fn engine_slash_help_dispatches() {
    let mock = helpers::mock_api::MockAnthropicApi::new();
    let dir = tempfile::tempdir().unwrap();
    let mut engine = helpers::create_test_engine(&mock, dir.path().to_path_buf()).await;

    let (tx, mut rx) = mpsc::channel::<QueryEvent>(32);
    let result = engine
        .process_user_input("/help".to_string(), tx)
        .await;
    assert!(result.is_ok(), "slash command should not error");

    // Collect events — we should get StreamDelta + QueryComplete
    let mut got_text = false;
    let mut got_complete = false;
    while let Ok(event) = rx.try_recv() {
        match event {
            QueryEvent::StreamDelta { text } => {
                assert!(
                    text.contains("help") || text.contains("Available"),
                    "help output should contain relevant text: {text}"
                );
                got_text = true;
            }
            QueryEvent::QueryComplete => {
                got_complete = true;
            }
            _ => {}
        }
    }
    assert!(got_text, "should have received StreamDelta with help text");
    assert!(got_complete, "should have received QueryComplete");
}

#[tokio::test]
async fn engine_unknown_slash_command_falls_through() {
    let mock = helpers::mock_api::MockAnthropicApi::new();
    let dir = tempfile::tempdir().unwrap();
    let mut engine = helpers::create_test_engine(&mock, dir.path().to_path_buf()).await;

    let (tx, mut rx) = mpsc::channel::<QueryEvent>(32);
    // Unknown slash command should fall through to a regular query
    // (which will fail against the mock, but that's OK — we test the dispatch logic)
    let _ = engine
        .process_user_input("/nonexistent_command_xyz".to_string(), tx)
        .await;

    // Since this falls through to run_query against a mock that returns nothing useful,
    // we should get an Error or QueryComplete event (not a slash command response).
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    // At minimum, the engine should have added a user message
    assert!(
        !engine.messages().is_empty(),
        "unknown slash command should be treated as user input"
    );
}

#[tokio::test]
async fn engine_regular_text_adds_message() {
    let mock = helpers::mock_api::MockAnthropicApi::new();
    let dir = tempfile::tempdir().unwrap();
    let mut engine = helpers::create_test_engine(&mock, dir.path().to_path_buf()).await;

    let (tx, _rx) = mpsc::channel::<QueryEvent>(32);
    // Regular text (no slash) should go to the API path
    // It will error because mock returns nothing, but a message should be added
    let _ = engine
        .process_user_input("hello world".to_string(), tx)
        .await;

    assert!(
        !engine.messages().is_empty(),
        "regular input should add a user message"
    );
}
