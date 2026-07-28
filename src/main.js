const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// DOM Elements
let viewConfig, viewConnecting, viewOtp, viewConnected;
let formConfig, formOtp;
let inputHost, inputRemoteId, inputUsername, inputPsk, inputPassword, inputOtp;
let connectingMessage, uptimeCounter;
let statVpnIp, statGateway, statGatewayIp, statProtocol;
let trafficReceived, trafficSent;
let sessionLogPre, engineLogPre;
let btnToggleLogs, btnClearLogs, logDrawer;
let tabs, tabPanes;

// Profiles DOM Elements
let profileSelector, btnNewProfile, btnDeleteProfile;

// State helper variables
let unlistenVpnEvent = null;
let profiles = [];
let activeProfileId = 'default';

// Helpers to format metrics
function formatBytes(bytes) {
  if (bytes === 0 || isNaN(bytes)) return '0.00 MB';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatSeconds(secs) {
  if (isNaN(secs)) return '00:00:00';
  const hours = Math.floor(secs / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  const seconds = secs % 60;
  return [
    hours.toString().padStart(2, '0'),
    minutes.toString().padStart(2, '0'),
    seconds.toString().padStart(2, '0')
  ].join(':');
}

// UI Panel transition helper
function showPanel(panel) {
  [viewConfig, viewConnecting, viewOtp, viewConnected].forEach(p => {
    if (p) p.classList.remove('active');
  });
  panel.classList.add('active');
}

// Append messages to logger drawer
function appendSessionLog(message, isError = false) {
  const timestamp = new Date().toLocaleTimeString();
  const color = isError ? 'color: #f43f5e; font-weight: 600;' : '';
  sessionLogPre.innerHTML += `<span style="color: #6b7280;">[${timestamp}]</span> <span style="${color}">${message}</span><br>`;
  sessionLogPre.scrollTop = sessionLogPre.scrollHeight;
}

// Append logs to engine tab
function appendEngineLog(message) {
  const timestamp = new Date().toLocaleTimeString();
  engineLogPre.innerHTML += `<span style="color: #4b5563;">[${timestamp}]</span> ${message}<br>`;
  engineLogPre.scrollTop = engineLogPre.scrollHeight;
}

// Load and populate profiles list
async function loadProfiles() {
  try {
    const config = await invoke('get_profiles');
    profiles = config.profiles;
    activeProfileId = config.active_profile_id;

    // Repopulate select dropdown
    profileSelector.innerHTML = '';
    profiles.forEach(p => {
      const opt = document.createElement('option');
      opt.value = p.id;
      opt.textContent = p.name;
      profileSelector.appendChild(opt);
    });
    profileSelector.value = activeProfileId;

    // Load active profile data
    const activeProfile = profiles.find(p => p.id === activeProfileId);
    if (activeProfile) {
      inputHost.value = activeProfile.host || '';
      inputRemoteId.value = activeProfile.remote_identity || '';
      inputUsername.value = activeProfile.username || '';

      // Clear password values for safety when switching profiles
      inputPsk.value = '';
      inputPassword.value = '';

      // Check key storage secrets status for placeholders
      const status = await invoke('get_profile_secrets_status', { profileId: activeProfileId });
      if (status.has_psk) {
        inputPsk.placeholder = '•••••••••••••••• (Saved)';
        inputPsk.removeAttribute('required');
      } else {
        inputPsk.placeholder = '••••••••••••••••';
        inputPsk.setAttribute('required', 'true');
      }

      if (status.has_password) {
        inputPassword.placeholder = '•••••••••••••••• (Saved)';
        inputPassword.removeAttribute('required');
      } else {
        inputPassword.placeholder = '••••••••••••••••';
        inputPassword.setAttribute('required', 'true');
      }
    }
  } catch (err) {
    appendSessionLog(`Error loading profiles: ${err}`, true);
  }
}

// Save active profile parameters to disk and keyring
async function saveConfigData() {
  const activeProfile = profiles.find(p => p.id === activeProfileId);
  if (!activeProfile) {
    appendSessionLog('No active profile to save configuration settings.', true);
    return;
  }

  activeProfile.host = inputHost.value.trim();
  activeProfile.remote_identity = inputRemoteId.value.trim();
  activeProfile.username = inputUsername.value.trim();
  const psk = inputPsk.value;
  const password = inputPassword.value;

  try {
    await invoke('save_profile', {
      profile: activeProfile,
      psk: psk ? psk : null,
      password: password ? password : null
    });
    appendSessionLog(`Profile "${activeProfile.name}" updated successfully.`);
    await loadProfiles();
  } catch (err) {
    appendSessionLog(`Failed to save configuration: ${err}`, true);
  }
}

// Initialize and register callbacks
window.addEventListener('DOMContentLoaded', async () => {
  // Bind views
  viewConfig = document.getElementById('view-config');
  viewConnecting = document.getElementById('view-connecting');
  viewOtp = document.getElementById('view-otp');
  viewConnected = document.getElementById('view-connected');

  // Bind forms and inputs
  formConfig = document.getElementById('vpn-config-form');
  formOtp = document.getElementById('otp-form');
  inputHost = document.getElementById('host');
  inputRemoteId = document.getElementById('remote-identity');
  inputUsername = document.getElementById('username');
  inputPsk = document.getElementById('psk');
  inputPassword = document.getElementById('password');
  inputOtp = document.getElementById('otp-code');

  // Bind profiles inputs
  profileSelector = document.getElementById('profile-selector');
  btnNewProfile = document.getElementById('btn-new-profile');
  btnDeleteProfile = document.getElementById('btn-delete-profile');

  // Bind status/metric elements
  connectingMessage = document.getElementById('connecting-message');
  uptimeCounter = document.getElementById('uptime-counter');
  statVpnIp = document.getElementById('stat-vpn-ip');
  statGateway = document.getElementById('stat-gateway');
  statGatewayIp = document.getElementById('stat-gateway-ip');
  statProtocol = document.getElementById('stat-protocol');
  trafficReceived = document.getElementById('traffic-received');
  trafficSent = document.getElementById('traffic-sent');

  // Bind log elements
  sessionLogPre = document.getElementById('session-log-pre');
  engineLogPre = document.getElementById('engine-log-pre');
  btnToggleLogs = document.getElementById('btn-toggle-logs');
  btnClearLogs = document.getElementById('btn-clear-logs');
  logDrawer = document.getElementById('log-drawer');

  // Load profiles list and data
  await loadProfiles();

  // Profile Selector Change Handler
  profileSelector.addEventListener('change', async (e) => {
    const selectedId = e.target.value;
    try {
      await invoke('set_active_profile', { profileId: selectedId });
      await loadProfiles();
      appendSessionLog(`Switched active profile.`);
    } catch (err) {
      appendSessionLog(`Failed to change profile: ${err}`, true);
    }
  });

  // Create New Profile Button Handler
  btnNewProfile.addEventListener('click', async () => {
    const name = prompt("Enter a name for the new profile:");
    if (name && name.trim()) {
      const newProfile = {
        id: `profile_${Date.now()}`,
        name: name.trim(),
        host: '',
        remote_identity: '',
        username: ''
      };
      try {
        await invoke('save_profile', { profile: newProfile, psk: '', password: '' });
        await loadProfiles();
        appendSessionLog(`Created new profile: "${newProfile.name}".`);
      } catch (err) {
        appendSessionLog(`Failed to create profile: ${err}`, true);
      }
    }
  });

  // Delete Selected Profile Button Handler
  btnDeleteProfile.addEventListener('click', async () => {
    const activeProfile = profiles.find(p => p.id === activeProfileId);
    if (!activeProfile) return;

    if (profiles.length <= 1) {
      alert("Cannot delete the last remaining profile.");
      return;
    }

    if (confirm(`Are you sure you want to delete the profile "${activeProfile.name}"?`)) {
      try {
        await invoke('delete_profile', { profileId: activeProfileId });
        await loadProfiles();
        appendSessionLog(`Deleted profile "${activeProfile.name}".`);
      } catch (err) {
        appendSessionLog(`Failed to delete profile: ${err}`, true);
      }
    }
  });

  // Handle configuration connect submission
  formConfig.addEventListener('submit', async (e) => {
    e.preventDefault();
    await saveConfigData();
    
    // Switch to connecting view
    showPanel(viewConnecting);
    connectingMessage.textContent = 'Launching privileged tunnel helper...';
    appendSessionLog('Initiating connection flow...');

    try {
      await invoke('connect_vpn');
    } catch (err) {
      showPanel(viewConfig);
      appendSessionLog(`Elevated connection initiation failed: ${err}`, true);
    }
  });

  // Handle OTP verification submission
  formOtp.addEventListener('submit', async (e) => {
    e.preventDefault();
    const code = inputOtp.value.trim();
    if (!code) return;

    appendSessionLog('Submitting verification code...');
    inputOtp.value = '';
    showPanel(viewConnecting);
    connectingMessage.textContent = 'Submitting email security code...';

    try {
      await invoke('submit_otp', { otp: code });
    } catch (err) {
      appendSessionLog(`Failed to submit verification code: ${err}`, true);
      showPanel(viewOtp);
    }
  });

  // Handle Cancels and Disconnects
  const disconnectHandler = async () => {
    appendSessionLog('Disconnecting tunnel...');
    try {
      await invoke('disconnect_vpn');
      showPanel(viewConfig);
    } catch (err) {
      appendSessionLog(`Disconnect operation failed: ${err}`, true);
    }
  };

  document.getElementById('btn-cancel-connecting').addEventListener('click', disconnectHandler);
  document.getElementById('btn-cancel-otp').addEventListener('click', disconnectHandler);
  document.getElementById('btn-disconnect').addEventListener('click', disconnectHandler);

  // Toggle Log Drawer
  btnToggleLogs.addEventListener('click', () => {
    logDrawer.classList.toggle('expanded');
  });

  // Clear Logs
  btnClearLogs.addEventListener('click', () => {
    sessionLogPre.innerHTML = '';
    engineLogPre.innerHTML = '';
  });

  // Tab Panel selections
  tabs = document.querySelectorAll('.tab-btn');
  tabPanes = document.querySelectorAll('.tab-pane');

  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tabPanes.forEach(pane => pane.classList.remove('active'));

      tab.classList.add('active');
      const activeTabId = tab.getAttribute('data-tab');
      document.getElementById(activeTabId).classList.add('active');
    });
  });

  // Setup Tauri global listener to receive helper events
  unlistenVpnEvent = await listen('vpn-event', (event) => {
    const rawData = event.payload;
    
    try {
      const msg = JSON.parse(rawData);
      
      switch (msg.type) {
        case 'Status':
          appendSessionLog(msg.message);
          
          if (msg.state === 'Resolving' || msg.state === 'Connecting' || msg.state === 'EstablishingTunnel') {
            showPanel(viewConnecting);
            connectingMessage.textContent = msg.message;
          } else if (msg.state === 'WaitingForOtp') {
            showPanel(viewOtp);
            inputOtp.focus();
          } else if (msg.state === 'Connected') {
            showPanel(viewConnected);
          } else if (msg.state === 'Disconnected') {
            showPanel(viewConfig);
          }
          break;

        case 'TunnelInfo':
          statVpnIp.textContent = msg.vpn_ip;
          statGateway.textContent = msg.gateway_ip;
          statGatewayIp.textContent = msg.gateway_ip;
          statProtocol.textContent = msg.protocol;
          appendSessionLog(`Tunnel established. VPN IP: ${msg.vpn_ip}`);
          showPanel(viewConnected);
          break;

        case 'Stats':
          trafficSent.textContent = formatBytes(msg.bytes_sent);
          trafficReceived.textContent = formatBytes(msg.bytes_received);
          uptimeCounter.textContent = formatSeconds(msg.uptime_secs);
          break;

        case 'Log':
          appendEngineLog(msg.message);
          break;

        case 'Error':
          appendSessionLog(`Helper Error: ${msg.message}`, true);
          break;
      }
    } catch (err) {
      // Fallback: treat as engine raw log line
      appendEngineLog(rawData);
    }
  });
});

// Clean up listener on window close
window.addEventListener('beforeunload', () => {
  if (unlistenVpnEvent) unlistenVpnEvent();
});
