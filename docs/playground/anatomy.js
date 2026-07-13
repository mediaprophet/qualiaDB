// Qualia 3D anatomy demo — renders a REAL HRA/CCF reference body from a `.qualia`
// asset pack via the native Qualia renderer compiled to WebGPU/WASM
// (`QualiaPortal`) — the same renderer that runs natively on the desktop.
//
// Includes the first slice of the ATTENTION MIXER (docs/plans/attention-mixer.md):
// an ambient-field channel (off by default) and a per-body-system channel row, so
// the viewer composes what they attend to rather than an algorithm deciding it.

import { loadQualiaPortal } from "../js/qualia-shell.js";

const container = document.getElementById("canvas-container");
const statusEl = document.getElementById("status");

let portal = null;
let canvas = null;
let currentBody = "male";
let lastT = 0;
let bodyBytes = null; // the loaded .qualia pack bytes, cached for mixer re-renders
// The loaded pack's manifest — [{ key, label, system, systems }] per part — read from the pack itself
// (not hardcoded), so the mixer + parts list are DYNAMIC: they reflect exactly what this body contains.
let packParts = [];
// Parts individually deselected (by entry key) via the parts list — hidden on the next render.
const disabledParts = new Set();
const cam = { yaw: 0.5, pitch: 0.12, zoom: 2.4 };

// Mixer: per-body-system level 0..1 (absent = full). v1 is mute/show (the mesh
// pipeline is opaque); smooth opacity lands with alpha blending (mixer plan v2).
const systemLevels = {};
// The full body-system taxonomy — all 17, mirroring wellfare-core's system registry (the seed the
// evaluation engine uses). The third field marks a DISTRIBUTED network (ECS / ENS / glymphatic): it has
// no standalone organ mesh, so it is evaluated and — in the app — painted as an overlay on its host
// organs, rather than shown/hidden as a discrete mesh here. A discrete system with no mesh in the loaded
// pack is still listed (you can attend to it) but has nothing to show/hide until that mesh ships.
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
// Systems muted by default: opaque skin would occlude everything, so the mixer starts
// with it peeled off — turn the Skin fader up to wrap the body (translucent skin = v2).
const DEFAULT_MUTED = new Set(["integumentary"]);

// CC-BY attribution for the 3D assets, per body. The CCF male/female bodies are permissive CC-BY-4.0;
// the BodyParts3D "complete" body is CC-BY-SA-2.1-JP (share-alike) and requires the exact attribution +
// citation the database's terms specify.
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
// The sources that apply to the body currently loaded.
function sourcesForBody() {
  return currentBody === "complete" ? [BP3D_SOURCE] : [CCF_SOURCE];
}

// The BodyParts3D complete pack is large (exceeds GitHub Pages' 100 MB/file limit), so it ships as a
// GitHub Release asset. NOTE: release assets are NOT fetchable cross-origin (no CORS) — this URL is used
// for a DOWNLOAD link (navigation bypasses CORS), then the user loads the local file. Pinned to the tag
// the packs are uploaded to.
const RELEASE_BASE = "https://github.com/mediaprophet/qualiaDB/releases/download/v0.0.24";
function bodyUrl(key) {
  return key === "complete" ? `${RELEASE_BASE}/anatomy-bodyparts3d.qualia` : `anatomy-${key}.qualia`;
}

// The body's full provenance/semantics `.q42` graph — a byte-identical sidecar beside the bundle
// (anatomy-<body>.q42), directly linkable. For CCF it carries organ→system + provenance; for the
// complete body it is the FMA ontology (OBO IRIs + is-a + part-of + system + geometry) the meshes cite.
function provenanceQ42Url() {
  return currentBody === "complete" ? `${RELEASE_BASE}/anatomy-bodyparts3d.q42` : `anatomy-${currentBody}.q42`;
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
    `<div class="note">The Qualia engine and the .10d / .qualia container formats are separate works; ` +
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

function onResize() {
  if (portal && portal.resize) {
    try {
      portal.resize(canvas, container.clientWidth, container.clientHeight);
    } catch (_) {}
  }
}

async function boot() {
  renderAttribution();
  // A canvas that fills the container — the old demo's canvas had no CSS size, so
  // the renderer sized to 0×0 and drew nothing. `100%` fixes that.
  canvas = document.createElement("canvas");
  canvas.style.cssText = "width:100%;height:100%;display:block;touch-action:none";
  container.appendChild(canvas);

  // Build the attention mixer first — the full 17-system taxonomy is meaningful (and inspectable) even
  // when the body itself can't render. The faders are inert until a body loads (applyMixer is guarded).
  buildMixer();

  if (!navigator.gpu) {
    setStatus(
      "This browser has no WebGPU. The real 3D body needs a WebGPU-capable browser (Chrome/Edge, or Firefox Nightly). The system mixer below still lists what the engine evaluates.",
      "error",
    );
    return;
  }

  setStatus("Loading the Qualia renderer (WASM · WebGPU)…");
  let res;
  try {
    res = await loadQualiaPortal(canvas);
  } catch (e) {
    setStatus("Renderer failed to load: " + e, "error");
    return;
  }
  portal = res.portal;
  window.__portal = portal; // debug handle
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
  startLoop();
  await loadBody(currentBody);
}

function bodyLabel(key) {
  return key === "complete" ? "complete (BodyParts3D)" : key === "male" ? "XY" : "XX";
}

// Render a body from its raw .qualia bytes (shared by the same-origin fetch and the local-file load).
// Reads the pack's OWN manifest → the mixer + parts list are DYNAMIC (built from what the body contains).
function renderPackBytes(bytes) {
  bodyBytes = bytes;
  try {
    packParts = typeof portal.pack_manifest === "function" ? Array.from(portal.pack_manifest(bodyBytes) || []) : [];
  } catch (e) {
    packParts = [];
  }
  disabledParts.clear();
  buildMixer();
  buildPartsList();
  applyMixer();
}

// Show/hide the complete-body loader (download link + local-file picker) and point the link at the pack.
function showCompleteLoader(show) {
  const el = document.getElementById("complete-loader");
  if (el) el.style.display = show ? "block" : "none";
  const dl = document.getElementById("complete-dl");
  if (dl) dl.href = `${RELEASE_BASE}/anatomy-bodyparts3d.qualia`;
}

async function loadBody(key) {
  // The provenance link + attribution are per-body — refresh them for the current body.
  renderAttribution();
  showCompleteLoader(key === "complete");

  if (key === "complete") {
    // GitHub Release assets are NOT fetchable cross-origin (no Access-Control-Allow-Origin), and the
    // pack is far larger than GitHub Pages' 100 MB/file limit — so the browser can't fetch it. The web
    // path is: download the pack (a navigation download bypasses CORS), then load the local file below.
    // The native desktop app reads the pack directly — no CORS, no download step.
    setStatus(
      "The complete body is a large pack. Browsers can't fetch GitHub-release assets cross-origin, so download it below and load the file — or open it in the desktop app, which loads it directly.",
      "error",
    );
    return;
  }

  // CCF male/female ship same-origin on Pages, so a normal fetch works.
  setStatus(`Fetching ${bodyLabel(key)} reference body…`);
  let resp;
  try {
    resp = await fetch(bodyUrl(key), { cache: "no-store" });
  } catch (e) {
    setStatus("Body pack fetch failed: " + e, "error");
    return;
  }
  if (!resp.ok) {
    setStatus(
      `Body pack not found (HTTP ${resp.status}). The .qualia packs ship as build artifacts — produce them with the build_anatomy_pack tool.`,
      "error",
    );
    return;
  }
  renderPackBytes(new Uint8Array(await resp.arrayBuffer()));
}

// Load the complete body from a .qualia file the user downloaded (local read — no CORS).
window.onCompleteFile = (file) => {
  if (!file || !portal) return;
  setStatus(`Loading ${file.name} (${(file.size / 1e6).toFixed(0)} MB)…`);
  const reader = new FileReader();
  reader.onload = () => {
    try {
      renderPackBytes(new Uint8Array(reader.result));
    } catch (e) {
      setStatus("Render error: " + e, "error");
    }
  };
  reader.onerror = () => setStatus("Could not read the file.", "error");
  reader.readAsArrayBuffer(file);
};

// Re-render the cached body with the current mixer settings.
function applyMixer() {
  if (!portal || !bodyBytes) return;
  const noun = currentBody === "complete" ? "structures" : "organs";
  try {
    const r = portal.load_body_from_qualia_bundle_mixed(bodyBytes, systemLevels, Array.from(disabledParts));
    window.__lastRender = r;
    const organs = r && r.organs_loaded != null ? r.organs_loaded : "?";
    const tris = r && r.total_triangles != null ? Number(r.total_triangles).toLocaleString() : "?";
    setStatus(`${bodyLabel(currentBody)} body · ${organs} ${noun} · ${tris} triangles · drag to orbit`, "ok");
  } catch (e) {
    setStatus("Render error: " + e, "error");
  }
}

// The set of systems the loaded pack actually contains (from the manifest).
function systemsInPack() {
  const s = new Set();
  for (const p of packParts) {
    const list = p.systems && p.systems.length ? p.systems : [p.system];
    for (const sys of list) if (sys) s.add(sys);
  }
  return s;
}

// Build the mixer channel rows — DYNAMIC: with a pack loaded, only the systems it actually contains
// (plus the mesh-less overlay networks, which are evaluated everywhere). Before a pack loads, the full
// 17-system taxonomy. A fader at 0 mutes that system; >0 shows it.
function buildMixer() {
  const host = document.getElementById("mixer-systems");
  if (!host) return;
  host.innerHTML = "";
  const present = systemsInPack();
  const havePack = packParts.length > 0;
  const overlayTip =
    "A distributed network with no standalone organ mesh — evaluated and painted as an overlay on its host organs (ENS→gut, glymphatic→brain, ECS→whole-body), not shown/hidden as a discrete mesh here.";
  for (const [id, label, overlay] of SYSTEMS) {
    // With a body loaded, hide systems that body has no meshes for (keep overlay networks, always).
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
    if (overlay) {
      // No discrete mesh to show/hide. Keep the row so the taxonomy is complete (you can see the system
      // is evaluated), but make plain the fader isn't adjustable here — overlay rendering is a later slice.
      fader.disabled = true;
      name.title = overlayTip;
      fader.title = overlayTip;
    } else {
      // Update the level live while dragging; re-render on release (a re-render
      // re-uploads the body, so we don't do it every drag tick).
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

// The human label for a system id (from the taxonomy), or the id itself.
function systemLabel(id) {
  const s = SYSTEMS.find((e) => e[0] === id);
  return s ? s[1] : id;
}

// Build the selectable PARTS list from the pack manifest — every individual structure, grouped by its
// primary system in a collapsible section, each a checkbox (checked = shown). Unchecking a part hides
// just that structure on the next render. Searchable (essential for the ~900-part complete body).
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

// Filter the parts list by name; open groups that have a match.
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

// Select or deselect every part at once (respects the current search filter → visible parts only).
window.setAllParts = (on) => {
  const host = document.getElementById("parts-list");
  if (!host) return;
  host.querySelectorAll(".part-row").forEach((row) => {
    if (row.style.display === "none") return; // only the visible (filtered) parts
    const cb = row.querySelector("input");
    const p = packParts.find((x) => (x.label || x.key).toLowerCase() === row.dataset.q);
    if (!cb || !p) return;
    cb.checked = on;
    if (on) disabledParts.delete(p.key);
    else disabledParts.add(p.key);
  });
  applyMixer();
};

// Ambient σ field channel (off by default).
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

function setupOrbit() {
  applyCam();
  let dragging = false;
  let lx = 0;
  let ly = 0;
  canvas.addEventListener("pointerdown", (e) => {
    dragging = true;
    lx = e.clientX;
    ly = e.clientY;
    canvas.setPointerCapture?.(e.pointerId);
  });
  const stop = () => {
    dragging = false;
  };
  canvas.addEventListener("pointerup", stop);
  canvas.addEventListener("pointercancel", stop);
  canvas.addEventListener("pointermove", (e) => {
    if (!dragging) return;
    cam.yaw += (e.clientX - lx) * 0.008;
    cam.pitch = Math.max(-1.4, Math.min(1.4, cam.pitch + (e.clientY - ly) * 0.008));
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
}

// Sidebar body selector (CCF XY / XX, or the complete BodyParts3D body). Mixer settings persist.
window.setSex = (key) => {
  if (key === currentBody || !portal) return;
  currentBody = key;
  document.getElementById("btn-male")?.classList.toggle("active", key === "male");
  document.getElementById("btn-female")?.classList.toggle("active", key === "female");
  document.getElementById("btn-complete")?.classList.toggle("active", key === "complete");
  loadBody(key);
};

boot().catch((e) => setStatus("Boot failed: " + e, "error"));
