// Qualia 3D anatomy demo — renders a REAL HRA/CCF reference body from a `.hmc`
// asset pack via the native Qualia renderer compiled to WebGPU/WASM
// (`QualiaPortal`) — the same renderer that runs natively on the desktop.
//
// Includes the first slice of the ATTENTION MIXER (docs/plans/attention-mixer.md):
// an ambient-field channel (off by default) and a per-body-system channel row.
// Mobile: full-viewport canvas + bottom-sheet controls, pinch zoom, orbit drag.

import { ensureCanvasBackingStore, loadQualiaPortal } from "../js/qualia-shell.js?v=0.0.29-mobile-recovery4";
import {
  getBrowserCapabilityReceipt,
  recordBackendDeviceOutcome,
} from "../js/browser-capability.js?v=0.0.29-mobile-recovery4";

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
let capabilityReceipt = null;
let anatomyRenderer = "unsupported";
let renderGeneration = 0;
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
// On phones keep only a lite organ set until the person opts in (peak RAM on Pixel-class devices).
const DEFAULT_MUTED = new Set(["integumentary"]);
const MOBILE_EXTRA_MUTED = new Set([
  "muscular",
  "integumentary",
  "digestive",
  "immune_lymphatic",
  "endocrine",
  "urinary",
  "reproductive",
  "sensory",
  "exocrine",
  "vestibular",
  "ecs",
  "ens",
  "glymphatic",
]);
// Phone lite default keeps: circulatory, respiratory, nervous, skeletal.

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

// Keep in lockstep with crates/qualia-core-db Cargo.toml / release tag.
const ENGINE_VERSION = "0.0.29";
const RELEASE_BASE = `https://github.com/mediaprophet/qualiaDB/releases/download/v${ENGINE_VERSION}`;
const isCoarsePointer = () =>
  (typeof window !== "undefined" &&
    (window.matchMedia?.("(pointer: coarse)").matches ||
      window.matchMedia?.("(max-width: 860px)").matches)) ||
  false;
const isMobileUA = () =>
  /Android|iPhone|iPad|iPod|Mobile/i.test(navigator.userAgent || "");
/** True on phones / coarse pointers — cap canvas DPR and mute heavier systems. */
const isPhonePath = () => isMobileUA() || isCoarsePointer();
const deviceMemoryGb = () => {
  const m = navigator.deviceMemory;
  return typeof m === "number" && m > 0 ? m : null;
};
// Pixel-class phones often report 4–8 GB; Chrome still kills tabs near ~100 MB pack × multi-copy.
// Cap backing-store DPR so WebGPU surfaces stay within budget after mesh upload.
// Pixel DPR is often 2.6–3.5; full-res WebGPU surfaces + ~100 MB mesh = tab kill.
const anatomyMaxDpr = () => (isPhonePath() ? 1.25 : 3);

/** Same-origin only for browser fetch. GitHub Releases do not send CORS ACAO for XHR/fetch,
 *  so a release URL fallback always fails in Chrome — prefer Pages-hosted packs (CI must ship them). */
function bodyUrls(key) {
  if (key === "complete") {
    // Complete body is file-picker only in the browser (CORS + ~700 MB).
    return [];
  }
  const name = `anatomy-${key}.hmc`;
  return [name];
}

/** OPFS cache key for a body pack (versioned so engine bumps invalidate). */
function packCacheName(key) {
  return `anatomy-v${ENGINE_VERSION}-${key}.hmc`;
}

async function opfsRoot() {
  try {
    if (!navigator?.storage?.getDirectory) return null;
    return await navigator.storage.getDirectory();
  } catch {
    return null;
  }
}

/** Read a previously cached pack from OPFS (if any). */
async function readPackFromOpfs(key) {
  const root = await opfsRoot();
  if (!root) return null;
  try {
    const fh = await root.getFileHandle(packCacheName(key));
    const file = await fh.getFile();
    if (file.size < 1024) return null;
    setProgress(true, 100, `OPFS cache · ${(file.size / 1e6).toFixed(0)} MB`);
    return new Uint8Array(await file.arrayBuffer());
  } catch {
    return null;
  }
}

/** Best-effort OPFS write after a successful network fetch. */
async function writePackToOpfs(key, bytes) {
  const root = await opfsRoot();
  if (!root || !bytes || !bytes.length) return;
  const name = packCacheName(key);
  const part = name + ".part";
  try {
    const partHandle = await root.getFileHandle(part, { create: true });
    const writable = await partHandle.createWritable();
    const CHUNK = 8 * 1024 * 1024;
    for (let off = 0; off < bytes.length; off += CHUNK) {
      await writable.write(bytes.subarray(off, Math.min(off + CHUNK, bytes.length)));
    }
    await writable.close();
    if (typeof partHandle.move === "function") {
      await partHandle.move(name);
    } else {
      const fin = await root.getFileHandle(name, { create: true });
      const w = await fin.createWritable();
      for (let off = 0; off < bytes.length; off += CHUNK) {
        await w.write(bytes.subarray(off, Math.min(off + CHUNK, bytes.length)));
      }
      await w.close();
      try {
        await root.removeEntry(part);
      } catch {
        /* ignore */
      }
    }
  } catch (e) {
    console.warn("[anatomy] OPFS cache write failed:", e);
    try {
      await root.removeEntry(part);
    } catch {
      /* ignore */
    }
  }
}
function bodyUrl(key) {
  return bodyUrls(key)[0];
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

function layoutSize() {
  if (!container) return { w: 0, h: 0 };
  const w = Math.round(container.clientWidth || container.getBoundingClientRect().width || 0);
  const h = Math.round(container.clientHeight || container.getBoundingClientRect().height || 0);
  return { w, h };
}

function onResize() {
  if (!canvas || !container) return;
  const { w, h } = layoutSize();
  // Skip 0×0 — common on first mobile paint before flex/dvh settles; resizing the
  // WebGPU surface to zero blanks the body and looks like a failed render.
  if (w < 2 || h < 2) return;
  ensureCanvasBackingStore(canvas, w, h, { maxDpr: anatomyMaxDpr() });
  if (portal && portal.resize) {
    try {
      portal.resize(canvas, w, h);
    } catch (_) {}
  }
}

/** Wait until the stage has a real box (phones often report 0×0 on first frame). */
async function waitForLayout(maxMs = 2500) {
  const start = performance.now();
  while (performance.now() - start < maxMs) {
    const { w, h } = layoutSize();
    if (w >= 32 && h >= 32) {
      onResize();
      return { w, h };
    }
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  }
  // Last resort: force a usable backing store so WebGPU can bind.
  if (canvas) ensureCanvasBackingStore(canvas, 360, 640, { maxDpr: anatomyMaxDpr() });
  onResize();
  return layoutSize();
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

  if (!container) {
    setStatus("Missing #canvas-container — anatomy page markup is incomplete.", "error");
    return;
  }

  canvas = document.createElement("canvas");
  canvas.style.cssText = "width:100%;height:100%;display:block;touch-action:none;background:transparent";
  canvas.setAttribute("aria-label", "3D anatomy body");
  container.appendChild(canvas);

  // Mixer is useful even without WebGPU (taxonomy inspectable).
  buildMixer();

  // Phones: sheet closed first so the flex stage gets the full viewport height
  // before we measure the canvas / bind WebGPU.
  if (isSheetMode()) setSheetOpen(false);
  await waitForLayout();

  capabilityReceipt = await getBrowserCapabilityReceipt({
    engineVersion: ENGINE_VERSION,
    sessionId: new URL(location.href).searchParams.get("lab") || "",
  });
  window.__qualiaCapabilityReceipt = capabilityReceipt;
  const rendererOverride = new URL(location.href).searchParams.get("renderer");
  const selectedRenderer = rendererOverride === "webgl2" && capabilityReceipt.webgl2.available
    ? "webgl2"
    : capabilityReceipt.selection.anatomy;

  // API presence is not adapter capability. WebGL2 remains independently
  // usable on phones where Chrome suppresses every WebGPU adapter.
  // Chrome may expose navigator.gpu while blocklisting every adapter. Consume the
  // proposed shared capability receipt before selecting the Anatomy renderer.
  if (selectedRenderer === "unsupported") {
    const isIOS = /iPhone|iPad|iPod/i.test(navigator.userAgent);
    const isAndroid = /Android/i.test(navigator.userAgent);
    let hint =
      "This browser exposes neither WebGPU nor WebGL2, so the real 3D body cannot be rendered.";
    if (isIOS) hint += " On iPhone/iPad use a current Safari release with hardware acceleration enabled.";
    else if (isAndroid) hint += " On Android use current Chrome rather than an embedded in-app browser.";
    else hint += " Use a current hardware-accelerated browser.";
    hint += " The system mixer still lists what the engine evaluates — open Controls.";
    setStatus(hint, "error");
    if (isSheetMode()) setSheetOpen(true);
    return;
  }

  setStatus(`Loading the Qualia renderer (WASM · ${selectedRenderer})…`);
  let res;
  try {
    // Re-measure immediately before GPU init (rotation / address-bar hide changes height).
    await waitForLayout(1200);
    res = await loadQualiaPortal(canvas, {
      anatomyBackend: selectedRenderer,
      allowWebGl2: capabilityReceipt.webgl2.available,
      requireBodyRenderer: true,
    });
  } catch (e) {
    recordBackendDeviceOutcome(capabilityReceipt, "anatomy", {
      backend: selectedRenderer,
      state: "device_request_failed",
      error: e,
    });
    setStatus("Renderer failed to load: " + e, "error");
    if (isSheetMode()) setSheetOpen(true);
    return;
  }
  portal = res.portal;
  anatomyRenderer = res.renderer || "unsupported";
  recordBackendDeviceOutcome(capabilityReceipt, "anatomy", {
    backend: anatomyRenderer,
    state: portal ? "available" : "unsupported",
  });
  window.__portal = portal;
  if (!portal) {
    setStatus("Anatomy renderer unavailable (source: " + res.source + ").", "error");
    if (isSheetMode()) setSheetOpen(true);
    return;
  }
  if (typeof portal.load_body_from_qualia_bundle_mixed !== "function") {
    setStatus("The loaded renderer is stale — rebuild docs/pkg/qualia from current source.", "error");
    return;
  }

  new ResizeObserver(() => {
    onResize();
    // After a real size appears post-load, re-push the body so the mesh is not stuck at 0×0.
    // Debounced — mobile chrome (URL bar hide) must not re-decode the whole pack every frame.
    if (bodyBytes && layoutSize().w >= 32) {
      scheduleApplyMixer(false);
    }
  }).observe(container);
  window.addEventListener("resize", onResize);
  window.addEventListener("orientationchange", () => {
    setTimeout(async () => {
      await waitForLayout(800);
      if (bodyBytes) scheduleApplyMixer(true);
    }, 200);
  });
  setupOrbit();
  setupZoomButtons();
  startLoop();
  onResize();
  await loadBody(currentBody);
  // One more layout pass after the first body (mobile URL bar / safe-area settle).
  setTimeout(async () => {
    await waitForLayout(600);
    if (bodyBytes) scheduleApplyMixer(true);
  }, 400);
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
  lastApplySignature = ""; // force a real decode for the new pack
  buildMixer();
  buildPartsList();
  scheduleApplyMixer(true);
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
  // Prefer HTTP cache on revisit; first visit still streams once.
  const resp = await fetch(url, { cache: "force-cache" });
  if (!resp.ok) {
    setProgress(false);
    throw Object.assign(new Error(`HTTP ${resp.status}`), { status: resp.status, resp });
  }
  // Content-Length is often the *transfer* size. With Content-Encoding (gzip/br)
  // or CDN quirks, the decoded ReadableStream can be larger than CL — never treat
  // CL as a hard cap (that produced "download exceeded Content-Length" on Pages).
  const declared = Number(resp.headers.get("Content-Length") || 0);
  const encoding = (resp.headers.get("Content-Encoding") || "identity").toLowerCase();
  const trustDeclared =
    declared > 0 && (encoding === "identity" || encoding === "");
  if (!resp.body || !resp.body.getReader) {
    const buf = await resp.arrayBuffer();
    setProgress(true, 100, "Unpacking…");
    return new Uint8Array(buf);
  }
  const reader = resp.body.getReader();
  // Prefer single allocation when size is known (avoids double-memory spike on phones).
  // Grow if the stream overruns declared length (decompression / wrong CL).
  let capacity = trustDeclared ? declared : 0;
  let out = capacity > 0 ? new Uint8Array(capacity) : null;
  const chunks = capacity > 0 ? null : [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    if (!value || value.length === 0) continue;
    if (out) {
      if (loaded + value.length > out.length) {
        const next = new Uint8Array(Math.max(out.length * 2, loaded + value.length));
        next.set(out.subarray(0, loaded), 0);
        out = next;
      }
      out.set(value, loaded);
    } else {
      chunks.push(value);
    }
    loaded += value.length;
    if (declared > 0) {
      const pct = Math.min(99, Math.round((loaded / declared) * 100));
      const mb = (loaded / 1e6).toFixed(1);
      const tmb = (declared / 1e6).toFixed(0);
      setProgress(true, pct, `${label || "Downloading"} ${pct}% · ${mb}/${tmb} MB`);
    } else {
      setProgress(true, null, `${label || "Downloading"} ${(loaded / 1e6).toFixed(1)} MB…`);
    }
  }
  if (out) {
    if (loaded !== out.length) {
      out = out.subarray(0, loaded);
    }
  } else {
    out = new Uint8Array(loaded);
    let off = 0;
    for (const c of chunks) {
      out.set(c, off);
      off += c.length;
    }
  }
  // Only fail incomplete when we trusted CL and got *strictly less* (truncation).
  if (trustDeclared && loaded < declared) {
    throw new Error(`incomplete download ${loaded}/${declared}`);
  }
  setProgress(true, 100, "Building body…");
  return out;
}

async function loadBody(key) {
  renderAttribution();
  showCompleteLoader(key === "complete");
  currentBody = key;

  if (key === "complete") {
    setProgress(false);
    setStatus(
      "The complete body is a large pack (~700 MB). Prefer Male/Female on a phone. On desktop you can download the file below and load it, or open the desktop app.",
      "error",
    );
    if (isSheetMode()) setSheetOpen(true);
    return;
  }

  const sizeHint = isPhonePath() ? " (~90–120 MB — stay on Wi‑Fi)" : "";
  const mem = deviceMemoryGb();
  const memHint = mem != null && mem <= 4 ? ` · ~${mem} GB deviceMemory` : "";

  // 1) OPFS hit — skip network (second visit / retry after a failed GPU upload).
  try {
    setStatus(`Looking for cached ${bodyLabel(key)} body…`);
    const cached = await readPackFromOpfs(key);
    if (cached && cached.length >= 1024) {
      setStatus(`Building ${bodyLabel(key)} body from OPFS cache${memHint}…`);
      setProgress(true, 100, "Building body…");
      await waitForLayout(800);
      // Yield so status paints before the heavy decode/GPU path freezes the main thread.
      await new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
      renderPackBytes(cached);
      onResize();
      applyCam();
      return;
    }
  } catch (e) {
    console.warn("[anatomy] OPFS read failed:", e);
  }

  // 2) Same-origin fetch (Pages). Do not attempt GitHub Release — no CORS for browser fetch.
  setStatus(`Fetching ${bodyLabel(key)} reference body${sizeHint}${memHint}…`);
  const urls = bodyUrls(key);
  let lastErr = null;
  for (const url of urls) {
    try {
      const label = `Fetching ${bodyLabel(key)}`;
      const bytes = await fetchWithProgress(url, label);
      if (!bytes || bytes.length < 1024) {
        throw new Error("pack too small / empty");
      }
      // Cache before decode so a mid-render OOM still leaves a retry path without re-download.
      void writePackToOpfs(key, bytes);
      setStatus(`Building ${bodyLabel(key)} body (${(bytes.length / 1e6).toFixed(0)} MB pack)…`);
      setProgress(true, 100, "Building body…");
      await waitForLayout(800);
      await new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
      renderPackBytes(bytes);
      onResize();
      applyCam();
      return;
    } catch (e) {
      lastErr = e;
      console.warn("[anatomy] pack fetch/build failed for", url, e);
      const msg = String(e && e.message ? e.message : e);
      if (/out of memory|oom|allocation|RangeError/i.test(msg)) {
        setProgress(false);
        setStatus(
          "Phone ran out of memory while building the body. Keep Skin/Muscular off (default), close other tabs, hard-refresh, then try again — the pack is cached in OPFS so it will not re-download.",
          "error",
        );
        if (isSheetMode()) setSheetOpen(true);
        return;
      }
    }
  }
  setProgress(false);
  if (lastErr && lastErr.status) {
    setStatus(
      `Body pack not found (HTTP ${lastErr.status}). Pages must ship anatomy-male/female.hmc under playground/ (CI fetch_anatomy_packs_release.sh).`,
      "error",
    );
  } else {
    setStatus(
      "Body pack load failed: " +
        (lastErr && lastErr.message ? lastErr.message : lastErr) +
        ". On phones use Chrome (not an in-app browser), stay on Wi‑Fi, try Male if Female failed.",
      "error",
    );
  }
  if (isSheetMode()) setSheetOpen(true);
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

let applyMixerTimer = null;
let lastApplySignature = "";
let applyInFlight = false;

function mixerSignature() {
  const levels = Object.keys(systemLevels)
    .sort()
    .map((k) => `${k}:${systemLevels[k]}`)
    .join(",");
  const disabled = Array.from(disabledParts).sort().join(",");
  const { w, h } = layoutSize();
  // Quantise size so 1px chrome jitter does not re-decode the whole pack.
  return `${currentBody}|${levels}|${disabled}|${Math.round(w / 8)}x${Math.round(h / 8)}`;
}

/** Debounced mixer apply — phones re-fire resize/orientation and must not re-decode 100 MB packs. */
function scheduleApplyMixer(immediate = false) {
  if (immediate) {
    if (applyMixerTimer) {
      clearTimeout(applyMixerTimer);
      applyMixerTimer = null;
    }
    applyMixer();
    return;
  }
  if (applyMixerTimer) clearTimeout(applyMixerTimer);
  applyMixerTimer = setTimeout(() => {
    applyMixerTimer = null;
    applyMixer();
  }, isPhonePath() ? 220 : 60);
}

function applyMixer() {
  if (!portal || !bodyBytes || applyInFlight) return;
  const sig = mixerSignature();
  if (sig === lastApplySignature && window.__lastRender) {
    applyCam();
    return;
  }
  onResize();
  const noun = currentBody === "complete" ? "structures" : "organs";
  applyInFlight = true;
  try {
    const r = portal.load_body_from_qualia_bundle_mixed(
      bodyBytes,
      systemLevels,
      Array.from(disabledParts),
    );
    window.__lastRender = r;
    lastApplySignature = sig;
    const organs = r && r.organs_loaded != null ? r.organs_loaded : "?";
    const tris =
      r && r.total_triangles != null ? Number(r.total_triangles).toLocaleString() : "?";
    const { w, h } = layoutSize();
    if (w < 32 || h < 32) {
      setStatus(
        `${bodyLabel(currentBody)} body loaded (${organs} ${noun}) but the canvas is still 0×0 — rotate the phone or reopen Controls once so layout can settle.`,
        "error",
      );
      return;
    }
    const gesture = isCoarsePointer() ? "drag to orbit · pinch to zoom" : "drag to orbit · scroll to zoom";
    setStatus(`${bodyLabel(currentBody)} body · ${organs} ${noun} · ${tris} triangles · ${gesture}`, "ok");
    setProgress(false);
    setStatus(`${bodyLabel(currentBody)} body uploaded · waiting for ${anatomyRenderer} presentation…`);
    setProgress(true, 100, "Presenting body…");
    const generation = ++renderGeneration;
    void confirmBodyPresented(generation, { organs, noun, tris, gesture, upload: r });
    applyCam();
  } catch (e) {
    const msg = String(e);
    if (/out of memory|oom|allocation|RangeError/i.test(msg)) {
      setStatus(
        "Render ran out of memory. On phones keep Skin/Muscular off (default), close other tabs, or reload — pack stays in OPFS.",
        "error",
      );
    } else {
      setStatus("Render error: " + e, "error");
    }
  } finally {
    applyInFlight = false;
  }
}

async function confirmBodyPresented(generation, summary) {
  if (typeof portal?.body_render_receipt !== "function") {
    setStatus("The loaded renderer is stale: Anatomy lifecycle receipts are unavailable.", "error");
    return;
  }
  for (let frame = 0; frame < 20; frame++) {
    await new Promise((resolve) => requestAnimationFrame(resolve));
    if (generation !== renderGeneration) return;
    const receipt = portal.body_render_receipt();
    window.__anatomyRenderReceipt = {
      schema: "qualia.anatomy-render.v1",
      capability: capabilityReceipt,
      asset: `anatomy-${currentBody}.hmc`,
      upload: summary.upload,
      presentation: receipt,
    };
    if (receipt?.success) {
      setStatus(
        `${bodyLabel(currentBody)} body · ${summary.organs} ${summary.noun} · ${summary.tris} triangles · ${receipt.renderer} · ${summary.gesture}`,
        "ok",
      );
      setProgress(false);
      return;
    }
  }
  setProgress(false);
  setStatus(
    `Anatomy upload completed but ${anatomyRenderer} did not present a body frame.`,
    "error",
  );
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
  const muteSet = isPhonePath()
    ? new Set([...DEFAULT_MUTED, ...MOBILE_EXTRA_MUTED])
    : DEFAULT_MUTED;
  const overlayTip =
    "A distributed network with no standalone organ mesh — evaluated and painted as an overlay on its host organs.";
  for (const [id, label, overlay] of SYSTEMS) {
    if (havePack && !overlay && !present.has(id)) continue;
    const muted = muteSet.has(id);
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
        scheduleApplyMixer(true);
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
        scheduleApplyMixer(true);
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
  scheduleApplyMixer(true);
};

window.onAmbient = (checked) => {
  if (portal) portal.set_ambient_enabled(!!checked);
};

function startLoop() {
  let stopped = false;
  const frame = (t) => {
    if (stopped) return;
    const dt = lastT ? t - lastT : 16;
    lastT = t;
    try {
      portal.tick(canvas, dt);
    } catch (error) {
      stopped = true;
      const message = String(error?.message || error).slice(0, 400);
      window.__anatomyRenderError = {
        schema: "qualia.anatomy-render-error.v1",
        renderer: anatomyRenderer,
        message,
        observedAt: new Date().toISOString(),
      };
      setStatus(`Anatomy ${anatomyRenderer} render stopped: ${message}`, "error");
      return;
    }
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
