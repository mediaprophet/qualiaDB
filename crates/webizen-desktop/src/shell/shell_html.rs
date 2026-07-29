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

/* ── Command palette (U6-A) ─────────────────────────────────────────── */
#cmd-palette-backdrop {
  display: none; position: fixed; inset: 0; z-index: 1000;
  background: rgba(0,0,0,0.55); backdrop-filter: blur(4px);
  align-items: flex-start; justify-content: center; padding-top: 12vh;
}
#cmd-palette-backdrop.open { display: flex; }
#cmd-palette {
  width: min(560px, 92vw); background: var(--bg-light); border: 1px solid var(--border);
  border-radius: 12px; box-shadow: 0 16px 48px rgba(0,0,0,0.45); overflow: hidden;
}
#cmd-palette-input {
  width: 100%; border: none; border-bottom: 1px solid var(--border);
  background: var(--bg); color: var(--text); padding: 14px 16px; font-size: 14px;
  font-family: inherit; outline: none;
}
#cmd-palette-input::placeholder { color: var(--text-muted); }
#cmd-palette-list { max-height: 320px; overflow-y: auto; padding: 6px; }
.cmd-item {
  display: flex; align-items: center; gap: 10px; padding: 10px 12px;
  border-radius: 8px; cursor: pointer; color: var(--text); user-select: none;
}
.cmd-item:hover, .cmd-item.active { background: var(--surface); }
.cmd-item-icon { width: 22px; text-align: center; opacity: 0.85; }
.cmd-item-label { flex: 1; font-weight: 600; font-size: 13px; }
.cmd-item-hint { font-size: 11px; color: var(--text-muted); }
#cmd-palette-footer {
  padding: 8px 14px; border-top: 1px solid var(--border);
  font-size: 11px; color: var(--text-muted); display: flex; gap: 12px;
}
#cmd-palette-btn {
  width: 28px; height: 28px; border: 1px solid var(--border); background: var(--bg);
  color: var(--text-muted); cursor: pointer; border-radius: 6px;
  display: flex; align-items: center; justify-content: center; font-size: 12px;
}
#cmd-palette-btn:hover { color: var(--accent); border-color: var(--accent); }
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
    <input type="text" id="address-bar" placeholder="qualia://talk | keep | browser | …" spellcheck="false">
    <button class="nav-btn" id="cmd-palette-btn" title="Command palette (Ctrl+K)">⌘K</button>
    <button class="nav-btn" id="gpu-toggle" title="Toggle GPU Surface">⚡</button>
  </div>
  <div id="cmd-palette-backdrop" role="dialog" aria-modal="true" aria-label="Command palette">
    <div id="cmd-palette">
      <input type="text" id="cmd-palette-input" placeholder="Go to Talk, Browser, 10D, Settings…" autocomplete="off" spellcheck="false">
      <div id="cmd-palette-list" role="listbox"></div>
      <div id="cmd-palette-footer">
        <span>↑↓ navigate</span>
        <span>Enter open</span>
        <span>Esc close</span>
        <span style="margin-left:auto">Ctrl+K · Ctrl+P</span>
      </div>
    </div>
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

  // Human-first home: empty / home / legacy "dashboard" all resolve to Talk.
  function normalizeQappId(qappId) {
    if (qappId == null) return 'talk';
    const id = String(qappId).trim().toLowerCase();
    if (!id || id === 'talk' || id === 'dashboard' || id === 'home') return 'talk';
    return id;
  }

  function createTab(qappId) {
    // Default tab: Talk (human-first front door). Empty hash → studio `/` = TalkRoute.
    qappId = normalizeQappId(qappId);
    const path = studioPath(qappId);
    const url = '/studio/#/' + path;
    const title =
      qappId === 'talk'
        ? 'Talk'
        : qappId.charAt(0).toUpperCase() + qappId.slice(1);

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

  function studioPath(qappId) {
    qappId = normalizeQappId(qappId);
    if (qappId === 'talk') return '';
    if (qappId === 'keep') return 'keep';
    if (qappId === 'reach' || qappId === 'browser') return 'browser';
    return qappId;
  }

  function switchToTab(tab, qappId) {
    qappId = normalizeQappId(qappId);
    tabs.forEach(t => t.el.classList.remove('active'));
    tab.classList.add('active');
    activeTabId = qappId;

    const path = studioPath(qappId);
    const url = '/studio/#/' + path;
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
      createTab('talk');
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
    qappId = normalizeQappId(qappId);
    if (activeTabId === qappId) return;
    const tab = tabs.find(t => t.qappId === qappId);
    if (tab) {
      switchToTab(tab.el, qappId);
    } else {
      createTab(qappId);
    }
  }

  function navigateToUrl(url) {
    if (!url) {
      navigate('talk');
      return;
    }
    if (url.startsWith('qualia://')) {
      const rest = url.slice('qualia://'.length);
      const qappId = rest.split('/')[0];
      navigate(normalizeQappId(qappId));
    } else if (url === '/' || url === '/studio/' || url === '/studio/#' || url === '/studio/#/') {
      // Empty / home studio paths → Talk (not a dashboard default).
      navigate('talk');
    } else if (url.startsWith('/studio/')) {
      contentIframe.src = window.location.origin + url;
    } else if (url.startsWith('http')) {
      contentIframe.src = url;
    } else {
      // Bare token in address bar (e.g. "keep", "talk") → qualia qapp.
      navigate(normalizeQappId(url));
    }
  }

  newTabBtn.onclick = () => createTab('talk');

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
  listen('shell-new-tab', () => createTab('talk'));
  listen('shell-close-tab', () => {
    const active = tabs.find(t => t.el.classList.contains('active'));
    if (active) closeTab(active.el);
  });
  listen('shell-nav-back', () => navBack.onclick());
  listen('shell-nav-forward', () => navForward.onclick());
  listen('shell-nav-reload', () => navReload.onclick());
  listen('shell-toggle-gpu', () => gpuToggle.onclick());
  listen('shell-open-command-palette', () => openCommandPalette());

  // ── Command palette (U6-A) — ≥5 destinations, Ctrl+K / Ctrl+P ──────────
  const PALETTE_ITEMS = [
    { id: 'talk',        label: 'Talk',              icon: '💬', hint: 'Home · chat & people',   keys: 'talk chat agent home' },
    { id: 'browser',     label: 'Browser (Reach)',    icon: '🌐', hint: 'Web browser',            keys: 'browser reach web' },
    { id: '10d-browser', label: '10D / Infosphere',   icon: '◈',  hint: 'Anatomy & vision .10d',  keys: '10d ten-d infosphere anatomy vision' },
    { id: 'settings',    label: 'Settings',           icon: '⚙',  hint: 'Backend & preferences',  keys: 'settings prefs config' },
    { id: 'library',     label: 'Library',            icon: '📚', hint: 'Hypermedia shelf',       keys: 'library hypermedia models' },
    { id: 'qapps',       label: 'QApps',              icon: '⬡',  hint: 'QApp catalog',           keys: 'qapps apps catalog' },
    { id: 'keep',        label: 'Keep',               icon: '🗄',  hint: 'Vault & places hub',     keys: 'keep vault' },
    { id: 'logs',        label: 'Desktop logs',       icon: '📋', hint: 'Host log stream',        keys: 'logs log' },
  ];

  const paletteBackdrop = document.getElementById('cmd-palette-backdrop');
  const paletteInput = document.getElementById('cmd-palette-input');
  const paletteList = document.getElementById('cmd-palette-list');
  const paletteBtn = document.getElementById('cmd-palette-btn');
  let paletteOpen = false;
  let paletteActive = 0;
  let paletteFiltered = PALETTE_ITEMS.slice();

  function filterPalette(q) {
    const needle = (q || '').trim().toLowerCase();
    if (!needle) return PALETTE_ITEMS.slice();
    return PALETTE_ITEMS.filter((item) => {
      const hay = (item.label + ' ' + item.hint + ' ' + item.keys + ' ' + item.id).toLowerCase();
      return hay.includes(needle);
    });
  }

  function renderPalette() {
    paletteList.innerHTML = '';
    if (paletteFiltered.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'cmd-item';
      empty.style.color = 'var(--text-muted)';
      empty.textContent = 'No matching destination';
      paletteList.appendChild(empty);
      return;
    }
    paletteActive = Math.max(0, Math.min(paletteActive, paletteFiltered.length - 1));
    paletteFiltered.forEach((item, i) => {
      const row = document.createElement('div');
      row.className = 'cmd-item' + (i === paletteActive ? ' active' : '');
      row.setAttribute('role', 'option');
      row.dataset.id = item.id;
      row.innerHTML =
        '<span class="cmd-item-icon">' + item.icon + '</span>' +
        '<span class="cmd-item-label">' + item.label + '</span>' +
        '<span class="cmd-item-hint">' + item.hint + '</span>';
      row.onmouseenter = () => { paletteActive = i; renderPalette(); };
      row.onclick = () => runPaletteItem(item.id);
      paletteList.appendChild(row);
    });
  }

  function openCommandPalette() {
    paletteOpen = true;
    paletteBackdrop.classList.add('open');
    paletteInput.value = '';
    paletteFiltered = filterPalette('');
    paletteActive = 0;
    renderPalette();
    setTimeout(() => paletteInput.focus(), 0);
  }
  // Menu / Rust shell action (View → Command Palette… / Ctrl+K accelerator).
  window.__webizenOpenCommandPalette = openCommandPalette;

  function closeCommandPalette() {
    paletteOpen = false;
    paletteBackdrop.classList.remove('open');
    paletteInput.blur();
  }

  function runPaletteItem(id) {
    closeCommandPalette();
    navigate(id);
  }

  paletteBtn.onclick = () => openCommandPalette();
  paletteBackdrop.addEventListener('mousedown', (e) => {
    if (e.target === paletteBackdrop) closeCommandPalette();
  });
  paletteInput.addEventListener('input', () => {
    paletteFiltered = filterPalette(paletteInput.value);
    paletteActive = 0;
    renderPalette();
  });
  paletteInput.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { e.preventDefault(); closeCommandPalette(); return; }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (paletteFiltered.length) {
        paletteActive = (paletteActive + 1) % paletteFiltered.length;
        renderPalette();
      }
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (paletteFiltered.length) {
        paletteActive = (paletteActive - 1 + paletteFiltered.length) % paletteFiltered.length;
        renderPalette();
      }
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (paletteFiltered[paletteActive]) runPaletteItem(paletteFiltered[paletteActive].id);
    }
  });

  document.addEventListener('keydown', (e) => {
    const key = (e.key || '').toLowerCase();
    const mod = e.ctrlKey || e.metaKey;
    if (mod && (key === 'k' || key === 'p')) {
      e.preventDefault();
      if (paletteOpen) closeCommandPalette();
      else openCommandPalette();
      return;
    }
    if (e.key === 'Escape' && paletteOpen) {
      e.preventDefault();
      closeCommandPalette();
    }
  });

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

  createTab('talk');
})();
</script>
</body>
</html>
"#;
