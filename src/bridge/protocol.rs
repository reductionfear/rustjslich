use serde::{Deserialize, Serialize};

/// Message types sent from the browser extension to Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BrowserMessage {
    #[serde(rename = "game_state")]
    GameState(GameStateMessage),
    #[serde(rename = "rematch_available")]
    RematchAvailable { game_id: String },
    #[serde(rename = "rematch_accepted")]
    RematchAccepted { game_id: String },
    #[serde(rename = "new_game")]
    NewGame { game_id: String },
}

/// Game state information from the browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateMessage {
    pub game_id: String,
    pub fen: String,
    pub my_color: Color,
    pub is_my_turn: bool,
    pub white_clock_ms: u64,
    pub black_clock_ms: u64,
    pub moves: Vec<String>,
    pub game_ended: bool,
    pub result: Option<String>,
}

/// Message types sent from Rust to the browser extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RustMessage {
    #[serde(rename = "make_move")]
    MakeMove { uci: String },
    #[serde(rename = "accept_rematch")]
    AcceptRematch,
    #[serde(rename = "decline_rematch")]
    DeclineRematch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn is_white(&self) -> bool {
        matches!(self, Color::White)
    }
}
