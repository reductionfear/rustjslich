use anyhow::Result;
use clap::Parser;
use rustjslich::*;
use std::time::Duration;
use tracing::{error, info, warn};

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
    
    info!("Configuration:");
    info!("  Engine: {}", config.selected_engine.as_str());
    info!("  Mode: {}", config.config_mode.as_str());
    info!("  Auto-run: {}", config.auto_run);
    info!("  Human mode: {}", config.human_mode);
    info!("  Varied mode: {}", config.varied_mode);
    info!("  Panic mode: {}", config.panic_mode);
    info!("  Bridge port: {}", config.bridge_port);
    info!("  Auto-rematch: {}", config.auto_rematch);
    
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
    
    // Initialize game manager
    let game_manager = GameManager::new(engine_manager, config.clone());
    
    // Start browser bridge server
    info!("Starting browser bridge server on port {}...", config.bridge_port);
    let (bridge_server, bridge_handle) = BridgeServer::new(config.bridge_port);
    
    // Set bridge handle in game manager
    game_manager.set_bridge_handle(bridge_handle).await;
    
    // Spawn bridge server task
    tokio::spawn(async move {
        if let Err(e) = bridge_server.run().await {
            error!("Bridge server error: {}", e);
        }
    });
    
    info!("✓ All components initialized");
    info!("");
    
    // Check if we should run with UI
    if std::env::var("NO_UI").is_ok() {
        // Run without UI (for testing or headless mode)
        run_headless(game_manager, config).await?;
    } else {
        // Run with Terminal UI
        run_with_ui(game_manager, config).await?;
    }
    
    Ok(())
}

async fn run_with_ui(game_manager: GameManager, mut config: Config) -> Result<()> {
    info!("🎮 Starting with Terminal UI");
    info!("Use hotkeys to control the bot (see UI for details)");
    info!("");
    
    // Initialize terminal UI
    let mut ui = TerminalUI::new()?;
    ui.update_from_config(&config);
    
    // Main event loop
    let mut should_quit = false;
    
    // Start game manager tasks
    let lichess_client_ref = game_manager.lichess_client.clone();
    
    // Example: Connect to a game (in a real scenario, you'd get this from a challenge or API)
    // For now, we'll just show the UI and let the user see the interface
    
    while !should_quit {
        // Update UI from current state
        let state = game_manager.state.read().await;
        ui.update_from_game(&state);
        drop(state);
        
        ui.update_from_config(&config);
        
        // Draw UI
        ui.draw()?;
        
        // Handle input with timeout
        match ui.handle_input(Duration::from_millis(100))? {
            UICommand::Quit => {
                info!("Quit command received");
                should_quit = true;
            }
            UICommand::ToggleAuto => {
                config.auto_run = !config.auto_run;
                let mut cfg = game_manager.config.write().await;
                cfg.auto_run = config.auto_run;
                info!("Auto mode: {}", config.auto_run);
            }
            UICommand::TogglePanic => {
                config.panic_mode = !config.panic_mode;
                let mut cfg = game_manager.config.write().await;
                cfg.panic_mode = config.panic_mode;
                info!("Panic mode: {}", config.panic_mode);
            }
            UICommand::ToggleHuman => {
                config.human_mode = !config.human_mode;
                let mut cfg = game_manager.config.write().await;
                cfg.human_mode = config.human_mode;
                info!("Human mode: {}", config.human_mode);
            }
            UICommand::ToggleVaried => {
                config.varied_mode = !config.varied_mode;
                let mut cfg = game_manager.config.write().await;
                cfg.varied_mode = config.varied_mode;
                info!("Varied mode: {}", config.varied_mode);
            }
            UICommand::CycleEngine => {
                config.selected_engine = match config.selected_engine {
                    Engine::Stockfish => Engine::Stockfish8,
                    Engine::Stockfish8 => Engine::JsChess,
                    Engine::JsChess => Engine::Tomitank15,
                    Engine::Tomitank15 => Engine::Tomitank51,
                    Engine::Tomitank51 => Engine::Stockfish,
                };
                let mut cfg = game_manager.config.write().await;
                cfg.selected_engine = config.selected_engine;
                info!("Engine: {}", config.selected_engine.as_str());
            }
            UICommand::CycleConfig => {
                config.config_mode = match config.config_mode {
                    ConfigMode::Fast7_5s => ConfigMode::Normal15s,
                    ConfigMode::Normal15s => ConfigMode::Slow30s,
                    ConfigMode::Slow30s => ConfigMode::Fast7_5s,
                };
                let mut cfg = game_manager.config.write().await;
                cfg.config_mode = config.config_mode;
                let mut timing = game_manager.timing_engine.write().await;
                timing.set_preset(config.config_mode);
                info!("Config mode: {}", config.config_mode.as_str());
            }
            UICommand::Hint => {
                info!("Hint requested - processing turn");
                if let Err(e) = game_manager.process_turn().await {
                    warn!("Failed to process turn: {}", e);
                }
            }
            UICommand::None => {
                // Check for bridge messages
                let mut bridge_handle = game_manager.bridge_handle.write().await;
                if let Some(ref mut handle) = *bridge_handle {
                    if let Some(message) = handle.try_recv_message() {
                        drop(bridge_handle);
                        if let Err(e) = game_manager.handle_bridge_message(message).await {
                            warn!("Failed to handle bridge message: {}", e);
                        }
                    } else {
                        drop(bridge_handle);
                    }
                } else {
                    drop(bridge_handle);
                    
                    // Fallback to Lichess WebSocket events (legacy mode)
                    let mut client = lichess_client_ref.write().await;
                    if let Some(event) = client.try_recv_event() {
                        drop(client);
                        if let Err(e) = game_manager.handle_event(event).await {
                            warn!("Failed to handle event: {}", e);
                        }
                    }
                }
            }
        }
        
        // Small delay to avoid busy loop
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    ui.cleanup()?;
    info!("Shutting down...");
    
    Ok(())
}

async fn run_headless(game_manager: GameManager, _config: Config) -> Result<()> {
    info!("🎮 Running in headless mode");
    info!("Press Ctrl+C to exit");
    info!("");
    
    // In headless mode, just demonstrate the engine
    info!("Testing engine with starting position...");
    let mut engine_manager = game_manager.engine_manager.write().await;
    
    if let Some(engine) = engine_manager.get_active_engine() {
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
    info!("Headless mode demonstration complete.");
    info!("To connect to Lichess, the full integration needs a game ID.");
    info!("See SPECIFICATION.md for details on WebSocket integration.");
    
    Ok(())
}
