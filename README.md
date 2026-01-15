# 🦀 rustjslich - Lichess Chess Automation in Rust

A standalone Rust executable that provides chess automation for Lichess.org with **full feature parity** to the original JavaScript userscript.

## ✨ Status: **COMPLETE** - All Phases Implemented

All 8 phases are now complete with production-ready code!

## 📋 Features

- ✅ **Multiple Chess Engines**: Support for Stockfish and other engines via UCI protocol
- ✅ **Human-Like Timing**: Sophisticated timing delays that mimic human play patterns
- ✅ **Varied Move Selection**: Weighted move selection with configurable blunder chances
- ✅ **Panic Mode**: Ultra-fast, low-skill engine for time pressure situations
- ✅ **Lag Compensation**: Automatic network lag detection and compensation
- ✅ **Configurable Presets**: Three timing profiles (7.5s, 15s, 30s) for different play styles
- ✅ **WebSocket Integration**: Full Lichess.org WebSocket communication
- ✅ **Terminal UI**: Interactive interface with hotkey controls
- ✅ **Auto-play**: Automated game playing with event handling

## 🚀 Quick Start

### Prerequisites

1. **Rust**: Install from [rustup.rs](https://rustup.rs/)
2. **Stockfish**: Download from [stockfishchess.org](https://stockfishchess.org/download/)
   - Make sure `stockfish` is in your PATH

### Installation

```bash
# Clone the repository
git clone https://github.com/reductionfear/rustjslich.git
cd rustjslich

# Build the project
cargo build --release

# The executable will be at target/release/rustjslich
```

### Configuration

1. Get a Lichess API token from [https://lichess.org/account/oauth/token](https://lichess.org/account/oauth/token)
2. Copy the default config:
   ```bash
   cp config/default.toml config.toml
   ```
3. Edit `config.toml` and add your token, or set the `LICHESS_TOKEN` environment variable

### Usage

```bash
# Run with Terminal UI (interactive)
./target/release/rustjslich --token "lip_xxxxx"

# Run in headless mode (no UI)
NO_UI=1 ./target/release/rustjslich --token "lip_xxxxx"

# Run with specific options
./target/release/rustjslich --token "lip_xxxxx" --auto --engine stockfish

# Run in panic mode
./target/release/rustjslich --panic --config-mode 7.5s

# Enable human timing mode
./target/release/rustjslich --auto --human-mode
```

## 📦 CLI Options

```
Options:
  --token <TOKEN>              Lichess API token (or set LICHESS_TOKEN env var)
  --engine <ENGINE>            Engine to use [default: stockfish]
  --auto                       Enable auto-play mode
  --config-mode <CONFIG_MODE>  Configuration preset: 7.5s, 15s, or 30s [default: 15s]
  --config <CONFIG>            Path to config file [default: config.toml]
  --panic                      Enable panic mode (fast, low-skill)
  --human-mode                 Enable human-like timing delays
  -h, --help                   Print help
```

## 🏗️ Architecture

The project is organized into several modules:

- **config**: Configuration management and persistence
- **chess_logic**: Chess board state and move validation
- **engine**: Chess engine interface and Stockfish UCI wrapper
- **timing**: Human-like delay calculation and lag compensation
- **move_selector**: Varied move selection with blunder logic
- **lichess**: Lichess API and WebSocket client (planned)
- **ui**: Terminal UI for controls (planned)

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

## 🎯 Feature Parity Status

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
- [x] Lichess WebSocket client
- [x] Game state synchronization
- [x] Event handling (moves, acks, game end)
- [x] Terminal UI with controls
- [x] Hotkey controls (A/H/V/P/E/M/Q)
- [x] Auto-play functionality
- [x] Connection management with auto-reconnect

### 📋 Optional Enhancements

- [ ] Lichess HTTP API integration (for fetching games)
- [ ] Auto-rematch functionality
- [ ] System tray integration
- [ ] Global hotkeys
- [ ] Arrow visualization (GUI mode)
- [ ] Additional engine support (panic mode, native engines)
- [ ] Statistics tracking across games

## Terminal UI Hotkeys

When running with the Terminal UI, use these hotkeys:

- `A` - Toggle auto-play mode
- `H` - Toggle human timing
- `V` - Toggle varied move selection
- `P` - Toggle panic mode
- `E` - Cycle through engines
- `M` - Cycle through config modes (7.5s/15s/30s)
- `Q` or `Ctrl+C` - Quit

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
├── Cargo.toml              # Project dependencies
├── src/
│   ├── main.rs             # Entry point with UI/headless modes
│   ├── lib.rs              # Library exports
│   ├── cli.rs              # CLI argument parsing
│   ├── config.rs           # Configuration management
│   ├── chess_logic.rs      # Board state & validation
│   ├── timing.rs           # Human-like timing engine
│   ├── move_selector.rs    # Move selection logic
│   ├── game_manager.rs     # Central game coordinator
│   ├── engine/
│   │   ├── mod.rs          # Engine trait & manager
│   │   └── stockfish.rs    # Stockfish UCI wrapper
│   ├── lichess/
│   │   ├── mod.rs          # Lichess client
│   │   ├── events.rs       # Event types
│   │   └── websocket.rs    # WebSocket handler
│   └── ui/
│       ├── mod.rs          # UI module
│       └── tui.rs          # Terminal UI implementation
├── config/
│   └── default.toml        # Default configuration
├── SPECIFICATION.md        # Architecture specification
├── DEVELOPER.md            # Developer guide
├── IMPLEMENTATION_SUMMARY.md # Completion summary
└── move.user.js            # Original JavaScript implementation
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📝 License

This project is provided as-is for educational purposes.

## ⚠️ Disclaimer

This tool is for educational purposes only. Using automation on Lichess may violate their Terms of Service. Use at your own risk.

## 🙏 Acknowledgments

- Original JavaScript implementation by Michael, Ian, and Nuro
- Stockfish chess engine team
- Lichess.org for the chess platform
- Rust chess library contributors
