# Developer Guide

## Getting Started

### Prerequisites

1. **Rust toolchain**: Install from [rustup.rs](https://rustup.rs/)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Stockfish**: Download from [stockfishchess.org](https://stockfishchess.org/download/)
   - Linux: `sudo apt install stockfish`
   - macOS: `brew install stockfish`
   - Windows: Download and add to PATH

3. **Lichess Token**: Get from [lichess.org/account/oauth/token](https://lichess.org/account/oauth/token)

### Building

```bash
# Clone the repository
git clone https://github.com/reductionfear/rustjslich.git
cd rustjslich

# Build in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release

# Run tests
cargo test

# Check code without building
cargo check
```

### Running

```bash
# Show help
cargo run -- --help

# Run with token
cargo run -- --token "lip_xxxxx"

# Run with specific configuration
cargo run -- --token "lip_xxxxx" --engine stockfish --config-mode 15s --auto --human-mode

# Run release build
./target/release/rustjslich --token "lip_xxxxx" --auto
```

## Project Structure

```
rustjslich/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library re-exports
│   ├── cli.rs               # CLI argument parsing
│   ├── config.rs            # Configuration management
│   ├── chess_logic.rs       # Chess board & game state
│   ├── timing.rs            # Human-like timing engine
│   ├── move_selector.rs     # Move selection with variance
│   └── engine/
│       ├── mod.rs           # Engine trait & manager
│       └── stockfish.rs     # Stockfish UCI implementation
├── config/
│   └── default.toml         # Default configuration
├── Cargo.toml               # Dependencies & build config
├── README.md                # User documentation
└── SPECIFICATION.md         # Architecture specification
```

## Adding a New Chess Engine

1. Implement the `ChessEngine` trait in a new file (e.g., `src/engine/myengine.rs`):

```rust
use super::{ChessEngine, PVLine, SearchOptions};
use anyhow::Result;

pub struct MyEngine {
    // Your engine state
}

impl ChessEngine for MyEngine {
    fn name(&self) -> &str {
        "MyEngine"
    }
    
    fn set_position(&mut self, fen: &str) -> Result<()> {
        // Set board position
        Ok(())
    }
    
    fn get_best_move(&mut self, options: SearchOptions) -> Result<String> {
        // Return best move in UCI format
        Ok("e2e4".to_string())
    }
    
    fn get_multi_pv(&mut self, options: SearchOptions) -> Result<Vec<PVLine>> {
        // Return multiple principal variations
        Ok(vec![])
    }
    
    fn set_skill_level(&mut self, level: u8) -> Result<()> {
        // Set engine strength (0-20)
        Ok(())
    }
    
    fn stop(&mut self) {
        // Stop current search
    }
    
    fn is_ready(&self) -> bool {
        true
    }
}
```

2. Add the engine to `src/engine/mod.rs`:
```rust
pub mod myengine;
```

3. Register in `src/main.rs`:
```rust
let my_engine = myengine::MyEngine::new()?;
engine_manager.add_engine(Box::new(my_engine));
```

## Adding a New Timing Preset

Edit `src/timing.rs` and add a new variant to `ConfigMode`:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConfigMode {
    Fast7_5s,
    Normal15s,
    Slow30s,
    Custom60s,  // New preset
}
```

Then add the preset configuration in `TimingEngine::new()`:

```rust
presets.insert(
    ConfigMode::Custom60s,
    TimingPreset {
        engine_ms: 100,
        varied: VariedConfig {
            max_cp_loss: 150,
            weights: [40, 45, 10, 5],
            max_blunders_per_game: 3,
            blunder_threshold: 100,
            blunder_chance: 0.05,
        },
        human: HumanConfig {
            base_delay_ms: 800,
            max_delay_ms: 2000,
            // ... other timing parameters
        },
    },
);
```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` to catch common issues
- Add documentation comments for public APIs
- Keep functions small and focused
- Use `Result<T>` for fallible operations
- Use `anyhow::Result` for application errors

## Testing

### Unit Tests

Add tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

Run tests:
```bash
cargo test
cargo test --lib          # Only library tests
cargo test test_name      # Specific test
cargo test -- --nocapture # Show println output
```

### Integration Tests

Create tests in `tests/` directory:

```rust
// tests/integration_test.rs
use rustjslich::*;

#[test]
fn test_full_workflow() {
    // Test complete functionality
}
```

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run
RUST_LOG=rustjslich=trace cargo run  # Very verbose
```

### Use rust-gdb or lldb

```bash
rust-gdb target/debug/rustjslich
(gdb) run --token "test"
(gdb) bt  # backtrace on crash
```

### Profiling

```bash
# CPU profiling with perf (Linux)
cargo build --release
perf record --call-graph dwarf ./target/release/rustjslich --token "test"
perf report

# Memory profiling with valgrind
cargo build
valgrind --leak-check=full ./target/debug/rustjslich --token "test"
```

## Common Issues

### Stockfish not found
```
Error: Failed to spawn stockfish: No such file or directory
```
**Solution**: Install Stockfish and ensure it's in PATH

### Configuration file not found
```
Error: No such file or directory (os error 2)
```
**Solution**: Create `config.toml` or specify path with `--config`

### Chess library errors
```
Error: Failed to parse FEN
```
**Solution**: Verify FEN string format is correct

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes and test: `cargo test`
4. Format code: `cargo fmt`
5. Check for issues: `cargo clippy`
6. Commit with descriptive message
7. Push and create a Pull Request

## Performance Tips

1. **Release builds**: Always use `--release` for production
   ```bash
   cargo build --release
   ```

2. **Engine timeout**: Adjust `movetime_ms` in `SearchOptions` for faster/slower analysis

3. **Memory usage**: Bounded collections are used throughout for predictable memory

4. **Async runtime**: Tokio runtime is configured for efficient async I/O (when WebSocket is added)

## Troubleshooting Build Issues

### Linker errors (Windows)
Install Visual Studio Build Tools or MinGW

### OpenSSL errors (Linux)
```bash
sudo apt install libssl-dev pkg-config
```

### macOS architecture issues
```bash
rustup target add x86_64-apple-darwin
cargo build --target x86_64-apple-darwin
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Clap Documentation](https://docs.rs/clap/)
- [Chess Crate Docs](https://docs.rs/chess/)
- [UCI Protocol](https://www.wbec-ridderkerk.nl/html/UCIProtocol.html)

## Next Steps

See [SPECIFICATION.md](SPECIFICATION.md) for the full architecture and planned features.

Priority areas for contribution:
1. Lichess WebSocket client implementation
2. Game manager for turn-based coordination
3. Terminal UI with ratatui
4. Additional engine implementations
5. Comprehensive integration tests
