//! Integration tests for bridge session/auth types (claude_bridge).

use claude_bridge::{BridgeCredentials, BridgeMessage};

// --- auth tests -----------------------------------------------------------

#[test]
fn credentials_expired() {
    let c = BridgeCredentials {
        jwt: "x".to_owned(),
        environment_id: "e".to_owned(),
        expires_at: 0,
    };
    assert!(c.is_expired());
}

#[test]
fn credentials_not_expired() {
    let c = BridgeCredentials {
        jwt: "x".to_owned(),
        environment_id: "e".to_owned(),
        expires_at: u64::MAX,
    };
    assert!(!c.is_expired());
}

#[test]
fn credentials_serde_roundtrip() {
    let c = BridgeCredentials {
        jwt: "abc.def.ghi".to_owned(),
        environment_id: "env-1".to_owned(),
        expires_at: 1_700_000_000,
    };
    let json = serde_json::to_string(&c).unwrap();
    let parsed: BridgeCredentials = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.jwt, c.jwt);
    assert_eq!(parsed.expires_at, c.expires_at);
}

// --- messaging tests ------------------------------------------------------

#[test]
fn heartbeat_roundtrip() {
    let msg = BridgeMessage::Heartbeat;
    let json = msg.to_json().unwrap();
    let parsed = BridgeMessage::from_json(&json).unwrap();
    assert!(matches!(parsed, BridgeMessage::Heartbeat));
}

#[test]
fn tool_call_roundtrip() {
    let msg = BridgeMessage::ToolCall {
        request_id: "r1".to_owned(),
        tool_name: "bash".to_owned(),
        input: serde_json::json!({"cmd": "ls"}),
    };
    let json = msg.to_json().unwrap();
    let parsed = BridgeMessage::from_json(&json).unwrap();
    match parsed {
        BridgeMessage::ToolCall {
            request_id,
            tool_name,
            ..
        } => {
            assert_eq!(request_id, "r1");
            assert_eq!(tool_name, "bash");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn error_message_roundtrip() {
    let msg = BridgeMessage::Error {
        code: "FORBIDDEN".to_owned(),
        message: "access denied".to_owned(),
    };
    let json = msg.to_json().unwrap();
    let parsed = BridgeMessage::from_json(&json).unwrap();
    match parsed {
        BridgeMessage::Error { code, message } => {
            assert_eq!(code, "FORBIDDEN");
            assert_eq!(message, "access denied");
        }
        _ => panic!("wrong variant"),
    }
}
