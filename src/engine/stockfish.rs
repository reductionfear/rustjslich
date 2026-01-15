use super::{ChessEngine, EvalType, PVLine, SearchOptions};
use anyhow::{anyhow, Result};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct StockfishEngine {
    name: String,
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    output_lines: Arc<Mutex<Vec<String>>>,
    ready: bool,
}

impl StockfishEngine {
    pub fn new(name: &str, stockfish_path: Option<&str>) -> Result<Self> {
        let path = stockfish_path.unwrap_or("stockfish");
        
        let mut process = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn stockfish: {}. Make sure stockfish is installed.", e))?;
        
        let stdin = process.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = process.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;
        
        let output_lines = Arc::new(Mutex::new(Vec::new()));
        let output_lines_clone = output_lines.clone();
        
        // Spawn a thread to read stdout
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let mut lines = output_lines_clone.lock().unwrap();
                    lines.push(line);
                }
            }
        });
        
        let mut engine = StockfishEngine {
            name: name.to_string(),
            process: Some(process),
            stdin: Some(stdin),
            output_lines,
            ready: false,
        };
        
        // Initialize engine
        engine.send_command("uci")?;
        thread::sleep(Duration::from_millis(100));
        engine.send_command("setoption name Threads value 1")?;
        engine.send_command("setoption name Contempt value 20")?;
        engine.send_command("isready")?;
        
        // Wait for readyok
        for _ in 0..50 {
            if engine.wait_for_readyok(100) {
                engine.ready = true;
                break;
            }
        }
        
        Ok(engine)
    }
    
    fn send_command(&mut self, cmd: &str) -> Result<()> {
        if let Some(ref mut stdin) = self.stdin {
            writeln!(stdin, "{}", cmd)?;
            stdin.flush()?;
        }
        Ok(())
    }
    
    fn wait_for_readyok(&self, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(timeout_ms) {
            let lines = self.output_lines.lock().unwrap();
            if lines.iter().any(|l| l.contains("readyok")) {
                return true;
            }
            drop(lines);
            thread::sleep(Duration::from_millis(10));
        }
        false
    }
    
    fn get_and_clear_lines(&self) -> Vec<String> {
        let mut lines = self.output_lines.lock().unwrap();
        let result = lines.clone();
        lines.clear();
        result
    }
    
    fn parse_info_line(line: &str) -> Option<PVLine> {
        if !line.starts_with("info ") {
            return None;
        }
        
        let mut multipv = 1;
        let mut eval_cp = None;
        let mut mate_val = None;
        let mut eval_type = EvalType::Centipawn;
        let mut pv_moves = Vec::new();
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut i = 0;
        
        while i < parts.len() {
            match parts[i] {
                "multipv" => {
                    if i + 1 < parts.len() {
                        multipv = parts[i + 1].parse().unwrap_or(1);
                    }
                    i += 2;
                }
                "score" => {
                    if i + 2 < parts.len() {
                        match parts[i + 1] {
                            "cp" => {
                                eval_cp = parts[i + 2].parse().ok();
                                eval_type = EvalType::Centipawn;
                            }
                            "mate" => {
                                mate_val = parts[i + 2].parse().ok();
                                if let Some(m) = mate_val {
                                    eval_cp = Some(if m > 0 { 100000 + m } else { -100000 + m });
                                }
                                eval_type = EvalType::Mate;
                            }
                            _ => {}
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                "pv" => {
                    // Collect all remaining moves
                    for j in (i + 1)..parts.len() {
                        pv_moves.push(parts[j].to_string());
                    }
                    break;
                }
                _ => i += 1,
            }
        }
        
        if pv_moves.is_empty() {
            return None;
        }
        
        Some(PVLine {
            multipv,
            eval_cp,
            eval_type,
            mate_val,
            first_move: pv_moves[0].clone(),
            pv: pv_moves,
        })
    }
}

impl ChessEngine for StockfishEngine {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn set_position(&mut self, fen: &str) -> Result<()> {
        self.send_command(&format!("position fen {}", fen))
    }
    
    fn get_best_move(&mut self, options: SearchOptions) -> Result<String> {
        let pvs = self.get_multi_pv(options)?;
        pvs.first()
            .map(|pv| pv.first_move.clone())
            .ok_or_else(|| anyhow!("No best move found"))
    }
    
    fn get_multi_pv(&mut self, options: SearchOptions) -> Result<Vec<PVLine>> {
        // Set MultiPV option
        self.send_command(&format!("setoption name MultiPV value {}", options.multi_pv))?;
        thread::sleep(Duration::from_millis(10));
        
        // Clear previous output
        self.get_and_clear_lines();
        
        // Send go command
        let mut go_cmd = "go".to_string();
        if let Some(depth) = options.depth {
            go_cmd.push_str(&format!(" depth {}", depth));
        }
        if let Some(movetime) = options.movetime_ms {
            go_cmd.push_str(&format!(" movetime {}", movetime));
        }
        
        self.send_command(&go_cmd)?;
        
        // Wait for bestmove (with timeout)
        let timeout = options.movetime_ms.unwrap_or(2000) + 500;
        let start = std::time::Instant::now();
        let mut found_bestmove = false;
        
        while start.elapsed() < Duration::from_millis(timeout as u64) {
            thread::sleep(Duration::from_millis(10));
            let lines = self.output_lines.lock().unwrap();
            if lines.iter().any(|l| l.starts_with("bestmove")) {
                found_bestmove = true;
                break;
            }
        }
        
        if !found_bestmove {
            self.send_command("stop")?;
            thread::sleep(Duration::from_millis(50));
        }
        
        // Parse output
        let lines = self.get_and_clear_lines();
        let mut pvs = Vec::new();
        
        for line in lines {
            if let Some(pv) = Self::parse_info_line(&line) {
                // Update or add PV
                if let Some(existing) = pvs.iter_mut().find(|p: &&mut PVLine| p.multipv == pv.multipv) {
                    *existing = pv;
                } else {
                    pvs.push(pv);
                }
            }
        }
        
        pvs.sort_by_key(|pv| pv.multipv);
        Ok(pvs)
    }
    
    fn set_skill_level(&mut self, level: u8) -> Result<()> {
        self.send_command(&format!("setoption name Skill Level value {}", level))
    }
    
    fn stop(&mut self) {
        let _ = self.send_command("stop");
    }
    
    fn is_ready(&self) -> bool {
        self.ready
    }
}

impl Drop for StockfishEngine {
    fn drop(&mut self) {
        let _ = self.send_command("quit");
        if let Some(mut process) = self.process.take() {
            let _ = process.wait();
        }
    }
}
