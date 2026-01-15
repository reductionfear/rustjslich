use crate::bridge::{BridgeHandle, BrowserMessage, GameStateMessage};
use crate::chess_logic::GameState;
use crate::config::Config;
use crate::engine::{EngineManager, SearchOptions};
use crate::lichess::events::LichessEvent;
use crate::lichess::websocket::WebSocketSender;
use crate::lichess::LichessClient;
use crate::move_selector::MoveSelector;
use crate::timing::TimingEngine;
use anyhow::Result;
use chess::{Board, ChessMove};
use std::str::FromStr;
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
    pub bridge_handle: Arc<RwLock<Option<BridgeHandle>>>,
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
            bridge_handle: Arc::new(RwLock::new(None)),
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
        let config = self.config.read().await;
        
        // Check if we're using bridge mode
        let bridge_handle = self.bridge_handle.read().await;
        if bridge_handle.is_some() {
            // Bridge mode - send move through bridge
            let handle = bridge_handle.as_ref().unwrap();
            
            info!("Sending move via bridge: {}", uci);
            
            if let Err(e) = handle.send_move(uci.clone()) {
                error!("Failed to send move via bridge: {}", e);
                self.is_processing.store(false, Ordering::Relaxed);
                return Ok(());
            }
            
            // Apply move to our state
            let mut state = self.state.write().await;
            state.last_move_acked = false;
            if let Err(e) = state.apply_move(&uci) {
                error!("Failed to apply our own move: {}", e);
            }
            
            *self.pending_move_uci.write().await = Some(uci);
            self.is_processing.store(false, Ordering::Relaxed);
            
            return Ok(());
        }
        drop(bridge_handle);
        
        // Legacy Lichess WebSocket mode
        let ws_sender = self.ws_sender.read().await;
        if ws_sender.is_none() {
            error!("WebSocket sender not available");
            self.is_processing.store(false, Ordering::Relaxed);
            return Ok(());
        }
        
        let sender = ws_sender.as_ref().unwrap();
        
        let client = self.lichess_client.read().await;
        let ack = client.get_ack();
        
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
    
    /// Set the bridge handle for browser bridge mode
    pub async fn set_bridge_handle(&self, handle: BridgeHandle) {
        *self.bridge_handle.write().await = Some(handle);
        info!("Bridge handle set - running in browser bridge mode");
    }
    
    /// Handle a message from the browser bridge
    pub async fn handle_bridge_message(&self, message: BrowserMessage) -> Result<()> {
        match message {
            BrowserMessage::GameState(state_msg) => {
                self.handle_game_state(state_msg).await?;
            }
            BrowserMessage::RematchAvailable { game_id } => {
                info!("Rematch available for game {}", game_id);
                
                // Auto-accept if enabled
                let config = self.config.read().await;
                if config.auto_rematch {
                    info!("Auto-accepting rematch");
                    let bridge_handle = self.bridge_handle.read().await;
                    if let Some(handle) = bridge_handle.as_ref() {
                        handle.accept_rematch()?;
                    }
                }
            }
            BrowserMessage::RematchAccepted { game_id } => {
                info!("Rematch accepted for game {}", game_id);
                self.reset_game_state().await;
            }
            BrowserMessage::NewGame { game_id } => {
                info!("New game started: {}", game_id);
                self.reset_game_state().await;
            }
        }
        
        Ok(())
    }
    
    /// Handle game state update from browser
    async fn handle_game_state(&self, state_msg: GameStateMessage) -> Result<()> {
        debug!("Game state update: game_id={}, is_my_turn={}, ended={}", 
               state_msg.game_id, state_msg.is_my_turn, state_msg.game_ended);
        
        // Update game state by applying moves
        let mut state = self.state.write().await;
        
        // If we have moves, reconstruct the position
        if !state_msg.moves.is_empty() {
            // Reset to starting position if different game or move count mismatch
            if state.move_history.len() != state_msg.moves.len() {
                state.reset();
                
                // Apply all moves - try both UCI and SAN formats
                for move_str in &state_msg.moves {
                    // Try to parse as UCI first
                    if let Ok(uci_move) = ChessMove::from_str(move_str) {
                        // Valid UCI move
                        if state.board.legal(uci_move) {
                            if let Err(e) = state.apply_move(move_str) {
                                warn!("Failed to apply UCI move {}: {}", move_str, e);
                            }
                            continue;
                        }
                    }
                    
                    // Try to parse as SAN
                    if let Some(uci_move) = san_to_uci(&state.board, move_str) {
                        if let Err(e) = state.apply_move(&uci_move) {
                            warn!("Failed to apply SAN move {} ({}): {}", move_str, uci_move, e);
                        }
                    } else {
                        warn!("Could not parse move: {}", move_str);
                    }
                }
            }
        }
        
        // Update my color based on orientation
        state.my_color = Some(if state_msg.my_color.is_white() {
            chess::Color::White
        } else {
            chess::Color::Black
        });
        
        // Update game ended status
        state.game_ended = state_msg.game_ended;
        
        drop(state);
        
        // If it's our turn and auto mode is enabled, process the turn
        if state_msg.is_my_turn && !state_msg.game_ended {
            let config = self.config.read().await;
            if config.auto_run {
                drop(config);
                self.process_turn().await?;
            }
        }
        
        Ok(())
    }
}

/// Convert SAN (Standard Algebraic Notation) to UCI format
/// E.g., "Nf3" -> "g1f3", "e4" -> "e2e4"
fn san_to_uci(board: &Board, san: &str) -> Option<String> {
    // Generate all legal moves
    let legal_moves = chess::MoveGen::new_legal(board);
    
    // Try to match the SAN with a legal move
    for chess_move in legal_moves {
        // Convert move to SAN and compare
        let move_san = move_to_san(board, &chess_move);
        if move_san == san {
            return Some(format!("{}", chess_move));
        }
    }
    
    None
}

/// Convert a ChessMove to SAN notation
fn move_to_san(board: &Board, chess_move: &ChessMove) -> String {
    let piece = board.piece_on(chess_move.get_source());
    let dest = chess_move.get_dest();
    let source = chess_move.get_source();
    
    // Check if it's a capture
    let is_capture = board.piece_on(dest).is_some() || 
                    (piece == Some(chess::Piece::Pawn) && source.get_file() != dest.get_file());
    
    // Handle castling
    if piece == Some(chess::Piece::King) {
        let diff = dest.to_int() as i8 - source.to_int() as i8;
        if diff == 2 {
            return "O-O".to_string();
        } else if diff == -2 {
            return "O-O-O".to_string();
        }
    }
    
    let mut san = String::new();
    
    // Add piece letter (except for pawns)
    if piece != Some(chess::Piece::Pawn) {
        if let Some(p) = piece {
            san.push(match p {
                chess::Piece::Knight => 'N',
                chess::Piece::Bishop => 'B',
                chess::Piece::Rook => 'R',
                chess::Piece::Queen => 'Q',
                chess::Piece::King => 'K',
                _ => ' ',
            });
        }
    } else if is_capture {
        // For pawn captures, add the source file
        san.push((b'a' + source.get_file().to_index() as u8) as char);
    }
    
    // Add 'x' for captures
    if is_capture {
        san.push('x');
    }
    
    // Add destination square
    san.push((b'a' + dest.get_file().to_index() as u8) as char);
    san.push((b'1' + dest.get_rank().to_index() as u8) as char);
    
    // Handle promotion
    if let Some(promotion) = chess_move.get_promotion() {
        san.push('=');
        san.push(match promotion {
            chess::Piece::Knight => 'N',
            chess::Piece::Bishop => 'B',
            chess::Piece::Rook => 'R',
            chess::Piece::Queen => 'Q',
            _ => ' ',
        });
    }
    
    san
}
