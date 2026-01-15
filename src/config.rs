use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Engine Selection
    pub selected_engine: Engine,
    
    // Timing Presets
    pub config_mode: ConfigMode,
    
    // Feature Toggles
    pub auto_run: bool,
    pub show_arrows: bool,
    pub piece_select_mode: bool,
    pub human_mode: bool,
    pub varied_mode: bool,
    pub panic_mode: bool,
    
    // Network Settings
    pub vpn_ping_offset: u32,
    
    // Browser Bridge Settings
    pub bridge_port: u16,
    pub auto_rematch: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Engine {
    Stockfish,
    Stockfish8,
    JsChess,
    Tomitank15,
    Tomitank51,
}

impl Engine {
    pub fn as_str(&self) -> &'static str {
        match self {
            Engine::Stockfish => "stockfish",
            Engine::Stockfish8 => "stockfish8",
            Engine::JsChess => "jschess",
            Engine::Tomitank15 => "tomitank15",
            Engine::Tomitank51 => "tomitank51",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "stockfish" => Some(Engine::Stockfish),
            "stockfish8" => Some(Engine::Stockfish8),
            "jschess" => Some(Engine::JsChess),
            "tomitank15" => Some(Engine::Tomitank15),
            "tomitank51" => Some(Engine::Tomitank51),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConfigMode {
    #[serde(rename = "7.5s")]
    Fast7_5s,
    #[serde(rename = "15s")]
    Normal15s,
    #[serde(rename = "30s")]
    Slow30s,
}

impl ConfigMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigMode::Fast7_5s => "7.5s",
            ConfigMode::Normal15s => "15s",
            ConfigMode::Slow30s => "30s",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "7.5s" => Some(ConfigMode::Fast7_5s),
            "15s" => Some(ConfigMode::Normal15s),
            "30s" => Some(ConfigMode::Slow30s),
            _ => None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            selected_engine: Engine::Stockfish,
            config_mode: ConfigMode::Normal15s,
            auto_run: false,
            show_arrows: true,
            piece_select_mode: false,
            human_mode: true,
            varied_mode: true,
            panic_mode: false,
            vpn_ping_offset: 0,
            bridge_port: 9876,
            auto_rematch: false,
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
    
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
    
    pub fn merge_with_cli(&mut self, cli: &crate::cli::Args) {
        if let Some(engine) = Engine::from_str(&cli.engine) {
            self.selected_engine = engine;
        }
        
        if cli.auto {
            self.auto_run = true;
        }
        
        if let Some(mode) = ConfigMode::from_str(&cli.config_mode) {
            self.config_mode = mode;
        }
        
        if cli.panic {
            self.panic_mode = true;
        }
        
        if cli.human_mode {
            self.human_mode = true;
        }
        
        self.bridge_port = cli.bridge_port;
        
        if cli.auto_rematch {
            self.auto_rematch = true;
        }
    }
}
