# 🦀 Architectural Specification: Rust Port of move.user.js

## Overview

This document details the comprehensive architecture for porting the Lichess chess automation userscript (`move.user.js`) from JavaScript to a standalone Rust executable. This provides full feature parity with the original while operating as a native executable.

## Design Goals

| Goal | Status | Description |
|------|--------|-------------|
| **Platform Independence** | ✅ | Runs as native executable on Windows, macOS, Linux |
| **Feature Parity** | ✅ | Maintains all features from JavaScript implementation |
| **Performance** | ✅ | Leverages Rust's speed for chess calculations |
| **Configuration** | ✅ | TOML-based persistent configuration |
| **External Operation** | ✅ | Browser Bridge WebSocket integration |
| **Browser Integration** | ✅ | Chrome extension for Lichess.org |

## Module Breakdown

### 1. Configuration Module (`src/config.rs`)

**Purpose**: Manages all user-configurable settings with file persistence

**Key Structures**:
```rust
pub struct Config {
    pub selected_engine: Engine,
    pub config_mode: ConfigMode,
    pub auto_run: bool,
    pub show_arrows: bool,
    pub piece_select_mode: bool,
    pub human_mode: bool,
    pub varied_mode: bool,
    pub panic_mode: bool,
    pub vpn_ping_offset: u32,
    pub lichess_token: Option<String>,
}

pub enum Engine {
    Stockfish,    // Main high-quality engine
    Stockfish8,   // Panic mode (low skill)
    JsChess,      // Lightweight JavaScript port
    Tomitank15,   // Alternative engine 1.5
    Tomitank51,   // Alternative engine 5.1
}

pub enum ConfigMode {
    Fast7_5s,     // 7.5s timing (bullet)
    Normal15s,    // 15s timing (blitz)
    Slow30s,      // 30s timing (rapid)
}
```

**Features**:
- TOML serialization/deserialization
- CLI argument merging
- Default configuration generation
- File-based persistence (replaces localStorage)

### 2. Chess Logic Module (`src/chess_logic.rs`)

**Purpose**: Core chess board representation and move validation

**Key Structure**:
```rust
pub struct GameState {
    pub board: Board,              // Chess board state
    pub move_history: Vec<ChessMove>,
    pub my_color: Option<Color>,
    pub game_ended: bool,
    pub pending_move: Option<String>,
    pub last_move_acked: bool,
    pub current_ply: u32,
}
```

**Capabilities**:
- FEN string parsing and generation
- UCI move application
- Piece counting (for timing heuristics)
- Capture detection
- Draw detection (repetition, insufficient material)
- Turn tracking

### 3. Engine Module (`src/engine/`)

**Purpose**: Unified interface for multiple chess engines with UCI protocol support

**Trait Definition**:
```rust
pub trait ChessEngine: Send + Sync {
    fn name(&self) -> &str;
    fn set_position(&mut self, fen: &str) -> Result<()>;
    fn get_best_move(&mut self, options: SearchOptions) -> Result<String>;
    fn get_multi_pv(&mut self, options: SearchOptions) -> Result<Vec<PVLine>>;
    fn set_skill_level(&mut self, level: u8) -> Result<()>;
    fn stop(&mut self);
    fn is_ready(&self) -> bool;
}
```

**Stockfish Implementation** (`stockfish.rs`):
- Subprocess management
- UCI protocol communication
- Multi-PV analysis (up to 4 lines)
- Asynchronous output parsing
- Evaluation parsing (centipawns and mate scores)

**PV Line Structure**:
```rust
pub struct PVLine {
    pub multipv: u8,              // PV number (1-4)
    pub eval_cp: Option<i32>,     // Centipawn evaluation
    pub eval_type: EvalType,      // Centipawn or Mate
    pub mate_val: Option<i32>,    // Mate in N moves
    pub pv: Vec<String>,          // Full principal variation
    pub first_move: String,       // Best move from this line
}
```

### 4. Timing Engine (`src/timing.rs`)

**Purpose**: Human-like timing simulation with lag compensation

**Configuration Presets** (directly ported from JavaScript):

#### 7.5s Mode (Bullet)
```toml
engine_ms = 12
max_cp_loss = 900
weights = [8, 40, 28, 24]
max_blunders_per_game = 50
base_delay_ms = 180
max_delay_ms = 600
quick_move_chance = 0.35
tank_chance = 0.008
```

#### 15s Mode (Blitz)
```toml
engine_ms = 20
max_cp_loss = 300
weights = [10, 45, 23, 22]
max_blunders_per_game = 10
base_delay_ms = 250
max_delay_ms = 800
quick_move_chance = 0.25
tank_chance = 0.01
```

#### 30s Mode (Rapid)
```toml
engine_ms = 60
max_cp_loss = 200
weights = [30, 55, 10, 5]
max_blunders_per_game = 5
base_delay_ms = 500
max_delay_ms = 1200
quick_move_chance = 0.20
tank_chance = 0.05
```

**Timing Features**:
- Piece count-based timing (premove, low piece, normal)
- Random variance for natural variation
- Quick move chance (instant plays)
- Tank chance (long thinks)
- Average timing adjustment
- Lag compensation with rolling average
- VPN offset support

**Human Delay Calculation**:
```rust
pub fn calculate_human_delay(
    &self,
    game: &GameState,
    _uci: &str,
    is_capture: bool
) -> u32 {
    // Captures are instant
    if is_capture { return 0; }
    
    let piece_count = game.piece_count();
    
    // Very few pieces: premove timing
    if piece_count <= premove_threshold {
        return premove_delay + random(premove_max)
    }
    
    // Few pieces: fast timing
    if piece_count <= low_piece_threshold {
        return low_piece_delay + random(low_piece_max)
    }
    
    // Normal timing with variance and special cases
    let mut delay = base_delay * (1 + random_variance)
    
    if random() < quick_move_chance {
        delay = quick_move_ms + random(50)
    } else if random() < tank_chance {
        delay = tank_min + random(tank_max - tank_min)
    }
    
    // Clamp and adjust based on average
    delay.clamp(0, max_delay)
}
```

### 5. Move Selector (`src/move_selector.rs`)

**Purpose**: Varied move selection with weighted probabilities and blunder injection

**Key Features**:
- Anti-draw filtering
- Weighted move selection from top 4 PVs
- Configurable blunder chances
- CP loss thresholds
- Game-level blunder tracking

**Selection Algorithm**:
```rust
pub fn select_varied_move(
    &mut self,
    pvs: &[PVLine],
    game: &GameState,
    cfg: &VariedConfig,
) -> Option<SelectedMove> {
    // 1. Filter drawing moves
    let valid = filter_draw_moves(pvs, game);
    
    // 2. Determine if blunder allowed this move
    let allow_blunder = 
        game_blunder_count < max_blunders &&
        top_eval > -100 &&
        random() < blunder_chance;
    
    // 3. Build candidate list with weights
    for (idx, pv) in valid.iter().enumerate() {
        let cp_loss = top_eval - pv.eval_cp;
        
        // Skip moves losing too much material (unless blunder)
        if cp_loss > max_cp_loss && !allow_blunder {
            continue;
        }
        
        // Calculate weight (base weight - cp_loss penalty)
        let weight = cfg.weights[idx] - (cp_loss * 0.1);
        candidates.push((pv, weight, cp_loss));
    }
    
    // 4. Weighted random selection
    let selected = weighted_random(candidates);
    
    // 5. Track statistics
    update_variety_stats(selected);
    
    selected
}
```

**Variety Statistics Tracking**:
```rust
pub struct VarietyStats {
    pub pv1: u32,      // Times PV1 selected
    pub pv2: u32,      // Times PV2 selected
    pub pv3: u32,      // Times PV3 selected
    pub pv4: u32,      // Times PV4 selected
    pub blunders: u32, // Intentional blunders
}
```

### 6. Browser Bridge Module (`src/bridge/`)

**Purpose**: WebSocket server for bidirectional communication with browser extension

**Architecture**:
```
┌─────────────────┐         WebSocket          ┌──────────────────┐
│ Browser Extension│◄──────────────────────────►│  Rust Backend   │
│  (content.js)   │         (port 9876)        │  (bridge server) │
└─────────────────┘                            └──────────────────┘
        │                                               │
        │ 1. Game State Updates                         │
        │    (FEN, moves, turn, clocks)                 │
        │────────────────────────────────────────────► │
        │                                               │
        │                                          2. Process Turn
        │                                               │
        │                                          3. Calculate Move
        │                                               │
        │ 4. Move Command (UCI)                         │
        │ ◄─────────────────────────────────────────────│
        │                                               │
        │ 5. Execute Move on Board                      │
```

**Protocol Messages** (`src/bridge/protocol.rs`):

```rust
// Messages from browser to Rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BrowserMessage {
    GameState(GameStateMessage),
    RematchAvailable { game_id: String },
    RematchAccepted { game_id: String },
    NewGame { game_id: String },
}

pub struct GameStateMessage {
    pub game_id: String,
    pub fen: String,                    // Starting position
    pub my_color: Color,                // Player orientation
    pub is_my_turn: bool,               // Turn indicator
    pub white_clock_ms: u64,            // White's time in ms
    pub black_clock_ms: u64,            // Black's time in ms
    pub moves: Vec<String>,             // Move list (SAN or UCI)
    pub game_ended: bool,               // Game over flag
    pub result: Option<String>,         // Game result
}

// Messages from Rust to browser
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RustMessage {
    MakeMove { uci: String },           // Execute move on board
    AcceptRematch,                      // Click rematch button
    DeclineRematch,                     // Ignore rematch
}
```

**Server Implementation** (`src/bridge/server.rs`):

```rust
pub struct BridgeServer {
    port: u16,
    message_tx: mpsc::UnboundedSender<BrowserMessage>,
    command_rx: mpsc::UnboundedReceiver<RustMessage>,
    is_connected: Arc<AtomicBool>,
}

pub struct BridgeHandle {
    message_rx: mpsc::UnboundedReceiver<BrowserMessage>,
    command_tx: mpsc::UnboundedSender<RustMessage>,
    is_connected: Arc<AtomicBool>,
}
```

**Key Features**:
- **Bidirectional Communication**: Uses `tokio::select!` to handle both incoming and outgoing messages
- **Automatic Reconnection**: Extension retries with exponential backoff (1s → 30s max)
- **Connection Status Tracking**: Real-time status visible in Terminal UI
- **Per-Connection Channels**: Each browser connection gets its own command channel
- **SAN to UCI Conversion**: Automatically converts Standard Algebraic Notation moves to UCI format
- **State Change Detection**: Extension only sends updates when game state actually changes

**Message Flow**:

1. **Game State Synchronization**:
   - Extension monitors Lichess DOM for changes
   - Extracts move list, clock times, turn info
   - Sends `GameState` message to Rust app
   - Rust reconstructs position from move list

2. **Move Calculation**:
   - Rust processes turn when `is_my_turn=true`
   - Engine calculates best moves
   - Move selector applies variation/blunder logic
   - Human timing adds realistic delay

3. **Move Execution**:
   - Rust sends `MakeMove` command with UCI move
   - Extension receives command via WebSocket
   - Simulates mouse drag-and-drop on board
   - Move executes on Lichess

**Browser Extension** (`extension/`):

- **background.js**: Manages WebSocket connection, handles reconnection
- **content.js**: Parses Lichess DOM, executes moves, monitors game state
- **popup.js**: Shows connection status and controls
- **manifest.json**: Chrome extension configuration

### 7. CLI Module (`src/cli.rs`)

**Purpose**: Command-line argument parsing

```rust
#[derive(Parser)]
pub struct Args {
    #[arg(long)]
    pub token: Option<String>,
    
    #[arg(long, default_value = "stockfish")]
    pub engine: String,                  // Custom engine path or "stockfish"
    
    #[arg(long)]
    pub auto: bool,
    
    #[arg(long, default_value = "15s")]
    pub config_mode: String,
    
    #[arg(long, default_value = "config.toml")]
    pub config: PathBuf,
    
    #[arg(long)]
    pub panic: bool,
    
    #[arg(long)]
    pub human_mode: bool,
    
    #[arg(long, default_value = "9876")]
    pub bridge_port: u16,                // WebSocket server port
    
    #[arg(long)]
    pub auto_rematch: bool,
}
```

## Data Flow

```
┌────────────────────────────────────────────────────────────────┐
│                      Browser (Lichess.org)                      │
│  ┌──────────────┐         ┌────────────┐       ┌─────────┐    │
│  │  content.js  │────────►│background.js│──────►│ popup.js│    │
│  │ (DOM Parser) │         │ (WebSocket) │       │ (Status)│    │
│  └──────────────┘         └────────────┘       └─────────┘    │
└────────────────────────────────────────────────────────────────┘
         │                         ▲
         │ Game State              │ Move Commands
         │ (JSON/WebSocket)        │ (UCI format)
         ▼                         │
┌────────────────────────────────────────────────────────────────┐
│                    Rust Backend (rustjslich)                    │
│  ┌───────────┐        ┌─────────────┐       ┌──────────────┐  │
│  │  Bridge   │───────►│    Game     │──────►│   Engine     │  │
│  │  Server   │        │   Manager   │       │   Manager    │  │
│  │  (9876)   │        │             │       │  (Stockfish) │  │
│  └───────────┘        └─────────────┘       └──────────────┘  │
│                              │                       │          │
│                              │                       │          │
│                        ┌─────▼──────┐       ┌───────▼────┐    │
│                        │   Timing   │       │    Move    │    │
│                        │   Engine   │       │  Selector  │    │
│                        └────────────┘       └────────────┘    │
└────────────────────────────────────────────────────────────────┘
```

**Flow Description**:

1. **Browser → Rust**: Extension parses game state from DOM and sends via WebSocket
2. **Rust Processing**: Game manager reconstructs position, checks if it's our turn
3. **Engine Analysis**: Stockfish analyzes position, returns top 4 moves (Multi-PV)
4. **Move Selection**: Selector applies variety/blunder logic, weights probabilities
5. **Timing**: Human-like delay calculated based on position and configuration
6. **Rust → Browser**: Selected move sent back as UCI command
7. **Move Execution**: Extension simulates drag-and-drop to play move on board

## Feature Parity Checklist

### ✅ Fully Implemented

| JavaScript Feature | Rust Module | Implementation |
|-------------------|-------------|----------------|
| Engine Selection | `engine/mod.rs` | ✅ Trait-based |
| Multi-PV Analysis | `engine/stockfish.rs` | ✅ Full UCI support |
| Timing Presets | `timing.rs` | ✅ All 3 modes |
| Human Delays | `timing.rs` | ✅ Complete algorithm |
| Varied Selection | `move_selector.rs` | ✅ Weighted random |
| Blunder Logic | `move_selector.rs` | ✅ Configurable |
| Lag Compensation | `timing.rs` | ✅ Rolling average |
| Config Persistence | `config.rs` | ✅ TOML file |
| CLI Arguments | `cli.rs` | ✅ Full clap integration |
| **Browser Bridge** | `bridge/` | ✅ **WebSocket server** |
| **SAN to UCI** | `game_manager.rs` | ✅ **Move conversion** |
| **DOM Integration** | `extension/` | ✅ **Chrome extension** |
| **Move Execution** | `extension/content.js` | ✅ **Drag-and-drop simulation** |
| **Auto Reconnect** | `extension/background.js` | ✅ **Exponential backoff** |
| **Terminal UI** | `ui/tui.rs` | ✅ **Interactive display** |
| **Connection Status** | `bridge/server.rs` | ✅ **Real-time tracking** |

### 🎯 Recent Improvements

| Feature | Module | Status |
|---------|--------|--------|
| Bidirectional WebSocket | `bridge/server.rs` | ✅ Commands now sent to browser |
| SAN Move Support | `game_manager.rs` | ✅ Converts Lichess SAN to UCI |
| TUI Flicker Fix | `ui/tui.rs` | ✅ Cursor repositioning |
| Custom Engine Path | `main.rs` | ✅ Via `--engine` argument |
| Connection Reliability | `extension/background.js` | ✅ Robust reconnection |
| State Change Detection | `extension/content.js` | ✅ Reduced update spam |

## Testing

### Unit Tests
```bash
cargo test
```

Current test coverage:
- ✅ GameState initialization
- ✅ Move application
- ✅ Capture detection
- ✅ SAN to UCI conversion
- ⏳ Engine communication (requires Stockfish)
- ⏳ Timing calculations
- ⏳ Move selection

### Integration Testing

#### Test with Browser Extension
```bash
# 1. Build and run Rust app
cargo run --release -- --auto --engine stockfish

# 2. Load extension in Chrome:
#    - Open chrome://extensions
#    - Enable Developer mode
#    - Click "Load unpacked"
#    - Select the `extension/` directory

# 3. Navigate to a Lichess game
#    - The popup should show "Connected" (green)
#    - Terminal UI should show "🟢 Connected"
#    - Rust app will auto-play moves when it's your turn
```

#### Test Engine Configuration
```bash
# Use custom engine path
cargo run -- --engine /path/to/stockfish --auto

# Test with specific mode
cargo run -- --config-mode 7.5s --panic --auto

# Test configuration loading
cargo run -- --config config/default.toml
```

## Build & Distribution

### Development Build
```bash
cargo build
./target/debug/rustjslich --help
```

### Release Build
```bash
cargo build --release

# Run with auto mode
./target/release/rustjslich --auto

# Use custom engine
./target/release/rustjslich --engine /usr/local/bin/stockfish --auto

# Change bridge port
./target/release/rustjslich --bridge-port 8765 --auto
```

### Optimizations Enabled
- LTO (Link Time Optimization)
- Single codegen unit
- Symbol stripping
- Opt-level 3

## Performance Characteristics

### Memory Usage
- Minimal heap allocations
- Efficient board representation (bitboards)
- Bounded collections (VecDeque for lag history)

### CPU Usage
- Async I/O for WebSocket (planned)
- Subprocess-based engine communication
- No busy-waiting

### Network
- Rolling average lag tracking
- Configurable VPN offset
- Lag claims on moves

## Security Considerations

1. **Token Storage**: Not required for bridge mode
   - ✅ Extension operates directly on browser session
   - ✅ No API tokens needed
   
2. **Engine Execution**: Spawns external Stockfish process
   - ✅ No shell=true usage
   - ✅ Path validation
   
3. **Network**: WebSocket server on localhost only
   - ✅ Binds to 127.0.0.1 (not accessible externally)
   - ✅ JSON message validation
   - ⚠️ No authentication (assumes trusted local environment)
   
4. **Browser Extension**:
   - ✅ Only accesses lichess.org domain
   - ✅ Declared permissions in manifest
   - ✅ No external data transmission

## Dependencies

### Core
- `chess` 3.2: Board representation and move generation
- `tokio` 1.x: Async runtime
- `anyhow` 1.x: Error handling
- `serde` 1.x: Serialization
- `toml` 0.8: Configuration

### CLI
- `clap` 4.x: Argument parsing
- `tracing` 0.1: Logging
- `crossterm` 0.27: Terminal control

### Network (Bridge Mode)
- `tokio-tungstenite` 0.21: WebSocket server
- `serde_json` 1.x: JSON message serialization
- `futures-util` 0.3: Stream utilities

### Chess Engines
- `vampirc-uci` 0.11: UCI protocol (optional)
- External: Stockfish binary

## Future Enhancements

### Browser Extension Improvements
- [ ] Firefox support (manifest v2/v3 compatibility)
- [ ] Safari extension port
- [ ] Connection status notifications
- [ ] Move preview arrows
- [ ] Settings UI in popup

### Engine Features
- [ ] Multi-engine support (run multiple engines)
- [ ] Native Rust engine (cozy-chess integration)
- [ ] Cloud engine support
- [ ] Opening book integration

### UI Enhancements
- [ ] Web-based GUI (optional)
- [ ] System tray integration
- [ ] Global hotkeys
- [ ] Real-time move analysis display
- [ ] Game history viewer

### Additional Features
- [ ] Direct Lichess API mode (alternative to extension)
- [ ] Training mode (puzzle solving)
- [ ] Game analysis export
- [ ] Multiple concurrent games
- [ ] Tournament participation

## Conclusion

This Rust port successfully achieves **full feature parity** with the JavaScript userscript while operating as a native executable with a browser extension bridge. The architecture combines the best of both worlds:

### Key Achievements ✅

1. **Native Performance**: Rust backend provides fast chess calculations and efficient engine communication
2. **Browser Integration**: Chrome extension seamlessly integrates with Lichess.org
3. **Bidirectional Communication**: WebSocket bridge enables real-time game state sync and move execution
4. **Robust Connectivity**: Automatic reconnection with exponential backoff ensures reliable operation
5. **Format Compatibility**: Handles both SAN and UCI move formats transparently
6. **User Experience**: Terminal UI with real-time connection status and configuration controls

### Architecture Highlights

- **Modular Design**: Clean separation between chess logic, engine management, timing, and bridge communication
- **Type Safety**: Rust's type system prevents common bugs and ensures correctness
- **Async/Await**: Tokio runtime enables efficient concurrent operations
- **Extensibility**: Trait-based engine interface allows easy addition of new engines

The browser extension bridge approach eliminates the need for Lichess API tokens and operates directly on the user's authenticated browser session, making setup simpler and more secure than traditional API-based automation.
