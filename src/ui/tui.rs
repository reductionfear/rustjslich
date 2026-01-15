use crate::chess_logic::GameState;
use crate::config::{Config, ConfigMode, Engine};
use crate::move_selector::VarietyStats;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum UICommand {
    ToggleAuto,
    TogglePanic,
    ToggleHuman,
    ToggleVaried,
    CycleEngine,
    CycleConfig,
    Hint,
    Quit,
    None,
}

pub struct DisplayStats {
    pub moves_played: u32,
    pub avg_time_ms: u32,
    pub engine_time_ms: u32,
    pub variety_stats: VarietyStats,
}

pub struct UIState {
    pub auto_mode: bool,
    pub panic_mode: bool,
    pub human_mode: bool,
    pub varied_mode: bool,
    pub selected_engine: Engine,
    pub config_mode: ConfigMode,
    pub stats: DisplayStats,
    pub board_display: String,
    pub game_status: String,
    pub last_move: Option<String>,
    pub is_connected: bool,
}

impl Default for UIState {
    fn default() -> Self {
        UIState {
            auto_mode: false,
            panic_mode: false,
            human_mode: true,
            varied_mode: true,
            selected_engine: Engine::Stockfish,
            config_mode: ConfigMode::Normal15s,
            stats: DisplayStats {
                moves_played: 0,
                avg_time_ms: 0,
                engine_time_ms: 0,
                variety_stats: VarietyStats::default(),
            },
            board_display: String::new(),
            game_status: "Waiting for game...".to_string(),
            last_move: None,
            is_connected: false,
        }
    }
}

pub struct TerminalUI {
    state: UIState,
}

impl TerminalUI {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        
        Ok(TerminalUI {
            state: UIState::default(),
        })
    }
    
    pub fn update_from_config(&mut self, config: &Config) {
        self.state.auto_mode = config.auto_run;
        self.state.panic_mode = config.panic_mode;
        self.state.human_mode = config.human_mode;
        self.state.varied_mode = config.varied_mode;
        self.state.selected_engine = config.selected_engine;
        self.state.config_mode = config.config_mode;
    }
    
    pub fn update_from_game(&mut self, game: &GameState) {
        self.state.game_status = if game.game_ended {
            "Game ended".to_string()
        } else if game.is_my_turn() {
            "Our turn".to_string()
        } else {
            "Opponent's turn".to_string()
        };
        
        // Simple board representation
        self.state.board_display = format!("FEN: {}", game.fen());
    }
    
    pub fn set_connected(&mut self, connected: bool) {
        self.state.is_connected = connected;
    }
    
    pub fn set_last_move(&mut self, mv: String) {
        self.state.last_move = Some(mv);
    }
    
    pub fn update_stats(&mut self, stats: DisplayStats) {
        self.state.stats = stats;
    }
    
    pub fn draw(&self) -> Result<()> {
        // Move cursor to top-left instead of clearing entire screen
        print!("\x1B[H");
        
        println!("╔══════════════════════════════════════════════════════════════════════╗");
        println!("║              🦀 RUSTJSLICH - Lichess Automation                      ║");
        println!("╚══════════════════════════════════════════════════════════════════════╝");
        println!();
        
        // Status
        let conn_status = if self.state.is_connected { "🟢 Connected" } else { "🔴 Disconnected" };
        println!("Status: {} | {}", conn_status, self.state.game_status);
        println!();
        
        // Configuration
        println!("┌─ Configuration ──────────────────────────────────────────────────────┐");
        println!("│ Engine:  {} [E to cycle]", self.state.selected_engine.as_str());
        println!("│ Mode:    {} [M to cycle]", self.state.config_mode.as_str());
        println!("│ Auto:    {} [A to toggle]", if self.state.auto_mode { "ON ✓" } else { "OFF" });
        println!("│ Human:   {} [H to toggle]", if self.state.human_mode { "ON ✓" } else { "OFF" });
        println!("│ Varied:  {} [V to toggle]", if self.state.varied_mode { "ON ✓" } else { "OFF" });
        println!("│ Panic:   {} [P to toggle]", if self.state.panic_mode { "ON ✓" } else { "OFF" });
        println!("└──────────────────────────────────────────────────────────────────────┘");
        println!();
        
        // Game info
        println!("┌─ Game Info ──────────────────────────────────────────────────────────┐");
        if let Some(ref mv) = self.state.last_move {
            println!("│ Last Move: {}", mv);
        } else {
            println!("│ Last Move: None");
        }
        println!("│ {}", self.state.board_display);
        println!("└──────────────────────────────────────────────────────────────────────┘");
        println!();
        
        // Statistics
        println!("┌─ Statistics ─────────────────────────────────────────────────────────┐");
        println!("│ Moves:       {}", self.state.stats.moves_played);
        println!("│ Avg Time:    {}ms", self.state.stats.avg_time_ms);
        println!("│ Engine Time: {}ms", self.state.stats.engine_time_ms);
        println!("│");
        println!("│ Move Variety:");
        println!("│   PV1: {} | PV2: {} | PV3: {} | PV4: {}", 
            self.state.stats.variety_stats.pv1,
            self.state.stats.variety_stats.pv2,
            self.state.stats.variety_stats.pv3,
            self.state.stats.variety_stats.pv4
        );
        println!("│   Blunders: {}", self.state.stats.variety_stats.blunders);
        println!("└──────────────────────────────────────────────────────────────────────┘");
        println!();
        
        // Hotkeys
        println!("┌─ Hotkeys ────────────────────────────────────────────────────────────┐");
        println!("│ [A] Auto   [H] Human   [V] Varied   [P] Panic   [E] Engine   [M] Mode");
        println!("│ [Q] Quit   [Ctrl+C] Exit");
        println!("└──────────────────────────────────────────────────────────────────────┘");
        
        Ok(())
    }
    
    pub fn handle_input(&self, timeout: Duration) -> Result<UICommand> {
        if event::poll(timeout)? {
            if let Event::Key(key_event) = event::read()? {
                return Ok(self.process_key_event(key_event));
            }
        }
        Ok(UICommand::None)
    }
    
    fn process_key_event(&self, key_event: KeyEvent) -> UICommand {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => UICommand::Quit,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => UICommand::Quit,
            KeyCode::Char('a') | KeyCode::Char('A') => UICommand::ToggleAuto,
            KeyCode::Char('p') | KeyCode::Char('P') => UICommand::TogglePanic,
            KeyCode::Char('h') | KeyCode::Char('H') => UICommand::ToggleHuman,
            KeyCode::Char('v') | KeyCode::Char('V') => UICommand::ToggleVaried,
            KeyCode::Char('e') | KeyCode::Char('E') => UICommand::CycleEngine,
            KeyCode::Char('m') | KeyCode::Char('M') => UICommand::CycleConfig,
            _ => UICommand::None,
        }
    }
    
    pub fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }
}

impl Drop for TerminalUI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

impl Default for TerminalUI {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
