# Lichess Chess Bridge - Browser Extension

This browser extension connects Lichess games to the local rustjslich Rust application.

## Installation

### Chrome/Edge

1. Open `chrome://extensions/` (or `edge://extensions/`)
2. Enable "Developer mode" in the top right
3. Click "Load unpacked"
4. Select the `extension/` directory from this repository

### Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on"
3. Select the `manifest.json` file in the `extension/` directory

Note: For Firefox, you'll need to reload the extension every time you restart the browser, or sign and install it permanently.

## Usage

1. Start the Rust application: `cargo run -- --auto`
2. Install and enable this browser extension
3. Navigate to a Lichess game (e.g., https://lichess.org/game_id)
4. The extension will automatically connect and send game state to the Rust app
5. The Rust app will analyze positions and send moves back to the extension

## How It Works

- **Background Service Worker**: Maintains WebSocket connection to local Rust server (port 9876)
- **Content Script**: Parses Lichess DOM to extract game state and executes moves
- **Popup UI**: Shows connection status and game information

## Development

The extension uses Manifest V3 for compatibility with modern browsers.

### Files

- `manifest.json` - Extension configuration
- `background.js` - WebSocket connection manager
- `content.js` - DOM parser and move executor
- `popup.html/js` - User interface
- `icons/` - Extension icons

## Protocol

Messages are JSON formatted and follow the protocol defined in `src/bridge/protocol.rs`:

From Browser to Rust:
- `game_state` - Current game position and metadata
- `rematch_available` - Rematch button appeared
- `new_game` - New game started

From Rust to Browser:
- `make_move` - Execute a move (UCI format)
- `accept_rematch` - Click rematch button
- `decline_rematch` - Ignore rematch
