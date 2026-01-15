// Background service worker for Lichess Chess Bridge
// Manages WebSocket connection to local Rust application

let ws = null;
let reconnectTimer = null;
let isConnected = false;
const WS_PORT = 9876;

// Connect to local Rust WebSocket server
function connect() {
  if (ws && ws.readyState === WebSocket.OPEN) {
    return;
  }

  console.log('[Bridge] Connecting to local server...');
  
  ws = new WebSocket(`ws://127.0.0.1:${WS_PORT}`);
  
  ws.onopen = () => {
    console.log('[Bridge] Connected to Rust application');
    isConnected = true;
    
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
  
  ws.onclose = () => {
    console.log('[Bridge] Disconnected from Rust application');
    isConnected = false;
    ws = null;
    
    // Notify all tabs
    chrome.tabs.query({ url: 'https://lichess.org/*' }, (tabs) => {
      tabs.forEach(tab => {
        chrome.tabs.sendMessage(tab.id, { type: 'bridge_disconnected' }).catch(() => {});
      });
    });
    
    // Try to reconnect after 2 seconds
    if (!reconnectTimer) {
      reconnectTimer = setTimeout(connect, 2000);
    }
  };
  
  ws.onerror = (error) => {
    console.error('[Bridge] WebSocket error:', error);
  };
  
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
