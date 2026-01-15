pub mod events;
pub mod websocket;

use anyhow::Result;
use events::LichessEvent;
use tokio::sync::mpsc;
use websocket::WebSocketHandler;

pub struct LichessClient {
    pub game_id: Option<String>,
    pub current_ack: u32,
    event_rx: mpsc::UnboundedReceiver<LichessEvent>,
    event_tx: mpsc::UnboundedSender<LichessEvent>,
}

impl LichessClient {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        LichessClient {
            game_id: None,
            current_ack: 0,
            event_rx,
            event_tx,
        }
    }
    
    pub fn connect_to_game(&mut self, game_id: String) -> Result<WebSocketHandler> {
        self.game_id = Some(game_id.clone());
        self.current_ack = 0;
        
        // Lichess game WebSocket URL format
        let url = format!("wss://socket.lichess.org/play/{}/v6", game_id);
        
        Ok(WebSocketHandler::new(url, self.event_tx.clone()))
    }
    
    pub async fn recv_event(&mut self) -> Option<LichessEvent> {
        self.event_rx.recv().await
    }
    
    pub fn try_recv_event(&mut self) -> Option<LichessEvent> {
        self.event_rx.try_recv().ok()
    }
    
    pub fn update_ack(&mut self, ack: u32) {
        self.current_ack = ack;
    }
    
    pub fn get_ack(&self) -> u32 {
        self.current_ack
    }
}

impl Default for LichessClient {
    fn default() -> Self {
        Self::new()
    }
}
