pub mod stockfish;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVLine {
    pub multipv: u8,
    pub eval_cp: Option<i32>,
    pub eval_type: EvalType,
    pub mate_val: Option<i32>,
    pub pv: Vec<String>,
    pub first_move: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EvalType {
    Centipawn,
    Mate,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub depth: Option<u8>,
    pub movetime_ms: Option<u32>,
    pub multi_pv: u8,
}

pub trait ChessEngine: Send + Sync {
    fn name(&self) -> &str;
    fn set_position(&mut self, fen: &str) -> Result<()>;
    fn get_best_move(&mut self, options: SearchOptions) -> Result<String>;
    fn get_multi_pv(&mut self, options: SearchOptions) -> Result<Vec<PVLine>>;
    fn set_skill_level(&mut self, level: u8) -> Result<()>;
    fn stop(&mut self);
    fn is_ready(&self) -> bool;
}

pub struct EngineManager {
    engines: Vec<Box<dyn ChessEngine>>,
    active_engine: usize,
}

impl EngineManager {
    pub fn new() -> Self {
        EngineManager {
            engines: Vec::new(),
            active_engine: 0,
        }
    }
    
    pub fn add_engine(&mut self, engine: Box<dyn ChessEngine>) {
        self.engines.push(engine);
    }
    
    pub fn set_active_engine(&mut self, index: usize) {
        if index < self.engines.len() {
            self.active_engine = index;
        }
    }
    
    pub fn get_active_engine(&mut self) -> Option<&mut Box<dyn ChessEngine>> {
        self.engines.get_mut(self.active_engine)
    }
}

impl Default for EngineManager {
    fn default() -> Self {
        Self::new()
    }
}
