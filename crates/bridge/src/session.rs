//! Bridge session — handles registration, event loop, and heartbeat.

use tracing::{info, warn};

use crate::auth::BridgeCredentials;
use crate::messaging::BridgeMessage;
use crate::websocket::BridgeConnection;

/// A live bridge session backed by a WebSocket connection.
pub struct BridgeSession {
    /// Session id assigned by the server (populated after a successful register).
    pub session_id: Option<String>,
    /// The underlying connection.
    pub connection: BridgeConnection,
    /// The credentials used to establish this session.
    pub credentials: BridgeCredentials,
}

impl BridgeSession {
    /// Connect to the bridge, register, and wait for a `Registered` response.
    pub async fn create(
        ws_url: &str,
        credentials: BridgeCredentials,
        capabilities: Vec<String>,
    ) -> anyhow::Result<Self> {
        let mut conn = BridgeConnection::connect(ws_url, &credentials.jwt).await?;

        // Send register message
        let register = BridgeMessage::EnvironmentRegister {
            environment_id: credentials.environment_id.clone(),
            capabilities,
        };
        conn.send_message(&register).await?;

        // Wait for registered response
        let session_id = match conn.receive_message().await? {
            Some(BridgeMessage::Registered { session_id }) => {
                info!(session_id = %session_id, "bridge session created");
                session_id
            }
            Some(BridgeMessage::Error { code, message }) => {
                anyhow::bail!("bridge registration error {code}: {message}");
            }
            other => {
                anyhow::bail!("unexpected registration response: {other:?}");
            }
        };

        Ok(Self {
            session_id: Some(session_id),
            connection: conn,
            credentials,
        })
    }

    /// Run an event loop that dispatches incoming messages to `handler`.
    ///
    /// Heartbeats are answered automatically. For every other message, `handler`
    /// is called; if it returns `Some(response)`, the response is sent back.
    pub async fn run_event_loop<F>(&mut self, mut handler: F) -> anyhow::Result<()>
    where
        F: FnMut(BridgeMessage) -> Option<BridgeMessage>,
    {
        loop {
            match self.connection.receive_message().await? {
                Some(BridgeMessage::Heartbeat) => {
                    self.connection
                        .send_message(&BridgeMessage::HeartbeatAck)
                        .await?;
                }
                Some(msg) => {
                    if let Some(response) = handler(msg) {
                        self.connection.send_message(&response).await?;
                    }
                }
                None => {
                    warn!("bridge connection closed by peer");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Gracefully close the underlying connection.
    pub async fn close(&mut self) -> anyhow::Result<()> {
        self.connection.close().await
    }
}
