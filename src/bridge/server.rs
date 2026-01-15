use super::protocol::{BrowserMessage, RustMessage};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

pub struct BridgeServer {
    port: u16,
    message_tx: mpsc::UnboundedSender<BrowserMessage>,
    command_tx: broadcast::Sender<RustMessage>,
    is_connected: Arc<AtomicBool>,
}

impl BridgeServer {
    pub fn new(port: u16) -> (Self, BridgeHandle) {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (command_tx, _command_rx) = broadcast::channel(100); // Buffer for 100 commands
        let is_connected = Arc::new(AtomicBool::new(false));
        
        let handle = BridgeHandle {
            message_rx,
            command_tx: command_tx.clone(),
            is_connected: is_connected.clone(),
        };
        
        (
            Self {
                port,
                message_tx,
                command_tx,
                is_connected,
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
                    let is_connected = self.is_connected.clone();
                    let command_rx = self.command_tx.subscribe();
                    
                    tokio::spawn(async move {
                        is_connected.store(true, Ordering::Relaxed);
                        if let Err(e) = handle_connection(stream, message_tx, command_rx).await {
                            error!("Connection error: {}", e);
                        }
                        is_connected.store(false, Ordering::Relaxed);
                        info!("Browser extension disconnected");
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
    mut command_rx: broadcast::Receiver<RustMessage>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    info!("WebSocket handshake completed");
    
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    loop {
        tokio::select! {
            // Receive from browser
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
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
                    Some(Ok(Message::Close(_))) => {
                        info!("Browser extension closed connection");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                            error!("Failed to send pong: {}", e);
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
            // Send commands to browser
            command = command_rx.recv() => {
                match command {
                    Ok(cmd) => {
                        match serde_json::to_string(&cmd) {
                            Ok(json) => {
                                info!("Sending to browser: {}", json);
                                if let Err(e) = ws_sender.send(Message::Text(json)).await {
                                    error!("Failed to send command: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to serialize command: {}", e);
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Command receiver lagged, skipped {} messages", skipped);
                        // Continue receiving
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Command channel closed");
                        break;
                    }
                }
            }
        }
    }
    
    info!("Connection closed");
    
    Ok(())
}

/// Handle for interacting with the bridge server
pub struct BridgeHandle {
    message_rx: mpsc::UnboundedReceiver<BrowserMessage>,
    command_tx: broadcast::Sender<RustMessage>,
    is_connected: Arc<AtomicBool>,
}

impl BridgeHandle {
    /// Check if a browser extension is currently connected
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }
    
    /// Try to receive a message from the browser (non-blocking)
    pub fn try_recv_message(&mut self) -> Option<BrowserMessage> {
        self.message_rx.try_recv().ok()
    }
    
    /// Send a command to the browser
    pub fn send_command(&self, command: RustMessage) -> Result<()> {
        // broadcast::send returns the number of receivers, or error if no receivers
        // We don't care if there are no receivers (connection not established yet)
        let _ = self.command_tx.send(command);
        Ok(())
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
}
