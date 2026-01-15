// Popup script for Lichess Chess Bridge

document.addEventListener('DOMContentLoaded', () => {
  const statusDiv = document.getElementById('status');
  const statusText = document.getElementById('status-text');
  const gameInfo = document.getElementById('game-info');
  const reconnectBtn = document.getElementById('reconnect-btn');
  
  // Check connection status
  function updateStatus() {
    chrome.runtime.sendMessage({ type: 'check_connection' }, (response) => {
      const connected = response && response.connected;
      
      if (connected) {
        statusDiv.className = 'status connected';
        statusText.textContent = 'Connected to Rust app';
        reconnectBtn.disabled = true;
      } else {
        statusDiv.className = 'status disconnected';
        statusText.textContent = 'Disconnected - Start Rust app';
        reconnectBtn.disabled = false;
      }
    });
  }
  
  // Update game info
  function updateGameInfo() {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const tab = tabs[0];
      
      if (tab && tab.url && tab.url.includes('lichess.org')) {
        const match = tab.url.match(/lichess\.org\/([a-zA-Z0-9]{8,})/);
        if (match) {
          gameInfo.textContent = `Game ID: ${match[1]}`;
        } else {
          gameInfo.textContent = 'Navigate to a game page';
        }
      } else {
        gameInfo.textContent = 'Not on Lichess';
      }
    });
  }
  
  // Reconnect button
  reconnectBtn.addEventListener('click', () => {
    // Send a message to background to trigger reconnect
    chrome.runtime.sendMessage({ type: 'reconnect' });
    reconnectBtn.disabled = true;
    reconnectBtn.textContent = 'Connecting...';
    
    setTimeout(() => {
      updateStatus();
      reconnectBtn.textContent = 'Reconnect';
    }, 2000);
  });
  
  // Initial update
  updateStatus();
  updateGameInfo();
  
  // Update every 2 seconds
  setInterval(updateStatus, 2000);
  setInterval(updateGameInfo, 2000);
});
