use crate::chess_logic::GameState;
use crate::config::ConfigMode;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingPreset {
    pub engine_ms: u32,
    pub varied: VariedConfig,
    pub human: HumanConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanConfig {
    pub base_delay_ms: u32,
    pub max_delay_ms: u32,
    pub premove_delay_ms: u32,
    pub premove_max_ms: u32,
    pub low_piece_delay_ms: u32,
    pub low_piece_max_ms: u32,
    pub premove_piece_threshold: u8,
    pub low_piece_threshold: u8,
    pub quick_move_chance: f32,
    pub quick_move_ms: u32,
    pub tank_chance: f32,
    pub tank_min_ms: u32,
    pub tank_max_ms: u32,
    pub random_variance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariedConfig {
    pub max_cp_loss: i32,
    pub weights: [u8; 4],
    pub max_blunders_per_game: u8,
    pub blunder_threshold: i32,
    pub blunder_chance: f32,
}

#[derive(Debug, Clone, Default)]
pub struct TimingStats {
    pub total_moves: u32,
    pub total_time_ms: u32,
    pub engine_time_ms: u32,
}

pub struct TimingEngine {
    pub presets: HashMap<ConfigMode, TimingPreset>,
    pub active_preset: ConfigMode,
    pub stats: TimingStats,
    pub lag_history: VecDeque<u32>,
    pub vpn_offset: u32,
}

impl TimingEngine {
    pub fn new(vpn_offset: u32) -> Self {
        let mut presets = HashMap::new();
        
        // 7.5s preset
        presets.insert(
            ConfigMode::Fast7_5s,
            TimingPreset {
                engine_ms: 12,
                varied: VariedConfig {
                    max_cp_loss: 900,
                    weights: [8, 40, 28, 24],
                    max_blunders_per_game: 50,
                    blunder_threshold: 100,
                    blunder_chance: 0.45,
                },
                human: HumanConfig {
                    base_delay_ms: 180,
                    max_delay_ms: 600,
                    premove_delay_ms: 0,
                    premove_max_ms: 10,
                    low_piece_delay_ms: 25,
                    low_piece_max_ms: 120,
                    premove_piece_threshold: 12,
                    low_piece_threshold: 22,
                    quick_move_chance: 0.35,
                    quick_move_ms: 0,
                    tank_chance: 0.008,
                    tank_min_ms: 250,
                    tank_max_ms: 500,
                    random_variance: 0.25,
                },
            },
        );
        
        // 15s preset
        presets.insert(
            ConfigMode::Normal15s,
            TimingPreset {
                engine_ms: 20,
                varied: VariedConfig {
                    max_cp_loss: 300,
                    weights: [10, 45, 23, 22],
                    max_blunders_per_game: 10,
                    blunder_threshold: 100,
                    blunder_chance: 0.16,
                },
                human: HumanConfig {
                    base_delay_ms: 250,
                    max_delay_ms: 800,
                    premove_delay_ms: 0,
                    premove_max_ms: 20,
                    low_piece_delay_ms: 30,
                    low_piece_max_ms: 150,
                    premove_piece_threshold: 10,
                    low_piece_threshold: 20,
                    quick_move_chance: 0.25,
                    quick_move_ms: 0,
                    tank_chance: 0.01,
                    tank_min_ms: 400,
                    tank_max_ms: 600,
                    random_variance: 0.27,
                },
            },
        );
        
        // 30s preset
        presets.insert(
            ConfigMode::Slow30s,
            TimingPreset {
                engine_ms: 60,
                varied: VariedConfig {
                    max_cp_loss: 200,
                    weights: [30, 55, 10, 5],
                    max_blunders_per_game: 5,
                    blunder_threshold: 100,
                    blunder_chance: 0.08,
                },
                human: HumanConfig {
                    base_delay_ms: 500,
                    max_delay_ms: 1200,
                    premove_delay_ms: 50,
                    premove_max_ms: 150,
                    low_piece_delay_ms: 100,
                    low_piece_max_ms: 500,
                    premove_piece_threshold: 8,
                    low_piece_threshold: 16,
                    quick_move_chance: 0.20,
                    quick_move_ms: 60,
                    tank_chance: 0.05,
                    tank_min_ms: 1000,
                    tank_max_ms: 2000,
                    random_variance: 0.37,
                },
            },
        );
        
        let mut lag_history = VecDeque::new();
        lag_history.push_back(50);
        lag_history.push_back(50);
        lag_history.push_back(50);
        
        TimingEngine {
            presets,
            active_preset: ConfigMode::Normal15s,
            stats: TimingStats::default(),
            lag_history,
            vpn_offset,
        }
    }
    
    pub fn set_preset(&mut self, mode: ConfigMode) {
        self.active_preset = mode;
    }
    
    pub fn get_preset(&self) -> &TimingPreset {
        self.presets.get(&self.active_preset).unwrap()
    }
    
    pub fn calculate_human_delay(&self, game: &GameState, _uci: &str, is_capture: bool) -> u32 {
        let cfg = &self.get_preset().human;
        
        // Captures are instant
        if is_capture {
            return 0;
        }
        
        let piece_count = game.piece_count();
        
        // Premove situation (very few pieces)
        if piece_count <= cfg.premove_piece_threshold {
            let delay = cfg.premove_delay_ms as f32
                + rand::thread_rng().gen::<f32>() * (cfg.premove_max_ms - cfg.premove_delay_ms) as f32;
            return delay.max(0.0).round() as u32;
        }
        
        // Low piece count
        if piece_count <= cfg.low_piece_threshold {
            let delay = cfg.low_piece_delay_ms as f32
                + rand::thread_rng().gen::<f32>() * (cfg.low_piece_max_ms - cfg.low_piece_delay_ms) as f32;
            return delay.max(0.0).round() as u32;
        }
        
        // Normal move timing
        let mut delay = cfg.base_delay_ms as f32;
        delay *= 1.0 + (rand::thread_rng().gen::<f32>() * 2.0 - 1.0) * cfg.random_variance;
        
        let roll = rand::thread_rng().gen::<f32>();
        if roll < cfg.quick_move_chance {
            delay = cfg.quick_move_ms as f32 + rand::thread_rng().gen::<f32>() * 50.0;
        } else if roll < cfg.quick_move_chance + cfg.tank_chance {
            delay = cfg.tank_min_ms as f32
                + rand::thread_rng().gen::<f32>() * (cfg.tank_max_ms - cfg.tank_min_ms) as f32;
        }
        
        delay = delay.max(0.0).min(cfg.max_delay_ms as f32);
        
        // Adjust based on average timing
        if self.stats.total_moves > 5 {
            let avg = (self.stats.total_time_ms + self.stats.engine_time_ms) as f32
                / self.stats.total_moves as f32;
            if avg > 580.0 {
                delay *= (0.5_f32).max(580.0 / avg);
            }
        }
        
        delay.round() as u32
    }
    
    pub fn get_average_server_lag(&self) -> u32 {
        let sum: u32 = self.lag_history.iter().sum();
        sum / self.lag_history.len() as u32
    }
    
    pub fn get_lag_compensation(&self) -> u32 {
        let avg_server_lag = self.get_average_server_lag();
        let total_lag = avg_server_lag + self.vpn_offset;
        let max_reasonable = avg_server_lag.saturating_mul(2).max(100);
        total_lag.min(max_reasonable)
    }
    
    pub fn get_panic_lag_compensation(&self) -> u32 {
        let avg_server_lag = self.get_average_server_lag();
        let total_lag = avg_server_lag + self.vpn_offset + 30;
        let max_reasonable = avg_server_lag.saturating_mul(3).max(200);
        total_lag.min(max_reasonable)
    }
    
    pub fn update_lag(&mut self, lag_ms: u32) {
        self.lag_history.push_back(lag_ms);
        if self.lag_history.len() > 5 {
            self.lag_history.pop_front();
        }
    }
    
    pub fn update_stats(&mut self, delay_ms: u32, engine_ms: u32) {
        self.stats.total_moves += 1;
        self.stats.total_time_ms += delay_ms;
        self.stats.engine_time_ms += engine_ms;
    }
    
    pub fn reset_stats(&mut self) {
        self.stats = TimingStats::default();
    }
}
