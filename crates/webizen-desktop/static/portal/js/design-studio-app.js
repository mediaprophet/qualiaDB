/**
 * Design Studio — NL → qualia.design → Qualia Portal (full WASM T2 path).
 * Tech demo: same portal stack as spatial.html (not a canvas2d fallback).
 */

import {
  detectBrowserDevice,
  probeWebGpu,
  loadDocsCatalog,
  recommendFromCatalog,
  fetchPortalRecommendations,
  enqueueOntologyInstall,
} from "./asset-recommendations.js";
import { defaultTelemetry } from "./ambient-viz.js";
import { ensureCrossOriginIsolation } from "./qualia-coi.js";
import { debugEnv, debugError, debugLog, debugTime, debugWarn } from "./qualia-debug.js";
import { loadQualiaPortal, startPortalLoop, ensureCanvasBackingStore } from "./qualia-shell.js";

const STORAGE_KEY = "qualia-design-jobs-v1";
const DESIGN_TYPE = "qualia.design";
const DESIGN_VERSION = "1.0.0";

let portalLive = false;
let currentDesign = null;
let jobs = [];
let activeJobId = null;
let lastRecommendations = null;
let qualiaPortal = null;
let wasm = null;
let wasmSource = "none";
let portalReady = false;
/** Queued Care→iframe anatomy loads that arrived before the portal finished boot. */
let pendingAnatomyLoad = null;
let anatomyLoadInFlight = false;

const $ = (id) => document.getElementById(id);

/**
 * Desktop Care posts `{ type: 'anatomy-load-body', model: 'male'|'female' }` into this
 * design-studio iframe once the body-asset cache is ready. Fetch per-organ `.10d` via
 * the host protocol and upload through QualiaPortal (same path as playground packs).
 */
async function loadAnatomyBodyFromHost(model) {
  const m = (model || "male").toLowerCase() === "female" ? "female" : "male";
  if (!portalReady || !qualiaPortal) {
    pendingAnatomyLoad = m;
    debugLog("anatomy-load queued (portal not ready)", { model: m });
    return;
  }
  if (anatomyLoadInFlight) {
    pendingAnatomyLoad = m;
    return;
  }
  anatomyLoadInFlight = true;
  pendingAnatomyLoad = null;
  try {
    const canvas = $("design-canvas");
    const wrap = $("design-canvas-wrap");
    if (canvas && wrap) {
      const w = Math.max(wrap.clientWidth || 0, canvas.clientWidth || 0, 320);
      const h = Math.max(wrap.clientHeight || 0, canvas.clientHeight || 0, 240);
      ensureCanvasBackingStore(canvas, w, h);
      qualiaPortal.resize?.(canvas, w, h);
    }

    if ($("encode-status")) {
      $("encode-status").textContent = `Loading anatomy body (${m}) from host cache…`;
    }

    const bodyRes = await fetch(
      `webizen://localhost/anatomy/body.json?model=${encodeURIComponent(m)}`,
    );
    if (!bodyRes.ok) {
      throw new Error(`body.json HTTP ${bodyRes.status} — acquire body assets in Care first`);
    }
    const body = await bodyRes.json();
    const painted = Array.isArray(body.percepts) ? body.percepts : [];
    if (!painted.length) {
      throw new Error("body.json has no organ percepts");
    }

    const organs = [];
    let failed = 0;
    for (const entry of painted) {
      const key = entry.organ_key;
      if (!key) continue;
      const tenRes = await fetch(
        `webizen://localhost/anatomy/10d/${encodeURIComponent(m)}/${encodeURIComponent(key)}`,
      );
      if (!tenRes.ok) {
        failed += 1;
        continue;
      }
      const bytes = new Uint8Array(await tenRes.arrayBuffer());
      if (!bytes.byteLength) {
        failed += 1;
        continue;
      }
      const rgba = entry.percept?.rgba || entry.rgba || [0.55, 0.62, 0.78, 1];
      organs.push({
        bytes,
        r: Number(rgba[0] ?? 0.55),
        g: Number(rgba[1] ?? 0.62),
        b: Number(rgba[2] ?? 0.78),
        a: Number(rgba[3] ?? 1),
      });
    }

    if (!organs.length) {
      throw new Error(
        `no .10d organs loaded (percepts=${painted.length}, failed=${failed})`,
      );
    }

    if (typeof qualiaPortal.load_body_organs_colored !== "function") {
      throw new Error(
        "portal missing load_body_organs_colored — rebuild docs/pkg/qualia (package-qualia-wasm.ps1)",
      );
    }

    if (body.fit && typeof qualiaPortal.set_body_fit_json === "function") {
      qualiaPortal.set_body_fit_json(JSON.stringify(body.fit));
    }

    // wasm-bindgen expects a real JS Array of objects, not a plain array of records only.
    const organArr = organs;
    const summary = qualiaPortal.load_body_organs_colored(organArr);
    const loaded =
      summary && typeof summary === "object"
        ? summary.organs_loaded ?? organs.length
        : organs.length;
    const triangles =
      summary && typeof summary === "object" ? summary.total_triangles ?? "—" : "—";
    const refused =
      summary && typeof summary === "object" ? summary.organs_refused ?? 0 : 0;

    debugLog("anatomy body loaded", { model: m, loaded, triangles, refused, failed });
    if ($("encode-status")) {
      $("encode-status").textContent =
        `Anatomy ${m}: ${loaded} organs · ${triangles} tris` +
        (refused ? ` · ${refused} refused` : "") +
        (failed ? ` · ${failed} missing` : "");
    }
    if ($("canvas-hud")) {
      $("canvas-hud").innerHTML = [
        `<strong>Care · ${m} reference body</strong>`,
        `${loaded} organs · ${triangles} triangles`,
        "Qualia Portal · host-cached .10d",
      ].join("<br>");
    }
    qualiaPortal.set_telemetry?.(
      telemetryToFloats({ baking_crystallization: 0.4, epistemic_density: 0.55 }),
    );
  } catch (e) {
    debugError("anatomy-load-body failed", e);
    if ($("encode-status")) {
      $("encode-status").textContent = `Anatomy load failed: ${e?.message || e}`;
    }
  } finally {
    anatomyLoadInFlight = false;
    if (pendingAnatomyLoad) {
      const next = pendingAnatomyLoad;
      pendingAnatomyLoad = null;
      void loadAnatomyBodyFromHost(next);
    }
  }
}

function installAnatomyLoadListener() {
  window.addEventListener("message", (ev) => {
    const data = ev?.data;
    if (!data || typeof data !== "object") return;
    if (data.type !== "anatomy-load-body") return;
    void loadAnatomyBodyFromHost(data.model || "male");
  });
}

function telemetryToFloats(partial) {
  const base = defaultTelemetry();
  const merged = { ...base, ...partial };
  return new Float32Array([
    merged.memory_pressure,
    merged.network_ripple,
    merged.baking_crystallization,
    merged.logic_flashes,
    merged.llm_heat,
    merged.quantum_activity,
    merged.spectral_shift,
    merged.temporal_pulse,
    merged.epistemic_density,
    merged.manifold_pressure,
    0,
    0,
  ]);
}

function showLoading(show) {
  $("loading-overlay")?.classList.toggle("ds-hidden", !show);
  $("main-content")?.style && ($("main-content").style.display = show ? "none" : "");
}

function showError(msg) {
  $("error-message").textContent = msg || "WASM init failed";
  $("error-overlay")?.classList.remove("ds-hidden");
  showLoading(false);
}

function updateWasmBadge() {
  const dot = $("wasm-dot");
  const text = $("wasm-text");
  const badge = $("wasm-badge");
  if (!qualiaPortal) {
    dot.className = "w-2 h-2 rounded-full bg-amber-400";
    text.textContent = `Engine only (${wasmSource})`;
    return;
  }
  const tier = qualiaPortal.tier?.() ?? 0;
  const labels = ["T0 canvas", "T1 tensor", "T2 WebGPU"];
  dot.className = "w-2 h-2 rounded-full bg-emerald-400 animate-pulse";
  text.textContent = `Portal ${labels[tier] ?? tier}`;
  badge.className =
    "flex items-center gap-2 text-xs px-3 py-1.5 rounded-2xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 shrink-0";
}

function updateHud(meta) {
  const tier = qualiaPortal?.tier?.() ?? -1;
  const tierLabel = ["T0", "T1", "T2"][tier] ?? "?";
  const parts = meta?.part_count ?? currentDesign?.parts?.length ?? 0;
  const tensors = meta?.tensor_count ?? meta?.vertex_count ?? "—";
  const quins = meta?.quin_count ?? "—";
  $("canvas-hud").innerHTML = [
    `<strong>${currentDesign?.title ?? "Design Studio"}</strong>`,
    `${parts} parts · ${tensors} tensor nodes · ${quins} quins`,
    `Qualia Portal ${tierLabel} · pick to navigate`,
    portalLive ? "Native :8080 — asset installs available" : "GitHub Pages demo · open desktop for installs",
  ].join("<br>");
  if ($("metric-tensors")) $("metric-tensors").textContent = String(tensors);
  if ($("metric-quins")) $("metric-quins").textContent = String(quins);
  if ($("metric-tier")) $("metric-tier").textContent = tierLabel;
  if ($("encode-status") && meta) {
    $("encode-status").textContent = `Baked via ${meta.backend ?? wasmSource} · hash ${meta.design_hash ?? "—"}`;
  }
}

function bindPortalPick(portal, canvas) {
  if (!canvas?.addEventListener || !portal?.select_node_at) return;
  canvas.addEventListener("pointerdown", (ev) => {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / Math.max(rect.width, 1);
    const scaleY = canvas.height / Math.max(rect.height, 1);
    const px = (ev.clientX - rect.left) * scaleX;
    const py = (ev.clientY - rect.top) * scaleY;
    try {
      portal.select_node_at(px, py, canvas.width, canvas.height);
    } catch (e) {
      console.warn("select_node_at", e);
      return;
    }
    let attempts = 0;
    const wait = () => {
      const idx = portal.poll_selected_node?.() ?? -1;
      if (idx >= 0) {
        portal.navigate_to_node?.(idx);
        portal.collapse_node_q?.(idx);
        portal.set_telemetry?.(telemetryToFloats({ logic_flashes: 0.85 }));
        $("metric-selected") && ($("metric-selected").textContent = String(idx));
        return;
      }
      if (++attempts < 24) requestAnimationFrame(wait);
    };
    requestAnimationFrame(wait);
  });
}

async function initQualiaPortalLayer() {
  debugEnv({ page: "design-studio" });
  // COI is optional (SAB acoustic); do not block portal WASM boot on service-worker reload.
  ensureCrossOriginIsolation({ quiet: true })
    .then((ok) => debugLog("COI ensure", { crossOriginIsolated: ok }))
    .catch((e) => debugWarn("COI ensure failed", e));
  const canvas = $("design-canvas");
  const wrap = $("design-canvas-wrap");
  if (!canvas) throw new Error("canvas missing");
  ensureCanvasBackingStore(canvas, wrap?.clientWidth || 640, wrap?.clientHeight || 420);

  const t = debugTime("initQualiaPortalLayer");
  const { portal, mod, source, portalError } = await loadQualiaPortal(canvas);
  wasm = mod;
  wasmSource = source;
  qualiaPortal = portal;
  debugLog("portal load", { source, hasPortal: !!portal, portalError: portalError?.message });

  if (!portal) {
    debugError("QualiaPortal unavailable after load", { source, wasmSource, portalError });
    const detail = portalError?.message ? ` (${portalError.message})` : "";
    throw new Error(
      `QualiaPortal unavailable — run scripts/package-qualia-wasm.ps1 or deploy via GitHub Pages CI${detail}`,
    );
  }

  portal.set_display_mode?.("hybrid");
  const ro = new ResizeObserver(() => {
    const w = wrap?.clientWidth || 640;
    const h = wrap?.clientHeight || 420;
    portal.resize(canvas, w, h);
  });
  ro.observe(wrap || canvas);
  startPortalLoop(canvas, () => updateHud());
  bindPortalPick(portal, canvas);
  portalReady = true;
  updateWasmBadge();
  t.end({ tier: portal.tier?.(), wasmSource });
}

// ─── NL → qualia.design ─────────────────────────────────────────────────────

function slugId(text) {
  return (
    text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "")
      .slice(0, 32) || "part"
  );
}

function extractTitle(prompt) {
  const m = prompt.match(/^(?:design|create|build|make)\s+(?:a|an)?\s*(.+?)(?:\.|,|$)/i);
  if (m) return m[1].trim().slice(0, 80);
  return prompt.trim().slice(0, 60) || "Untitled design";
}

function detectInstallers(text) {
  const installers = [];
  if (/electrician|licensed|tradesperson|installer/i.test(text)) installers.push("electrician");
  if (/user|homeowner|owner|diy/i.test(text)) installers.push("user");
  return installers;
}

function detectComponents(text) {
  const comps = [];
  const patterns = [
    ["sensor", /motion sensor|proximity|temperature sensor|sensor/i],
    ["mcu", /\b(mcu|microcontroller|computer|processor|esp32|wifi)\b/i],
    ["wifi", /wi-?fi|wireless|network/i],
    ["bluetooth", /bluetooth|ble\b/i],
    ["display", /display|screen|led panel/i],
    ["battery", /battery|power cell/i],
    ["motor", /motor|actuator|servo/i],
    ["camera", /camera|vision/i],
  ];
  for (const [id, re] of patterns) {
    if (re.test(text)) comps.push(id);
  }
  return comps;
}

function detectParts(prompt) {
  const text = prompt.toLowerCase();
  const parts = [];
  const installers = detectInstallers(text);
  const components = detectComponents(text);

  if (/two[- ]?part|male.+female|female.+male|base.+face|face.+base|module.+base/i.test(text)) {
    parts.push({
      id: "base",
      label: "Base / back-box",
      role: "housing",
      installer: installers.includes("electrician") ? "electrician" : "installer",
      components: [],
      pos: [0, 0, 0],
      state: "active",
      intensity: 0.88,
      reasons: ["Fixed installation"],
    });
    parts.push({
      id: "face",
      label: "Smart face module",
      role: "smart-module",
      installer: installers.includes("user") ? "user" : "owner",
      components: components.length ? components : ["mcu"],
      pos: [0, 0.45, 0],
      state: "highlighted",
      intensity: 0.78,
      reasons: components.length ? components : ["User-swappable module"],
    });
  }

  if (!parts.length) {
    parts.push({
      id: "main",
      label: "Main assembly",
      role: "product",
      installer: installers[0] || "",
      components,
      pos: [0, 0, 0],
      state: "highlighted",
      intensity: 0.7,
      reasons: ["Inferred from description"],
    });
  }

  return parts;
}

function detectRelations(parts) {
  const relations = [];
  const ids = new Set(parts.map((p) => p.id));
  if (ids.has("face") && ids.has("base")) {
    relations.push({ from: "face", to: "base", type: "matesWith", label: "Snap fit" });
  }
  return relations;
}

function parsePromptToDesign(prompt) {
  const trimmed = prompt.trim();
  const parts = detectParts(trimmed);
  const relations = detectRelations(parts);
  return {
    type: DESIGN_TYPE,
    version: DESIGN_VERSION,
    title: extractTitle(trimmed),
    summary: trimmed.slice(0, 200),
    prompt: trimmed,
    parts,
    relations,
    explanations: [
      "Heuristic parse — connect native LLM via :8080 for richer qualia.design JSON.",
    ],
    sparql_context: [],
  };
}

// ─── Portal bake ────────────────────────────────────────────────────────────

async function bakeDesignToPortal(design) {
  if (!portalReady || !qualiaPortal) throw new Error("Qualia Portal not ready");
  const json = JSON.stringify(design);
  let meta = null;

  if (typeof wasm?.design_encode_wasm === "function") {
    meta = await wasm.design_encode_wasm(json);
    if (meta && typeof meta === "string") meta = JSON.parse(meta);
  }

  if (typeof wasm?.export_tensor_buffer_wasm !== "function") {
    throw new Error("export_tensor_buffer_wasm missing — rebuild portal WASM");
  }
  const buf = await wasm.export_tensor_buffer_wasm(json);
  qualiaPortal.upload_tensor_buffer(new Uint8Array(buf));
  qualiaPortal.set_telemetry?.(telemetryToFloats({ baking_crystallization: 0.75, logic_flashes: 0.5 }));

  if (!meta && typeof wasm?.spatial_encode_wasm === "function") {
    try {
      meta = await wasm.spatial_encode_wasm(json);
      if (typeof meta === "string") meta = JSON.parse(meta);
    } catch {
      /* design path preferred */
    }
  }

  updateHud(meta);
  return meta;
}

// ─── Assets + portal probe ─────────────────────────────────────────────────

async function probePortal() {
  const badge = $("portal-badge");
  for (const base of ["", "http://127.0.0.1:8080"]) {
    try {
      const res = await fetch(`${base}/api/status`);
      if (!res.ok) continue;
      const st = await res.json();
      portalLive = true;
      badge.textContent = st.graph_daemon_reachable
        ? `Desktop :8080 · daemon :${st.graph_daemon_port}`
        : "Desktop portal live";
      badge.className = "badge-live";
      return;
    } catch {
      /* */
    }
  }
  portalLive = false;
  badge.textContent = "Pages demo · :8080 for native installs";
  badge.className = "badge-off";
}

function renderAssets(rec) {
  if (!rec) return;
  const llmUl = $("asset-llms");
  const ontUl = $("asset-ontologies");
  llmUl.innerHTML = "";
  ontUl.innerHTML = "";
  const tier =
    typeof rec.device?.tier === "string" ? rec.device.tier : rec.device?.tier?.label || "unknown";
  const ram = rec.device?.ram_gb?.toFixed?.(1) ?? rec.device?.ram_gb ?? "?";
  const src = rec.source === "native_portal" ? " · native RAM profile" : "";
  $("device-summary").textContent = `Tier: ${tier} · ${ram} GB RAM${src} · domains: ${(rec.inferred_domains || []).join(", ")}`;

  for (const a of rec.llms || []) {
    const li = document.createElement("li");
    li.className = "asset-item";
    li.innerHTML = `<div><strong>${a.name}</strong></div><div class="meta">${a.reason} · ~${a.ram_estimate_mb || a.size_mb} MB · <code>${a.install?.cli_hint || ""}</code></div>`;
    llmUl.appendChild(li);
  }
  for (const a of rec.ontologies || []) {
    const li = document.createElement("li");
    li.className = "asset-item";
    const btn =
      portalLive && a.install?.kind === "ontology_catalog_import"
        ? `<button type="button" class="ds-btn secondary" data-ont="${a.id}" style="margin-top:4px">Enqueue install</button>`
        : "";
    li.innerHTML = `<div><strong>${a.name}</strong></div><div class="meta">${a.reason} · ${a.size_mb?.toFixed?.(1) ?? a.size_mb} MB ${btn}</div>`;
    ontUl.appendChild(li);
    const b = li.querySelector("button[data-ont]");
    if (b) {
      b.onclick = async () => {
        try {
          await enqueueOntologyInstall(a.id);
          b.textContent = "Queued ✓";
        } catch (e) {
          b.textContent = "Failed";
          console.warn(e);
        }
      };
    }
  }
}

async function refreshRecommendations(prompt) {
  const portalRec = await fetchPortalRecommendations(prompt);
  if (portalRec) {
    lastRecommendations = portalRec;
    renderAssets(portalRec);
    return;
  }
  const catalog = await loadDocsCatalog();
  const device = detectBrowserDevice();
  device.has_webgpu = await probeWebGpu();
  lastRecommendations = recommendFromCatalog(catalog, device, prompt);
  renderAssets(lastRecommendations);
}

async function generate() {
  const prompt = $("design-prompt").value.trim();
  if (!prompt) return;
  $("btn-generate").disabled = true;
  try {
    currentDesign = parsePromptToDesign(prompt);
    activeJobId = null;
    $("graph-preview").textContent = JSON.stringify(
      {
        title: currentDesign.title,
        parts: currentDesign.parts,
        relations: currentDesign.relations,
      },
      null,
      2,
    );
    await bakeDesignToPortal(currentDesign);
    await refreshRecommendations(prompt);
  } catch (e) {
    $("encode-status").textContent = `Error: ${e.message}`;
    console.error(e);
  } finally {
    $("btn-generate").disabled = false;
  }
}

function saveJob() {
  if (!currentDesign) return;
  const id = activeJobId || `job-${Date.now()}`;
  const entry = {
    id,
    savedAt: new Date().toISOString(),
    design: currentDesign,
    assets: lastRecommendations,
  };
  const idx = jobs.findIndex((j) => j.id === id);
  if (idx >= 0) jobs[idx] = entry;
  else jobs.unshift(entry);
  jobs = jobs.slice(0, 48);
  activeJobId = id;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(jobs));
  renderJobs();
}

function renderJobs() {
  try {
    jobs = JSON.parse(localStorage.getItem(STORAGE_KEY) || "[]");
  } catch {
    jobs = [];
  }
  const ul = $("job-list");
  ul.innerHTML = jobs.length ? "" : "<li class='meta'>No saved designs</li>";
  for (const j of jobs) {
    const li = document.createElement("li");
    li.className = "job-item" + (j.id === activeJobId ? " active" : "");
    li.textContent = `${j.design.title} · ${new Date(j.savedAt).toLocaleString()}`;
    li.onclick = async () => {
      activeJobId = j.id;
      currentDesign = j.design;
      $("design-prompt").value = j.design.prompt || "";
      $("graph-preview").textContent = JSON.stringify(j.design, null, 2);
      if (j.assets) renderAssets(j.assets);
      if (portalReady) {
        try {
          await bakeDesignToPortal(currentDesign);
        } catch (e) {
          console.warn(e);
        }
      }
      renderJobs();
    };
    ul.appendChild(li);
  }
}

async function boot() {
  debugLog("boot start");
  // Listen before portal boot so early postMessages from Care are queued.
  installAnatomyLoadListener();
  showLoading(true);
  try {
    // QualiaPortal needs a laid-out canvas (WebGPU surface); hidden main-content is 0×0.
    const main = $("main-content");
    if (main) main.style.display = "";
    // requestAnimationFrame is paused while a tab is hidden/backgrounded, which
    // would stall boot indefinitely. Race it against a short timeout so a page
    // opened in a background tab still finishes initialising.
    await new Promise((resolve) => {
      let settled = false;
      const done = () => { if (!settled) { settled = true; resolve(); } };
      requestAnimationFrame(done);
      setTimeout(done, 200);
    });
    await initQualiaPortalLayer();
    showLoading(false);
    debugLog("boot complete");
    if (pendingAnatomyLoad) {
      void loadAnatomyBodyFromHost(pendingAnatomyLoad);
    }
  } catch (e) {
    debugError("boot failed", e);
    showError(e.message);
    return;
  }

  renderJobs();
  probePortal();
  setInterval(probePortal, 20000);

  $("btn-generate").onclick = () => generate();
  $("btn-save").onclick = () => saveJob();
  $("btn-assets").onclick = () => refreshRecommendations($("design-prompt").value.trim());
  $("design-prompt").addEventListener("keydown", (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") generate();
  });

  loadDocsCatalog()
    .then((cat) => {
      const device = detectBrowserDevice();
      lastRecommendations = recommendFromCatalog(cat, device, "");
      renderAssets(lastRecommendations);
    })
    .catch(console.warn);
}

document.addEventListener("DOMContentLoaded", boot);