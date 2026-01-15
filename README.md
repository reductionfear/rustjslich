# 🦀 rustjslich - Lichess Chess Automation in Rust

A standalone Rust executable that provides chess automation for Lichess.org using a **browser bridge architecture** - no API tokens required!

## ✨ Status: **COMPLETE** - Browser Bridge Mode Implemented

All features are now complete with a browser extension bridge that connects directly to Lichess games through the DOM.

## 📋 Features

- ✅ **Browser Bridge Mode**: Connect via browser extension - no Lichess API token needed
- ✅ **Multiple Chess Engines**: Support for Stockfish and other engines via UCI protocol
- ✅ **Human-Like Timing**: Sophisticated timing delays that mimic human play patterns
- ✅ **Varied Move Selection**: Weighted move selection with configurable blunder chances
- ✅ **Panic Mode**: Ultra-fast, low-skill engine for time pressure situations
- ✅ **Lag Compensation**: Automatic network lag detection and compensation
- ✅ **Configurable Presets**: Three timing profiles (7.5s, 15s, 30s) for different play styles
- ✅ **Terminal UI**: Interactive interface with hotkey controls
- ✅ **Auto-play**: Automated game playing with event handling
- ✅ **Auto-rematch**: Automatically accept rematch offers

## 🚀 Quick Start

### Prerequisites

1. **Rust**: Install from [rustup.rs](https://rustup.rs/)
2. **Stockfish**: Download from [stockfishchess.org](https://stockfishchess.org/download/)
   - Make sure `stockfish` is in your PATH
3. **Web Browser**: Chrome, Edge, or Firefox

### Installation

```bash
# Clone the repository
git clone https://github.com/reductionfear/rustjslich.git
cd rustjslich

# Build the project
cargo build --release

# The executable will be at target/release/rustjslich
```

### Browser Extension Setup

1. **Load the Extension**:
   
   **For Chrome/Edge**:
   - Open `chrome://extensions/` (or `edge://extensions/`)
   - Enable "Developer mode" (top right)
   - Click "Load unpacked"
   - Select the `extension/` directory from this repository
   
   **For Firefox**:
   - Open `about:debugging#/runtime/this-firefox`
   - Click "Load Temporary Add-on"
   - Select `extension/manifest.json`

2. **Verify Extension**: Click the extension icon - you should see "Disconnected - Start Rust app"

### Usage

```bash
# Start the Rust application with auto-play enabled
./target/release/rustjslich --auto

# Or run with custom settings
./target/release/rustjslich --auto --config-mode 15s --human-mode

# Run in headless mode (no UI)
NO_UI=1 ./target/release/rustjslich --auto

# Enable auto-rematch
./target/release/rustjslich --auto --auto-rematch
```

3. **Play a Game**:
   - Navigate to a Lichess game in your browser
   - The extension will connect automatically
   - The Rust app will analyze positions and make moves
   - Watch the terminal for status updates

## 📦 CLI Options

```
Options:
  --engine <ENGINE>            Engine to use [default: stockfish]
  --auto                       Enable auto-play mode
  --config-mode <CONFIG_MODE>  Configuration preset: 7.5s, 15s, or 30s [default: 15s]
  --config <CONFIG>            Path to config file [default: config.toml]
  --panic                      Enable panic mode (fast, low-skill)
  --human-mode                 Enable human-like timing delays
  --bridge-port <PORT>         WebSocket port for browser bridge [default: 9876]
  --auto-rematch               Enable automatic rematch acceptance
  -h, --help                   Print help
```

## 🏗️ Architecture

The project uses a **browser bridge architecture**:

```
┌─────────────────────┐         WebSocket          ┌──────────────────────┐
│                     │      (localhost:9876)       │                      │
│  Browser Extension  │◄───────────────────────────►│   Rust Application   │
│   (JavaScript)      │                             │    (rustjslich)      │
│                     │                             │                      │
│  - DOM Parser       │     Game State Messages     │  - Chess Engine      │
│  - Move Executor    │────────────────────────────►│  - Move Analysis     │
│  - Rematch Handler  │                             │  - Timing Engine     │
│                     │◄────────────────────────────│  - Move Selection    │
│                     │      Move Commands          │                      │
└─────────────────────┘                             └──────────────────────┘
         │                                                     │
         │                                                     │
         ▼                                                     ▼
┌─────────────────────┐                             ┌──────────────────────┐
│   Lichess Website   │                             │   Stockfish Engine   │
│   (lichess.org)     │                             │   (UCI Protocol)     │
└─────────────────────┘                             └──────────────────────┘
```

### Components:

- **Browser Extension**: 
  - Runs on Lichess pages
  - Parses game state from DOM
  - Executes moves via simulated clicks
  - No API token required

- **Rust Application**:
  - WebSocket server for extension communication
  - Chess engine integration
  - Move analysis and selection
  - Human-like timing simulation

## 🔧 Configuration Presets

### 7.5s Mode (Fast)
- Engine time: 12ms
- Aggressive varied play with high blunder tolerance
- Quick human delays
- Suitable for bullet games

### 15s Mode (Normal)
- Engine time: 20ms
- Balanced varied play
- Moderate human delays
- Suitable for blitz games

### 30s Mode (Slow)
- Engine time: 60ms
- Precise play with minimal blunders
- Longer human delays
- Suitable for rapid games

## 🎯 Feature Status

### ✅ Fully Implemented

- [x] Core chess logic and board representation
- [x] Stockfish UCI engine integration
- [x] Multi-PV analysis
- [x] Timing presets (7.5s, 15s, 30s)
- [x] Human-like delay calculations
- [x] Varied move selection
- [x] Blunder logic
- [x] Lag compensation
- [x] Configuration management
- [x] CLI argument parsing
- [x] **Browser Bridge Architecture**
- [x] **Browser Extension (Chrome/Edge/Firefox)**
- [x] **DOM-based game state parsing**
- [x] **Move execution via browser**
- [x] **WebSocket communication**
- [x] Game state synchronization
- [x] Terminal UI with controls
- [x] Hotkey controls (A/H/V/P/E/M/Q)
- [x] Auto-play functionality
- [x] Auto-rematch support

### 📋 Legacy Features (Deprecated)

- [~] Lichess WebSocket client (replaced by browser bridge)
- [~] Lichess API token authentication (no longer needed)

## Terminal UI Hotkeys

When running with the Terminal UI, use these hotkeys:

- `A` - Toggle auto-play mode
- `H` - Toggle human timing
- `V` - Toggle varied move selection
- `P` - Toggle panic mode
- `E` - Cycle through engines
- `M` - Cycle through config modes (7.5s/15s/30s)
- `Q` or `Ctrl+C` - Quit

## 🔌 Browser Bridge Protocol

The extension and Rust app communicate via WebSocket with JSON messages:

### From Browser to Rust:
```json
{
  "type": "game_state",
  "game_id": "abcd1234",
  "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
  "my_color": "white",
  "is_my_turn": true,
  "white_clock_ms": 60000,
  "black_clock_ms": 60000,
  "moves": ["e2e4", "e7e5"],
  "game_ended": false
}
```

### From Rust to Browser:
```json
{
  "type": "make_move",
  "uci": "e2e4"
}
```

## 🛠️ Development

```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=info cargo run

# Build for release
cargo build --release

# Format code
cargo fmt

# Run linter
cargo clippy
```

## 📊 Project Structure

```
rustjslich/
├── Cargo.toml                  # Project dependencies
├── src/
│   ├── main.rs                 # Entry point with UI/headless modes
│   ├── lib.rs                  # Library exports
│   ├── cli.rs                  # CLI argument parsing
│   ├── config.rs               # Configuration management
│   ├── chess_logic.rs          # Board state & validation
│   ├── timing.rs               # Human-like timing engine
│   ├── move_selector.rs        # Move selection logic
│   ├── game_manager.rs         # Central game coordinator
│   ├── bridge/
│   │   ├── mod.rs              # Bridge module
│   │   ├── protocol.rs         # Message types
│   │   └── server.rs           # WebSocket server
│   ├── engine/
│   │   ├── mod.rs              # Engine trait & manager
│   │   └── stockfish.rs        # Stockfish UCI wrapper
│   ├── lichess/
│   │   ├── mod.rs              # Lichess client (legacy)
│   │   ├── events.rs           # Event types
│   │   └── websocket.rs        # WebSocket handler
│   └── ui/
│       ├── mod.rs              # UI module
│       └── tui.rs              # Terminal UI implementation
├── extension/
│   ├── manifest.json           # Extension config
│   ├── background.js           # Service worker
│   ├── content.js              # DOM parser/executor
│   ├── popup.html              # Extension popup
│   ├── popup.js                # Popup logic
│   └── icons/                  # Extension icons
├── config/
│   └── default.toml            # Default configuration
└── README.md                   # This file
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📝 License

This project is provided as-is for educational purposes.

## ⚠️ Disclaimer

This tool is for educational purposes only. Using automation on Lichess may violate their Terms of Service. Use at your own risk.

**Browser Bridge Mode**: This implementation uses a browser extension to interact with Lichess through the DOM, similar to how a human would interact with the website. However, automated play is still against Lichess Terms of Service.

## 🙏 Acknowledgments

- Original JavaScript implementation by Michael, Ian, and Nuro
- Stockfish chess engine team
- Lichess.org for the chess platform
- Rust chess library contributors
