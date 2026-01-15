// ==UserScript==
// @name        Lichess Funnies v44.5 (Conflict Fix + NoLogs)
// @version     44.5
// @description Chess automation with engine selection & full UI controls (jQuery Conflict Fixed)
// @author      Michael, Ian, Nuro (merged)
// @match       https://lichess.org/*
// @icon        https://www.google.com/s2/favicons?sz=64&domain=lichess.org
// @grant       none
// @run-at      document-start
// @require     https://raw.githubusercontent.com/workinworld/stkfish/main/stockfish8.js
// @require     https://code.jquery.com/jquery-3.6.0.min.js
// @require     https://raw.githubusercontent.com/mchappychen/lichess-funnies/main/chess.js
// @require     https://raw.githubusercontent.com/mchappychen/lichess-funnies/main/stockfish.js
// @require     https://raw.githubusercontent.com/redwhitedaffodil/josechjs/refs/heads/main/dist/js-chess-engine.js
// @require     https://raw.githubusercontent.com/reductionfear/jsengimore/refs/heads/main/tomitankChess_1_5.js
// @require     https://raw.githubusercontent.com/reductionfear/jsengimore/refs/heads/main/tomitankChess_5_1.js
// ==/UserScript==

/* globals jQuery, $, Chess, stockfish, STOCKFISH, lichess, game */

// --- CRITICAL FIX FOR POWERTIP ERROR ---
// This line prevents your script from breaking Lichess's UI.
// It keeps the userscript's jQuery local and restores Lichess's global jQuery.
const $ = window.jQuery.noConflict(true);

// --- ENGINE SELECTION ---
let selectedEngine = localStorage.getItem('selectedEngine') || 'stockfish';

// --- STRENGTH SETTINGS ---
const JS_CHESS_AI_LEVEL = 1;
const PANIC_LEVEL = 0;

// --- VPN/Network Lag Compensation ---
let vpnPingOffset = parseInt(localStorage.getItem('vpnPingOffset')) || 0;
let serverLagHistory = [50, 50, 50];
const MAX_LAG_HISTORY = 5;

function updateServerLag(clockData) {
  if (clockData && typeof clockData.lag === 'number') {
    const lagMs = clockData.lag * 10;
    serverLagHistory.push(lagMs);
    if (serverLagHistory.length > MAX_LAG_HISTORY) {
      serverLagHistory.shift();
    }
  }
}

function getAverageServerLag() {
  const sum = serverLagHistory.reduce((a, b) => a + b, 0);
  return Math.round(sum / serverLagHistory.length);
}

function getLagCompensation() {
  const avgServerLag = getAverageServerLag();
  const totalLag = avgServerLag + vpnPingOffset;
  const maxReasonable = Math.max(avgServerLag * 2, 100);
  return Math.min(totalLag, maxReasonable);
}

function getPanicLagCompensation() {
  const avgServerLag = getAverageServerLag();
  const totalLag = avgServerLag + vpnPingOffset + 30;
  const maxReasonable = Math.max(avgServerLag * 3, 200);
  return Math.min(totalLag, maxReasonable);
}

// --- JS-CHESS-ENGINE STATE TRACKING ---
let jsChessPendingMove = null;
let jsChessPendingFen = null;
let jsChessRetryCount = 0;
let jsChessWaitingForReconnect = false;
const JS_CHESS_MAX_RETRIES = 5;
const JS_CHESS_RETRY_INTERVAL = 100;

// --- TOMITANK ENGINE STATE TRACKING ---
let tomitank15Calculating = false;
let tomitank15PendingMove = null;
let tomitank15PendingFen = null;
const TOMITANK15_TIMEOUT_MS = 100;
const TOMITANK15_DEPTH = 2;
const TOMITANK15_PANIC_DEPTH = 1;
const TOMITANK15_PANIC_SEARCH_TIME = 50;
let tomitank51Calculating = false;
let tomitank51PendingMove = null;
let tomitank51PendingFen = null;
const TOMITANK51_TIMEOUT_MS = 100;
const TOMITANK51_DEPTH = 2;
const TOMITANK51_PANIC_DEPTH = 1;
const TOMITANK51_PANIC_SEARCH_TIME = 20;

// --- Game State Tracking ---
let gameEnded = false;
let lastMoveAcked = false;
let pendingMoveUci = null;

function resetGameState() {
  gameEnded = false;
  lastMoveAcked = false;
  pendingMoveUci = null;
  pendingMove = false;
  isProcessing = false;
  lastMoveSent = null;
  lastMoveSentTime = 0;
  cachedPVs = null;
  cachedPVsFen = null;
  cachedPieceCount = null;
  cachedFen = null;
  serverLagHistory = [50, 50, 50];
  humanTimingStats = { totalMoves: 0, totalTimeMs: 0, engineTimeMs: 0 };
  varietyStats = { pv1: 0, pv2: 0, pv3: 0, pv4: 0, blunders: 0 };
  gameBlunderCount = 0;
  panicModeEnabled = false;
  panicEngineCalculating = false;
  panicLastRequestTime = 0;
  panicLastFenRequested = null;
  jsChessPendingMove = null;
  jsChessPendingFen = null;
  jsChessRetryCount = 0;
  jsChessWaitingForReconnect = false;
  tomitank15PendingMove = null;
  tomitank15PendingFen = null;
  tomitank15Calculating = false;
  tomitank51PendingMove = null;
  tomitank51PendingFen = null;
  tomitank51Calculating = false;
}

// --- PANIC ENGINE (stockfish8.js - Skill Level 0) ---
let panicEngine = null;
let panicEngineReady = false;
let panicCurrentFen = "";
let panicBestMove = null;
let panicModeEnabled = false;

let panicEngineCalculating = false;
let panicLastRequestTime = 0;
let panicLastFenRequested = null;
let panicWatchdogTimer = null;
let panicEngineRetryCount = 0;
const PANIC_TIMEOUT_MS = 500;
const PANIC_MAX_RETRIES = 3;

function initializePanicEngine() {
  if (panicEngine) return;

  try {
    panicEngine = window.STOCKFISH();
    panicEngine.postMessage("uci");
    panicEngine.postMessage(`setoption name Skill Level value ${PANIC_LEVEL}`);
    panicEngine.postMessage("setoption name Hash value 16");

    panicEngine.onmessage = function(event) {
      if (event && typeof event === 'string' && event.includes("bestmove")) {
        panicBestMove = event.split(" ")[1];
        panicEngineCalculating = false;
        panicEngineRetryCount = 0;

        if (panicWatchdogTimer) {
          clearTimeout(panicWatchdogTimer);
          panicWatchdogTimer = null;
        }

        if (gameEnded) return;
        if (pendingMoveUci) return;

        if (!webSocketWrapper || webSocketWrapper.readyState !== 1) {
          scheduleReconnectRetry();
          return;
        }

        if (panicModeEnabled && panicBestMove) {
          const lagClaim = getPanicLagCompensation();
          webSocketWrapper.send(JSON.stringify({
            t: "move",
            d: { u: panicBestMove, a: currentAck, b: 1, l: lagClaim }
          }));
          pendingMove = false;
          isProcessing = false;
        }
      }
    };
    panicEngineReady = true;
  } catch (e) {
    panicEngineReady = false;
  }
}

function reinitializePanicEngine() {
  panicEngine = null;
  panicEngineReady = false;
  panicEngineCalculating = false;
  panicEngineRetryCount = 0;
  initializePanicEngine();
}

function handlePanicTimeout() {
  panicEngineCalculating = false;
  panicEngineRetryCount++;

  if (panicEngineRetryCount >= PANIC_MAX_RETRIES) {
    reinitializePanicEngine();
    panicEngineRetryCount = 0;
  }

  if (panicModeEnabled && !gameEnded && !pendingMoveUci) {
    const cgWrap = $('.cg-wrap')[0];
    if (cgWrap) {
      const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
      if (game.turn() === myCol) {
        setTimeout(() => {
          isProcessing = false;
          pendingMove = false;
          processTurn();
        }, 50);
      }
    }
  }
}

let reconnectRetryScheduled = false;

function scheduleReconnectRetry() {
  if (reconnectRetryScheduled) return;
  reconnectRetryScheduled = true;

  const checkReconnect = setInterval(() => {
    if (webSocketWrapper && webSocketWrapper.readyState === 1) {
      clearInterval(checkReconnect);
      reconnectRetryScheduled = false;

      if (!gameEnded) {
        panicEngineCalculating = false;
        isProcessing = false;
        pendingMove = false;
        pendingMoveUci = null;

        if (selectedEngine === 'jschess' && jsChessPendingMove) {
          jsChessWaitingForReconnect = false;
          const currentFen = game.fen();
          const cgWrap = $('.cg-wrap')[0];

          if (cgWrap) {
            const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';

            if (game.turn() === myCol) {
              const cachedFenPieces = jsChessPendingFen ? jsChessPendingFen.split(' ')[0] : null;
              const currentFenPieces = currentFen.split(' ')[0];

              if (cachedFenPieces === currentFenPieces && jsChessPendingMove) {
                const lagClaim = panicModeEnabled ? getPanicLagCompensation() : getLagCompensation();
                webSocketWrapper.send(JSON.stringify({
                  t: "move",
                  d: { u: jsChessPendingMove, a: currentAck, b: panicModeEnabled ? 1 : 0, l: lagClaim }
                }));
                jsChessPendingMove = null;
                jsChessPendingFen = null;
                jsChessRetryCount = 0;
                return;
              } else {
                jsChessPendingMove = null;
                jsChessPendingFen = null;
                jsChessRetryCount = 0;
              }
            }
          }
        }

        if (autoHint) {
          const cgWrap = $('.cg-wrap')[0];
          if (cgWrap) {
            const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
            if (game.turn() === myCol) {
              setTimeout(processTurn, 50);
            }
          }
        }
      }
    }
  }, 50);

  setTimeout(() => {
    clearInterval(checkReconnect);
    reconnectRetryScheduled = false;
    jsChessWaitingForReconnect = false;
  }, 10000);
}

function panicCalculateMoveSF8(fen) {
  if (panicEngineCalculating) {
    const elapsed = Date.now() - panicLastRequestTime;
    if (elapsed < PANIC_TIMEOUT_MS) {
      return;
    } else {
      panicEngineCalculating = false;
    }
  }

  if (fen === panicLastFenRequested && panicBestMove) {
    if (panicModeEnabled && !gameEnded && !pendingMoveUci && webSocketWrapper?.readyState === 1) {
      const lagClaim = getPanicLagCompensation();
      webSocketWrapper.send(JSON.stringify({
        t: "move",
        d: { u: panicBestMove, a: currentAck, b: 1, l: lagClaim }
      }));
      pendingMove = false;
      isProcessing = false;
    }
    return;
  }

  if (!panicEngine || !panicEngineReady) {
    initializePanicEngine();
    if (!panicEngineReady) {
      setTimeout(() => panicCalculateMoveSF8(fen), 50);
      return;
    }
  }

  panicEngineCalculating = true;
  panicLastRequestTime = Date.now();
  panicLastFenRequested = fen;
  panicCurrentFen = fen;
  panicBestMove = null;

  if (panicWatchdogTimer) {
    clearTimeout(panicWatchdogTimer);
  }
  panicWatchdogTimer = setTimeout(handlePanicTimeout, PANIC_TIMEOUT_MS);

  try {
    panicEngine.postMessage("stop");
    panicEngine.postMessage("position fen " + fen);
    panicEngine.postMessage("go depth 1");
  } catch (e) {
    panicEngineCalculating = false;
    if (panicWatchdogTimer) {
      clearTimeout(panicWatchdogTimer);
      panicWatchdogTimer = null;
    }
    reinitializePanicEngine();
  }
}

// --- JS-CHESS-ENGINE FUNCTIONS ---
function completeFen(partialFen) {
  const parts = partialFen.split(" ");
  if (parts.length === 2) {
    return `${parts[0]} ${parts[1]} KQkq - 0 1`;
  }
  return partialFen;
}

function convertMoveToLichess(move) {
  const from = Object.keys(move)[0].toLowerCase();
  const to = Object.values(move)[0].toLowerCase();
  return from + to;
}

function trySendJsChessMove() {
  if (!jsChessPendingMove) return false;

  if (gameEnded) {
    jsChessPendingMove = null;
    jsChessPendingFen = null;
    jsChessRetryCount = 0;
    jsChessWaitingForReconnect = false;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  if (pendingMoveUci) {
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  if (!webSocketWrapper || webSocketWrapper.readyState !== 1) {
    jsChessRetryCount++;

    if (jsChessRetryCount >= JS_CHESS_MAX_RETRIES) {
      jsChessWaitingForReconnect = true;
      isProcessing = false;
      pendingMove = false;
      scheduleReconnectRetry();
      return false;
    }

    setTimeout(() => {
      if (jsChessPendingMove && !jsChessWaitingForReconnect) {
        trySendJsChessMove();
      }
    }, JS_CHESS_RETRY_INTERVAL);

    return false;
  }

  const cgWrap = $('.cg-wrap')[0];
  if (!cgWrap) {
    jsChessPendingMove = null;
    jsChessPendingFen = null;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
  if (game.turn() !== myCol) {
    jsChessPendingMove = null;
    jsChessPendingFen = null;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  const currentFenPieces = game.fen().split(' ')[0];
  const cachedFenPieces = jsChessPendingFen ? jsChessPendingFen.split(' ')[0] : null;

  if (cachedFenPieces && currentFenPieces !== cachedFenPieces) {
    jsChessPendingMove = null;
    jsChessPendingFen = null;
    jsChessRetryCount = 0;
    isProcessing = false;
    pendingMove = false;
    setTimeout(processTurn, 50);
    return false;
  }

  const lagClaim = panicModeEnabled ? getPanicLagCompensation() : getLagCompensation();

  webSocketWrapper.send(JSON.stringify({
    t: "move",
    d: { u: jsChessPendingMove, a: currentAck, b: panicModeEnabled ? 1 : 0, l: lagClaim }
  }));

  jsChessPendingMove = null;
  jsChessPendingFen = null;
  jsChessRetryCount = 0;
  jsChessWaitingForReconnect = false;
  pendingMove = false;
  isProcessing = false;
  return true;
}

function jsChessCalculateMove(fen) {
  try {
    if (jsChessWaitingForReconnect && jsChessPendingMove) {
      const currentFenPieces = fen.split(' ')[0];
      const cachedFenPieces = jsChessPendingFen ? jsChessPendingFen.split(' ')[0] : null;

      if (cachedFenPieces === currentFenPieces) {
        return;
      }
    }

    if (jsChessPendingMove && jsChessPendingFen) {
      const currentFenPieces = fen.split(' ')[0];
      const cachedFenPieces = jsChessPendingFen.split(' ')[0];

      if (cachedFenPieces === currentFenPieces) {
        trySendJsChessMove();
        return;
      }
    }

    const fullFen = completeFen(fen);
    const jsChessEngine = window["js-chess-engine"];

    if (!jsChessEngine) {
      isProcessing = false;
      pendingMove = false;
      return;
    }

    const levelToUse = panicModeEnabled ? PANIC_LEVEL : JS_CHESS_AI_LEVEL;
    const move = jsChessEngine.aiMove(fullFen, levelToUse);
    const bestMove = convertMoveToLichess(move);

    jsChessPendingMove = bestMove;
    jsChessPendingFen = fen;
    jsChessRetryCount = 0;
    jsChessWaitingForReconnect = false;

    trySendJsChessMove();

  } catch (error) {
    jsChessPendingMove = null;
    jsChessPendingFen = null;
    jsChessRetryCount = 0;
    jsChessWaitingForReconnect = false;
    isProcessing = false;
    pendingMove = false;
  }
}

// --- TOMITANK 1.5 ENGINE FUNCTIONS ---
function tomitank15CalculateMove(fen) {
  if (tomitank15Calculating) return;

  if (tomitank15PendingMove && tomitank15PendingFen) {
    const currentFenPieces = fen.split(' ')[0];
    const cachedFenPieces = tomitank15PendingFen.split(' ')[0];
    if (cachedFenPieces === currentFenPieces) {
      trySendTomitank15Move();
      return;
    }
  }

  tomitank15Calculating = true;
  tomitank15PendingFen = fen;
  tomitank15PendingMove = null;
  panicLastRequestTime = Date.now();

  const depthToUse = panicModeEnabled ? 1 : TOMITANK15_DEPTH;
  const searchTime = panicModeEnabled ? 20 : 50;
  const timeoutMs = panicModeEnabled ? 50 : TOMITANK15_TIMEOUT_MS;

  try {
    const timeoutId = setTimeout(() => {
      if (tomitank15Calculating) {
        tomitank15Calculating = false;
        isProcessing = false;
        pendingMove = false;

        if (panicModeEnabled && !gameEnded) {
          jsChessCalculateMove(fen);
        }
      }
    }, timeoutMs);

    let move = null;

    if (typeof window.initBoard === 'function' && typeof window.search === 'function') {
      window.initBoard(fen);
      move = window.search(depthToUse, searchTime);
    }
    else if (typeof window.SetFEN === 'function' && typeof window.Search === 'function') {
      window.SetFEN(fen);
      move = window.Search(depthToUse);
    }
    else if (typeof window.FENToBoard === 'function' && typeof window.SearchPosition === 'function') {
      window.START_FEN = fen;
      window.FENToBoard();
      if (typeof window.maxSearchTime !== 'undefined') {
        window.maxSearchTime = searchTime;
      }
      window.SearchPosition(depthToUse);
      if (window.bestMove) {
        move = typeof window.FormatMove === 'function'
          ? window.FormatMove(window.bestMove)
          : window.bestMove;
      }
    }
    else if (typeof window.TomitankChess !== 'undefined') {
      const engine = new window.TomitankChess();
      engine.setFEN(fen);
      move = engine.search(depthToUse);
    }
    else if (typeof window.engine !== 'undefined' && typeof window.engine.search === 'function') {
      window.engine.setFEN ? window.engine.setFEN(fen) : null;
      move = window.engine.search(depthToUse);
    }

    if (move) {
      if (typeof move === 'object') {
        move = move.from + move.to + (move.promotion || '');
      }
      move = String(move).toLowerCase().replace(/[^a-h1-8qrbn]/g, '');

      if (move.length >= 4) {
        tomitank15PendingMove = move;
        clearTimeout(timeoutId);
        tomitank15Calculating = false;
        trySendTomitank15Move();
        return;
      }
    }

    clearTimeout(timeoutId);
    tomitank15Calculating = false;
    isProcessing = false;
    pendingMove = false;

  } catch (e) {
    tomitank15Calculating = false;
    isProcessing = false;
    pendingMove = false;
  }
}

function trySendTomitank15Move() {
  if (!tomitank15PendingMove) {
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  if (gameEnded) {
    tomitank15PendingMove = null;
    tomitank15PendingFen = null;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  if (pendingMoveUci) {
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  if (!webSocketWrapper || webSocketWrapper.readyState !== 1) {
    scheduleReconnectRetry();
    return false;
  }

  const cgWrap = $('.cg-wrap')[0];
  if (!cgWrap) {
    tomitank15PendingMove = null;
    tomitank15PendingFen = null;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
  if (game.turn() !== myCol) {
    tomitank15PendingMove = null;
    tomitank15PendingFen = null;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  const lagClaim = panicModeEnabled ? getPanicLagCompensation() : getLagCompensation();

  webSocketWrapper.send(JSON.stringify({
    t: "move",
    d: { u: tomitank15PendingMove, a: currentAck, b: panicModeEnabled ? 1 : 0, l: lagClaim }
  }));

  tomitank15PendingMove = null;
  tomitank15PendingFen = null;
  isProcessing = false;
  pendingMove = false;
  return true;
}

// --- TOMITANK 5.1 ENGINE FUNCTIONS ---
function tomitank51CalculateMove(fen) {
  if (tomitank51Calculating) return;

  if (tomitank51PendingMove && tomitank51PendingFen) {
    const currentFenPieces = fen.split(' ')[0];
    const cachedFenPieces = tomitank51PendingFen.split(' ')[0];
    if (cachedFenPieces === currentFenPieces) {
      trySendTomitank51Move();
      return;
    }
  }

  tomitank51Calculating = true;
  tomitank51PendingFen = fen;
  tomitank51PendingMove = null;
  panicLastRequestTime = Date.now();

  const depthToUse = panicModeEnabled ? TOMITANK51_PANIC_DEPTH : TOMITANK51_DEPTH;
  const searchTime = panicModeEnabled ? TOMITANK51_PANIC_SEARCH_TIME : 50;
  const timeoutMs = panicModeEnabled ? 50 : TOMITANK51_TIMEOUT_MS;

  try {
    const timeoutId = setTimeout(() => {
      if (tomitank51Calculating) {
        tomitank51Calculating = false;
        isProcessing = false;
        pendingMove = false;
      }
    }, timeoutMs);

    if (typeof window.START_FEN !== 'undefined') {
      window.START_FEN = fen;
    }
    if (typeof window.FENToBoard === 'function') {
      window.FENToBoard();
    }
    if (typeof window.maxSearchTime !== 'undefined') {
      window.maxSearchTime = searchTime;
    }

    if (typeof window.SearchPosition === 'function') {
      window.SearchPosition(depthToUse);

      if (typeof window.bestMove !== 'undefined' && window.bestMove) {
        const move = typeof window.FormatMove === 'function'
          ? window.FormatMove(window.bestMove.move || window.bestMove)
          : String(window.bestMove);

        tomitank51PendingMove = move;
        clearTimeout(timeoutId);
        tomitank51Calculating = false;
        trySendTomitank51Move();
      } else {
        clearTimeout(timeoutId);
        tomitank51Calculating = false;
        isProcessing = false;
        pendingMove = false;
      }
    }
  } catch (e) {
    tomitank51Calculating = false;
    isProcessing = false;
    pendingMove = false;
  }
}

function trySendTomitank51Move() {
  if (!tomitank51PendingMove) {
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  if (gameEnded) {
    tomitank51PendingMove = null;
    tomitank51PendingFen = null;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  if (pendingMoveUci) {
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  if (!webSocketWrapper || webSocketWrapper.readyState !== 1) {
    scheduleReconnectRetry();
    return false;
  }

  const cgWrap = $('.cg-wrap')[0];
  if (!cgWrap) {
    tomitank51PendingMove = null;
    tomitank51PendingFen = null;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
  if (game.turn() !== myCol) {
    tomitank51PendingMove = null;
    tomitank51PendingFen = null;
    isProcessing = false;
    pendingMove = false;
    return false;
  }

  const lagClaim = panicModeEnabled ? getPanicLagCompensation() : getLagCompensation();

  webSocketWrapper.send(JSON.stringify({
    t: "move",
    d: { u: tomitank51PendingMove, a: currentAck, b: panicModeEnabled ? 1 : 0, l: lagClaim }
  }));

  tomitank51PendingMove = null;
  tomitank51PendingFen = null;
  isProcessing = false;
  pendingMove = false;
  return true;
}

// --- socket wrapper ---
let webSocketWrapper = null;
let currentAck = 0;
let lastWebSocketState = null;

const webSocketProxy = new Proxy(window.WebSocket, {
  construct: function(target, args) {
    let ws = new target(...args);
    webSocketWrapper = ws;

    const originalSend = ws.send.bind(ws);
    ws.send = function(data) {
      try {
        const msg = JSON.parse(data);
        if (msg.t === 'move' && msg.d && msg.d.u) {
          if (gameEnded) return;
          if (pendingMoveUci === msg.d.u && !lastMoveAcked) return;
          pendingMoveUci = msg.d.u;
          lastMoveAcked = false;
        }
      } catch (e) {}
      return originalSend(data);
    };

    ws.addEventListener("open", function() {
      const wasDisconnected = lastWebSocketState === 3 || lastWebSocketState === null;
      lastWebSocketState = 1;

      panicEngineCalculating = false;
      pendingMoveUci = null;
      isProcessing = false;
      pendingMove = false;

      if (selectedEngine === 'jschess' && jsChessPendingMove && !gameEnded) {
        jsChessWaitingForReconnect = false;

        const cgWrap = $('.cg-wrap')[0];
        if (cgWrap) {
          const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';

          if (game.turn() === myCol) {
            const currentFenPieces = game.fen().split(' ')[0];
            const cachedFenPieces = jsChessPendingFen ? jsChessPendingFen.split(' ')[0] : null;

            if (cachedFenPieces === currentFenPieces) {
              setTimeout(() => {
                if (webSocketWrapper && webSocketWrapper.readyState === 1 && jsChessPendingMove) {
                  trySendJsChessMove();
                }
              }, 50);
              return;
            } else {
              jsChessPendingMove = null;
              jsChessPendingFen = null;
              jsChessRetryCount = 0;
            }
          }
        }
      }

      if (!gameEnded && autoHint) {
        const cgWrap = $('.cg-wrap')[0];
        if (cgWrap) {
          const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
          if (game.turn() === myCol) {
            setTimeout(processTurn, 100);
          }
        }
      }
    });

    ws.addEventListener("close", function() {
      lastWebSocketState = 3;

      if (selectedEngine === 'jschess' && jsChessPendingMove) {
        jsChessWaitingForReconnect = true;
      }
    });

    ws.addEventListener("error", function() {
      panicEngineCalculating = false;

      if (selectedEngine === 'jschess' && jsChessPendingMove) {
        jsChessWaitingForReconnect = true;
      }
    });

    ws.addEventListener("message", function(event) {
      try {
        let msg = JSON.parse(event.data);

        if (msg.t === 'ack') {
          lastMoveAcked = true;
          pendingMoveUci = null;

          if (selectedEngine === 'jschess') {
            jsChessPendingMove = null;
            jsChessPendingFen = null;
            jsChessRetryCount = 0;
            jsChessWaitingForReconnect = false;
          }
        }

        if (msg.t === 'endData' || (msg.d && msg.d.status && msg.d.winner)) {
          gameEnded = true;
          isProcessing = false;
          pendingMove = false;
          panicEngineCalculating = false;
          jsChessPendingMove = null;
          jsChessPendingFen = null;
          jsChessWaitingForReconnect = false;
          tomitank15PendingMove = null;
          tomitank15PendingFen = null;
          tomitank15Calculating = false;
          tomitank51PendingMove = null;
          tomitank51PendingFen = null;
          tomitank51Calculating = false;
        }

        if (msg.t === 'move' && msg.d) {
          if (typeof msg.d.ply !== 'undefined') {
            currentAck = msg.d.ply;
          }

          if (msg.d.clock) {
            updateServerLag(msg.d.clock);
          }

          if (msg.d.status || msg.d.winner) {
            gameEnded = true;
            isProcessing = false;
            pendingMove = false;
            panicEngineCalculating = false;
            jsChessPendingMove = null;
            jsChessPendingFen = null;
            jsChessWaitingForReconnect = false;
          }

          if (msg.d.uci === pendingMoveUci) {
            pendingMoveUci = null;
          }
        }

        if (!gameEnded && panicModeEnabled && msg.d && typeof msg.d.fen === "string" && typeof msg.v === "number") {
          if (autoHint && !pendingMoveUci) {
            let interceptedFen = msg.d.fen;
            let isWhitesTurn = msg.v % 2 == 0;
            interceptedFen += isWhitesTurn ? " w" : " b";

            const cgWrap = $('.cg-wrap')[0];
            if (cgWrap) {
              const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
              const turnChar = isWhitesTurn ? 'w' : 'b';
              if (myCol === turnChar) {
                if (selectedEngine === 'jschess') {
                  jsChessCalculateMove(interceptedFen);
                } else if (selectedEngine === 'tomitank15') {
                  tomitank15CalculateMove(interceptedFen);
                } else if (selectedEngine === 'tomitank51') {
                  tomitank51CalculateMove(interceptedFen);
                } else {
                  panicCalculateMoveSF8(interceptedFen);
                }
              }
            }
          }
        }

        if (msg.t === 'reload' || msg.t === 'resync') {
          panicEngineCalculating = false;
          pendingMoveUci = null;
          isProcessing = false;
          pendingMove = false;
          jsChessPendingMove = null;
          jsChessPendingFen = null;
          jsChessRetryCount = 0;
          jsChessWaitingForReconnect = false;
        }
      } catch (e) {}
    });
    return ws;
  }
});
window.WebSocket = webSocketProxy;

window.lichess = window.site;
window.game = new Chess();

// --- Settings State ---
var autoRun = localStorage.getItem('autorun') ?? "0";
var showArrows = localStorage.getItem('showArrows') !== "0";
var autoHint = autoRun == "1";
var pieceSelectMode = localStorage.getItem('pieceSelectMode') === "1";
var humanMode = localStorage.getItem('humanMode') === "1";
var variedMode = localStorage.getItem('variedMode') !== "0";
var configMode = localStorage.getItem('configMode') || "15s";

// --- CONFIG PRESETS ---
const PRESETS = {
  '7.5s': {
    engineMs: 12,
    varied: {
      maxCpLoss: 900,
      weights: [8, 40, 28, 24],
      maxBlundersPerGame: 50,
      blunderThreshold: 100,
      blunderChance: 0.45,
    },
    human: {
      baseDelayMs: 180,
      maxDelayMs: 600,
      premoveDelayMs: 0,
      premoveMaxMs: 10,
      lowPieceDelayMs: 25,
      lowPieceMaxMs: 120,
      premovePieceThreshold: 12,
      lowPieceThreshold: 22,
      quickMoveChance: 0.35,
      quickMoveMs: 0,
      tankChance: 0.008,
      tankMinMs: 250,
      tankMaxMs: 500,
      randomVariance: 0.25,
    }
  },
  '15s': {
    engineMs: 20,
    varied: {
      maxCpLoss: 300,
      weights: [10, 45, 23, 22],
      maxBlundersPerGame: 10,
      blunderThreshold: 100,
      blunderChance: 0.16,
    },
    human: {
      baseDelayMs: 250,
      maxDelayMs: 800,
      premoveDelayMs: 0,
      premoveMaxMs: 20,
      lowPieceDelayMs: 30,
      lowPieceMaxMs: 150,
      premovePieceThreshold: 10,
      lowPieceThreshold: 20,
      quickMoveChance: 0.25,
      quickMoveMs: 0,
      tankChance: 0.01,
      tankMinMs: 400,
      tankMaxMs: 600,
      randomVariance: 0.27,
    }
  },
  '30s': {
    engineMs: 60,
    varied: {
      maxCpLoss: 200,
      weights: [30, 55, 10, 5],
      maxBlundersPerGame: 5,
      blunderThreshold: 100,
      blunderChance: 0.08,
    },
    human: {
      baseDelayMs: 500,
      maxDelayMs: 1200,
      premoveDelayMs: 50,
      premoveMaxMs: 150,
      lowPieceDelayMs: 100,
      lowPieceMaxMs: 500,
      premovePieceThreshold: 8,
      lowPieceThreshold: 16,
      quickMoveChance: 0.20,
      quickMoveMs: 60,
      tankChance: 0.05,
      tankMinMs: 1000,
      tankMaxMs: 2000,
      randomVariance: 0.37,
    }
  }
};

// --- Active Config Globals ---
if (!PRESETS[configMode]) configMode = '15s';
let activeHuman = PRESETS[configMode].human;
let activeVaried = PRESETS[configMode].varied;
let activeEngineMs = PRESETS[configMode].engineMs;

function applyConfig(mode) {
  configMode = mode;
  localStorage.setItem('configMode', mode);
  activeHuman = PRESETS[mode].human;
  activeVaried = PRESETS[mode].varied;
  activeEngineMs = PRESETS[mode].engineMs;
}

// --- Stats & State Variables ---
let humanTimingStats = { totalMoves: 0, totalTimeMs: 0, engineTimeMs: 0 };
let varietyStats = { pv1: 0, pv2: 0, pv3: 0, pv4: 0, blunders: 0 };
let gameBlunderCount = 0;
let cachedPVs = null;
let cachedPVsFen = null;
let cachedPieceCount = null;
let cachedFen = null;
let engineReady = false;
let pendingMove = false;
let isProcessing = false;
let lastMoveSent = null;
let lastMoveSentTime = 0;

// --- Helpers ---
function waitForElement(sel) {
  return new Promise(res => {
    const el = document.querySelector(sel);
    if (el) { res(el); return; }
    const obs = new MutationObserver(() => {
      const el = document.querySelector(sel);
      if (el) { obs.disconnect(); res(el); }
    });
    obs.observe(document.documentElement, { childList: true, subtree: true });
  });
}

// --- Clock Parser ---
function getClockSeconds() {
  let clockEl = document.querySelector('.rclock-bottom .time');

  if (!clockEl) {
    const cgWrap = document.querySelector('.cg-wrap');
    if (cgWrap) {
      const isWhite = cgWrap.classList.contains('orientation-white');
      const colorClass = isWhite ? 'rclock-white' : 'rclock-black';
      clockEl = document.querySelector(`.rclock-bottom.${colorClass} .time`);
    }
  }

  if (!clockEl) {
    clockEl = document.querySelector('.rclock.rclock-bottom .time');
  }

  if (!clockEl) {
    return 999;
  }

  const text = clockEl.textContent || '';
  const match = text.match(/(\d+):(\d+)(?:.(\d))?/);
  if (!match) {
    return 999;
  }

  const mins = parseInt(match[1], 10);
  const secs = parseInt(match[2], 10);
  const tenths = match[3] ? parseInt(match[3], 10) / 10 : 0;

  return mins * 60 + secs + tenths;
}

function getArrowCoords(sq, color) {
  const f = sq[0].toLowerCase(), r = sq[1];
  let x = { a: -3.5, b: -2.5, c: -1.5, d: -0.5, e: 0.5, f: 1.5, g: 2.5, h: 3.5 }[f];
  let y = { 1: 3.5, 2: 2.5, 3: 1.5, 4: 0.5, 5: -0.5, 6: -1.5, 7: -2.5, 8: -3.5 }[r];
  if (color == "black") { x = -x; y = -y; }
  return [x, y];
}

function coordsToSquare(x, y, board) {
  const rect = board.getBoundingClientRect();
  const sz = rect.width / 8;
  const isWhite = board.classList.contains('orientation-white');
  let fi = Math.floor((x - rect.left) / sz);
  let ri = Math.floor((y - rect.top) / sz);
  if (isWhite) ri = 7 - ri; else fi = 7 - fi;
  if (fi < 0 || fi > 7 || ri < 0 || ri > 7) return null;
  return 'abcdefgh'[fi] + (ri + 1);
}

function countPieces() {
  const fen = game.fen();
  if (fen === cachedFen && cachedPieceCount !== null) return cachedPieceCount;
  cachedFen = fen;
  cachedPieceCount = (fen.split(' ')[0].match(/[pnbrqkPNBRQK]/g) || []).length;
  return cachedPieceCount;
}

// Optimized Anti-Draw & Selection
function selectVariedMove(pvs) {
  if (!pvs || pvs.length === 0) return null;

  const valid = [];
  const pgn = game.pgn();

  try {
    const tempGame = new Chess();
    tempGame.load_pgn(pgn);

    for (let i = 0; i < pvs.length && i < 4; i++) {
      if (!pvs[i]?.firstMove) continue;

      const uci = pvs[i].firstMove;

      const moveResult = tempGame.move({
        from: uci.substring(0, 2),
        to: uci.substring(2, 4),
        promotion: 'q'
      });

      if (moveResult) {
        const isDraw = tempGame.in_threefold_repetition() || tempGame.in_draw();
        tempGame.undo();

        if (isDraw) {
          continue;
        }

        valid.push({ ...pvs[i], idx: i });
      }
    }
  } catch (e) {
    for (let i = 0; i < pvs.length && i < 4; i++) {
      if (pvs[i]) valid.push({ ...pvs[i], idx: i });
    }
  }

  if (valid.length === 0) {
    for (let i = 0; i < pvs.length && i < 4; i++) {
      if (pvs[i]) valid.push({ ...pvs[i], idx: i });
    }
  }

  if (valid.length === 0) return null;

  const cfg = activeVaried;
  const topEval = valid[0].evalCp || 0;

  let allowBlunder = false;
  if (gameBlunderCount < cfg.maxBlundersPerGame && topEval > -100 && Math.random() < cfg.blunderChance) {
    allowBlunder = true;
  }

  const candidates = [];

  for (const pv of valid) {
    const cpLoss = topEval - (pv.evalCp || 0);
    const isBlunder = cpLoss >= cfg.blunderThreshold;

    if (pv.evalType === 'mate' && pv.mateVal !== null && pv.mateVal < 0 && pv.mateVal >= -3) continue;

    if (cpLoss > cfg.maxCpLoss) {
      if (!allowBlunder) continue;
    }

    let weight = cfg.weights[pv.idx] || 5;
    if (cfg.maxCpLoss < 1000) weight = weight - (cpLoss * 0.1);
    weight = Math.max(weight, 3);

    candidates.push({ ...pv, weight, cpLoss, isBlunder });
  }

  if (candidates.length === 0) {
    varietyStats.pv1++;
    return { ...valid[0], move: valid[0].firstMove };
  }

  const totalWeight = candidates.reduce((s, c) => s + c.weight, 0);
  let rand = Math.random() * totalWeight;
  let selected = candidates[0];

  for (const c of candidates) {
    rand -= c.weight;
    if (rand <= 0) { selected = c; break; }
  }

  if (selected.idx === 0) varietyStats.pv1++;
  else if (selected.idx === 1) varietyStats.pv2++;
  else if (selected.idx === 2) varietyStats.pv3++;
  else varietyStats.pv4++;

  if (selected.isBlunder) {
    gameBlunderCount++;
    varietyStats.blunders++;
  }

  return { ...selected, move: selected.firstMove };
}

// --- TIMING ---
function calculateHumanDelay(uci) {
  const cfg = activeHuman;
  const clockSecs = getClockSeconds();

  if (panicModeEnabled) {
    return 0;
  }

  if (uci && uci.length >= 4) {
    const targetSquare = uci.substring(2, 4);
    const targetPiece = game.get(targetSquare);
    if (targetPiece) {
      return 0;
    }
  }

  const pc = countPieces();

  if (pc <= cfg.premovePieceThreshold) {
    const delay = cfg.premoveDelayMs + Math.random() * (cfg.premoveMaxMs - cfg.premoveDelayMs);
    return Math.max(0, Math.round(delay));
  }

  if (pc <= cfg.lowPieceThreshold) {
    const delay = cfg.lowPieceDelayMs + Math.random() * (cfg.lowPieceMaxMs - cfg.lowPieceDelayMs);
    return Math.max(0, Math.round(delay));
  }

  let delay = cfg.baseDelayMs;
  delay *= (1 + (Math.random() * 2 - 1) * cfg.randomVariance);

  const roll = Math.random();
  if (roll < cfg.quickMoveChance) {
    delay = cfg.quickMoveMs + Math.random() * 50;
  } else if (roll < cfg.quickMoveChance + cfg.tankChance) {
    delay = cfg.tankMinMs + Math.random() * (cfg.tankMaxMs - cfg.tankMinMs);
  }

  delay = Math.max(0, Math.min(delay, cfg.maxDelayMs));

  if (humanTimingStats.totalMoves > 5) {
    const avg = (humanTimingStats.totalTimeMs + humanTimingStats.engineTimeMs) / humanTimingStats.totalMoves;
    if (avg > 580) delay *= Math.max(0.5, 580 / avg);
  }

  return Math.round(delay);
}

function updateTimingStats(delayMs, engineMs = 0) {
  humanTimingStats.totalMoves++;
  humanTimingStats.totalTimeMs += delayMs;
  humanTimingStats.engineTimeMs += engineMs;
}

function resetStats() {
  humanTimingStats = { totalMoves: 0, totalTimeMs: 0, engineTimeMs: 0 };
  varietyStats = { pv1: 0, pv2: 0, pv3: 0, pv4: 0, blunders: 0 };
  gameBlunderCount = 0;
  cachedPieceCount = null;
  cachedFen = null;
  cachedPVsFen = null;
  pendingMove = false;
  isProcessing = false;
  lastMoveSent = null;
  lastMoveSentTime = 0;
}

// --- Main Stockfish (stockfish.js) ---
const SF_THREADS = 4;
const sfListeners = new Set();

stockfish.onmessage = (e) => {
  const data = String(e.data || '');
  if (data === 'readyok') {
    engineReady = true;
  }
  for (const fn of sfListeners) {
    try { fn(e); } catch(x) {}
  }
};

function configureEngine() {
  return new Promise((resolve) => {
    stockfish.postMessage('uci');
    stockfish.postMessage('setoption name Threads value 1');
    stockfish.postMessage('setoption name Contempt value 20');
    stockfish.postMessage(`setoption name MultiPV value ${SF_THREADS}`);
    stockfish.postMessage('isready');

    const checkReady = setInterval(() => {
      if (engineReady) {
        clearInterval(checkReady);
        resolve();
      }
    }, 50);

    setTimeout(() => {
      clearInterval(checkReady);
      engineReady = true;
      resolve();
    }, 3000);
  });
}

function parseInfoLine(text) {
  if (!text.startsWith('info ')) return null;
  const mpv = text.match(/multipv (\d+)/);
  const cp = text.match(/score cp (-?\d+)/);
  const mate = text.match(/score mate (-?\d+)/);
  const pv = text.match(/ pv (.+)$/);
  if (!pv) return null;

  let evalCp = null, evalType = 'cp', mateVal = null;
  if (cp) {
    evalCp = parseInt(cp[1], 10);
  } else if (mate) {
    mateVal = parseInt(mate[1], 10);
    evalCp = (mateVal > 0 ? 100000 : -100000) + mateVal;
    evalType = 'mate';
  } else {
    return null;
  }

  return {
    multipv: mpv ? parseInt(mpv[1], 10) : 1,
    evalType, evalCp, mateVal,
    pv: pv[1].trim(),
    firstMove: pv[1].trim().split(' ')[0]
  };
}

function getMultiPV(fen, retryCount = 0) {
  return new Promise((resolve) => {
    if (panicModeEnabled) {
      if (selectedEngine === 'jschess') {
        jsChessCalculateMove(fen);
      } else if (selectedEngine === 'tomitank15') {
        tomitank15CalculateMove(fen);
      } else if (selectedEngine === 'tomitank51') {
        tomitank51CalculateMove(fen);
      } else {
        panicCalculateMoveSF8(fen);
      }
      resolve([]);
      return;
    }

    if (!engineReady) {
      setTimeout(() => getMultiPV(fen, retryCount).then(resolve), 100);
      return;
    }

    const pvs = new Map();
    let resolved = false;

    const engineTime = activeEngineMs;

    const handler = (e) => {
      if (resolved) return;
      const txt = String(e.data || '');

      if (txt.startsWith('info ')) {
        const p = parseInfoLine(txt);
        if (p && p.firstMove) pvs.set(p.multipv, p);
      }

      if (txt.startsWith('bestmove')) {
        resolved = true;
        sfListeners.delete(handler);
        const arr = [...pvs.entries()].sort((a, b) => a[0] - b[0]).map(([, v]) => v);

        if (arr.length === 0 && retryCount < 3) {
          setTimeout(() => getMultiPV(fen, retryCount + 1).then(resolve), 150);
          return;
        }

        cachedPVs = arr;
        cachedPVsFen = fen;

        resolve(arr);
      }
    };

    setTimeout(() => {
      if (!resolved) {
        resolved = true;
        sfListeners.delete(handler);
        const arr = [...pvs.entries()].sort((a, b) => a[0] - b[0]).map(([, v]) => v);
        cachedPVs = arr;
        cachedPVsFen = fen;
        resolve(arr);
      }
    }, 2000);

    sfListeners.add(handler);
    stockfish.postMessage('stop');
    stockfish.postMessage('position fen ' + fen);
    stockfish.postMessage(`go movetime ${engineTime}`);
  });
}

// --- Drawing ---
const PV_COLORS = [
  { name: 'pv1', hex: '#15781B' },
  { name: 'pv2', hex: '#D35400' },
  { name: 'pv3', hex: '#2980B9' },
  { name: 'pv4', hex: '#8E44AD' }
];

function ensureMarkers() {
  const defs = $('svg.cg-shapes defs')[0];
  if (!defs) return;
  for (const { name, hex } of PV_COLORS) {
    if (!document.getElementById(`arrowhead-${name}`)) {
      defs.innerHTML += `<marker id="arrowhead-${name}" orient="auto" markerWidth="4" markerHeight="8" refX="2.05" refY="2"><path d="M0,0 V4 L3,2 Z" fill="${hex}"></path></marker>`;
    }
  }
}

function drawArrows(pvs) {
  if (!showArrows || !pvs || !pvs.length) return;
  ensureMarkers();
  const layer = $('svg.cg-shapes g')[0];
  if (!layer) return;
  layer.innerHTML = '';

  const seen = new Set();
  const col = $('.cg-wrap')[0].classList.contains('orientation-white') ? 'white' : 'black';
  const topEval = pvs[0]?.evalCp || 0;

  pvs.slice(0, 4).forEach((pv, i) => {
    const m = pv.firstMove;
    if (!m || seen.has(m)) return;
    seen.add(m);

    const pal = PV_COLORS[i];
    const [x1, y1] = getArrowCoords(m.substring(0, 2), col);
    const [x2, y2] = getArrowCoords(m.substring(2, 4), col);

    layer.innerHTML += `<line stroke="${pal.hex}" stroke-width="${0.22 - i*0.015}" stroke-linecap="round" marker-end="url(#arrowhead-${pal.name})" opacity="${1 - i*0.1}" x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}"></line>`;

    const cp = pv.evalCp || 0;
    const cpLoss = topEval - cp;
    let label = pv.evalType === 'mate'
      ? `${pv.mateVal > 0 ? '+' : ''}M${pv.mateVal}`
      : `${cp >= 0 ? '+' : ''}${(cp/100).toFixed(1)}`;
    if (i > 0 && cpLoss > 0) label += ` (-${(cpLoss/100).toFixed(1)})`;

    const w = label.length * 0.13 + 0.5;
    layer.innerHTML += `<rect x="${x2 - w/2}" y="${y2 - 0.4}" width="${w}" height="0.34" rx="0.06" fill="#FFF" opacity="0.9" stroke="${pal.hex}" stroke-width="0.02"></rect>`;
    layer.innerHTML += `<text x="${x2}" y="${y2 - 0.18}" fill="${pal.hex}" text-anchor="middle" font-size="0.24" font-weight="bold">${label}</text>`;
  });
}

// --- Execute Move ---
function executeMove(uci) {
  if (!uci) return false;
  if (gameEnded) return false;
  if (pendingMoveUci) return false;
  if (!webSocketWrapper || webSocketWrapper.readyState !== 1) {
    scheduleReconnectRetry();
    return false;
  }

  const cgWrap = $('.cg-wrap')[0];
  if (!cgWrap) return false;

  const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
  if (game.turn() !== myCol) return false;

  const now = Date.now();

  if (uci === lastMoveSent && (now - lastMoveSentTime) < 500) {
    return false;
  }

  lastMoveSent = uci;
  lastMoveSentTime = now;

  const lagClaim = panicModeEnabled ? getPanicLagCompensation() : getLagCompensation();

  webSocketWrapper.send(JSON.stringify({ t: "move", d: { u: uci, a: currentAck, b: panicModeEnabled ? 1 : 0, l: lagClaim } }));
  pendingMove = false;
  isProcessing = false;
  return true;
}

function executeMoveHumanized(uci, engineMs = 0) {
  if (!uci) { isProcessing = false; return; }

  if (panicModeEnabled) {
    executeMove(uci);
    return;
  }

  if (uci.length >= 4) {
    const targetSquare = uci.substring(2, 4);
    const targetPiece = game.get(targetSquare);
    if (targetPiece) {
      executeMove(uci);
      return;
    }
  }

  const delay = calculateHumanDelay(uci);
  updateTimingStats(delay, engineMs);

  if (delay <= 0) executeMove(uci);
  else setTimeout(() => executeMove(uci), delay);
}

// --- Main Selection ---
function selectBestMove(pvs) {
  if (!pvs || pvs.length === 0) return null;

  let result = null;
  if (variedMode) result = selectVariedMove(pvs);

  if (!result || !result.move) {
    if (pvs[0] && pvs[0].firstMove) {
      result = { ...pvs[0], move: pvs[0].firstMove, idx: 0 };
      varietyStats.pv1++;
    }
  }
  return result;
}

function actOnHint(data, engineMs = 0) {
  if (!data || !data.move) { isProcessing = false; return; }
  const uci = data.move;

  const colorIdx = data.idx || 0;
  const cgWrap = $('.cg-wrap')[0];
  if (cgWrap && showArrows) {
    const col = cgWrap.classList.contains('orientation-white') ? 'white' : 'black';
    const [x1, y1] = getArrowCoords(uci.substring(0, 2), col);
    const [x2, y2] = getArrowCoords(uci.substring(2, 4), col);
    const layer = $('svg.cg-shapes g')[0];
    if (layer) layer.innerHTML += `<line stroke="${PV_COLORS[colorIdx].hex}" stroke-width="0.3" stroke-linecap="round" marker-end="url(#arrowhead-pv1)" opacity="1" x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}"></line>`;
  }

  if (!autoHint) { isProcessing = false; return; }
  if (!webSocketWrapper || !cgWrap) { isProcessing = false; return; }

  const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
  if (game.turn() !== myCol) { isProcessing = false; return; }

  if (humanMode) executeMoveHumanized(uci, engineMs);
  else executeMove(uci);
}

// --- Process Turn ---
async function processTurn() {
  if (gameEnded) return;
  if (isProcessing) return;
  if (pendingMoveUci) return;

  const cgWrap = $('.cg-wrap')[0];
  if (!cgWrap) return;

  const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
  if (game.turn() !== myCol) return;

  if (!autoHint) return;

  isProcessing = true;
  pendingMove = true;

  const currentFen = game.fen();
  const t0 = performance.now();

  // PANIC MODE CHECK
  if (panicModeEnabled) {
    if (selectedEngine === 'jschess') {
      jsChessCalculateMove(currentFen);
    } else if (selectedEngine === 'tomitank15') {
      tomitank15CalculateMove(currentFen);
    } else if (selectedEngine === 'tomitank51') {
      tomitank51CalculateMove(currentFen);
    } else {
      panicCalculateMoveSF8(currentFen);
    }
    return;
  }

  try {
    let pvs;
    let engineTime;

    // Normal mode - use main stockfish.js
    pvs = await getMultiPV(currentFen);
    engineTime = Math.round(performance.now() - t0);

    if (!pvs || pvs.length === 0) {
      isProcessing = false;
      pendingMove = false;
      setTimeout(processTurn, 500);
      return;
    }

    if (!pieceSelectMode) drawArrows(pvs);

    if (!pieceSelectMode) {
      const chosen = selectBestMove(pvs);
      if (chosen && chosen.move) actOnHint(chosen, engineTime);
      else { isProcessing = false; pendingMove = false; }
    } else {
      isProcessing = false;
      pendingMove = false;
    }
  } catch (err) {
    isProcessing = false;
    pendingMove = false;
  }
}

// --- Sync game state ---
function syncGameState() {
  try {
    game = new Chess();
    const moves = $('kwdb, u8t');
    for (let i = 0; i < moves.length; i++) {
      const moveText = moves[i].textContent.replace('✓', '').trim();
      if (moveText) try { game.move(moveText); } catch(e) {}
    }
  } catch(e) {}
}

// --- Piece Select Mode ---
function setupPieceSelectMode() {
  const board = $('.cg-wrap')[0];
  if (!board) return;

  board.addEventListener('click', async (e) => {
    if (!pieceSelectMode) return;
    const sq = coordsToSquare(e.clientX, e.clientY, board);
    if (!sq) return;
    const myCol = board.classList.contains('orientation-white') ? 'w' : 'b';
    if (game.turn() !== myCol) return;
    const piece = game.get(sq);
    if (!piece || piece.color !== myCol) return;

    if (!cachedPVs) cachedPVs = await getMultiPV(game.fen());
    const moves = cachedPVs.filter(p => p.firstMove?.substring(0, 2) === sq);
    if (moves.length > 0) {
      const best = moves[0].firstMove;
      setTimeout(() => humanMode ? executeMoveHumanized(best) : executeMove(best), 50);
    }
  });
}

// --- Floating UI Dock ---
function createBottomDock() {
  const existing = document.getElementById('lf-bottom-dock-content');
  if (existing) return existing;

  const root = document.createElement('div');
  root.id = 'lf-bottom-dock';
  root.style.cssText = [
    'position: fixed',
    'left: 50%',
    'bottom: 8px',
    'transform: translateX(-50%)',
    'z-index: 999999',
    'pointer-events: none'
  ].join(';');

  const bar = document.createElement('div');
  bar.style.cssText = [
    'pointer-events: auto',
    'display: flex',
    'align-items: center',
    'gap: 6px',
    'padding: 6px 8px',
    'background: rgba(0,0,0,0.35)',
    'border-radius: 12px',
    'backdrop-filter: blur(4px)',
    'max-width: calc(100vw - 16px)',
    'overflow-x: auto'
  ].join(';');

  const toggle = document.createElement('button');
  toggle.classList.add('fbt');
  toggle.style.fontSize = '10px';
  toggle.style.padding = '2px 6px';
  toggle.style.minWidth = '28px';

  const content = document.createElement('div');
  content.id = 'lf-bottom-dock-content';
  content.style.cssText = 'display: flex;align-items: center;gap: 6px;';

  bar.appendChild(toggle);
  bar.appendChild(content);
  root.appendChild(bar);
  document.body.appendChild(root);

  let collapsed = localStorage.getItem('lfDockCollapsed') === '1';
  function render() {
    content.style.display = collapsed ? 'none' : 'flex';
    toggle.textContent = collapsed ? '▲' : '▼';
    toggle.title = collapsed ? 'Expand' : 'Minimize';
    localStorage.setItem('lfDockCollapsed', collapsed ? '1' : '0');
  }
  toggle.onclick = () => { collapsed = !collapsed; render(); };
  render();

  return content;
}

// Panic watchdog
function startPanicWatchdog() {
  setInterval(() => {
    if (!panicModeEnabled || gameEnded) return;

    const anyEngineCalculating = panicEngineCalculating ||
                                 tomitank15Calculating ||
                                 tomitank51Calculating;

    if (anyEngineCalculating) {
      const elapsed = Date.now() - panicLastRequestTime;
      if (elapsed > PANIC_TIMEOUT_MS * 2) {

        panicEngineCalculating = false;
        tomitank15Calculating = false;
        tomitank51Calculating = false;
        panicEngineRetryCount++;

        if (panicEngineRetryCount >= PANIC_MAX_RETRIES) {
          reinitializePanicEngine();
        }

        const cgWrap = $('.cg-wrap')[0];
        if (cgWrap && !pendingMoveUci) {
          const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
          if (game.turn() === myCol) {
            isProcessing = false;
            pendingMove = false;
            processTurn();
          }
        }
      }
    }

    if (isProcessing && !anyEngineCalculating && !pendingMoveUci) {
      isProcessing = false;
      pendingMove = false;

      const cgWrap = $('.cg-wrap')[0];
      if (cgWrap) {
        const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
        if (game.turn() === myCol) {
          processTurn();
        }
      }
    }
  }, 1000);
}

// --- Main ---
async function run() {
  initializePanicEngine();

  startPanicWatchdog();

  await configureEngine();
  setupPieceSelectMode();
  syncGameState();

  const cgWrap = $('.cg-wrap')[0];
  if (cgWrap) {
    const myCol = cgWrap.classList.contains('orientation-white') ? 'w' : 'b';
    if (game.turn() === myCol && autoHint) setTimeout(processTurn, 500);
  }

  // Move observer
  const moveObs = new MutationObserver((muts) => {
    for (const mut of muts) {
      if (mut.addedNodes.length === 0) continue;
      if (mut.addedNodes[0].tagName === "I5Z") continue;

      const lastEl = $('l4x')[0]?.lastChild;
      if (!lastEl) continue;

      try { game.move(lastEl.textContent); } catch (e) {}

      cachedPVs = null;
      cachedPVsFen = null;
      cachedPieceCount = null;
      cachedFen = null;
      isProcessing = false;
      pendingMove = false;

      lastMoveSent = null;
      lastMoveSentTime = 0;
      pendingMoveUci = null;
      lastMoveAcked = false;

      panicEngineCalculating = false;
      panicLastFenRequested = null;
      panicBestMove = null;

      // Reset JS Chess state on new move
      if (selectedEngine === 'jschess') {
        jsChessPendingMove = null;
        jsChessPendingFen = null;
        jsChessRetryCount = 0;
        jsChessWaitingForReconnect = false;
      }

      // Reset Tomitank state on new move
      if (selectedEngine === 'tomitank15') {
        tomitank15PendingMove = null;
        tomitank15PendingFen = null;
        tomitank15Calculating = false;
      }
      if (selectedEngine === 'tomitank51') {
        tomitank51PendingMove = null;
        tomitank51PendingFen = null;
        tomitank51Calculating = false;
      }

      setTimeout(processTurn, 100);
    }
  });

  waitForElement('rm6').then((el) => {
    moveObs.observe(el, { childList: true, subtree: true });
    syncGameState();
    setTimeout(processTurn, 500);
  });

  const endObs = new MutationObserver(() => {
    if ($('div.rcontrols')[0]?.textContent.includes("Rematch")) {
      resetGameState();
      if (window.lichess?.socket?.send) window.lichess.socket.send("rematch-yes");
      setTimeout(() => { try { $('a.fbt[href^="/?hook_like"]')[0]?.click(); } catch(e) {} }, 1000);
      endObs.disconnect();
    }
  });
  if ($('div.rcontrols')[0]) endObs.observe($('div.rcontrols')[0], { childList: true, subtree: true });

  setInterval(() => {
    if (!gameEnded && !isProcessing && !pendingMoveUci && autoHint && !pieceSelectMode) {
      const cg = $('.cg-wrap')[0];
      if (cg) {
        const myCol = cg.classList.contains('orientation-white') ? 'w' : 'b';
        if (game.turn() === myCol) processTurn();
      }
    }
  }, 3000);

  // --- UI BUTTONS ---
  const btnCont = createBottomDock() || $('div.ricons')[0];

  // 1. Hint
  const hintBtn = document.createElement('button');
  hintBtn.innerText = 'Hint';
  hintBtn.classList.add('fbt');
  hintBtn.onclick = () => {
    getMultiPV(game.fen()).then(pvs => { if (pvs.length) { cachedPVs = pvs; drawArrows(pvs); } });
  };
  if (btnCont) btnCont.appendChild(hintBtn);

  // 2. Auto Toggle
  const autoBtn = document.createElement('button');
  autoBtn.innerText = autoHint ? 'Auto-ON' : 'Auto-OFF';
  autoBtn.classList.add('fbt');
  autoBtn.style.backgroundColor = autoHint ? "green" : "";
  autoBtn.onclick = () => {
    autoHint = !autoHint;
    autoRun = autoHint ? "1" : "0";
    localStorage.setItem('autorun', autoRun);
    autoBtn.innerText = autoHint ? 'Auto-ON' : 'Auto-OFF';
    autoBtn.style.backgroundColor = autoHint ? "green" : "";
    if (autoHint) processTurn();
  };
  if (btnCont) btnCont.appendChild(autoBtn);

  // 3. Panic Button
  const panicBtn = document.createElement('button');
  panicBtn.innerText = panicModeEnabled ? 'PANIC: ON' : 'Panic: Off';
  panicBtn.classList.add('fbt');
  panicBtn.style.backgroundColor = panicModeEnabled ? "red" : "";
  panicBtn.style.color = panicModeEnabled ? "white" : "";
  panicBtn.onclick = () => {
    panicModeEnabled = !panicModeEnabled;
    panicBtn.innerText = panicModeEnabled ? 'PANIC: ON' : 'Panic: Off';
    panicBtn.style.backgroundColor = panicModeEnabled ? "red" : "";
    panicBtn.style.color = panicModeEnabled ? "white" : "";

    if (panicModeEnabled) {
      const cg = document.querySelector('.cg-wrap');
      if (cg) {
        const myCol = cg.classList.contains('orientation-white') ? 'w' : 'b';
        if (game.turn() === myCol) {
          isProcessing = false;
          pendingMove = false;
          processTurn();
        }
      }
    }
  };
  if (btnCont) btnCont.appendChild(panicBtn);

  // 4. Engine Selector (Fixed "Body Append" Drop-up)
  const engines = ['stockfish', 'stockfish8', 'jschess', 'tomitank15', 'tomitank51'];

  // 1. Create the Button (stays in the toolbar)
  const engBtn = document.createElement('button');
  engBtn.innerText = `Eng: ${selectedEngine}`;
  engBtn.classList.add('fbt');
  engBtn.style.cursor = 'pointer';

  // 2. Create the Menu (attached to the entire page body to avoid being cut off)
  const menuList = document.createElement('div');
  menuList.id = 'custom-engine-menu';
  Object.assign(menuList.style, {
      display: 'none',
      position: 'fixed',
      zIndex: '10000',
      backgroundColor: '#262421',
      border: '1px solid #404040',
      boxShadow: '0 4px 10px rgba(0,0,0,0.5)',
      minWidth: '120px',
      borderRadius: '4px',
      padding: '4px 0'
  });

  // 3. Generate Menu Options
  engines.forEach(engineName => {
      const option = document.createElement('div');
      option.innerText = engineName;
      Object.assign(option.style, {
          padding: '8px 12px',
          cursor: 'pointer',
          color: '#e0e0e0',
          fontSize: '13px',
          borderBottom: '1px solid #333'
      });

      // Hover effects
      option.onmouseenter = () => option.style.backgroundColor = '#3e3e3e';
      option.onmouseleave = () => option.style.backgroundColor = 'transparent';

      // Click Logic
      option.onclick = (e) => {
          e.stopPropagation();
          selectedEngine = engineName;
          localStorage.setItem('selectedEngine', selectedEngine);
          engBtn.innerText = `Eng: ${selectedEngine}`;
          menuList.style.display = 'none';
      };
      menuList.appendChild(option);
  });

  // 4. Toggle Logic (Calculates position dynamically)
  engBtn.onclick = (e) => {
      e.stopPropagation();

      if (menuList.style.display === 'block') {
          menuList.style.display = 'none';
          return;
      }

      // Get the button's exact position on screen
      const rect = engBtn.getBoundingClientRect();

      // Position the menu directly ABOVE the button
      menuList.style.display = 'block';
      menuList.style.left = `${rect.left}px`;
      menuList.style.top = `${rect.top}px`;
      menuList.style.transform = 'translateY(-100%)';
  };

  // 5. Close menu if clicking elsewhere
  document.addEventListener('click', (e) => {
      if (e.target !== engBtn && !menuList.contains(e.target)) {
          menuList.style.display = 'none';
      }
  });

  // 6. Attach elements
  document.body.appendChild(menuList);
  if (btnCont) btnCont.appendChild(engBtn);

  // 5. Config Selector
  const cfgBtn = document.createElement('button');
  cfgBtn.innerText = `Cfg: ${configMode}`;
  cfgBtn.classList.add('fbt');
  cfgBtn.onclick = () => {
    const modes = Object.keys(PRESETS);
    let idx = modes.indexOf(configMode);
    idx = (idx + 1) % modes.length;
    applyConfig(modes[idx]);
    cfgBtn.innerText = `Cfg: ${configMode}`;
  };
  if (btnCont) btnCont.appendChild(cfgBtn);

  // 6. Visuals (Arrow) Toggle
  const visBtn = document.createElement('button');
  visBtn.innerText = showArrows ? 'Arrow: ON' : 'Arrow: OFF';
  visBtn.classList.add('fbt');
  visBtn.onclick = () => {
    showArrows = !showArrows;
    localStorage.setItem('showArrows', showArrows ? "1" : "0");
    visBtn.innerText = showArrows ? 'Arrow: ON' : 'Arrow: OFF';
    if (!showArrows) {
       const layer = document.querySelector('svg.cg-shapes g');
       if (layer) layer.innerHTML = '';
    }
  };
  if (btnCont) btnCont.appendChild(visBtn);

  // 7. Piece Select Button
  const pieceBtn = document.createElement('button');
  pieceBtn.innerText = pieceSelectMode ? 'Piece: ON' : 'Piece: OFF';
  pieceBtn.classList.add('fbt');
  pieceBtn.onclick = () => {
    pieceSelectMode = !pieceSelectMode;
    localStorage.setItem('pieceSelectMode', pieceSelectMode ? "1" : "0");
    pieceBtn.innerText = pieceSelectMode ? 'Piece: ON' : 'Piece: OFF';
  };
  if (btnCont) btnCont.appendChild(pieceBtn);

  // 8. Human Mode Button
  const humanBtn = document.createElement('button');
  humanBtn.innerText = humanMode ? 'Human: ON' : 'Human: OFF';
  humanBtn.classList.add('fbt');
  humanBtn.onclick = () => {
    humanMode = !humanMode;
    localStorage.setItem('humanMode', humanMode ? "1" : "0");
    humanBtn.innerText = humanMode ? 'Human: ON' : 'Human: OFF';
  };
  if (btnCont) btnCont.appendChild(humanBtn);

  // 9. Variance Button
  const varBtn = document.createElement('button');
  varBtn.innerText = variedMode ? 'Var: ON' : 'Var: OFF';
  varBtn.classList.add('fbt');
  varBtn.onclick = () => {
    variedMode = !variedMode;
    localStorage.setItem('variedMode', variedMode ? "1" : "0");
    varBtn.innerText = variedMode ? 'Var: ON' : 'Var: OFF';
  };
  if (btnCont) btnCont.appendChild(varBtn);

  // 10. Lag Button
  const lagBtn = document.createElement('button');
  lagBtn.innerText = `Lag: ${vpnPingOffset}ms`;
  lagBtn.classList.add('fbt');
  lagBtn.onclick = () => {
    if (vpnPingOffset === 0) vpnPingOffset = 50;
    else if (vpnPingOffset === 50) vpnPingOffset = 100;
    else if (vpnPingOffset === 100) vpnPingOffset = 200;
    else vpnPingOffset = 0;
    localStorage.setItem('vpnPingOffset', vpnPingOffset);
    lagBtn.innerText = `Lag: ${vpnPingOffset}ms`;
  };
  if (btnCont) btnCont.appendChild(lagBtn);

}

// --- ENTRY POINT ---
const startCheck = setInterval(() => {
    if (document.querySelector('.cg-wrap') || document.querySelector('#main-wrap')) {
        clearInterval(startCheck);
        run();
    }
}, 200);
