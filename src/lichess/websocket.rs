use super::events::{LichessEvent, LichessMessage, MoveRequest};
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

pub struct WebSocketHandler {
    url: String,
    state: ConnectionState,
    reconnect_attempts: u8,
    max_reconnect_attempts: u8,
    event_tx: mpsc::UnboundedSender<LichessEvent>,
}

impl WebSocketHandler {
    pub fn new(url: String, event_tx: mpsc::UnboundedSender<LichessEvent>) -> Self {
        WebSocketHandler {
            url,
            state: ConnectionState::Disconnected,
            reconnect_attempts: 0,
            max_reconnect_attempts: 10,
            event_tx,
        }
    }
    
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }
    
    pub fn state(&self) -> ConnectionState {
        self.state
    }
    
    pub async fn connect(&mut self) -> Result<WsStream> {
        self.state = ConnectionState::Connecting;
        info!("Connecting to WebSocket: {}", self.url);
        
        let (ws_stream, _) = connect_async(&self.url).await
            .map_err(|e| anyhow!("Failed to connect to WebSocket: {}", e))?;
        
        self.state = ConnectionState::Connected;
        self.reconnect_attempts = 0;
        info!("WebSocket connected successfully");
        
        let _ = self.event_tx.send(LichessEvent::Connected);
        
        Ok(ws_stream)
    }
    
    pub async fn reconnect(&mut self) -> Result<WsStream> {
        self.reconnect_attempts += 1;
        
        if self.reconnect_attempts > self.max_reconnect_attempts {
            return Err(anyhow!("Max reconnection attempts reached"));
        }
        
        self.state = ConnectionState::Reconnecting;
        warn!("Reconnecting (attempt {})", self.reconnect_attempts);
        
        // Exponential backoff
        let delay = std::time::Duration::from_millis(500 * 2u64.pow(self.reconnect_attempts as u32));
        tokio::time::sleep(delay).await;
        
        self.connect().await
    }
    
    pub async fn handle_connection(mut self) -> Result<()> {
        let mut ws_stream = self.connect().await?;
        
        loop {
            tokio::select! {
                msg = ws_stream.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            debug!("Received message: {}", text);
                            if let Err(e) = self.handle_message(&text).await {
                                error!("Error handling message: {}", e);
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("WebSocket closed by server");
                            let _ = self.event_tx.send(LichessEvent::Disconnected);
                            self.state = ConnectionState::Disconnected;
                            
                            // Attempt reconnection
                            match self.reconnect().await {
                                Ok(new_stream) => {
                                    ws_stream = new_stream;
                                    continue;
                                }
                                Err(e) => {
                                    error!("Reconnection failed: {}", e);
                                    return Err(e);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            let _ = self.event_tx.send(LichessEvent::Error(e.to_string()));
                            self.state = ConnectionState::Disconnected;
                            
                            // Attempt reconnection
                            match self.reconnect().await {
                                Ok(new_stream) => {
                                    ws_stream = new_stream;
                                    continue;
                                }
                                Err(e) => {
                                    error!("Reconnection failed: {}", e);
                                    return Err(e);
                                }
                            }
                        }
                        None => {
                            info!("WebSocket stream ended");
                            let _ = self.event_tx.send(LichessEvent::Disconnected);
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    async fn handle_message(&self, text: &str) -> Result<()> {
        // Try to parse as LichessMessage
        match serde_json::from_str::<LichessMessage>(text) {
            Ok(msg) => {
                let event = match msg {
                    LichessMessage::Move(data) => LichessEvent::Move(data),
                    LichessMessage::Ack => LichessEvent::Ack,
                    LichessMessage::EndData(data) => LichessEvent::GameEnd(data),
                    LichessMessage::Reload => LichessEvent::Reload,
                    LichessMessage::Resync => LichessEvent::Resync,
                };
                let _ = self.event_tx.send(event);
            }
            Err(e) => {
                debug!("Failed to parse message as LichessMessage: {} - {}", e, text);
            }
        }
        
        Ok(())
    }
}

pub struct WebSocketSender {
    tx: mpsc::UnboundedSender<Message>,
}

impl WebSocketSender {
    pub fn new(mut ws_stream: WsStream) -> (Self, tokio::task::JoinHandle<Result<()>>) {
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        
        let handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = ws_stream.send(msg).await {
                    error!("Failed to send message: {}", e);
                    return Err(anyhow!("Send failed: {}", e));
                }
            }
            Ok(())
        });
        
        (WebSocketSender { tx }, handle)
    }
    
    pub fn send_move(&self, uci: String, ack: u32, berserked: bool, lag_claim: u32) -> Result<()> {
        let request = MoveRequest::new(uci, ack, berserked, lag_claim);
        let json = serde_json::to_string(&request)?;
        debug!("Sending move: {}", json);
        self.tx.send(Message::Text(json))
            .map_err(|e| anyhow!("Failed to send move: {}", e))?;
        Ok(())
    }
    
    pub fn send_raw(&self, text: String) -> Result<()> {
        self.tx.send(Message::Text(text))
            .map_err(|e| anyhow!("Failed to send message: {}", e))?;
        Ok(())
    }
}
