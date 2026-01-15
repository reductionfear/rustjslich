# Lichess Chess Bridge - Browser Extension

This browser extension connects Lichess games to the local rustjslich Rust application via a WebSocket bridge.

## 🚀 Quick Start

### Installation

#### Chrome/Edge

1. Open `chrome://extensions/` (or `edge://extensions/`)
2. Enable "Developer mode" in the top right corner
3. Click "Load unpacked"
4. Navigate to and select the `extension/` directory from the rustjslich repository
5. The extension icon should appear in your toolbar

#### Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on"
3. Navigate to the `extension/` directory and select `manifest.json`
4. Note: In Firefox, temporary extensions are removed when you close the browser

**For permanent Firefox installation**: You'll need to [sign the extension](https://extensionworkshop.com/documentation/publish/signing-and-distribution-overview/) or disable signature verification in `about:config` (for Developer Edition/Nightly).

### Verify Installation

1. Click the extension icon in your browser toolbar
2. You should see a popup showing "Disconnected - Start Rust app"
3. The extension is now ready to connect once you start the Rust application

## 🎮 Usage

### Step 1: Start the Rust Application

Open a terminal and start rustjslich:

```bash
cd /path/to/rustjslich
cargo run --release -- --auto
```

You should see:
```
🌉 Bridge WebSocket server listening on 127.0.0.1:9876
Waiting for browser extension to connect...
```

### Step 2: Navigate to a Lichess Game

1. Open your browser and go to [lichess.org](https://lichess.org)
2. Start or join a game
3. The extension will automatically:
   - Connect to the Rust application
   - Parse the game state from the page
   - Send position updates to the Rust app
   - Execute moves received from the Rust app

### Step 3: Verify Connection

- Click the extension icon - it should show "Connected to Rust app"
- Check the Rust app terminal - you should see "Browser extension connected"
- The Rust app will now automatically analyze positions and make moves

## 🔧 How It Works

The extension consists of three main components:

### 1. Background Service Worker (`background.js`)

- Maintains WebSocket connection to `ws://127.0.0.1:9876`
- Automatically reconnects if connection is lost
- Routes messages between content script and Rust application
- Runs persistently in the background

### 2. Content Script (`content.js`)

Runs on all Lichess pages and performs:

**Game State Parsing:**
- Detects board orientation (which color you're playing)
- Parses piece positions from DOM
- Extracts move list
- Reads clock times
- Detects game end conditions
- Monitors for rematch offers

**Move Execution:**
- Receives move commands from Rust app
- Simulates mouse events to make moves
- Handles piece promotion
- Clicks rematch button if auto-rematch enabled

### 3. Popup UI (`popup.html` + `popup.js`)

- Shows connection status
- Displays current game information
- Provides manual reconnect button
- Real-time status updates

## 📡 Communication Protocol

### Messages from Browser → Rust

#### Game State Update
```json
{
  "type": "game_state",
  "game_id": "abcd1234",
  "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
  "my_color": "white",
  "is_my_turn": true,
  "white_clock_ms": 60000,
  "black_clock_ms": 60000,
  "moves": ["e2e4", "e7e5", "g1f3"],
  "game_ended": false,
  "result": null
}
```

Sent every 500ms while on a game page.

#### Rematch Available
```json
{
  "type": "rematch_available",
  "game_id": "abcd1234"
}
```

Sent when rematch button appears.

#### New Game
```json
{
  "type": "new_game",
  "game_id": "xyz78910"
}
```

Sent when navigating to a different game.

### Messages from Rust → Browser

#### Make Move
```json
{
  "type": "make_move",
  "uci": "e2e4"
}
```

Instructs the extension to make a move in UCI notation.

#### Accept Rematch
```json
{
  "type": "accept_rematch"
}
```

Clicks the rematch button.

## 🐛 Troubleshooting

### Extension Won't Connect

**Problem**: Extension shows "Disconnected"

**Solutions**:
1. Make sure the Rust application is running
2. Check that port 9876 is not blocked by a firewall
3. Try restarting both the extension and Rust app
4. Check browser console for errors (F12 → Console tab)

### Moves Not Executing

**Problem**: Rust app sends moves but nothing happens on the board

**Solutions**:
1. Refresh the Lichess page
2. Make sure you're on a game page (not just lichess.org homepage)
3. Check that it's actually your turn
4. Look for JavaScript errors in browser console
5. Try disabling other Lichess extensions that might interfere

### Game State Not Updating

**Problem**: Rust app doesn't respond to opponent moves

**Solutions**:
1. Check the browser console for parsing errors
2. Verify the content script is loaded (check browser console for "[Bridge] Content script loaded")
3. Make sure auto-play is enabled in Rust app (`--auto` flag)
4. Restart the Rust application

### Connection Keeps Dropping

**Problem**: Extension frequently disconnects/reconnects

**Solutions**:
1. Check system resources - Rust app might be crashing
2. Look for errors in Rust app terminal
3. Check for WebSocket errors in browser console
4. Try increasing system process limits

### Firefox Specific Issues

**Problem**: Extension not working after browser restart

**Solution**: This is expected - temporary extensions must be reloaded. Either:
- Reload the extension each time you open Firefox
- Sign and install the extension permanently
- Use Firefox Developer Edition with signature verification disabled

## 🔍 Debugging

### Enable Verbose Logging

**Rust Application:**
```bash
RUST_LOG=debug cargo run -- --auto
```

**Browser Console:**
1. Press F12 to open Developer Tools
2. Go to Console tab
3. Filter for "[Bridge]" to see extension messages

### Check WebSocket Connection

In browser console:
```javascript
// Check if WebSocket is connected
chrome.runtime.sendMessage({ type: 'check_connection' }, console.log);
```

### View Extension Background Logs

**Chrome/Edge:**
1. Go to `chrome://extensions/`
2. Find "Lichess Chess Bridge"
3. Click "service worker" under "Inspect views"

**Firefox:**
1. Go to `about:debugging#/runtime/this-firefox`
2. Find "Lichess Chess Bridge"
3. Click "Inspect"

## 🛠️ Development

### Testing Changes

After modifying extension files:

**Chrome/Edge:**
1. Go to `chrome://extensions/`
2. Click the refresh icon on the extension card
3. Reload any open Lichess tabs

**Firefox:**
1. Go to `about:debugging#/runtime/this-firefox`
2. Click "Reload" next to the extension
3. Reload any open Lichess tabs

### Modifying the Protocol

If you change message formats in `src/bridge/protocol.rs`:

1. Update corresponding types in `content.js`
2. Update parsing/handling logic in `background.js`
3. Test thoroughly with console logging
4. Update this README with new message formats

## 📋 File Reference

- `manifest.json` - Extension metadata and permissions
- `background.js` - WebSocket connection manager (service worker)
- `content.js` - DOM parsing and move execution (runs on Lichess pages)
- `popup.html` - Extension popup interface
- `popup.js` - Popup logic and status updates
- `icons/` - Extension icons (16x16, 48x48, 128x128)

## 🔒 Security & Privacy

- **Local Only**: All communication is between your browser and local Rust app
- **No External Servers**: No data sent to third parties
- **No Data Collection**: Extension doesn't collect or store any personal data
- **Open Source**: Full source code available for audit

## 📝 License

Same as parent project - for educational purposes only.

## ⚠️ Legal Notice

Using automation tools on Lichess.org may violate their Terms of Service. This extension is provided for educational purposes only. Use at your own risk.
