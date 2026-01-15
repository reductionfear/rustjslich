# Testing Guide - Bridge Communication Fixes

## What Was Fixed

This PR fixes the critical issue where moves calculated by the Rust engine were not being sent back to the browser. The fixes include:

1. ✅ **Bidirectional WebSocket Communication** - Commands now flow from Rust → Browser
2. ✅ **SAN to UCI Conversion** - Lichess SAN moves (e.g., "Nf3") converted to UCI (e.g., "g1f3")
3. ✅ **Connection Stability** - Fixed reconnection loop with broadcast channels
4. ✅ **TUI Improvements** - No flickering, shows connection status
5. ✅ **Custom Engine Support** - Use any Stockfish-compatible engine

## Testing Instructions

### Prerequisites

1. **Install Stockfish** (or have your custom engine like `deep.exe`)
   - Download from: https://stockfishchess.org/download/
   - Or use custom engine with `--engine` flag

2. **Build the Rust Application**
   ```bash
   cargo build --release
   ```

3. **Install Browser Extension**
   - Open Chrome
   - Navigate to `chrome://extensions`
   - Enable "Developer mode"
   - Click "Load unpacked"
   - Select the `extension/` directory from this repo

### Test Scenario 1: Basic Connection

**Steps:**
```bash
# Start Rust app
./target/release/rustjslich --auto --config-mode 15s --human-mode

# Or with custom engine (e.g., deep.exe)
./rustjslich.exe --auto --config-mode 15s --human-mode --engine deep.exe
```

**Expected Results:**
- ✅ Terminal shows: `🌉 Bridge WebSocket server listening on 127.0.0.1:9876`
- ✅ Terminal shows: `Waiting for browser extension to connect...`
- ✅ Connection status: `🔴 Disconnected` (until browser connects)

**In Browser:**
1. Open Lichess.org
2. Start or join a game
3. Click the extension icon

**Expected Results:**
- ✅ Extension popup shows: `Status: Connected` (green)
- ✅ Terminal shows: `Browser extension connected from 127.0.0.1:XXXXX`
- ✅ Terminal shows: `🟢 Connected`

### Test Scenario 2: Move Execution

**Setup:**
- Rust app running with `--auto` flag
- Browser extension connected
- Active Lichess game where it's your turn

**Expected Behavior:**
1. Extension detects it's your turn
2. Sends game state to Rust app
3. Rust calculates best move (you'll see engine logs)
4. Rust sends move back to extension
5. Extension executes move on board

**What to Look For:**

In Terminal:
```
[INFO] Received from browser: {"type":"game_state","game_id":"..."}
[INFO] Processing turn for position: rnbqkbnr/...
[INFO] Engine found 4 moves in XXms
[INFO] Selected move: e2e4 (PV1)
[INFO] Sending to browser: {"type":"make_move","uci":"e2e4"}
```

In Browser Console (F12):
```
[Bridge] Sending game state: {type: 'game_state', ...}
[Bridge] Received from Rust: {type: 'make_move', uci: 'e2e4'}
[Bridge] Executing move: e2e4
```

On Board:
- ✅ Move is executed automatically
- ✅ Piece moves smoothly (drag animation)
- ✅ Game continues

### Test Scenario 3: Reconnection Stability

**Purpose:** Verify the reconnection fix works properly

**Steps:**
1. Start Rust app
2. Connect browser extension
3. **Reload the Lichess page** (simulates disconnect)
4. Wait for automatic reconnection

**Expected Results:**
- ✅ Extension shows "Disconnected" briefly
- ✅ Extension reconnects within 1-2 seconds
- ✅ Terminal shows: `Browser extension disconnected`
- ✅ Terminal shows: `Browser extension connected from 127.0.0.1:XXXXX`
- ✅ Connection remains stable (no reconnect loop)
- ✅ Moves still work after reconnection

**What NOT to See:**
- ❌ Continuous reconnect/disconnect messages
- ❌ Connection stuck in "Connecting..." state
- ❌ Moves not executing after reconnection

### Test Scenario 4: Multiple Reconnections

**Stress Test:**
1. Start Rust app
2. Reload Lichess page 3-4 times quickly
3. Let extension reconnect each time

**Expected Results:**
- ✅ Each reconnection succeeds
- ✅ Final connection is stable
- ✅ Moves work after stress test
- ✅ No memory leaks or zombie connections

### Test Scenario 5: Custom Engine

**Steps:**
```bash
# Windows
./rustjslich.exe --auto --engine deep.exe --config-mode 15s

# Linux/Mac
./rustjslich --auto --engine /path/to/stockfish --config-mode 15s
```

**Expected Results:**
- ✅ Terminal shows: `✓ deep.exe engine initialized` (or your engine path)
- ✅ Engine calculates moves correctly
- ✅ All other functionality works

### Test Scenario 6: SAN to UCI Conversion

**Purpose:** Verify move format conversion works

**What Happens:**
- Lichess DOM contains moves in SAN format: `["e4", "e5", "Nf3", "Nc6"]`
- Extension sends these to Rust
- Rust converts to UCI: `["e2e4", "e7e5", "g1f3", "b8c6"]`
- Position reconstructed correctly

**Verification:**
- ✅ Game state syncs correctly
- ✅ Engine analyzes correct position
- ✅ Suggested moves are legal for the position

## Troubleshooting

### Issue: Extension shows "Disconnected"

**Possible Causes:**
1. Rust app not running
2. Wrong port (default is 9876)
3. Firewall blocking localhost connections

**Solutions:**
- Check Rust app is running: `ps aux | grep rustjslich`
- Verify port: Look for `listening on 127.0.0.1:9876` in terminal
- Check firewall settings (localhost should be allowed)

### Issue: Moves not executing

**Check:**
1. Is `--auto` flag set? (required for automatic play)
2. Is it your turn?
3. Check browser console for errors (F12)
4. Check terminal for "Sending to browser" messages

**Debug:**
```bash
# Run with verbose logging
RUST_LOG=debug ./rustjslich --auto
```

### Issue: Reconnection loop

**If you still see reconnection loops after this fix:**
1. Check browser console for WebSocket errors
2. Verify extension manifest.json has correct permissions
3. Check if other apps are using port 9876
4. Try a different port: `--bridge-port 8765`

### Issue: Engine not found

```
⚠️  Failed to initialize engine 'deep.exe': ...
```

**Solutions:**
- Provide full path: `--engine C:\path\to\deep.exe`
- Add engine to PATH
- Verify engine is UCI-compatible: `echo "uci" | deep.exe`

## Success Criteria

✅ **All checks must pass:**

1. Extension connects successfully
2. Terminal shows `🟢 Connected`
3. Extension popup shows green "Connected" status
4. Game state updates appear in terminal logs
5. Moves are calculated by engine
6. Moves are sent back to browser (visible in logs)
7. Moves execute on board automatically
8. Reconnection works without loops
9. Connection remains stable during gameplay
10. Custom engine (if used) works correctly

## Performance Notes

**Expected Timings:**
- Connection establishment: < 100ms
- Game state update: < 50ms
- Engine calculation: 15-60ms (depending on config)
- Human delay: 180-800ms (configurable)
- Move execution: < 100ms

**Resource Usage:**
- CPU: Low when idle, spikes during engine calculation
- Memory: ~10-50MB (Rust app)
- Network: Localhost only, minimal bandwidth

## Reporting Issues

If you encounter problems, please include:

1. Full command line used
2. Terminal output (especially error messages)
3. Browser console output (F12)
4. Connection status in Terminal UI
5. Extension popup status
6. Lichess game URL (if applicable)
7. Operating system and versions

## Additional Testing

### Performance Test
```bash
# Fast mode (bullet)
./rustjslich --auto --config-mode 7.5s

# Rapid mode
./rustjslich --auto --config-mode 30s
```

### Panic Mode Test
```bash
# Uses weaker engine settings
./rustjslich --auto --panic
```

### Varied Mode Test
```bash
# Uses variety in move selection
./rustjslich --auto --varied-mode
```

## Conclusion

This fix makes the bridge communication bidirectional and stable. The Rust backend can now successfully:
- Receive game state from browser
- Calculate moves with engine
- Send moves back to browser
- Execute moves on Lichess board
- Maintain stable connection through reconnections

The broadcast channel architecture ensures that reconnections don't break command delivery, solving the continuous reconnect/disconnect issue.
