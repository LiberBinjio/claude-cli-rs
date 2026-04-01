//! Claude Code — bridge (WebSocket relay, JWT auth, remote sessions).

pub mod auth;
pub mod messaging;
pub mod session;
pub mod websocket;

pub use auth::BridgeCredentials;
pub use messaging::BridgeMessage;
pub use session::BridgeSession;
pub use websocket::BridgeConnection;
