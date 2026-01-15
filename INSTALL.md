# Installation and Setup Guide

This guide will walk you through setting up rustjslich with the browser bridge extension.

## Prerequisites

### 1. Install Rust

```bash
# On Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# On Windows
# Download and run: https://rustup.rs/
```

Verify installation:
```bash
rustc --version
cargo --version
```

### 2. Install Stockfish

**Linux (Ubuntu/Debian)**:
```bash
sudo apt-get update
sudo apt-get install stockfish
```

**macOS**:
```bash
brew install stockfish
```

**Windows**:
1. Download from [stockfishchess.org](https://stockfishchess.org/download/)
2. Extract to a folder
3. Add the folder to your PATH

Verify installation:
```bash
stockfish
# Should start stockfish - press Ctrl+C to exit
```

## Installation Steps

### Step 1: Clone the Repository

```bash
git clone https://github.com/reductionfear/rustjslich.git
cd rustjslich
```

### Step 2: Build the Application

```bash
# Development build (faster compilation)
cargo build

# Or release build (optimized, recommended for actual use)
cargo build --release
```

The executable will be at:
- Development: `target/debug/rustjslich`
- Release: `target/release/rustjslich`

### Step 3: Install Browser Extension

#### For Chrome/Edge:

1. Open your browser
2. Navigate to `chrome://extensions/` (Chrome) or `edge://extensions/` (Edge)
3. Enable **Developer mode** (toggle in top-right corner)
4. Click **Load unpacked**
5. Navigate to the `rustjslich/extension/` directory
6. Click **Select Folder**
7. The extension should now appear in your extensions list

#### For Firefox:

1. Open Firefox
2. Navigate to `about:debugging#/runtime/this-firefox`
3. Click **Load Temporary Add-on...**
4. Navigate to the `rustjslich/extension/` directory
5. Select the `manifest.json` file
6. The extension will be loaded (note: it will be removed when you close Firefox)

**For permanent Firefox installation**, you need to either:
- Sign the extension through Mozilla's add-on store
- Use Firefox Developer Edition with signature checking disabled

### Step 4: Verify Extension Installation

1. Click the extension icon in your browser toolbar
2. You should see a popup showing "Disconnected - Start Rust app"
3. If you don't see the icon, click the puzzle piece icon and pin the extension

## Usage

### Basic Usage

1. **Start the Rust application**:
   ```bash
   cd rustjslich
   
   # With release build
   ./target/release/rustjslich --auto
   
   # Or with development build
   ./target/debug/rustjslich --auto
   ```

   You should see:
   ```
   🦀 rustjslich - Lichess Chess Automation
   ✓ Stockfish engine initialized
   🌉 Bridge WebSocket server listening on 127.0.0.1:9876
   Waiting for browser extension to connect...
   ```

2. **Open Lichess and start a game**:
   - Navigate to [lichess.org](https://lichess.org)
   - Start a game (Play → Create a game, or play against computer)
   - The extension will automatically connect

3. **Verify connection**:
   - Click the extension icon - should show "Connected to Rust app"
   - Check the terminal - should show "Browser extension connected"
   - The bot will now automatically make moves!

### Advanced Usage

#### Custom Configuration

Create a config file:
```bash
cp config/default.toml my-config.toml
```

Edit `my-config.toml`:
```toml
selected_engine = "stockfish"
config_mode = "15s"
auto_run = true
human_mode = true
varied_mode = true
panic_mode = false
vpn_ping_offset = 0
bridge_port = 9876
auto_rematch = true
```

Run with custom config:
```bash
./target/release/rustjslich --config my-config.toml
```

#### Different Time Controls

For bullet (fast) games:
```bash
./target/release/rustjslich --auto --config-mode 7.5s
```

For rapid (slow) games:
```bash
./target/release/rustjslich --auto --config-mode 30s
```

#### Custom WebSocket Port

If port 9876 is already in use:
```bash
./target/release/rustjslich --auto --bridge-port 8888
```

Note: You'll need to modify `extension/background.js` to match the new port.

#### Headless Mode (No UI)

Run without the terminal UI:
```bash
NO_UI=1 ./target/release/rustjslich --auto
```

#### With Verbose Logging

For debugging:
```bash
RUST_LOG=debug ./target/release/rustjslich --auto
```

### Terminal UI Hotkeys

When running with the terminal UI (default):

- `A` - Toggle auto-play mode on/off
- `H` - Toggle human-like timing
- `V` - Toggle varied move selection
- `P` - Toggle panic mode
- `E` - Cycle through available engines
- `M` - Cycle through timing modes (7.5s/15s/30s)
- `Q` or `Ctrl+C` - Quit application

## Troubleshooting

### "Failed to initialize Stockfish"

**Problem**: Stockfish not found in PATH

**Solution**:
```bash
# Check if stockfish is installed
which stockfish  # Linux/macOS
where stockfish  # Windows

# If not found, install it (see Prerequisites above)
```

### Extension shows "Disconnected"

**Problem**: Can't connect to Rust application

**Solutions**:
1. Make sure the Rust app is running
2. Check that you used the `--auto` flag
3. Verify port 9876 is not blocked by firewall
4. Check terminal for errors
5. Try restarting both the extension and app

### Moves not executing in browser

**Problem**: Rust app sends moves but nothing happens

**Solutions**:
1. Refresh the Lichess page
2. Make sure you're on an active game page
3. Check browser console (F12) for JavaScript errors
4. Verify it's your turn to move
5. Try disabling other Lichess extensions

### Extension disconnects frequently

**Problem**: Connection drops repeatedly

**Solutions**:
1. Check system resources - app might be crashing
2. Look for panic messages in terminal
3. Try reducing engine analysis time (`--config-mode 7.5s`)
4. Check network/firewall settings

### Firefox: Extension gone after restart

**Problem**: Extension disappears when closing Firefox

**Solution**: This is expected behavior for "temporary" extensions. Options:
- Reload the extension each time (quick)
- Sign and permanently install the extension
- Use Firefox Developer Edition with signature checks disabled

## Testing the Setup

### Quick Test

1. Start the Rust app:
   ```bash
   ./target/release/rustjslich --auto
   ```

2. Go to [lichess.org/setup/ai](https://lichess.org/setup/ai)

3. Set up a game against the computer:
   - Choose "Computer" opponent
   - Select your color
   - Start the game

4. If playing as Black, make your first move
5. The bot should automatically respond!

### Verification Checklist

- [ ] Rust app starts without errors
- [ ] "Bridge WebSocket server listening" appears
- [ ] Extension icon shows "Connected to Rust app"
- [ ] Terminal shows "Browser extension connected"
- [ ] Bot makes moves automatically
- [ ] Moves appear reasonable (not random)
- [ ] Bot responds to opponent moves

## Performance Tips

### For Faster Games (Bullet)

```bash
./target/release/rustjslich --auto --config-mode 7.5s --panic
```

This uses:
- Minimal engine time (12ms)
- Panic mode for faster responses
- Quick human-like delays

### For Stronger Play (Rapid)

```bash
./target/release/rustjslich --auto --config-mode 30s --human-mode
```

This uses:
- More engine time (60ms)
- Human-like timing delays
- More accurate move selection

### Resource Usage

- CPU: Depends on Stockfish analysis time
- Memory: ~50-100 MB for Rust app
- Network: Minimal (local WebSocket only)

## Uninstallation

### Remove Extension

**Chrome/Edge**:
1. Go to `chrome://extensions/`
2. Find "Lichess Chess Bridge"
3. Click "Remove"

**Firefox**:
1. Go to `about:addons`
2. Find "Lichess Chess Bridge"
3. Click "Remove"

### Remove Application

```bash
cd rustjslich
cargo clean  # Remove build artifacts
cd ..
rm -rf rustjslich  # Remove entire directory
```

## Next Steps

- Read [README.md](README.md) for full feature documentation
- Check [extension/README.md](extension/README.md) for extension details
- See [SPECIFICATION.md](SPECIFICATION.md) for architecture details
- Explore configuration options in `config/default.toml`

## Getting Help

If you encounter issues:

1. Check this guide's troubleshooting section
2. Look for error messages in:
   - Terminal running Rust app
   - Browser console (F12 → Console)
   - Extension logs (chrome://extensions → Details → Inspect service worker)
3. Enable debug logging: `RUST_LOG=debug cargo run -- --auto`
4. Check the GitHub repository for known issues

## Legal Notice

⚠️ **Important**: Using automation on Lichess.org violates their Terms of Service. This software is provided for educational purposes only. Use at your own risk. Your account may be banned if detected.
