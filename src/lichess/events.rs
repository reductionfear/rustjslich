use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockData {
    pub white: f32,
    pub black: f32,
    #[serde(default)]
    pub lag: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveData {
    pub uci: String,
    pub fen: String,
    pub ply: u32,
    #[serde(default)]
    pub clock: Option<ClockData>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub winner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndData {
    pub status: String,
    #[serde(default)]
    pub winner: Option<String>,
}

#[derive(Debug, Clone)]
pub enum LichessEvent {
    Move(MoveData),
    Ack,
    GameEnd(EndData),
    Reload,
    Resync,
    Connected,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "d")]
pub enum LichessMessage {
    #[serde(rename = "move")]
    Move(MoveData),
    #[serde(rename = "ack")]
    Ack,
    #[serde(rename = "endData")]
    EndData(EndData),
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "resync")]
    Resync,
}

#[derive(Debug, Clone, Serialize)]
pub struct MoveRequest {
    pub t: String,
    pub d: MoveRequestData,
}

#[derive(Debug, Clone, Serialize)]
pub struct MoveRequestData {
    pub u: String,     // UCI move
    pub a: u32,        // Ack number
    pub b: u8,         // Berserked (0 or 1)
    pub l: u32,        // Lag claim in milliseconds
}

impl MoveRequest {
    pub fn new(uci: String, ack: u32, berserked: bool, lag_claim: u32) -> Self {
        MoveRequest {
            t: "move".to_string(),
            d: MoveRequestData {
                u: uci,
                a: ack,
                b: if berserked { 1 } else { 0 },
                l: lag_claim,
            },
        }
    }
}
