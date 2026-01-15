use crate::chess_logic::GameState;
use crate::engine::PVLine;
use crate::timing::VariedConfig;
use rand::Rng;

#[derive(Debug, Clone, Default)]
pub struct VarietyStats {
    pub pv1: u32,
    pub pv2: u32,
    pub pv3: u32,
    pub pv4: u32,
    pub blunders: u32,
}

#[derive(Debug, Clone)]
pub struct SelectedMove {
    pub uci: String,
    pub pv_index: u8,
    pub eval_cp: Option<i32>,
    pub is_blunder: bool,
}

pub struct MoveSelector {
    pub game_blunder_count: u8,
    pub variety_stats: VarietyStats,
}

impl MoveSelector {
    pub fn new() -> Self {
        MoveSelector {
            game_blunder_count: 0,
            variety_stats: VarietyStats::default(),
        }
    }
    
    pub fn select_move(
        &mut self,
        pvs: &[PVLine],
        game: &GameState,
        varied_mode: bool,
        varied_config: &VariedConfig,
    ) -> Option<SelectedMove> {
        if pvs.is_empty() {
            return None;
        }
        
        if varied_mode {
            self.select_varied_move(pvs, game, varied_config)
        } else {
            // Just return the best move
            let pv = &pvs[0];
            self.variety_stats.pv1 += 1;
            Some(SelectedMove {
                uci: pv.first_move.clone(),
                pv_index: 0,
                eval_cp: pv.eval_cp,
                is_blunder: false,
            })
        }
    }
    
    pub fn select_varied_move(
        &mut self,
        pvs: &[PVLine],
        game: &GameState,
        cfg: &VariedConfig,
    ) -> Option<SelectedMove> {
        if pvs.is_empty() {
            return None;
        }
        
        // Filter out drawing moves
        let valid = self.filter_draw_moves(pvs, game);
        
        if valid.is_empty() {
            // Fall back to best move
            self.variety_stats.pv1 += 1;
            return Some(SelectedMove {
                uci: pvs[0].first_move.clone(),
                pv_index: 0,
                eval_cp: pvs[0].eval_cp,
                is_blunder: false,
            });
        }
        
        let top_eval = valid[0].eval_cp.unwrap_or(0);
        
        // Determine if we allow a blunder this move
        let allow_blunder = self.game_blunder_count < cfg.max_blunders_per_game
            && top_eval > -100
            && rand::thread_rng().gen::<f32>() < cfg.blunder_chance;
        
        // Build candidates with weights
        let mut candidates = Vec::new();
        
        for (idx, pv) in valid.iter().enumerate().take(4) {
            let cp_loss = top_eval - pv.eval_cp.unwrap_or(0);
            let is_blunder = cp_loss >= cfg.blunder_threshold;
            
            // Skip getting mated
            if pv.mate_val.is_some() && pv.mate_val.unwrap() < 0 && pv.mate_val.unwrap() >= -3 {
                continue;
            }
            
            // Skip moves with too much CP loss unless we're allowing a blunder
            if cp_loss > cfg.max_cp_loss && !allow_blunder {
                continue;
            }
            
            // Calculate weight
            let base_weight = if idx < cfg.weights.len() {
                cfg.weights[idx] as f32
            } else {
                5.0
            };
            
            let mut weight = base_weight;
            if cfg.max_cp_loss < 1000 {
                weight -= cp_loss as f32 * 0.1;
            }
            weight = weight.max(3.0);
            
            candidates.push((pv, idx, weight, cp_loss, is_blunder));
        }
        
        if candidates.is_empty() {
            self.variety_stats.pv1 += 1;
            return Some(SelectedMove {
                uci: valid[0].first_move.clone(),
                pv_index: 0,
                eval_cp: valid[0].eval_cp,
                is_blunder: false,
            });
        }
        
        // Weighted random selection
        let total_weight: f32 = candidates.iter().map(|(_, _, w, _, _)| w).sum();
        let mut rand_val = rand::thread_rng().gen::<f32>() * total_weight;
        
        let mut selected = &candidates[0];
        for candidate in &candidates {
            rand_val -= candidate.2;
            if rand_val <= 0.0 {
                selected = candidate;
                break;
            }
        }
        
        let (pv, idx, _, _, is_blunder) = selected;
        
        // Update stats
        match idx {
            0 => self.variety_stats.pv1 += 1,
            1 => self.variety_stats.pv2 += 1,
            2 => self.variety_stats.pv3 += 1,
            _ => self.variety_stats.pv4 += 1,
        }
        
        if *is_blunder {
            self.game_blunder_count += 1;
            self.variety_stats.blunders += 1;
        }
        
        Some(SelectedMove {
            uci: pv.first_move.clone(),
            pv_index: *idx as u8,
            eval_cp: pv.eval_cp,
            is_blunder: *is_blunder,
        })
    }
    
    fn filter_draw_moves(&self, pvs: &[PVLine], game: &GameState) -> Vec<PVLine> {
        pvs.iter()
            .filter(|pv| !game.would_draw(&pv.first_move))
            .cloned()
            .collect()
    }
    
    pub fn reset(&mut self) {
        self.game_blunder_count = 0;
        // Don't reset variety_stats as they track across games
    }
}

impl Default for MoveSelector {
    fn default() -> Self {
        Self::new()
    }
}
