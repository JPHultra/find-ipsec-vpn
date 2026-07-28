const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// DOM Elements
let viewConfig, viewProfiles, viewConnecting, viewOtp, viewConnected;
let formConnect, formEditor, formOtp;
let inputOtp;
let connectingMessage, uptimeCounter;
let statVpnIp, statGateway, statGatewayIp, statProtocol;
let trafficReceived, trafficSent;
let sessionLogPre, engineLogPre;
let btnToggleLogs, btnClearLogs, logDrawer;
let tabs, tabPanes;

// Navigation & Error elements
let appNavTabs, navConnect, navProfiles;
let configErrorBanner;

// Connect view elements
let profileSelector, summaryHost, summaryUsername;

// Profiles Editor view elements
let editorProfileSelector, btnEditorNew, btnEditorDelete;
let inputEditProfileName, inputEditHost, inputEditRemoteId, inputEditUsername, inputEditPsk, inputEditPassword;

// State helper variables
let unlistenVpnEvent = null;
let profiles = [];
let activeProfileId = 'default';
let editorProfileId = 'default';

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

// Error Banner Display Helpers
function showErrorBanner(message) {
  if (configErrorBanner) {
    configErrorBanner.innerHTML = `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z"/></svg><span>${message}</span>`;
    configErrorBanner.classList.remove('hidden');
  }
}

function hideErrorBanner() {
  if (configErrorBanner) {
    configErrorBanner.classList.add('hidden');
    configErrorBanner.innerHTML = '';
  }
}

// UI Panel transition helper
function showPanel(panel) {
  // Hide all main panels
  [viewConfig, viewProfiles, viewConnecting, viewOtp, viewConnected].forEach(p => {
    if (p) p.classList.remove('active');
  });
  panel.classList.add('active');

  // Manage header navigation visibility
  if (panel === viewConfig || panel === viewProfiles) {
    appNavTabs.style.display = 'flex';
    // Sync tab button active states
    if (panel === viewConfig) {
      navConnect.classList.add('active');
      navProfiles.classList.remove('active');
    } else {
      navProfiles.classList.add('active');
      navConnect.classList.remove('active');
    }
  } else {
    // Hide navigation bar during active connection transitions/connections
    appNavTabs.style.display = 'none';
  }
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

    const btnConnect = document.getElementById('btn-connect');

    if (profiles.length === 0) {
      // Empty state
      profileSelector.innerHTML = '<option value="">No Profiles Configured</option>';
      editorProfileSelector.innerHTML = '<option value="">No Profiles Configured</option>';
      profileSelector.disabled = true;
      editorProfileSelector.disabled = true;

      summaryHost.textContent = 'None (Create one in Profiles tab)';
      summaryUsername.textContent = 'None';

      inputEditProfileName.value = '';
      inputEditHost.value = '';
      inputEditRemoteId.value = '';
      inputEditUsername.value = '';
      inputEditPsk.value = '';
      inputEditPassword.value = '';
      inputEditPsk.placeholder = '••••••••••••••••';
      inputEditPassword.placeholder = '••••••••••••••••';

      inputEditProfileName.disabled = true;
      inputEditHost.disabled = true;
      inputEditRemoteId.disabled = true;
      inputEditUsername.disabled = true;
      inputEditPsk.disabled = true;
      inputEditPassword.disabled = true;

      btnEditorDelete.disabled = true;
      if (btnConnect) btnConnect.disabled = true;
      return;
    }

    // Non-empty state: enable all elements
    profileSelector.disabled = false;
    editorProfileSelector.disabled = false;
    inputEditProfileName.disabled = false;
    inputEditHost.disabled = false;
    inputEditRemoteId.disabled = false;
    inputEditUsername.disabled = false;
    inputEditPsk.disabled = false;
    inputEditPassword.disabled = false;
    btnEditorDelete.disabled = false;
    if (btnConnect) btnConnect.disabled = false;

    // Check if current editorProfileId exists, if not default to active
    if (!profiles.some(p => p.id === editorProfileId)) {
      editorProfileId = activeProfileId;
    }

    // Populate Connect view dropdown
    profileSelector.innerHTML = '';
    // Populate Profiles Editor view dropdown
    editorProfileSelector.innerHTML = '';

    profiles.forEach(p => {
      // Option for connect view selector
      const optConnect = document.createElement('option');
      optConnect.value = p.id;
      optConnect.textContent = p.name;
      profileSelector.appendChild(optConnect);

      // Option for editor view selector
      const optEdit = document.createElement('option');
      optEdit.value = p.id;
      optEdit.textContent = p.name;
      editorProfileSelector.appendChild(optEdit);
    });

    profileSelector.value = activeProfileId;
    editorProfileSelector.value = editorProfileId;

    // Update Connect View Summary Card
    const activeProfile = profiles.find(p => p.id === activeProfileId);
    if (activeProfile) {
      summaryHost.textContent = activeProfile.host || 'Not Configured';
      summaryUsername.textContent = activeProfile.username || 'Not Configured';
    }

    // Update Profile Editor View inputs
    const editProfile = profiles.find(p => p.id === editorProfileId);
    if (editProfile) {
      inputEditProfileName.value = editProfile.name || '';
      inputEditHost.value = editProfile.host || '';
      inputEditRemoteId.value = editProfile.remote_identity || '';
      inputEditUsername.value = editProfile.username || '';

      // Clear password values for safety when loading profile in editor
      inputEditPsk.value = '';
      inputEditPassword.value = '';

      // Check key storage secrets status for placeholders
      const status = await invoke('get_profile_secrets_status', { profileId: editorProfileId });
      if (status.has_psk) {
        inputEditPsk.placeholder = '•••••••••••••••• (Saved)';
      } else {
        inputEditPsk.placeholder = '••••••••••••••••';
      }

      if (status.has_password) {
        inputEditPassword.placeholder = '•••••••••••••••• (Saved)';
      } else {
        inputEditPassword.placeholder = '••••••••••••••••';
      }
    }
  } catch (err) {
    appendSessionLog(`Error loading profiles: ${err}`, true);
  }
}

// Save profile changes from Editor form
async function saveEditorProfile(e) {
  e.preventDefault();
  hideErrorBanner();
  const activeEditProfile = profiles.find(p => p.id === editorProfileId);
  if (!activeEditProfile) {
    appendSessionLog('No profile selected to edit.', true);
    return;
  }

  activeEditProfile.name = inputEditProfileName.value.trim();
  activeEditProfile.host = inputEditHost.value.trim();
  activeEditProfile.remote_identity = inputEditRemoteId.value.trim();
  activeEditProfile.username = inputEditUsername.value.trim();
  const psk = inputEditPsk.value;
  const password = inputEditPassword.value;

  try {
    await invoke('save_profile', {
      profile: activeEditProfile,
      psk: psk ? psk : null,
      password: password ? password : null
    });
    appendSessionLog(`Profile "${activeEditProfile.name}" saved successfully.`);
    // Make saved profile active and reload
    activeProfileId = activeEditProfile.id;
    await invoke('set_active_profile', { profileId: activeProfileId });
    await loadProfiles();
  } catch (err) {
    showErrorBanner(`Failed to save profile: ${err}`);
    appendSessionLog(`Failed to save profile: ${err}`, true);
  }
}

// Initialize and register callbacks
window.addEventListener('DOMContentLoaded', async () => {
  // Bind views
  viewConfig = document.getElementById('view-config');
  viewProfiles = document.getElementById('view-profiles');
  viewConnecting = document.getElementById('view-connecting');
  viewOtp = document.getElementById('view-otp');
  viewConnected = document.getElementById('view-connected');

  // Bind error banner
  configErrorBanner = document.getElementById('config-error-banner');

  // Bind navigation tabs
  appNavTabs = document.getElementById('app-nav-tabs');
  navConnect = document.getElementById('nav-connect');
  navProfiles = document.getElementById('nav-profiles');

  // Bind forms and inputs
  formConnect = document.getElementById('vpn-connect-form');
  formEditor = document.getElementById('profile-editor-form');
  formOtp = document.getElementById('otp-form');
  inputOtp = document.getElementById('otp-code');

  // Bind Connect View fields
  profileSelector = document.getElementById('profile-selector');
  summaryHost = document.getElementById('summary-host');
  summaryUsername = document.getElementById('summary-username');

  // Bind Editor View fields
  editorProfileSelector = document.getElementById('editor-profile-selector');
  btnEditorNew = document.getElementById('btn-editor-new');
  btnEditorDelete = document.getElementById('btn-editor-delete');
  inputEditProfileName = document.getElementById('edit-profile-name');
  inputEditHost = document.getElementById('edit-host');
  inputEditRemoteId = document.getElementById('edit-remote-identity');
  inputEditUsername = document.getElementById('edit-username');
  inputEditPsk = document.getElementById('edit-psk');
  inputEditPassword = document.getElementById('edit-password');

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

  // Bind password eye toggle buttons
  document.querySelectorAll('.btn-toggle-password').forEach(btn => {
    btn.addEventListener('click', () => {
      const targetId = btn.getAttribute('data-for');
      const input = document.getElementById(targetId);
      if (!input) return;

      const isPassword = input.type === 'password';
      input.type = isPassword ? 'text' : 'password';

      const eyeOpen = `<path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" /><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />`;
      const eyeClosed = `<path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />`;

      const svg = btn.querySelector('svg');
      if (svg) svg.innerHTML = isPassword ? eyeClosed : eyeOpen;
    });
  });

  // Load profiles list and data
  await loadProfiles();

  // Navigation Tab Handlers
  navConnect.addEventListener('click', () => {
    hideErrorBanner();
    showPanel(viewConfig);
  });

  navProfiles.addEventListener('click', () => {
    hideErrorBanner();
    editorProfileId = activeProfileId;
    showPanel(viewProfiles);
    loadProfiles();
  });

  // Connect View Profile Selector change
  profileSelector.addEventListener('change', async (e) => {
    hideErrorBanner();
    const selectedId = e.target.value;
    try {
      await invoke('set_active_profile', { profileId: selectedId });
      await loadProfiles();
      appendSessionLog(`Active profile changed.`);
    } catch (err) {
      showErrorBanner(`Failed to change active profile: ${err}`);
    }
  });

  // Editor View Profile Selector change
  editorProfileSelector.addEventListener('change', async (e) => {
    hideErrorBanner();
    editorProfileId = e.target.value;
    await loadProfiles();
  });

  // Create New Profile Button Handler (inside Editor Header)
  btnEditorNew.addEventListener('click', async () => {
    hideErrorBanner();
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
        editorProfileId = newProfile.id;
        await loadProfiles();
        appendSessionLog(`Created new profile: "${newProfile.name}".`);
      } catch (err) {
        showErrorBanner(`Failed to create profile: ${err}`);
      }
    }
  });

  // Delete Profile Button Handler (inside Editor Actions)
  btnEditorDelete.addEventListener('click', async () => {
    hideErrorBanner();
    const activeEditProfile = profiles.find(p => p.id === editorProfileId);
    if (!activeEditProfile) return;

    if (confirm(`Are you sure you want to delete the profile "${activeEditProfile.name}"?`)) {
      try {
        const config = await invoke('delete_profile', { profileId: editorProfileId });
        editorProfileId = config.active_profile_id;
        await loadProfiles();
        appendSessionLog(`Deleted profile "${activeEditProfile.name}".`);
      } catch (err) {
        showErrorBanner(`Failed to delete profile: ${err}`);
      }
    }
  });

  // Save Profile Form Submission
  formEditor.addEventListener('submit', saveEditorProfile);

  // Handle configuration connect submission
  formConnect.addEventListener('submit', async (e) => {
    e.preventDefault();
    hideErrorBanner();

    // Switch to connecting view
    showPanel(viewConnecting);
    connectingMessage.textContent = 'Launching privileged tunnel helper...';
    appendSessionLog('Initiating connection flow...');

    try {
      await invoke('connect_vpn');
    } catch (err) {
      showPanel(viewConfig);
      showErrorBanner(`Connection Failed: ${err}`);
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
          showPanel(viewConfig);
          showErrorBanner(msg.message);
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
