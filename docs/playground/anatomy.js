// Qualia 3D anatomy demo — renders a REAL HRA/CCF reference body from a `.hmc`
// asset pack via the native Qualia renderer compiled to WebGPU/WASM
// (`QualiaPortal`) — the same renderer that runs natively on the desktop.
//
// Includes the first slice of the ATTENTION MIXER (docs/plans/attention-mixer.md):
// an ambient-field channel (off by default) and a per-body-system channel row.
// Mobile: full-viewport canvas + bottom-sheet controls, pinch zoom, orbit drag.

import { loadQualiaPortal } from "../js/qualia-shell.js";

const container = document.getElementById("canvas-container");
const statusEl = document.getElementById("status");
const sidebar = document.getElementById("sidebar");
const sheetScrim = document.getElementById("sheet-scrim");
const controlsBtn = document.getElementById("btn-controls");
const progressWrap = document.getElementById("progress-wrap");
const progressFill = document.getElementById("progress-fill");
const progressLabel = document.getElementById("progress-label");

let portal = null;
let canvas = null;
let currentBody = "male";
let lastT = 0;
let bodyBytes = null;
// Pack manifest — [{ key, label, system, systems }] per part — built from the pack itself.
let packParts = [];
const disabledParts = new Set();
const cam = { yaw: 0.5, pitch: 0.12, zoom: 2.4 };
const DEFAULT_CAM = { yaw: 0.5, pitch: 0.12, zoom: 2.4 };

const systemLevels = {};
// Full 17-system taxonomy (mirrors wellfare-core registry seed). Third field = distributed overlay.
const SYSTEMS = [
  ["circulatory", "Circulatory", false],
  ["respiratory", "Respiratory", false],
  ["digestive", "Digestive", false],
  ["nervous", "Nervous", false],
  ["muscular", "Muscular", false],
  ["skeletal", "Skeletal", false],
  ["endocrine", "Endocrine", false],
  ["immune_lymphatic", "Immune / Lymphatic", false],
  ["integumentary", "Skin", false],
  ["urinary", "Urinary", false],
  ["reproductive", "Reproductive", false],
  ["sensory", "Sensory", false],
  ["vestibular", "Vestibular", false],
  ["exocrine", "Exocrine", false],
  ["ecs", "Endocannabinoid (ECS)", true],
  ["ens", "Enteric (ENS)", true],
  ["glymphatic", "Glymphatic", true],
];
// Opaque skin occludes everything — peel by default.
const DEFAULT_MUTED = new Set(["integumentary"]);

const CCF_SOURCE = {
  what: "3D reference-organ meshes",
  creator: "Human Reference Atlas (CCF) / HuBMAP",
  url: "https://humanatlas.io",
  licence: "CC BY 4.0",
  licenceUrl: "https://creativecommons.org/licenses/by/4.0/",
  scope: "organ mesh geometry only",
};
const BP3D_SOURCE = {
  what: "Complete anatomy meshes + FMA ontology (muscles, bones, glands, nerves)",
  creator: "BodyParts3D, © The Database Center for Life Science licensed under CC Attribution-Share Alike 2.1 Japan",
  url: "https://lifesciencedb.jp/bp3d/",
  licence: "CC BY-SA 2.1 JP",
  licenceUrl: "https://creativecommons.org/licenses/by-sa/2.1/jp/deed.en",
  scope: "3D mesh geometry + FMA anatomy ontology",
  cite: "Mitsuhashi et al., Nucleic Acids Res. 2009 (doi:10.1093/nar/gkn613) · data doi:10.18908/lsdba.nbdc00837-000",
};

const RELEASE_BASE = "https://github.com/mediaprophet/qualiaDB/releases/download/v0.0.24";
const isCoarsePointer = () =>
  (typeof window !== "undefined" &&
    (window.matchMedia?.("(pointer: coarse)").matches ||
      window.matchMedia?.("(max-width: 860px)").matches)) ||
  false;
const isMobileUA = () =>
  /Android|iPhone|iPad|iPod|Mobile/i.test(navigator.userAgent || "");

function bodyUrl(key) {
  return key === "complete" ? `${RELEASE_BASE}/anatomy-bodyparts3d.hmc` : `anatomy-${key}.hmc`;
}
function provenanceQ42Url() {
  return currentBody === "complete"
    ? `${RELEASE_BASE}/anatomy-bodyparts3d.q42`
    : `anatomy-${currentBody}.q42`;
}
function sourcesForBody() {
  return currentBody === "complete" ? [BP3D_SOURCE] : [CCF_SOURCE];
}

function renderAttribution() {
  const el = document.getElementById("attribution-body");
  if (!el) return;
  const rows = sourcesForBody()
    .map(
      (s) =>
        `<div><span class="src">${s.what}</span> — ` +
        `<a href="${s.url}" target="_blank" rel="noopener">${s.creator}</a> · ` +
        `<a href="${s.licenceUrl}" target="_blank" rel="noopener">${s.licence}</a>` +
        (s.scope ? ` <span class="scope">— applies to: ${s.scope}</span>` : "") +
        (s.cite ? `<div class="scope">Cite: ${s.cite}</div>` : "") +
        `</div>`,
    )
    .join("");
  const prov =
    `<div><a href="${provenanceQ42Url()}" rel="noopener" download>` +
    `Full provenance &amp; semantics — .q42 graph volume ↓</a></div>`;
  const note =
    `<div class="note">The Qualia engine and the .10d / .hmc container formats are separate works; ` +
    `no rights are claimed over the source datasets beyond the attribution each licence requires.</div>`;
  el.innerHTML = rows + prov + note;
}

function setStatus(msg, cls) {
  if (statusEl) {
    statusEl.textContent = msg;
    statusEl.className = "status" + (cls ? " " + cls : "");
  }
  console.log("[anatomy]", msg);
}

function setProgress(visible, pct, label) {
  if (!progressWrap) return;
  progressWrap.classList.toggle("visible", !!visible);
  if (progressLabel && label != null) progressLabel.textContent = label;
  if (progressFill && pct != null) {
    progressFill.style.width = `${Math.max(0, Math.min(100, pct))}%`;
  }
}

function onResize() {
  if (portal && portal.resize && canvas) {
    try {
      portal.resize(canvas, container.clientWidth, container.clientHeight);
    } catch (_) {}
  }
}

// ── Controls sheet (mobile bottom sheet / desktop always-on sidebar) ──
function isSheetMode() {
  return window.matchMedia("(max-width: 860px)").matches;
}

function setSheetOpen(open) {
  if (!sidebar) return;
  if (!isSheetMode()) {
    sidebar.classList.remove("open");
    sheetScrim?.classList.remove("open");
    controlsBtn?.setAttribute("aria-expanded", "false");
    return;
  }
  sidebar.classList.toggle("open", open);
  sheetScrim?.classList.toggle("open", open);
  controlsBtn?.setAttribute("aria-expanded", open ? "true" : "false");
  // After sheet animates, remeasure canvas (flex stage may reflow when overlays paint).
  setTimeout(onResize, open ? 50 : 320);
}

function setupSheet() {
  controlsBtn?.addEventListener("click", () => {
    const open = !sidebar?.classList.contains("open");
    setSheetOpen(open);
  });
  sheetScrim?.addEventListener("click", () => setSheetOpen(false));
  // Escape closes sheet
  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape") setSheetOpen(false);
  });
  window.addEventListener("resize", () => {
    if (!isSheetMode()) setSheetOpen(false);
    onResize();
  });
  // Swipe-down on handle area to dismiss
  let startY = 0;
  sidebar?.addEventListener(
    "touchstart",
    (e) => {
      if (!isSheetMode() || !sidebar.classList.contains("open")) return;
      if (sidebar.scrollTop > 0) return;
      startY = e.touches[0].clientY;
    },
    { passive: true },
  );
  sidebar?.addEventListener(
    "touchend",
    (e) => {
      if (!startY) return;
      const dy = e.changedTouches[0].clientY - startY;
      startY = 0;
      if (dy > 70) setSheetOpen(false);
    },
    { passive: true },
  );
}

async function boot() {
  renderAttribution();
  setupSheet();

  canvas = document.createElement("canvas");
  canvas.style.cssText = "width:100%;height:100%;display:block;touch-action:none";
  canvas.setAttribute("aria-label", "3D anatomy body");
  container.appendChild(canvas);

  // Mixer is useful even without WebGPU (taxonomy inspectable).
  buildMixer();

  if (!navigator.gpu) {
    const isIOS = /iPhone|iPad|iPod/i.test(navigator.userAgent);
    const isAndroid = /Android/i.test(navigator.userAgent);
    let hint =
      "This browser has no WebGPU. The real 3D body needs a WebGPU-capable browser.";
    if (isIOS) hint += " On iPhone/iPad use Safari 18+ (iOS 18+), or enable WebGPU under Settings → Safari → Advanced → Feature Flags.";
    else if (isAndroid) hint += " On Android use Chrome 121+ on a device with a supported GPU.";
    else hint += " Use Chrome, Edge, or Firefox Nightly with hardware acceleration.";
    hint += " The system mixer still lists what the engine evaluates — open Controls.";
    setStatus(hint, "error");
    if (isSheetMode()) setSheetOpen(true);
    return;
  }

  setStatus("Loading the Qualia renderer (WASM · WebGPU)…");
  let res;
  try {
    res = await loadQualiaPortal(canvas);
  } catch (e) {
    setStatus("Renderer failed to load: " + e, "error");
    if (isSheetMode()) setSheetOpen(true);
    return;
  }
  portal = res.portal;
  window.__portal = portal;
  if (!portal) {
    setStatus("WebGPU portal unavailable (source: " + res.source + ").", "error");
    return;
  }
  if (typeof portal.load_body_from_qualia_bundle_mixed !== "function") {
    setStatus("The loaded renderer is stale — rebuild docs/pkg/qualia from current source.", "error");
    return;
  }

  new ResizeObserver(onResize).observe(container);
  window.addEventListener("resize", onResize);
  setupOrbit();
  setupZoomButtons();
  startLoop();
  // On phones, leave the sheet closed so the body fills the screen first.
  if (isSheetMode()) setSheetOpen(false);
  await loadBody(currentBody);
}

function bodyLabel(key) {
  return key === "complete" ? "complete (BodyParts3D)" : key === "male" ? "XY" : "XX";
}

function renderPackBytes(bytes) {
  bodyBytes = bytes;
  try {
    packParts =
      typeof portal.pack_manifest === "function"
        ? Array.from(portal.pack_manifest(bodyBytes) || [])
        : [];
  } catch (e) {
    packParts = [];
  }
  disabledParts.clear();
  buildMixer();
  buildPartsList();
  applyMixer();
  setProgress(false);
  // After first body on mobile, keep sheet closed so the person sees the body.
  if (isSheetMode()) setSheetOpen(false);
}

function showCompleteLoader(show) {
  const el = document.getElementById("complete-loader");
  if (el) el.style.display = show ? "block" : "none";
  const dl = document.getElementById("complete-dl");
  if (dl) dl.href = `${RELEASE_BASE}/anatomy-bodyparts3d.hmc`;
}

async function fetchWithProgress(url, label) {
  setProgress(true, 0, label || "Downloading…");
  const resp = await fetch(url, { cache: "no-store" });
  if (!resp.ok) {
    setProgress(false);
    throw Object.assign(new Error(`HTTP ${resp.status}`), { status: resp.status, resp });
  }
  const total = Number(resp.headers.get("Content-Length") || 0);
  if (!resp.body || !resp.body.getReader) {
    const buf = await resp.arrayBuffer();
    setProgress(true, 100, "Unpacking…");
    return new Uint8Array(buf);
  }
  const reader = resp.body.getReader();
  // Prefer single allocation when size is known (avoids double-memory spike on phones).
  if (total > 0) {
    const out = new Uint8Array(total);
    let loaded = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      if (loaded + value.length > total) throw new Error("download exceeded Content-Length");
      out.set(value, loaded);
      loaded += value.length;
      const pct = Math.round((loaded / total) * 100);
      const mb = (loaded / 1e6).toFixed(1);
      const tmb = (total / 1e6).toFixed(0);
      setProgress(true, pct, `${label || "Downloading"} ${pct}% · ${mb}/${tmb} MB`);
    }
    if (loaded !== total) throw new Error(`incomplete download ${loaded}/${total}`);
    setProgress(true, 100, "Building body…");
    return out;
  }
  const chunks = [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    loaded += value.length;
    setProgress(true, null, `${label || "Downloading"} ${(loaded / 1e6).toFixed(1)} MB…`);
  }
  const out = new Uint8Array(loaded);
  let off = 0;
  for (const c of chunks) {
    out.set(c, off);
    off += c.length;
  }
  setProgress(true, 100, "Building body…");
  return out;
}

async function loadBody(key) {
  renderAttribution();
  showCompleteLoader(key === "complete");

  if (key === "complete") {
    setProgress(false);
    setStatus(
      "The complete body is a large pack. Browsers can't fetch GitHub-release assets cross-origin — download it below and load the file, or open it in the desktop app. On a phone, Male/Female CCF packs are the better path.",
      "error",
    );
    if (isSheetMode()) setSheetOpen(true);
    return;
  }

  // CCF male/female ship same-origin on Pages.
  const sizeHint = isMobileUA() || isCoarsePointer()
    ? " (~90–120 MB — stay on Wi‑Fi)"
    : "";
  setStatus(`Fetching ${bodyLabel(key)} reference body${sizeHint}…`);
  try {
    const bytes = await fetchWithProgress(bodyUrl(key), `Fetching ${bodyLabel(key)}`);
    renderPackBytes(bytes);
  } catch (e) {
    setProgress(false);
    if (e && e.status) {
      setStatus(
        `Body pack not found (HTTP ${e.status}). The .hmc packs ship as build artifacts — produce them with the build_anatomy_pack tool.`,
        "error",
      );
    } else {
      setStatus("Body pack fetch failed: " + e, "error");
    }
  }
}

window.onCompleteFile = (file) => {
  if (!file || !portal) return;
  setStatus(`Loading ${file.name} (${(file.size / 1e6).toFixed(0)} MB)…`);
  setProgress(true, 30, `Reading ${file.name}…`);
  const reader = new FileReader();
  reader.onprogress = (ev) => {
    if (ev.lengthComputable) {
      setProgress(true, Math.round((ev.loaded / ev.total) * 90), `Reading ${(ev.loaded / 1e6).toFixed(0)} MB…`);
    }
  };
  reader.onload = () => {
    try {
      setProgress(true, 95, "Building body…");
      renderPackBytes(new Uint8Array(reader.result));
    } catch (e) {
      setProgress(false);
      setStatus("Render error: " + e, "error");
    }
  };
  reader.onerror = () => {
    setProgress(false);
    setStatus("Could not read the file.", "error");
  };
  reader.readAsArrayBuffer(file);
};

function applyMixer() {
  if (!portal || !bodyBytes) return;
  const noun = currentBody === "complete" ? "structures" : "organs";
  try {
    const r = portal.load_body_from_qualia_bundle_mixed(
      bodyBytes,
      systemLevels,
      Array.from(disabledParts),
    );
    window.__lastRender = r;
    const organs = r && r.organs_loaded != null ? r.organs_loaded : "?";
    const tris =
      r && r.total_triangles != null ? Number(r.total_triangles).toLocaleString() : "?";
    const gesture = isCoarsePointer() ? "drag to orbit · pinch to zoom" : "drag to orbit · scroll to zoom";
    setStatus(`${bodyLabel(currentBody)} body · ${organs} ${noun} · ${tris} triangles · ${gesture}`, "ok");
  } catch (e) {
    const msg = String(e);
    if (/out of memory|oom|allocation|RangeError/i.test(msg)) {
      setStatus(
        "Render ran out of memory. On phones, deselect Skin and heavy systems in Controls, or use a desktop browser.",
        "error",
      );
    } else {
      setStatus("Render error: " + e, "error");
    }
  }
}

function systemsInPack() {
  const s = new Set();
  for (const p of packParts) {
    const list = p.systems && p.systems.length ? p.systems : [p.system];
    for (const sys of list) if (sys) s.add(sys);
  }
  return s;
}

function buildMixer() {
  const host = document.getElementById("mixer-systems");
  if (!host) return;
  host.innerHTML = "";
  const present = systemsInPack();
  const havePack = packParts.length > 0;
  const overlayTip =
    "A distributed network with no standalone organ mesh — evaluated and painted as an overlay on its host organs.";
  for (const [id, label, overlay] of SYSTEMS) {
    if (havePack && !overlay && !present.has(id)) continue;
    const muted = DEFAULT_MUTED.has(id);
    systemLevels[id] = muted ? 0 : 1.0;
    const row = document.createElement("div");
    row.className = "mixer-row" + (overlay ? " mixer-row-overlay" : "");
    const name = document.createElement("span");
    name.className = "mixer-label";
    name.textContent = overlay ? label + " ·overlay" : label;
    const fader = document.createElement("input");
    fader.type = "range";
    fader.min = "0";
    fader.max = "100";
    fader.value = muted ? "0" : "100";
    fader.className = "mixer-fader";
    fader.setAttribute("aria-label", label);
    if (overlay) {
      fader.disabled = true;
      name.title = overlayTip;
      fader.title = overlayTip;
    } else {
      fader.addEventListener("input", () => {
        systemLevels[id] = fader.value / 100;
      });
      fader.addEventListener("change", () => {
        systemLevels[id] = fader.value / 100;
        applyMixer();
      });
    }
    row.appendChild(name);
    row.appendChild(fader);
    host.appendChild(row);
  }
}

function systemLabel(id) {
  const s = SYSTEMS.find((e) => e[0] === id);
  return s ? s[1] : id;
}

function buildPartsList() {
  const host = document.getElementById("parts-list");
  const count = document.getElementById("parts-count");
  if (!host) return;
  host.innerHTML = "";
  if (!packParts.length) {
    host.innerHTML = `<div class="mixer-note">Load a body to list its parts. (Older packs without a manifest show none.)</div>`;
    if (count) count.textContent = "";
    return;
  }
  if (count) count.textContent = `${packParts.length} parts`;
  const bySys = new Map();
  for (const p of packParts) {
    const sys = p.system || "other";
    if (!bySys.has(sys)) bySys.set(sys, []);
    bySys.get(sys).push(p);
  }
  for (const [sys, parts] of [...bySys.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
    parts.sort((a, b) => (a.label || a.key).localeCompare(b.label || b.key));
    const det = document.createElement("details");
    det.className = "parts-group";
    const sum = document.createElement("summary");
    sum.textContent = `${systemLabel(sys)} (${parts.length})`;
    det.appendChild(sum);
    for (const p of parts) {
      const label = p.label || p.key;
      const row = document.createElement("label");
      row.className = "part-row";
      row.dataset.q = label.toLowerCase();
      row.dataset.key = p.key;
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = !disabledParts.has(p.key);
      cb.addEventListener("change", () => {
        if (cb.checked) disabledParts.delete(p.key);
        else disabledParts.add(p.key);
        applyMixer();
      });
      const span = document.createElement("span");
      span.textContent = label;
      row.appendChild(cb);
      row.appendChild(span);
      det.appendChild(row);
    }
    host.appendChild(det);
  }
}

window.onPartSearch = (q) => {
  q = (q || "").trim().toLowerCase();
  const host = document.getElementById("parts-list");
  if (!host) return;
  host.querySelectorAll(".parts-group").forEach((det) => {
    let any = false;
    det.querySelectorAll(".part-row").forEach((row) => {
      const match = !q || row.dataset.q.includes(q);
      row.style.display = match ? "" : "none";
      if (match) any = true;
    });
    det.style.display = any ? "" : "none";
    det.open = q ? any : false;
  });
};

window.setAllParts = (on) => {
  const host = document.getElementById("parts-list");
  if (!host) return;
  host.querySelectorAll(".part-row").forEach((row) => {
    if (row.style.display === "none") return;
    const cb = row.querySelector("input");
    const key = row.dataset.key;
    const p = packParts.find((x) => x.key === key) ||
      packParts.find((x) => (x.label || x.key).toLowerCase() === row.dataset.q);
    if (!cb || !p) return;
    cb.checked = on;
    if (on) disabledParts.delete(p.key);
    else disabledParts.add(p.key);
  });
  applyMixer();
};

window.onAmbient = (checked) => {
  if (portal) portal.set_ambient_enabled(!!checked);
};

function startLoop() {
  const frame = (t) => {
    const dt = lastT ? t - lastT : 16;
    lastT = t;
    try {
      portal.tick(canvas, dt);
    } catch (_) {}
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

function applyCam() {
  try {
    portal.set_camera(cam.yaw, cam.pitch, cam.zoom);
  } catch (_) {}
}

function setupZoomButtons() {
  const clampZoom = (z) => Math.max(0.8, Math.min(6, z));
  document.getElementById("btn-zoom-in")?.addEventListener("click", () => {
    cam.zoom = clampZoom(cam.zoom * 0.85);
    applyCam();
  });
  document.getElementById("btn-zoom-out")?.addEventListener("click", () => {
    cam.zoom = clampZoom(cam.zoom * 1.15);
    applyCam();
  });
  document.getElementById("btn-zoom-reset")?.addEventListener("click", () => {
    cam.yaw = DEFAULT_CAM.yaw;
    cam.pitch = DEFAULT_CAM.pitch;
    cam.zoom = DEFAULT_CAM.zoom;
    applyCam();
  });
}

function setupOrbit() {
  applyCam();
  let dragging = false;
  let lx = 0;
  let ly = 0;
  let pinchDist = 0;
  let activePointers = new Map();

  const distance = (a, b) => Math.hypot(a.x - b.x, a.y - b.y);

  canvas.addEventListener("pointerdown", (e) => {
    // Don't fight the sheet / UI
    if (e.button === 2) return;
    activePointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    canvas.setPointerCapture?.(e.pointerId);
    if (activePointers.size === 1) {
      dragging = true;
      lx = e.clientX;
      ly = e.clientY;
    } else if (activePointers.size === 2) {
      dragging = false;
      const pts = [...activePointers.values()];
      pinchDist = distance(pts[0], pts[1]);
    }
  });

  const stopPointer = (e) => {
    activePointers.delete(e.pointerId);
    if (activePointers.size < 2) pinchDist = 0;
    if (activePointers.size === 0) dragging = false;
    if (activePointers.size === 1) {
      const only = [...activePointers.values()][0];
      dragging = true;
      lx = only.x;
      ly = only.y;
    }
  };
  canvas.addEventListener("pointerup", stopPointer);
  canvas.addEventListener("pointercancel", stopPointer);

  canvas.addEventListener("pointermove", (e) => {
    if (!activePointers.has(e.pointerId)) return;
    activePointers.set(e.pointerId, { x: e.clientX, y: e.clientY });

    if (activePointers.size === 2 && pinchDist > 0) {
      const pts = [...activePointers.values()];
      const d = distance(pts[0], pts[1]);
      if (d > 0) {
        // Pinch out → zoom in (smaller cam.zoom distance)
        const scale = pinchDist / d;
        cam.zoom = Math.max(0.8, Math.min(6, cam.zoom * scale));
        pinchDist = d;
        applyCam();
      }
      return;
    }

    if (!dragging) return;
    // Slightly higher gain on coarse pointers for finger-sized screens
    const sens = isCoarsePointer() ? 0.01 : 0.008;
    cam.yaw += (e.clientX - lx) * sens;
    cam.pitch = Math.max(-1.4, Math.min(1.4, cam.pitch + (e.clientY - ly) * sens));
    lx = e.clientX;
    ly = e.clientY;
    applyCam();
  });

  canvas.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      cam.zoom = Math.max(0.8, Math.min(6, cam.zoom * (1 + Math.sign(e.deltaY) * 0.1)));
      applyCam();
    },
    { passive: false },
  );

  // Double-tap zoom toggle (mobile)
  let lastTap = 0;
  canvas.addEventListener("pointerup", (e) => {
    if (e.pointerType === "mouse") return;
    const now = performance.now();
    if (now - lastTap < 280 && activePointers.size === 0) {
      cam.zoom = cam.zoom < 2 ? 3.2 : DEFAULT_CAM.zoom;
      applyCam();
    }
    lastTap = now;
  });
}

window.setSex = (key) => {
  if (key === currentBody && key !== "complete") return;
  if (!portal && key !== "complete") return;
  currentBody = key;
  document.getElementById("btn-male")?.classList.toggle("active", key === "male");
  document.getElementById("btn-female")?.classList.toggle("active", key === "female");
  document.getElementById("btn-complete")?.classList.toggle("active", key === "complete");
  if (portal) loadBody(key);
  else if (key === "complete") {
    showCompleteLoader(true);
    setSheetOpen(true);
  }
};

boot().catch((e) => setStatus("Boot failed: " + e, "error"));
