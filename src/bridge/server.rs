use super::protocol::{BrowserMessage, RustMessage};
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

pub struct BridgeServer {
    port: u16,
    message_tx: mpsc::UnboundedSender<BrowserMessage>,
}

impl BridgeServer {
    pub fn new(port: u16) -> (Self, BridgeHandle) {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        
        let handle = BridgeHandle {
            message_rx,
            command_tx,
            command_rx: Some(command_rx),
        };
        
        (
            Self {
                port,
                message_tx,
            },
            handle,
        )
    }
    
    pub async fn run(self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("🌉 Bridge WebSocket server listening on {}", addr);
        info!("Waiting for browser extension to connect...");
        
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("Browser extension connected from {}", addr);
                    
                    let message_tx = self.message_tx.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, message_tx).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    message_tx: mpsc::UnboundedSender<BrowserMessage>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    info!("WebSocket handshake completed");
    
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received from browser: {}", text);
                match serde_json::from_str::<BrowserMessage>(&text) {
                    Ok(browser_msg) => {
                        if let Err(e) = message_tx.send(browser_msg) {
                            error!("Failed to forward message: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse message: {} - {}", e, text);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Browser extension closed connection");
                break;
            }
            Ok(Message::Ping(data)) => {
                if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
    
    info!("Connection closed");
    
    Ok(())
}

/// Handle for interacting with the bridge server
pub struct BridgeHandle {
    message_rx: mpsc::UnboundedReceiver<BrowserMessage>,
    command_tx: mpsc::UnboundedSender<RustMessage>,
    command_rx: Option<mpsc::UnboundedReceiver<RustMessage>>,
}

impl BridgeHandle {
    /// Try to receive a message from the browser (non-blocking)
    pub fn try_recv_message(&mut self) -> Option<BrowserMessage> {
        self.message_rx.try_recv().ok()
    }
    
    /// Send a command to the browser
    pub fn send_command(&self, command: RustMessage) -> Result<()> {
        self.command_tx
            .send(command)
            .map_err(|e| anyhow!("Failed to send command: {}", e))
    }
    
    /// Send a move command to the browser
    pub fn send_move(&self, uci: String) -> Result<()> {
        self.send_command(RustMessage::MakeMove { uci })
    }
    
    /// Send accept rematch command to the browser
    pub fn accept_rematch(&self) -> Result<()> {
        self.send_command(RustMessage::AcceptRematch)
    }
    
    /// Send decline rematch command to the browser
    pub fn decline_rematch(&self) -> Result<()> {
        self.send_command(RustMessage::DeclineRematch)
    }
    
    /// Take the command receiver (can only be called once)
    pub fn take_command_rx(&mut self) -> Option<mpsc::UnboundedReceiver<RustMessage>> {
        self.command_rx.take()
    }
}
