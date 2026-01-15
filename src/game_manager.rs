use crate::chess_logic::GameState;
use crate::config::Config;
use crate::engine::{EngineManager, SearchOptions};
use crate::lichess::events::LichessEvent;
use crate::lichess::websocket::WebSocketSender;
use crate::lichess::LichessClient;
use crate::move_selector::MoveSelector;
use crate::timing::TimingEngine;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct GameManager {
    pub state: Arc<RwLock<GameState>>,
    pub engine_manager: Arc<RwLock<EngineManager>>,
    pub timing_engine: Arc<RwLock<TimingEngine>>,
    pub move_selector: Arc<RwLock<MoveSelector>>,
    pub lichess_client: Arc<RwLock<LichessClient>>,
    pub config: Arc<RwLock<Config>>,
    pub is_processing: Arc<AtomicBool>,
    pub pending_move_uci: Arc<RwLock<Option<String>>>,
    pub ws_sender: Arc<RwLock<Option<WebSocketSender>>>,
}

impl GameManager {
    pub fn new(
        engine_manager: EngineManager,
        config: Config,
    ) -> Self {
        let vpn_offset = config.vpn_ping_offset;
        let mut timing_engine = TimingEngine::new(vpn_offset);
        timing_engine.set_preset(config.config_mode);
        
        GameManager {
            state: Arc::new(RwLock::new(GameState::new())),
            engine_manager: Arc::new(RwLock::new(engine_manager)),
            timing_engine: Arc::new(RwLock::new(timing_engine)),
            move_selector: Arc::new(RwLock::new(MoveSelector::new())),
            lichess_client: Arc::new(RwLock::new(LichessClient::new())),
            config: Arc::new(RwLock::new(config)),
            is_processing: Arc::new(AtomicBool::new(false)),
            pending_move_uci: Arc::new(RwLock::new(None)),
            ws_sender: Arc::new(RwLock::new(None)),
        }
    }
    
    pub fn set_ws_sender(&self, sender: WebSocketSender) {
        let ws_sender = self.ws_sender.clone();
        tokio::spawn(async move {
            *ws_sender.write().await = Some(sender);
        });
    }
    
    pub async fn handle_event(&self, event: LichessEvent) -> Result<()> {
        match event {
            LichessEvent::Move(move_data) => {
                info!("Move event: {} (ply: {})", move_data.uci, move_data.ply);
                
                // Update ack
                {
                    let mut client = self.lichess_client.write().await;
                    client.update_ack(move_data.ply);
                }
                
                // Update lag if available
                if let Some(ref clock) = move_data.clock {
                    if let Some(lag) = clock.lag {
                        let mut timing = self.timing_engine.write().await;
                        timing.update_lag(lag * 10); // Convert to milliseconds
                    }
                }
                
                // Check if game ended
                if move_data.status.is_some() || move_data.winner.is_some() {
                    let mut state = self.state.write().await;
                    state.game_ended = true;
                    info!("Game ended");
                    return Ok(());
                }
                
                // Apply the move if it's the opponent's move
                {
                    let mut state = self.state.write().await;
                    let pending = self.pending_move_uci.read().await;
                    
                    if pending.as_ref() == Some(&move_data.uci) {
                        // This is our move being acknowledged
                        *self.pending_move_uci.write().await = None;
                        state.last_move_acked = true;
                    } else {
                        // Opponent's move
                        if let Err(e) = state.apply_move(&move_data.uci) {
                            error!("Failed to apply move {}: {}", move_data.uci, e);
                        }
                    }
                }
                
                // Process our turn if auto mode is enabled
                let config = self.config.read().await;
                if config.auto_run {
                    self.process_turn().await?;
                }
            }
            LichessEvent::Ack => {
                debug!("Ack received");
                let mut state = self.state.write().await;
                state.last_move_acked = true;
                *self.pending_move_uci.write().await = None;
            }
            LichessEvent::GameEnd(end_data) => {
                info!("Game ended: {:?}", end_data);
                let mut state = self.state.write().await;
                state.game_ended = true;
                self.reset_game_state().await;
            }
            LichessEvent::Reload | LichessEvent::Resync => {
                warn!("Reload/Resync event received");
                self.is_processing.store(false, Ordering::Relaxed);
                *self.pending_move_uci.write().await = None;
            }
            LichessEvent::Connected => {
                info!("WebSocket connected");
            }
            LichessEvent::Disconnected => {
                warn!("WebSocket disconnected");
            }
            LichessEvent::Error(err) => {
                error!("WebSocket error: {}", err);
            }
        }
        
        Ok(())
    }
    
    pub async fn process_turn(&self) -> Result<()> {
        // Check if already processing
        if self.is_processing.swap(true, Ordering::Relaxed) {
            return Ok(());
        }
        
        let state = self.state.read().await;
        
        // Check if it's our turn
        if !state.is_my_turn() {
            self.is_processing.store(false, Ordering::Relaxed);
            return Ok(());
        }
        
        // Check if game ended
        if state.game_ended {
            self.is_processing.store(false, Ordering::Relaxed);
            return Ok(());
        }
        
        // Check if we have a pending move
        {
            let pending = self.pending_move_uci.read().await;
            if pending.is_some() {
                self.is_processing.store(false, Ordering::Relaxed);
                return Ok(());
            }
        }
        
        let fen = state.fen();
        drop(state);
        
        info!("Processing turn for position: {}", fen);
        
        // Get engine analysis
        let config = self.config.read().await;
        let timing = self.timing_engine.read().await;
        let preset = timing.get_preset();
        
        let search_options = SearchOptions {
            depth: None,
            movetime_ms: Some(preset.engine_ms),
            multi_pv: 4,
        };
        
        let mut engine_manager = self.engine_manager.write().await;
        let engine = engine_manager.get_active_engine();
        
        if engine.is_none() {
            error!("No active engine available");
            self.is_processing.store(false, Ordering::Relaxed);
            return Ok(());
        }
        
        let engine = engine.unwrap();
        engine.set_position(&fen)?;
        
        let start = std::time::Instant::now();
        let pvs = engine.get_multi_pv(search_options)?;
        let engine_time = start.elapsed().as_millis() as u32;
        
        drop(engine_manager);
        
        if pvs.is_empty() {
            error!("No moves found from engine");
            self.is_processing.store(false, Ordering::Relaxed);
            return Ok(());
        }
        
        info!("Engine found {} moves in {}ms", pvs.len(), engine_time);
        
        // Select move
        let mut move_selector = self.move_selector.write().await;
        let selected = move_selector.select_move(
            &pvs,
            &*self.state.read().await,
            config.varied_mode,
            &preset.varied,
        );
        
        drop(move_selector);
        
        if selected.is_none() {
            error!("No move selected");
            self.is_processing.store(false, Ordering::Relaxed);
            return Ok(());
        }
        
        let selected = selected.unwrap();
        info!("Selected move: {} (PV{})", selected.uci, selected.pv_index + 1);
        
        // Calculate delay if human mode is enabled
        let delay_ms = if config.human_mode {
            let state = self.state.read().await;
            let is_capture = state.is_capture(&selected.uci);
            timing.calculate_human_delay(&state, &selected.uci, is_capture)
        } else {
            0
        };
        
        drop(config);
        drop(timing);
        
        if delay_ms > 0 {
            info!("Waiting {}ms before move (human timing)", delay_ms);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms as u64)).await;
        }
        
        // Execute the move
        self.execute_move(selected.uci, engine_time).await?;
        
        Ok(())
    }
    
    async fn execute_move(&self, uci: String, engine_time: u32) -> Result<()> {
        let ws_sender = self.ws_sender.read().await;
        if ws_sender.is_none() {
            error!("WebSocket sender not available");
            self.is_processing.store(false, Ordering::Relaxed);
            return Ok(());
        }
        
        let sender = ws_sender.as_ref().unwrap();
        
        let client = self.lichess_client.read().await;
        let ack = client.get_ack();
        
        let config = self.config.read().await;
        let timing = self.timing_engine.read().await;
        
        let lag_claim = if config.panic_mode {
            timing.get_panic_lag_compensation()
        } else {
            timing.get_lag_compensation()
        };
        
        drop(client);
        drop(config);
        
        // Update timing stats
        let mut timing = self.timing_engine.write().await;
        timing.update_stats(0, engine_time);
        drop(timing);
        
        info!("Sending move: {} (ack: {}, lag: {}ms)", uci, ack, lag_claim);
        
        *self.pending_move_uci.write().await = Some(uci.clone());
        
        sender.send_move(uci.clone(), ack, false, lag_claim)?;
        
        // Apply move to our state
        let mut state = self.state.write().await;
        state.last_move_acked = false;
        if let Err(e) = state.apply_move(&uci) {
            error!("Failed to apply our own move: {}", e);
        }
        
        self.is_processing.store(false, Ordering::Relaxed);
        
        Ok(())
    }
    
    pub async fn reset_game_state(&self) {
        info!("Resetting game state");
        
        let mut state = self.state.write().await;
        state.reset();
        
        *self.pending_move_uci.write().await = None;
        self.is_processing.store(false, Ordering::Relaxed);
        
        let mut move_selector = self.move_selector.write().await;
        move_selector.reset();
        
        let mut timing = self.timing_engine.write().await;
        timing.reset_stats();
    }
}
