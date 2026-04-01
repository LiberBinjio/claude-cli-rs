//! WebSocket connection management for the bridge relay.

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::debug;

use crate::messaging::BridgeMessage;

/// A WebSocket connection to the bridge relay service.
pub struct BridgeConnection {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    /// Whether the connection is still open.
    pub connected: bool,
}

impl BridgeConnection {
    /// Open a WebSocket connection to the given URL, passing the JWT as a query
    /// parameter.
    ///
    /// # Security
    ///
    /// Only `ws://` and `wss://` schemes are accepted.
    pub async fn connect(url: &str, jwt: &str) -> anyhow::Result<Self> {
        if !url.starts_with("wss://") && !url.starts_with("ws://") {
            anyhow::bail!("bridge URL must use ws:// or wss:// scheme");
        }

        let url_with_auth = format!("{url}?token={jwt}");
        let (ws, _response) = connect_async(&url_with_auth).await?;
        debug!("bridge WebSocket connected");

        Ok(Self {
            ws,
            connected: true,
        })
    }

    /// Send a [`BridgeMessage`] as a text frame.
    pub async fn send_message(&mut self, msg: &BridgeMessage) -> anyhow::Result<()> {
        let json = msg.to_json()?;
        self.ws.send(WsMessage::Text(json)).await?;
        Ok(())
    }

    /// Receive the next [`BridgeMessage`].
    ///
    /// Returns `None` when the peer closes the connection.
    /// Automatically responds to WebSocket `Ping` frames with `Pong`.
    pub async fn receive_message(&mut self) -> anyhow::Result<Option<BridgeMessage>> {
        loop {
            match self.ws.next().await {
                Some(Ok(WsMessage::Text(text))) => {
                    let msg = BridgeMessage::from_json(&text)?;
                    return Ok(Some(msg));
                }
                Some(Ok(WsMessage::Close(_))) => {
                    self.connected = false;
                    return Ok(None);
                }
                Some(Ok(WsMessage::Ping(data))) => {
                    self.ws.send(WsMessage::Pong(data)).await?;
                    // continue reading the next real message
                }
                Some(Ok(_)) => {
                    // ignore Binary, Pong, Frame
                }
                Some(Err(e)) => {
                    self.connected = false;
                    return Err(e.into());
                }
                None => {
                    self.connected = false;
                    return Ok(None);
                }
            }
        }
    }

    /// Send a raw text frame (e.g. pre-serialized JSON).
    pub async fn send_raw(&mut self, text: &str) -> anyhow::Result<()> {
        self.ws
            .send(WsMessage::Text(text.to_owned()))
            .await?;
        Ok(())
    }

    /// Gracefully close the WebSocket connection.
    pub async fn close(&mut self) -> anyhow::Result<()> {
        self.ws.close(None).await?;
        self.connected = false;
        Ok(())
    }
}
