# 🦀 rustjslich - Lichess Chess Automation in Rust

A standalone Rust executable that provides chess automation for Lichess.org with full feature parity to the original JavaScript userscript.

## 📋 Features

- **Multiple Chess Engines**: Support for Stockfish and other engines via UCI protocol
- **Human-Like Timing**: Sophisticated timing delays that mimic human play patterns
- **Varied Move Selection**: Weighted move selection with configurable blunder chances
- **Panic Mode**: Ultra-fast, low-skill engine for time pressure situations
- **Lag Compensation**: Automatic network lag detection and compensation
- **Configurable Presets**: Three timing profiles (7.5s, 15s, 30s) for different play styles

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
# Run with default configuration
./target/release/rustjslich

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

### ✅ Implemented
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

### 🚧 In Progress
- [ ] Lichess WebSocket client
- [ ] Lichess HTTP API integration
- [ ] Game state synchronization
- [ ] Auto-rematch functionality
- [ ] Terminal UI with controls
- [ ] Additional engine support (panic mode, native engines)

### 📋 Planned
- [ ] System tray integration
- [ ] Global hotkeys
- [ ] Arrow visualization (GUI mode)
- [ ] Piece selection mode
- [ ] Statistics tracking across games

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
│   ├── main.rs             # Entry point
│   ├── lib.rs              # Library exports
│   ├── cli.rs              # CLI argument parsing
│   ├── config.rs           # Configuration management
│   ├── chess_logic.rs      # Board state & validation
│   ├── timing.rs           # Human-like timing engine
│   ├── move_selector.rs    # Move selection logic
│   ├── engine/
│   │   ├── mod.rs          # Engine trait & manager
│   │   └── stockfish.rs    # Stockfish UCI wrapper
│   ├── lichess/            # (Planned) Lichess integration
│   └── ui/                 # (Planned) User interface
├── config/
│   └── default.toml        # Default configuration
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
