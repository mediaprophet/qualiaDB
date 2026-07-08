pub const SHELL_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Webizen</title>
<style>
:root {
  --bg: #1a1a2e;
  --bg-light: #16213e;
  --surface: #0f3460;
  --accent: #e07a5f;
  --text: #e4e4e4;
  --text-muted: #8b8178;
  --border: #2a2a4a;
  --tab-active: #e07a5f;
  --tab-hover: #2a2a4a;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { height: 100%; overflow: hidden; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--text); font-size: 13px; }
#shell { display: flex; flex-direction: column; height: 100vh; }

#tab-strip {
  display: flex; align-items: center; background: var(--bg-light);
  border-bottom: 1px solid var(--border); padding: 4px 8px 0 8px;
  gap: 2px; flex-shrink: 0; min-height: 36px;
}
.tab {
  display: flex; align-items: center; gap: 6px;
  padding: 6px 12px; border-radius: 6px 6px 0 0;
  cursor: pointer; color: var(--text-muted);
  border: 1px solid transparent; border-bottom: none;
  transition: all 0.15s ease; max-width: 200px;
  user-select: none;
}
.tab:hover { background: var(--tab-hover); color: var(--text); }
.tab.active { background: var(--bg); color: var(--text); border-color: var(--border); border-bottom-color: var(--bg); }
.tab-title { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex: 1; }
.tab-close { width: 16px; height: 16px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 11px; opacity: 0.5; }
.tab-close:hover { opacity: 1; background: rgba(255,255,255,0.1); }
.tab-spinner { width: 12px; height: 12px; border: 2px solid var(--text-muted); border-top-color: transparent; border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
#new-tab-btn { padding: 6px 10px; cursor: pointer; color: var(--text-muted); border-radius: 6px; }
#new-tab-btn:hover { background: var(--tab-hover); color: var(--text); }

#address-bar-container {
  display: flex; align-items: center; gap: 4px;
  padding: 6px 12px; background: var(--bg-light);
  border-bottom: 1px solid var(--border); flex-shrink: 0;
}
.nav-btn { width: 28px; height: 28px; border: none; background: transparent; color: var(--text-muted); cursor: pointer; border-radius: 6px; display: flex; align-items: center; justify-content: center; font-size: 14px; }
.nav-btn:hover { background: var(--tab-hover); color: var(--text); }
.nav-btn:disabled { opacity: 0.3; cursor: default; }
#address-bar {
  flex: 1; height: 28px; background: var(--bg); border: 1px solid var(--border);
  border-radius: 6px; color: var(--text); padding: 0 10px; font-size: 12px;
  font-family: inherit;
}
#address-bar:focus { outline: none; border-color: var(--accent); }
#gpu-toggle { width: 28px; height: 28px; border: 1px solid var(--border); background: var(--bg); color: var(--text-muted); cursor: pointer; border-radius: 6px; display: flex; align-items: center; justify-content: center; font-size: 14px; }
#gpu-toggle:hover { color: var(--accent); }
#gpu-toggle.active { color: var(--accent); border-color: var(--accent); }

#content-area { flex: 1; position: relative; overflow: hidden; background: var(--bg); }
#content-iframe { width: 100%; height: 100%; border: none; display: block; }
#gpu-surface-overlay {
  position: absolute; top: 0; right: 0; width: 50%; height: 100%;
  pointer-events: auto; display: none; z-index: 10;
}
#gpu-surface-overlay.visible { display: block; }
#gpu-surface-overlay iframe { width: 100%; height: 100%; border: none; }

#status-bar {
  display: flex; align-items: center; gap: 16px;
  padding: 4px 12px; background: var(--bg-light);
  border-top: 1px solid var(--border); flex-shrink: 0;
  font-size: 11px; color: var(--text-muted); min-height: 24px;
}
.status-item { display: flex; align-items: center; gap: 4px; }
.status-dot { width: 8px; height: 8px; border-radius: 50%; }
.status-dot.green { background: #4caf50; }
.status-dot.red { background: #f44336; }
.status-dot.yellow { background: #ff9800; }
.status-dot.gray { background: #666; }
#status-spacer { flex: 1; }
</style>
</head>
<body>
<div id="shell">
  <div id="tab-strip">
    <div id="new-tab-btn" title="New Tab (Ctrl+T)">+</div>
  </div>
  <div id="address-bar-container">
    <button class="nav-btn" id="nav-back" title="Back (Alt+Left)">←</button>
    <button class="nav-btn" id="nav-forward" title="Forward (Alt+Right)">→</button>
    <button class="nav-btn" id="nav-reload" title="Reload (Ctrl+R)">↻</button>
    <input type="text" id="address-bar" placeholder="qualia://…" spellcheck="false">
    <button class="nav-btn" id="gpu-toggle" title="Toggle GPU Surface">⚡</button>
  </div>
  <div id="content-area">
    <iframe id="content-iframe" src="about:blank"></iframe>
    <div id="gpu-surface-overlay">
      <iframe src="about:blank"></iframe>
    </div>
  </div>
  <div id="status-bar">
    <div class="status-item"><span class="status-dot gray" id="daemon-dot"></span><span id="daemon-status">Daemon: …</span></div>
    <div class="status-item"><span class="status-dot gray" id="vault-dot"></span><span id="vault-status">Vault: …</span></div>
    <div class="status-item"><span id="sync-status">Sync: idle</span></div>
    <div class="status-item"><span id="gpu-status">GPU: off</span></div>
    <div id="status-spacer"></div>
    <div class="status-item"><span id="version-status">Webizen v0.0.24</span></div>
  </div>
</div>
<script>
(function() {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  let tabs = [];
  let activeTabId = null;
  let history = [];
  let historyIndex = -1;

  const tabStrip = document.getElementById('tab-strip');
  const newTabBtn = document.getElementById('new-tab-btn');
  const contentIframe = document.getElementById('content-iframe');
  const addressBar = document.getElementById('address-bar');
  const navBack = document.getElementById('nav-back');
  const navForward = document.getElementById('nav-forward');
  const navReload = document.getElementById('nav-reload');
  const gpuToggle = document.getElementById('gpu-toggle');
  const gpuOverlay = document.getElementById('gpu-surface-overlay');
  const daemonDot = document.getElementById('daemon-dot');
  const daemonStatus = document.getElementById('daemon-status');
  const vaultDot = document.getElementById('vault-dot');
  const vaultStatus = document.getElementById('vault-status');
  const gpuStatus = document.getElementById('gpu-status');

  function createTab(qappId) {
    qappId = qappId || 'dashboard';
    const url = '/studio/#/' + (qappId === 'dashboard' ? '' : qappId);
    const title = qappId.charAt(0).toUpperCase() + qappId.slice(1);

    const tab = document.createElement('div');
    tab.className = 'tab';
    tab.dataset.qapp = qappId;

    const spinner = document.createElement('div');
    spinner.className = 'tab-spinner';
    tab.appendChild(spinner);

    const tabTitle = document.createElement('span');
    tabTitle.className = 'tab-title';
    tabTitle.textContent = title;
    tab.appendChild(tabTitle);

    const closeBtn = document.createElement('div');
    closeBtn.className = 'tab-close';
    closeBtn.textContent = '×';
    closeBtn.onclick = (e) => { e.stopPropagation(); closeTab(tab); };
    tab.appendChild(closeBtn);

    tab.onclick = () => switchToTab(tab, qappId);

    tabStrip.insertBefore(tab, newTabBtn);
    tabs.push({ el: tab, qappId, url, title });
    switchToTab(tab, qappId);
  }

  function switchToTab(tab, qappId) {
    tabs.forEach(t => t.el.classList.remove('active'));
    tab.classList.add('active');
    activeTabId = qappId;

    const url = '/studio/#/' + (qappId === 'dashboard' ? '' : qappId);
    const fullUrl = window.location.origin + url;
    contentIframe.src = fullUrl;
    addressBar.value = 'qualia://' + qappId;

    navigateHistory(fullUrl);
    updateNavButtons();
  }

  function closeTab(tab) {
    const idx = tabs.findIndex(t => t.el === tab);
    if (idx === -1) return;
    tab.remove();
    tabs.splice(idx, 1);
    if (tabs.length === 0) {
      createTab('dashboard');
    } else if (tab.classList.contains('active')) {
      const next = tabs[Math.min(idx, tabs.length - 1)];
      switchToTab(next.el, next.qappId);
    }
  }

  function navigateHistory(url) {
    history = history.slice(0, historyIndex + 1);
    history.push(url);
    historyIndex = history.length - 1;
    updateNavButtons();
  }

  function updateNavButtons() {
    navBack.disabled = historyIndex <= 0;
    navForward.disabled = historyIndex >= history.length - 1;
  }

  function navigate(qappId) {
    if (activeTabId === qappId) return;
    const tab = tabs.find(t => t.qappId === qappId);
    if (tab) {
      switchToTab(tab.el, qappId);
    } else {
      createTab(qappId);
    }
  }

  function navigateToUrl(url) {
    if (url.startsWith('qualia://')) {
      const qappId = url.slice('qualia://'.length).split('/')[0];
      navigate(qappId);
    } else if (url.startsWith('/studio/')) {
      contentIframe.src = window.location.origin + url;
    } else if (url.startsWith('http')) {
      contentIframe.src = url;
    }
  }

  newTabBtn.onclick = () => createTab('dashboard');

  navBack.onclick = () => {
    if (historyIndex > 0) { historyIndex--; contentIframe.src = history[historyIndex]; updateNavButtons(); }
  };
  navForward.onclick = () => {
    if (historyIndex < history.length - 1) { historyIndex++; contentIframe.src = history[historyIndex]; updateNavButtons(); }
  };
  navReload.onclick = () => { contentIframe.src = contentIframe.src; };

  addressBar.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { navigateToUrl(addressBar.value.trim()); }
  });

  let gpuActive = false;
  gpuToggle.onclick = async () => {
    gpuActive = !gpuActive;
    gpuToggle.classList.toggle('active', gpuActive);
    gpuOverlay.classList.toggle('visible', gpuActive);
    gpuStatus.textContent = 'GPU: ' + (gpuActive ? 'on' : 'off');
    try {
      if (gpuActive) {
        await invoke('mount_gpu_surface', { x: 50, y: 50, width: 800, height: 600 });
      } else {
        await invoke('unmount_gpu_surface', {});
      }
    } catch(e) { console.error('GPU toggle:', e); }
  };

  listen('shell-navigate', (event) => { navigate(event.payload); });
  listen('shell-new-tab', () => createTab('dashboard'));
  listen('shell-close-tab', () => {
    const active = tabs.find(t => t.el.classList.contains('active'));
    if (active) closeTab(active.el);
  });
  listen('shell-nav-back', () => navBack.onclick());
  listen('shell-nav-forward', () => navForward.onclick());
  listen('shell-nav-reload', () => navReload.onclick());
  listen('shell-toggle-gpu', () => gpuToggle.onclick());

  async function updateStatus() {
    try {
      const status = await invoke('get_hardware_status');
      daemonDot.className = 'status-dot green';
      daemonStatus.textContent = 'Daemon: running';
    } catch(e) {
      daemonDot.className = 'status-dot red';
      daemonStatus.textContent = 'Daemon: off';
    }
    try {
      const snap = await invoke('wellfair_host_snapshot');
      const parsed = JSON.parse(snap);
      if (parsed.vault === 'unlocked') {
        vaultDot.className = 'status-dot green';
        vaultStatus.textContent = 'Vault: unlocked';
      } else if (parsed.vault === 'locked') {
        vaultDot.className = 'status-dot yellow';
        vaultStatus.textContent = 'Vault: locked';
      } else {
        vaultDot.className = 'status-dot gray';
        vaultStatus.textContent = 'Vault: none';
      }
    } catch(e) {
      vaultDot.className = 'status-dot gray';
      vaultStatus.textContent = 'Vault: …';
    }
  }

  setInterval(updateStatus, 5000);
  updateStatus();

  createTab('dashboard');
})();
</script>
</body>
</html>
"#;
