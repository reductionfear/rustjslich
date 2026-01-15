pub mod chess_logic;
pub mod cli;
pub mod config;
pub mod engine;
pub mod game_manager;
pub mod lichess;
pub mod move_selector;
pub mod timing;
pub mod ui;

// Re-exports
pub use chess_logic::GameState;
pub use config::{Config, ConfigMode, Engine};
pub use engine::{ChessEngine, EngineManager, PVLine, SearchOptions};
pub use game_manager::GameManager;
pub use lichess::LichessClient;
pub use move_selector::{MoveSelector, SelectedMove, VarietyStats};
pub use timing::{TimingEngine, TimingPreset};
pub use ui::{TerminalUI, UICommand};
