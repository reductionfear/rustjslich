// Content script for Lichess Chess Bridge
// Parses game state from DOM and executes moves

console.log('[Bridge] Content script loaded');

let isConnected = false;
let gameId = null;
let updateInterval = null;

// Check connection status
chrome.runtime.sendMessage({ type: 'check_connection' }, (response) => {
  isConnected = response && response.connected;
  console.log('[Bridge] Connection status:', isConnected);
});

// Listen for connection status updates
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'bridge_connected') {
    isConnected = true;
    console.log('[Bridge] Connected to Rust app');
    startMonitoring();
  } else if (message.type === 'bridge_disconnected') {
    isConnected = false;
    console.log('[Bridge] Disconnected from Rust app');
    stopMonitoring();
  } else if (message.type === 'rust_command') {
    handleRustCommand(message.command);
  }
});

// Parse board orientation
function getBoardOrientation() {
  const cgWrap = document.querySelector('.cg-wrap');
  if (!cgWrap) return null;
  return cgWrap.classList.contains('orientation-white') ? 'white' : 'black';
}

// Convert pixel coordinates to chess square
function pixelToSquare(x, y, orientation) {
  const squareSize = 73; // pixels per square on Lichess
  const file = Math.floor(x / squareSize);
  const rank = 7 - Math.floor(y / squareSize);
  
  if (orientation === 'black') {
    return String.fromCharCode(104 - file) + (rank + 1); // 'h' is 104
  }
  return String.fromCharCode(97 + file) + (rank + 1); // 'a' is 97
}

// Parse piece positions from DOM
function parsePiecePositions() {
  const pieces = {};
  const orientation = getBoardOrientation();
  if (!orientation) return null;
  
  const pieceElements = document.querySelectorAll('.cg-board piece');
  pieceElements.forEach(piece => {
    const style = piece.getAttribute('style');
    if (!style) return;
    
    const match = style.match(/translate\((\d+)px,\s*(\d+)px\)/);
    if (match) {
      const x = parseInt(match[1]);
      const y = parseInt(match[2]);
      const square = pixelToSquare(x, y, orientation);
      
      const colorClass = piece.classList.contains('white') ? 'w' : 'b';
      let pieceType = '';
      
      if (piece.classList.contains('pawn')) pieceType = 'P';
      else if (piece.classList.contains('knight')) pieceType = 'N';
      else if (piece.classList.contains('bishop')) pieceType = 'B';
      else if (piece.classList.contains('rook')) pieceType = 'R';
      else if (piece.classList.contains('queen')) pieceType = 'Q';
      else if (piece.classList.contains('king')) pieceType = 'K';
      
      pieces[square] = colorClass + pieceType;
    }
  });
  
  return pieces;
}

// Get FEN from current position
function getFEN() {
  // Try to get FEN from Lichess's internal data
  if (typeof lichess !== 'undefined' && lichess.analysis && lichess.analysis.node) {
    return lichess.analysis.node.fen;
  }
  
  // Fallback: construct from DOM (simplified - may not be accurate for all positions)
  const pieces = parsePiecePositions();
  if (!pieces) return null;
  
  // This is a simplified FEN builder - for production, we'd need full parsing
  // For now, return null and rely on move list
  return null;
}

// Get move list
function getMoveList() {
  const moves = [];
  const moveElements = document.querySelectorAll('l4x kwdb');
  moveElements.forEach(el => {
    const moveText = el.textContent.trim();
    if (moveText) moves.push(moveText);
  });
  return moves;
}

// Get clock times
function getClockTimes() {
  const whiteClock = document.querySelector('.rclock-white .time');
  const blackClock = document.querySelector('.rclock-black .time');
  
  const whiteTime = whiteClock ? parseClock(whiteClock.textContent) : 0;
  const blackTime = blackClock ? parseClock(blackClock.textContent) : 0;
  
  return { white: whiteTime, black: blackTime };
}

// Parse clock time (MM:SS or SS.d) to milliseconds
function parseClock(timeStr) {
  if (!timeStr) return 0;
  
  if (timeStr.includes(':')) {
    const [min, sec] = timeStr.split(':').map(s => parseInt(s) || 0);
    return (min * 60 + sec) * 1000;
  } else {
    return Math.round(parseFloat(timeStr) * 1000);
  }
}

// Determine whose turn it is
function isWhiteTurn() {
  const whiteClock = document.querySelector('.rclock-white');
  return whiteClock && whiteClock.classList.contains('running');
}

// Check if game has ended
function isGameEnded() {
  return !!document.querySelector('.result-wrap') || !!document.querySelector('.status');
}

// Get game ID from URL
function getGameId() {
  const path = window.location.pathname;
  const match = path.match(/^\/([a-zA-Z0-9]{8,})/);
  return match ? match[1] : null;
}

// Check for rematch button
function checkRematch() {
  const rematchButton = document.querySelector('button.rematch');
  const rematchOffered = !!document.querySelector('.rematch-offer');
  return { available: !!rematchButton, offered: rematchOffered };
}

// Parse and send game state
function parseAndSendGameState() {
  if (!isConnected) return;
  
  const currentGameId = getGameId();
  if (!currentGameId) return;
  
  // Check if game changed
  if (gameId && gameId !== currentGameId) {
    console.log('[Bridge] New game detected');
    chrome.runtime.sendMessage({
      type: 'new_game',
      game_id: currentGameId
    });
  }
  gameId = currentGameId;
  
  const orientation = getBoardOrientation();
  if (!orientation) return;
  
  const moves = getMoveList();
  const clocks = getClockTimes();
  const whiteTurn = isWhiteTurn();
  const gameEnded = isGameEnded();
  
  // Construct FEN from move list (start position + moves)
  let fen = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
  // TODO: Apply moves to get current FEN (would need chess library)
  // For now, we send the move list and let Rust reconstruct the position
  
  const gameState = {
    type: 'game_state',
    game_id: gameId,
    fen: fen,
    my_color: orientation,
    is_my_turn: (orientation === 'white' && whiteTurn) || (orientation === 'black' && !whiteTurn),
    white_clock_ms: clocks.white,
    black_clock_ms: clocks.black,
    moves: moves,
    game_ended: gameEnded,
    result: gameEnded ? getGameResult() : null
  };
  
  chrome.runtime.sendMessage(gameState);
  
  // Check for rematch
  const rematch = checkRematch();
  if (rematch.available && !rematch.offered) {
    chrome.runtime.sendMessage({
      type: 'rematch_available',
      game_id: gameId
    });
  }
}

// Get game result
function getGameResult() {
  const result = document.querySelector('.result-wrap');
  return result ? result.textContent.trim() : null;
}

// Execute a move received from Rust
function executeMove(uci) {
  console.log('[Bridge] Executing move:', uci);
  
  if (uci.length < 4) {
    console.error('[Bridge] Invalid UCI move:', uci);
    return;
  }
  
  const from = uci.substring(0, 2);
  const to = uci.substring(2, 4);
  const promotion = uci.length > 4 ? uci[4] : null;
  
  // Find the squares
  const fromSquare = document.querySelector(`.cg-board square.${from}`);
  const toSquare = document.querySelector(`.cg-board square.${to}`);
  
  if (!fromSquare || !toSquare) {
    console.error('[Bridge] Could not find squares for move:', from, to);
    
    // Fallback: try using Lichess API if available
    if (typeof lichess !== 'undefined' && lichess.socket) {
      lichess.socket.send('move', {
        u: uci,
        b: 1 // blur
      });
      console.log('[Bridge] Sent move via Lichess socket');
    }
    return;
  }
  
  // Simulate drag and drop
  const fromRect = fromSquare.getBoundingClientRect();
  const toRect = toSquare.getBoundingClientRect();
  
  // Create and dispatch mouse events
  const mouseDownEvent = new MouseEvent('mousedown', {
    bubbles: true,
    cancelable: true,
    view: window,
    clientX: fromRect.left + fromRect.width / 2,
    clientY: fromRect.top + fromRect.height / 2
  });
  
  const mouseUpEvent = new MouseEvent('mouseup', {
    bubbles: true,
    cancelable: true,
    view: window,
    clientX: toRect.left + toRect.width / 2,
    clientY: toRect.top + toRect.height / 2
  });
  
  fromSquare.dispatchEvent(mouseDownEvent);
  
  setTimeout(() => {
    toSquare.dispatchEvent(mouseUpEvent);
    
    // Handle promotion
    if (promotion) {
      setTimeout(() => {
        const promotionPiece = document.querySelector(`.promotion-choice .${promotion}`);
        if (promotionPiece) {
          promotionPiece.click();
        }
      }, 100);
    }
  }, 50);
}

// Handle command from Rust
function handleRustCommand(command) {
  console.log('[Bridge] Received command:', command);
  
  switch (command.type) {
    case 'make_move':
      executeMove(command.uci);
      break;
    case 'accept_rematch':
      const rematchButton = document.querySelector('button.rematch');
      if (rematchButton) {
        rematchButton.click();
        console.log('[Bridge] Accepted rematch');
      }
      break;
    case 'decline_rematch':
      // No action needed - just don't click rematch
      console.log('[Bridge] Declined rematch');
      break;
  }
}

// Start monitoring game state
function startMonitoring() {
  if (updateInterval) return;
  
  console.log('[Bridge] Starting game state monitoring');
  
  // Send initial state
  parseAndSendGameState();
  
  // Update every 500ms
  updateInterval = setInterval(parseAndSendGameState, 500);
}

// Stop monitoring
function stopMonitoring() {
  if (updateInterval) {
    clearInterval(updateInterval);
    updateInterval = null;
  }
  console.log('[Bridge] Stopped game state monitoring');
}

// Start monitoring if on a game page and connected
if (window.location.pathname.match(/^\/[a-zA-Z0-9]{8,}/)) {
  if (isConnected) {
    startMonitoring();
  }
}

// Monitor URL changes (for SPA navigation)
let lastUrl = location.href;
new MutationObserver(() => {
  const url = location.href;
  if (url !== lastUrl) {
    lastUrl = url;
    gameId = null; // Reset game ID
    
    if (url.match(/lichess\.org\/[a-zA-Z0-9]{8,}/)) {
      console.log('[Bridge] Navigated to game page');
      if (isConnected) {
        startMonitoring();
      }
    } else {
      stopMonitoring();
    }
  }
}).observe(document, { subtree: true, childList: true });
