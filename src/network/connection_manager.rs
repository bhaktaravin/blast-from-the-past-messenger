//! Connection manager for handling resilient WebSocket connections
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use url::Url;

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Manages WebSocket connections with automatic reconnection
pub struct ConnectionManager {
    url: String,
    state: Arc<Mutex<ConnectionState>>,
    max_retries: u32,
    retry_delay: Duration,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(url: String) -> Self {
        Self {
            url,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            max_retries: 5,
            retry_delay: Duration::from_secs(5),
        }
    }

    /// Set maximum number of reconnection attempts
    pub fn set_max_retries(&mut self, max_retries: u32) {
        self.max_retries = max_retries;
    }

    /// Set delay between reconnection attempts
    pub fn set_retry_delay(&mut self, delay: Duration) {
        self.retry_delay = delay;
    }

    /// Get current connection state
    pub async fn get_state(&self) -> ConnectionState {
        self.state.lock().await.clone()
    }

    /// Connect to the WebSocket server with automatic reconnection
    pub async fn connect_with_retry(
        &self,
    ) -> Result<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let mut retries = 0;

        loop {
            {
                let mut state = self.state.lock().await;
                *state = ConnectionState::Connecting;
            }

            match self.attempt_connection().await {
                Ok(stream) => {
                    {
                        let mut state = self.state.lock().await;
                        *state = ConnectionState::Connected;
                    }
                    return Ok(stream);
                }
                Err(e) => {
                    {
                        let mut state = self.state.lock().await;
                        *state = ConnectionState::Disconnected;
                    }

                    if retries >= self.max_retries {
                        return Err(format!(
                            "Failed to connect after {} retries: {}",
                            self.max_retries, e
                        )
                        .into());
                    }

                    retries += 1;
                    eprintln!(
                        "Connection attempt {} failed: {}. Retrying in {:?}...",
                        retries, e, self.retry_delay
                    );

                    {
                        let mut state = self.state.lock().await;
                        *state = ConnectionState::Reconnecting;
                    }

                    sleep(self.retry_delay).await;
                }
            }
        }
    }

    /// Attempt a single connection
    async fn attempt_connection(
        &self,
    ) -> Result<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let url = Url::parse(&self.url)?;
        let (ws_stream, _) = connect_async(url).await?;
        Ok(ws_stream)
    }

    /// Send a message with retry logic
    pub async fn send_message_with_retry(
        &self,
        ws_stream: &mut WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        message: Message,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut retries = 0;
        let max_send_retries = 3;

        loop {
            match ws_stream.send(message.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if retries >= max_send_retries {
                        return Err(format!(
                            "Failed to send message after {} retries: {}",
                            max_send_retries, e
                        )
                        .into());
                    }

                    retries += 1;
                    eprintln!(
                        "Failed to send message (attempt {}): {}. Retrying...",
                        retries, e
                    );
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Listen for messages with automatic reconnection
    pub async fn listen_with_reconnect<F, Fut>(
        &self,
        mut message_handler: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(Message) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        loop {
            match self.connect_with_retry().await {
                Ok(mut ws_stream) => {
                    println!("Connected to server successfully");

                    // Listen for messages
                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(message) => {
                                message_handler(message).await;
                            }
                            Err(e) => {
                                eprintln!("WebSocket error: {}", e);
                                break; // Break to trigger reconnection
                            }
                        }
                    }

                    // If we get here, the connection was lost
                    {
                        let mut state = self.state.lock().await;
                        *state = ConnectionState::Reconnecting;
                    }
                    eprintln!("Connection lost, attempting to reconnect...");
                }
                Err(e) => {
                    eprintln!("Failed to establish connection: {}", e);
                    {
                        let mut state = self.state.lock().await;
                        *state = ConnectionState::Disconnected;
                    }
                    sleep(self.retry_delay).await;
                }
            }
        }
    }

    /// Send periodic heartbeat to keep connection alive
    pub async fn send_heartbeat(
        &self,
        ws_stream: &mut WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let heartbeat_msg = Message::Ping(vec![]);
        self.send_message_with_retry(ws_stream, heartbeat_msg).await
    }
}
