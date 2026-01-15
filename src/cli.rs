use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rustjslich")]
#[command(about = "Standalone Rust executable for Lichess chess automation", long_about = None)]
pub struct Args {
    /// Lichess API token
    #[arg(long)]
    pub token: Option<String>,
    
    /// Engine to use
    #[arg(long, default_value = "stockfish")]
    pub engine: String,
    
    /// Enable auto-play mode
    #[arg(long)]
    pub auto: bool,
    
    /// Configuration preset (7.5s, 15s, 30s)
    #[arg(long, default_value = "15s")]
    pub config_mode: String,
    
    /// Path to config file
    #[arg(long, default_value = "config.toml")]
    pub config: PathBuf,
    
    /// Enable panic mode
    #[arg(long)]
    pub panic: bool,
    
    /// Enable human timing
    #[arg(long)]
    pub human_mode: bool,
}
