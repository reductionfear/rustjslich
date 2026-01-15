// Background service worker for Lichess Chess Bridge
// Manages WebSocket connection to local Rust application

let ws = null;
let reconnectTimer = null;
let isConnected = false;
let reconnectAttempts = 0;
const WS_PORT = 9876;
const MAX_RECONNECT_DELAY = 30000; // 30 seconds max
const INITIAL_RECONNECT_DELAY = 1000; // 1 second initial

// Connect to local Rust WebSocket server
function connect() {
  if (ws && ws.readyState === WebSocket.OPEN) {
    return;
  }
  
  // Close any existing connection
  if (ws) {
    try {
      ws.close();
    } catch (e) {
      // Ignore close errors
    }
    ws = null;
  }

  console.log('[Bridge] Connecting to local server... (attempt ' + (reconnectAttempts + 1) + ')');
  
  try {
    ws = new WebSocket(`ws://127.0.0.1:${WS_PORT}`);
    
    ws.onopen = () => {
      console.log('[Bridge] ✓ Connected to Rust application');
      isConnected = true;
      reconnectAttempts = 0; // Reset counter on successful connection
      
      // Notify all tabs
      chrome.tabs.query({ url: 'https://lichess.org/*' }, (tabs) => {
        tabs.forEach(tab => {
          chrome.tabs.sendMessage(tab.id, { type: 'bridge_connected' }).catch(() => {});
        });
      });
      
      // Clear reconnect timer
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };
    
    ws.onclose = (event) => {
      const wasConnected = isConnected;
      console.log('[Bridge] Disconnected from Rust application (code: ' + event.code + ')');
      isConnected = false;
      ws = null;
      
      // Notify all tabs
      if (wasConnected) {
        chrome.tabs.query({ url: 'https://lichess.org/*' }, (tabs) => {
          tabs.forEach(tab => {
            chrome.tabs.sendMessage(tab.id, { type: 'bridge_disconnected' }).catch(() => {});
          });
        });
      }
      
      // Schedule reconnect with exponential backoff
      scheduleReconnect();
    };
    
    ws.onerror = (error) => {
      console.log('[Bridge] Connection error (server may not be running)');
      // Error will trigger onclose, which will handle reconnection
    };
  } catch (e) {
    console.error('[Bridge] Failed to create WebSocket:', e);
    scheduleReconnect();
  }
}

// Schedule reconnect with exponential backoff
function scheduleReconnect() {
  if (reconnectTimer) {
    return; // Already scheduled
  }
  
  reconnectAttempts++;
  
  // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s (max)
  const delay = Math.min(
    INITIAL_RECONNECT_DELAY * Math.pow(2, reconnectAttempts - 1),
    MAX_RECONNECT_DELAY
  );
  
  console.log('[Bridge] Will retry connection in ' + (delay / 1000) + 's...');
  
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connect();
  }, delay);
}
  
  
  ws.onmessage = (event) => {
    try {
      const message = JSON.parse(event.data);
      console.log('[Bridge] Received from Rust:', message);
      
      // Forward message to content script
      chrome.tabs.query({ url: 'https://lichess.org/*' }, (tabs) => {
        tabs.forEach(tab => {
          chrome.tabs.sendMessage(tab.id, {
            type: 'rust_command',
            command: message
          }).catch(() => {});
        });
      });
    } catch (e) {
      console.error('[Bridge] Failed to parse message:', e);
    }
  };
}

// Send message to Rust application
function sendToRust(message) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(message));
    console.log('[Bridge] Sent to Rust:', message);
  } else {
    console.warn('[Bridge] Not connected to Rust application');
  }
}

// Listen for messages from content script
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'game_state' || 
      message.type === 'rematch_available' || 
      message.type === 'rematch_accepted' ||
      message.type === 'new_game') {
    sendToRust(message);
    sendResponse({ success: true });
  } else if (message.type === 'check_connection') {
    sendResponse({ connected: isConnected });
  }
  return true;
});

// Initialize connection on startup
connect();

// Keep service worker alive
chrome.runtime.onInstalled.addListener(() => {
  console.log('[Bridge] Extension installed');
  connect();
});

// Reconnect when browser starts
chrome.runtime.onStartup.addListener(() => {
  console.log('[Bridge] Browser started');
  connect();
});
