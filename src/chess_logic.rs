use chess::{Board, ChessMove, Color, Square};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct GameState {
    pub board: Board,
    pub move_history: Vec<ChessMove>,
    pub my_color: Option<Color>,
    pub game_ended: bool,
    pub pending_move: Option<String>,
    pub last_move_acked: bool,
    pub current_ply: u32,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            board: Board::default(),
            move_history: Vec::new(),
            my_color: None,
            game_ended: false,
            pending_move: None,
            last_move_acked: false,
            current_ply: 0,
        }
    }
    
    pub fn from_fen(fen: &str) -> anyhow::Result<Self> {
        let board = Board::from_str(fen)
            .map_err(|e| anyhow::anyhow!("Failed to parse FEN: {:?}", e))?;
        Ok(GameState {
            board,
            move_history: Vec::new(),
            my_color: None,
            game_ended: false,
            pending_move: None,
            last_move_acked: false,
            current_ply: 0,
        })
    }
    
    pub fn apply_move(&mut self, uci: &str) -> anyhow::Result<()> {
        let chess_move = ChessMove::from_str(uci)
            .map_err(|e| anyhow::anyhow!("Failed to parse move: {:?}", e))?;
        self.board = self.board.make_move_new(chess_move);
        self.move_history.push(chess_move);
        self.current_ply += 1;
        Ok(())
    }
    
    pub fn fen(&self) -> String {
        format!("{}", self.board)
    }
    
    pub fn is_my_turn(&self) -> bool {
        if let Some(my_color) = self.my_color {
            self.board.side_to_move() == my_color
        } else {
            false
        }
    }
    
    pub fn piece_count(&self) -> u8 {
        let mut count = 0;
        for i in 0..64 {
            // SAFETY: i is in range 0..64 which is valid for Square::new
            let square = unsafe { Square::new(i) };
            if self.board.piece_on(square).is_some() {
                count += 1;
            }
        }
        count
    }
    
    pub fn is_capture(&self, uci: &str) -> bool {
        if let Ok(chess_move) = ChessMove::from_str(uci) {
            let dest = chess_move.get_dest();
            self.board.piece_on(dest).is_some()
        } else {
            false
        }
    }
    
    pub fn would_draw(&self, uci: &str) -> bool {
        if let Ok(chess_move) = ChessMove::from_str(uci) {
            let new_board = self.board.make_move_new(chess_move);
            
            // Check for insufficient material
            let white_pieces = new_board.color_combined(Color::White).popcnt();
            let black_pieces = new_board.color_combined(Color::Black).popcnt();
            
            if white_pieces <= 2 && black_pieces <= 2 {
                return true;
            }
            
            // Check for threefold repetition (simplified)
            // In a real implementation, we'd track position history
            
            false
        } else {
            false
        }
    }
    
    pub fn reset(&mut self) {
        *self = GameState::new();
    }
    
    pub fn set_my_color(&mut self, color: Color) {
        self.my_color = Some(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_game_state_new() {
        let state = GameState::new();
        assert_eq!(state.piece_count(), 32);
        assert!(!state.game_ended);
    }
    
    #[test]
    fn test_apply_move() {
        let mut state = GameState::new();
        assert!(state.apply_move("e2e4").is_ok());
        assert_eq!(state.current_ply, 1);
    }
    
    #[test]
    fn test_is_capture() {
        let mut state = GameState::new();
        state.apply_move("e2e4").unwrap();
        state.apply_move("d7d5").unwrap();
        assert!(state.is_capture("e4d5"));
    }
}
