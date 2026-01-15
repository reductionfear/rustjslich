use anyhow::Result;
use clap::Parser;
use rustjslich::*;
use tracing::{info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Parse CLI arguments
    let args = cli::Args::parse();
    
    info!("🦀 rustjslich - Lichess Chess Automation");
    info!("Starting with config: {:?}", args.config);
    
    // Load or create config
    let mut config = if args.config.exists() {
        info!("Loading config from: {}", args.config.display());
        Config::load_from_file(&args.config)?
    } else {
        info!("Using default configuration");
        Config::default()
    };
    
    // Merge CLI args into config
    config.merge_with_cli(&args);
    
    // Validate token
    if config.lichess_token.is_none() {
        warn!("⚠️  No Lichess token provided!");
        warn!("Please provide a token via --token or LICHESS_TOKEN environment variable");
        warn!("You can get a token from: https://lichess.org/account/oauth/token");
        return Ok(());
    }
    
    info!("Configuration:");
    info!("  Engine: {}", config.selected_engine.as_str());
    info!("  Mode: {}", config.config_mode.as_str());
    info!("  Auto-run: {}", config.auto_run);
    info!("  Human mode: {}", config.human_mode);
    info!("  Varied mode: {}", config.varied_mode);
    info!("  Panic mode: {}", config.panic_mode);
    
    // Initialize components
    info!("Initializing chess engine...");
    let mut engine_manager = EngineManager::new();
    
    // Try to add Stockfish engine
    match engine::stockfish::StockfishEngine::new("stockfish", None) {
        Ok(engine) => {
            info!("✓ Stockfish engine initialized");
            engine_manager.add_engine(Box::new(engine));
        }
        Err(e) => {
            warn!("⚠️  Failed to initialize Stockfish: {}", e);
            warn!("Make sure Stockfish is installed and in your PATH");
            warn!("You can install it from: https://stockfishchess.org/download/");
            return Ok(());
        }
    }
    
    // Initialize other components
    let _timing_engine = TimingEngine::new(config.vpn_ping_offset);
    let _move_selector = MoveSelector::new();
    let _game_state = GameState::new();
    
    info!("✓ All components initialized");
    info!("");
    info!("🎮 Ready to play!");
    info!("Note: Full Lichess integration (WebSocket, API) is not yet implemented.");
    info!("This is a demonstration of the core engine and timing systems.");
    info!("");
    
    // Demo: Test the engine with a position
    if let Some(engine) = engine_manager.get_active_engine() {
        info!("Testing engine with starting position...");
        engine.set_position("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?;
        
        let search_options = SearchOptions {
            depth: None,
            movetime_ms: Some(100),
            multi_pv: 4,
        };
        
        let pvs = engine.get_multi_pv(search_options)?;
        
        info!("Engine analysis (top {} moves):", pvs.len());
        for (i, pv) in pvs.iter().enumerate() {
            let eval_str = if let Some(eval) = pv.eval_cp {
                format!("{:+.2}", eval as f32 / 100.0)
            } else {
                "N/A".to_string()
            };
            info!("  {}. {} (eval: {})", i + 1, pv.first_move, eval_str);
        }
    }
    
    info!("");
    info!("To connect to Lichess, the WebSocket client needs to be implemented.");
    info!("See src/lichess/ for the planned implementation.");
    
    Ok(())
}
