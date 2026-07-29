//! Installable **remote Surface Controller** PWA (phone as control surface).
//!
//! Pure generation: HTML + service worker + manifest. No native mobile app.
//! The shell talks to the desktop host via same-origin `/api/view/*` when served
//! from the Qualia settings control plane (localhost dogfood; HTTPS LAN later).
//!
//! Honest: this is a **thin installable controller shell** (not a full wasm app binary).
//! A future pass can swap the logic for a true wasm module; packaging stays PWA.

use super::pwa::{PwaBundle, PwaContent, PwaFile};

const CACHE: &str = "webizen-remote-controller-v1";

/// Generate installable remote-controller PWA files (index, manifest, sw, app.js).
pub fn generate_remote_controller_pwa() -> PwaBundle {
    PwaBundle {
        files: vec![
            PwaFile {
                path: "index.html".into(),
                content: PwaContent::Text(index_html()),
            },
            PwaFile {
                path: "manifest.webmanifest".into(),
                content: PwaContent::Text(webmanifest()),
            },
            PwaFile {
                path: "sw.js".into(),
                content: PwaContent::Text(service_worker()),
            },
            PwaFile {
                path: "app.js".into(),
                content: PwaContent::Text(app_js()),
            },
            PwaFile {
                path: "icon.svg".into(),
                content: PwaContent::Text(icon_svg()),
            },
        ],
    }
}

fn webmanifest() -> String {
    r###"{
  "id": "webizen.remote-controller",
  "name": "Webizen Remote",
  "short_name": "Webizen",
  "description": "Phone Surface Controller for Qualia mindware — local apparatus, not cloud.",
  "start_url": "./",
  "scope": "./",
  "display": "standalone",
  "orientation": "portrait-primary",
  "theme_color": "#0b1220",
  "background_color": "#0b1220",
  "icons": [
    {
      "src": "icon.svg",
      "sizes": "any",
      "type": "image/svg+xml",
      "purpose": "any maskable"
    }
  ]
}"###
        .into()
}

fn service_worker() -> String {
    format!(
        r###"const CACHE = '{CACHE}';
const PRECACHE = ['./', './index.html', './app.js', './manifest.webmanifest', './icon.svg'];
self.addEventListener('install', (e) => {{
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(PRECACHE)).then(() => self.skipWaiting()));
}});
self.addEventListener('activate', (e) => {{
  e.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)))
    ).then(() => self.clients.claim())
  );
}});
self.addEventListener('fetch', (e) => {{
  if (e.request.method !== 'GET') return;
  const url = new URL(e.request.url);
  if (url.pathname.includes('/api/')) return;
  e.respondWith(
    caches.match(e.request).then((cached) => {{
      if (cached) return cached;
      return fetch(e.request).then((res) => {{
        if (res && res.ok && res.type === 'basic') {{
          const copy = res.clone();
          caches.open(CACHE).then((c) => c.put(e.request, copy));
        }}
        return res;
      }});
    }})
  );
}});
"###
    )
}

fn icon_svg() -> String {
    r###"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <rect width="128" height="128" rx="28" fill="#0b1220"/>
  <circle cx="64" cy="64" r="36" fill="none" stroke="#8b5cf6" stroke-width="6"/>
  <circle cx="64" cy="64" r="12" fill="#a78bfa"/>
</svg>"###
        .into()
}

fn index_html() -> String {
    r###"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <meta name="theme-color" content="#0b1220" />
  <meta name="apple-mobile-web-app-capable" content="yes" />
  <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
  <link rel="manifest" href="manifest.webmanifest" />
  <link rel="icon" href="icon.svg" type="image/svg+xml" />
  <title>Webizen Remote</title>
  <style>
    :root {
      --bg: #0b1220; --panel: #111827; --border: #334155; --text: #e2e8f0;
      --muted: #94a3b8; --accent: #8b5cf6; --ok: #34d399; --warn: #fbbf24;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0; min-height: 100dvh; font: 15px/1.45 system-ui, sans-serif;
      background: var(--bg); color: var(--text);
      padding: max(0.75rem, env(safe-area-inset-top)) 0.85rem max(1rem, env(safe-area-inset-bottom));
    }
    h1 { font-size: 1.15rem; margin: 0 0 0.25rem; color: #e9d5ff; }
    .sub { color: var(--muted); font-size: 0.78rem; margin: 0 0 1rem; line-height: 1.4; }
    .chip {
      display: inline-flex; align-items: center; gap: 0.35rem;
      font-size: 0.65rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em;
      padding: 0.2rem 0.5rem; border-radius: 999px; border: 1px solid var(--border); color: var(--muted);
    }
    .chip.ok { color: var(--ok); border-color: #065f46; }
    .chip.warn { color: var(--warn); border-color: #92400e; }
    .card {
      background: var(--panel); border: 1px solid var(--border); border-radius: 14px;
      padding: 0.85rem; margin-bottom: 0.75rem;
    }
    label { display: block; font-size: 0.68rem; color: var(--muted); font-weight: 600; margin: 0 0 0.25rem; }
    select, button, input {
      width: 100%; font: inherit; border-radius: 10px; border: 1px solid var(--border);
      background: #0b1220; color: var(--text); padding: 0.55rem 0.65rem; margin-bottom: 0.5rem;
    }
    button {
      background: var(--accent); border-color: #6d28d9; font-weight: 700; cursor: pointer;
    }
    button.secondary { background: #1e293b; border-color: var(--border); font-weight: 600; }
    button.row { width: auto; flex: 1; }
    .row { display: flex; gap: 0.4rem; flex-wrap: wrap; }
    .mono { font-family: ui-monospace, monospace; font-size: 0.72rem; color: #c4b5fd; word-break: break-all; }
    .status { font-size: 0.75rem; color: var(--muted); min-height: 1.2em; }
    .err { color: #fca5a5; }
  </style>
</head>
<body>
  <h1>Webizen Remote</h1>
  <p class="sub">Surface Controller for your local Qualia apparatus. Install to home screen when served from a secure origin (localhost or HTTPS). Not a cloud product.</p>
  <div style="display:flex;gap:0.35rem;flex-wrap:wrap;margin-bottom:0.75rem;">
    <span class="chip" id="host-chip">host: …</span>
    <span class="chip" id="honesty-chip">Partial · controller shell</span>
  </div>

  <div class="card">
    <label>Observer</label>
    <select id="observer">
      <option value="principal">Principal</option>
      <option value="peer">Peer</option>
      <option value="public">Public</option>
      <option value="instrument">Instrument</option>
      <option value="guardian">Guardian</option>
      <option value="steward">Steward</option>
      <option value="auditor">Auditor</option>
    </select>
    <label>Morph</label>
    <div class="row">
      <button type="button" class="secondary row" id="btn-flat">Flatten</button>
      <button type="button" class="secondary row" id="btn-spatial">Spatialize</button>
      <button type="button" class="secondary row" id="btn-both">Both</button>
    </div>
    <button type="button" id="btn-sync">Sync session</button>
  </div>

  <div class="card">
    <label>Shared selection</label>
    <div class="mono" id="selection">—</div>
    <label style="margin-top:0.5rem;">Attention URL</label>
    <div class="mono" id="attention">—</div>
  </div>

  <div class="card">
    <label>Select by URI (Memory / page)</label>
    <input id="uri" type="url" placeholder="https://… or urn:…" inputmode="url" />
    <button type="button" id="btn-select">Select entity</button>
  </div>

  <div class="card">
    <label>Sensors (web APIs · opt-in)</label>
    <div class="row">
      <button type="button" class="secondary row" id="btn-orient">Read orientation</button>
      <button type="button" class="secondary row" id="btn-geo">Read place</button>
    </div>
    <div class="mono" id="sensor-out">Off until you tap. Sanctuary: leave off.</div>
  </div>

  <p class="status" id="status"></p>
  <script src="app.js"></script>
</body>
</html>"###
        .into()
}

fn app_js() -> String {
    r###"
(function () {
  'use strict';
  if ('serviceWorker' in navigator) {
    window.addEventListener('load', function () {
      navigator.serviceWorker.register('./sw.js').catch(function () {});
    });
  }

  var statusEl = document.getElementById('status');
  var hostChip = document.getElementById('host-chip');
  var honesty = document.getElementById('honesty-chip');

  function setStatus(msg, isErr) {
    statusEl.textContent = msg || '';
    statusEl.className = 'status' + (isErr ? ' err' : '');
  }

  function apiBase() {
    // Same origin when served under /remote-controller/
    return '';
  }

  async function api(path, opts) {
    var res = await fetch(apiBase() + path, opts || {});
    if (!res.ok) {
      var t = await res.text();
      throw new Error(t || res.statusText);
    }
    return res.json();
  }

  function renderSession(s) {
    var sel = (s.selection && s.selection[0]) || null;
    var id = typeof sel === 'number' ? sel : (sel && (sel['0'] || sel));
    document.getElementById('selection').textContent = id ? String(id) : '—';
    document.getElementById('attention').textContent = s.attention_url || '—';
    if (s.observer) document.getElementById('observer').value = s.observer;
    hostChip.textContent = 'host: paired';
    hostChip.className = 'chip ok';
    honesty.textContent = 'Partial · installable controller';
    honesty.className = 'chip warn';
  }

  async function sync() {
    setStatus('Syncing…');
    try {
      var s = await api('/api/view/session');
      renderSession(s);
      setStatus('Session synced');
    } catch (e) {
      hostChip.textContent = 'host: offline';
      hostChip.className = 'chip warn';
      setStatus('Host unreachable — open this PWA from the desktop control plane URL. ' + e.message, true);
    }
  }

  document.getElementById('btn-sync').onclick = sync;
  document.getElementById('observer').onchange = async function () {
    var v = document.getElementById('observer').value;
    try {
      var s = await api('/api/view/set_observer', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ status: v }),
      });
      renderSession(s);
      setStatus('Observer → ' + v);
    } catch (e) {
      setStatus(String(e.message || e), true);
    }
  };

  function morph(mode) {
    return async function () {
      try {
        await api('/api/view/morph', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ mode: mode }),
        });
        setStatus('Morph → ' + mode + ' (desktop habitat)');
        await sync();
      } catch (e) {
        setStatus(String(e.message || e), true);
      }
    };
  }
  document.getElementById('btn-flat').onclick = morph('flatten');
  document.getElementById('btn-spatial').onclick = morph('spatialize');
  document.getElementById('btn-both').onclick = morph('both');

  document.getElementById('btn-select').onclick = async function () {
    var uri = (document.getElementById('uri').value || '').trim();
    if (!uri) {
      setStatus('Enter a URI', true);
      return;
    }
    try {
      var s = await api('/api/view/select_uri', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ uri: uri }),
      });
      renderSession(s);
      setStatus('Selected ' + uri);
    } catch (e) {
      setStatus(String(e.message || e), true);
    }
  };

  document.getElementById('btn-orient').onclick = async function () {
    var out = document.getElementById('sensor-out');
    if (!window.DeviceOrientationEvent) {
      out.textContent = 'Orientation API unavailable (honesty: Partial)';
      return;
    }
    function once(e) {
      out.textContent =
        'alpha=' + (e.alpha != null ? e.alpha.toFixed(1) : '—') +
        ' beta=' + (e.beta != null ? e.beta.toFixed(1) : '—') +
        ' gamma=' + (e.gamma != null ? e.gamma.toFixed(1) : '—');
      window.removeEventListener('deviceorientation', once);
    }
    if (typeof DeviceOrientationEvent.requestPermission === 'function') {
      try {
        var p = await DeviceOrientationEvent.requestPermission();
        if (p !== 'granted') {
          out.textContent = 'Permission denied';
          return;
        }
      } catch (err) {
        out.textContent = 'Permission error';
        return;
      }
    }
    window.addEventListener('deviceorientation', once);
    out.textContent = 'Listening for one sample…';
  };

  document.getElementById('btn-geo').onclick = function () {
    var out = document.getElementById('sensor-out');
    if (!navigator.geolocation) {
      out.textContent = 'Geolocation unavailable (honesty: Partial)';
      return;
    }
    navigator.geolocation.getCurrentPosition(
      function (pos) {
        out.textContent =
          pos.coords.latitude.toFixed(5) + ', ' + pos.coords.longitude.toFixed(5);
      },
      function () {
        out.textContent = 'Place denied or unavailable';
      },
      { enableHighAccuracy: false, maximumAge: 60000, timeout: 10000 }
    );
  };

  sync();
})();
"###
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_controller_bundle_has_install_shell() {
        let b = generate_remote_controller_pwa();
        assert!(b.get("index.html").is_some());
        assert!(b.get("manifest.webmanifest").is_some());
        assert!(b.get("sw.js").is_some());
        assert!(b.get("app.js").is_some());
        let html = b.text_of("index.html").unwrap();
        assert!(html.contains("Webizen Remote"));
        assert!(html.contains("manifest.webmanifest"));
        let mf = b.text_of("manifest.webmanifest").unwrap();
        assert!(mf.contains("standalone"));
        let app = b.text_of("app.js").unwrap();
        assert!(app.contains("/api/view/session"));
    }
}
