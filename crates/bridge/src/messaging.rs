//! Bridge message protocol — defines the bidirectional message envelope.

use serde::{Deserialize, Serialize};

/// Messages exchanged between the CLI and the remote bridge service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BridgeMessage {
    /// Client → server: request to register with an environment.
    EnvironmentRegister {
        environment_id: String,
        capabilities: Vec<String>,
    },
    /// Server → client: confirms registration with a session id.
    Registered {
        session_id: String,
    },
    /// Server → client: request the client to execute a tool.
    ToolCall {
        request_id: String,
        tool_name: String,
        input: serde_json::Value,
    },
    /// Client → server: result of a tool execution.
    ToolResult {
        request_id: String,
        output: String,
        is_error: bool,
    },
    /// Server → client: current status information.
    Status {
        message: String,
    },
    /// Either direction: error report.
    Error {
        code: String,
        message: String,
    },
    /// Ping — keep the connection alive.
    Heartbeat,
    /// Pong — acknowledge a heartbeat.
    HeartbeatAck,
}

impl BridgeMessage {
    /// Serialize to a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (unlikely for well-formed enums).
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON does not match any `BridgeMessage` variant.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_roundtrip() {
        let msg = BridgeMessage::EnvironmentRegister {
            environment_id: "env1".to_owned(),
            capabilities: vec!["bash".to_owned(), "file_read".to_owned()],
        };
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        match parsed {
            BridgeMessage::EnvironmentRegister {
                environment_id,
                capabilities,
            } => {
                assert_eq!(environment_id, "env1");
                assert_eq!(capabilities.len(), 2);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn registered_roundtrip() {
        let msg = BridgeMessage::Registered {
            session_id: "sess-42".to_owned(),
        };
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        match parsed {
            BridgeMessage::Registered { session_id } => assert_eq!(session_id, "sess-42"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn tool_call_roundtrip() {
        let msg = BridgeMessage::ToolCall {
            request_id: "req-1".to_owned(),
            tool_name: "bash".to_owned(),
            input: serde_json::json!({"command": "ls"}),
        };
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        match parsed {
            BridgeMessage::ToolCall {
                request_id,
                tool_name,
                ..
            } => {
                assert_eq!(request_id, "req-1");
                assert_eq!(tool_name, "bash");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn tool_result_roundtrip() {
        let msg = BridgeMessage::ToolResult {
            request_id: "req-1".to_owned(),
            output: "hello world".to_owned(),
            is_error: false,
        };
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        match parsed {
            BridgeMessage::ToolResult {
                request_id,
                output,
                is_error,
            } => {
                assert_eq!(request_id, "req-1");
                assert_eq!(output, "hello world");
                assert!(!is_error);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn error_roundtrip() {
        let msg = BridgeMessage::Error {
            code: "AUTH_FAILED".to_owned(),
            message: "invalid token".to_owned(),
        };
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        match parsed {
            BridgeMessage::Error { code, message } => {
                assert_eq!(code, "AUTH_FAILED");
                assert_eq!(message, "invalid token");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn heartbeat_roundtrip() {
        let msg = BridgeMessage::Heartbeat;
        let json = msg.to_json().unwrap();
        assert!(json.contains("heartbeat"));
        let parsed = BridgeMessage::from_json(&json).unwrap();
        assert!(matches!(parsed, BridgeMessage::Heartbeat));
    }

    #[test]
    fn heartbeat_ack_roundtrip() {
        let msg = BridgeMessage::HeartbeatAck;
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        assert!(matches!(parsed, BridgeMessage::HeartbeatAck));
    }

    #[test]
    fn status_roundtrip() {
        let msg = BridgeMessage::Status {
            message: "connected".to_owned(),
        };
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        match parsed {
            BridgeMessage::Status { message } => assert_eq!(message, "connected"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn invalid_json_returns_error() {
        assert!(BridgeMessage::from_json("not json").is_err());
        assert!(BridgeMessage::from_json(r#"{"type":"unknown"}"#).is_err());
    }

    #[test]
    fn tool_result_error_flag() {
        let msg = BridgeMessage::ToolResult {
            request_id: "r1".to_owned(),
            output: "something went wrong".to_owned(),
            is_error: true,
        };
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        match parsed {
            BridgeMessage::ToolResult {
                is_error, output, ..
            } => {
                assert!(is_error);
                assert_eq!(output, "something went wrong");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn register_with_empty_capabilities() {
        let msg = BridgeMessage::EnvironmentRegister {
            environment_id: "e".to_owned(),
            capabilities: vec![],
        };
        let json = msg.to_json().unwrap();
        let parsed = BridgeMessage::from_json(&json).unwrap();
        match parsed {
            BridgeMessage::EnvironmentRegister { capabilities, .. } => {
                assert!(capabilities.is_empty());
            }
            _ => panic!("wrong variant"),
        }
    }
}
