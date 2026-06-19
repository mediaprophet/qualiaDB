/**
 * Qualia Design Studio — NL → design graph → 10D portal projection.
 * Works standalone; enriches via :8080 SPARQL proxy when the settings portal is live.
 */

const STORAGE_KEY = "qualia-design-jobs-v1";
const DESIGN_TYPE = "qualia.design";
const DESIGN_VERSION = "1.0.0";

let portalLive = false;
let daemonReachable = false;
let daemonPort = 4242;
let sparqlEndpoints = [];
let currentDesign = null;
let jobs = [];
let activeJobId = null;
let animTime = 0;
let rafId = null;
let lastRecommendations = null;

const canvas = () => document.getElementById("design-canvas");
const hud = () => document.getElementById("canvas-hud");

// ─── Portal / SPARQL ─────────────────────────────────────────────────────────

async function probePortal() {
  const badge = document.getElementById("portal-badge");
  try {
    const res = await fetch("/api/status");
    if (!res.ok) throw new Error("status failed");
    const st = await res.json();
    portalLive = true;
    daemonReachable = !!st.graph_daemon_reachable;
    daemonPort = st.graph_daemon_port || 4242;
    badge.textContent = daemonReachable
      ? `Portal live · daemon :${daemonPort}`
      : `Portal live · daemon offline`;
    badge.className = "portal-badge live";
    await loadSparqlEndpoints();
  } catch {
    portalLive = false;
    daemonReachable = false;
    badge.textContent = "Offline — browser-only mode";
    badge.className = "portal-badge offline";
  }
  document.getElementById("btn-enrich").disabled = !portalLive;
}

async function loadSparqlEndpoints() {
  try {
    const res = await fetch("/api/sparql/endpoints");
    if (!res.ok) return;
    const data = await res.json();
    sparqlEndpoints = data.endpoints || [];
    const sel = document.getElementById("sparql-target");
    sel.innerHTML = "";
    const local = document.createElement("option");
    local.value = "local";
    local.textContent = `Local graph (:${data.local_daemon_port || daemonPort})`;
    sel.appendChild(local);
    for (const ep of sparqlEndpoints) {
      if (ep.endpoint === "local") continue;
      const o = document.createElement("option");
      o.value = ep.endpoint;
      o.textContent = ep.name;
      sel.appendChild(o);
    }
  } catch (e) {
    console.warn("endpoints", e);
  }
}

async function runSparqlQuery(query, target, endpoint) {
  if (!portalLive) return null;
  const body = { query, target: target === "local" ? "local" : "remote", endpoint };
  const res = await fetch("/api/sparql/query", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const err = await res.text();
    throw new Error(err || res.statusText);
  }
  return res.json();
}

function logSparql(msg) {
  const el = document.getElementById("sparql-log");
  const line = document.createElement("div");
  line.textContent = msg;
  el.prepend(line);
  while (el.childNodes.length > 8) el.removeChild(el.lastChild);
}

// ─── Natural language → design document ──────────────────────────────────────

function slugId(text) {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 32) || "part";
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

  const twoPart =
    /two[- ]?part|male.+female|female.+male|base.+face|face.+base|module.+base/i.test(text);

  if (twoPart) {
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

  const segmentRe =
    /(?:part|module|section|component|piece)\s+(?:called\s+)?["']?([a-z0-9][\w\s-]{1,40})/gi;
  let match;
  while ((match = segmentRe.exec(prompt)) !== null) {
    const label = match[1].trim();
    const id = slugId(label);
    if (parts.some((p) => p.id === id)) continue;
    parts.push({
      id,
      label,
      role: "component",
      installer: "",
      components: [],
      pos: null,
      state: "default",
      intensity: 0.6,
      reasons: [],
    });
  }

  if (parts.length === 0) {
    const nouns = prompt.match(
      /\b(switch|socket|powerpoint|light|device|assembly|product|gadget|tool|container|housing|frame|panel)\b/gi
    );
    const label = nouns ? nouns[0] : "Main assembly";
    parts.push({
      id: slugId(label),
      label: label.charAt(0).toUpperCase() + label.slice(1).toLowerCase(),
      role: "product",
      installer: installers[0] || "",
      components,
      pos: [0, 0, 0],
      state: "highlighted",
      intensity: 0.72,
      reasons: ["Inferred from description"],
    });
    if (components.length > 1) {
      components.forEach((c, i) => {
        parts.push({
          id: c,
          label: c.replace(/-/g, " "),
          role: "sub-component",
          installer: "",
          components: [],
          pos: [1.2 + i * 0.8, 0.3, 0.5],
          state: "active",
          intensity: 0.55,
          reasons: [`Contained in ${parts[0].id}`],
        });
      });
    }
  }

  return parts;
}

function detectRelations(parts, prompt) {
  const relations = [];
  const ids = new Set(parts.map((p) => p.id));
  const text = prompt.toLowerCase();

  if (ids.has("face") && ids.has("base")) {
    relations.push({ from: "face", to: "base", type: "matesWith", label: "Snap fit" });
  }

  if (/contain|includes|has a|with a/i.test(text)) {
    const root = parts[0]?.id;
    if (root) {
      for (const p of parts.slice(1)) {
        if (p.role === "sub-component" || p.components?.length === 0) {
          relations.push({ from: root, to: p.id, type: "contains", label: "" });
        }
      }
    }
  }

  for (let i = 1; i < parts.length; i++) {
    const prev = parts[i - 1].id;
    const cur = parts[i].id;
    if (!relations.some((r) => r.from === prev && r.to === cur)) {
      if (/connect|attach|snap|mate|join|plug/i.test(text)) {
        relations.push({ from: cur, to: prev, type: "connectsTo", label: "" });
      }
    }
  }

  return relations;
}

function parsePromptToDesign(prompt) {
  const trimmed = prompt.trim();
  const parts = detectParts(trimmed);
  const relations = detectRelations(parts, trimmed);
  return {
    type: DESIGN_TYPE,
    version: DESIGN_VERSION,
    title: extractTitle(trimmed),
    summary: trimmed.slice(0, 200),
    prompt: trimmed,
    parts,
    relations,
    explanations: [
      "Parsed from natural language (demo heuristic). Connect an LLM or remote agent for richer structure.",
    ],
    sparql_context: [],
  };
}

// ─── SPARQL enrichment ───────────────────────────────────────────────────────

function keywordsFromDesign(design) {
  const words = new Set();
  for (const p of design.parts) {
    for (const w of [p.id, p.label, p.role, ...(p.components || [])]) {
      w.split(/\W+/).forEach((t) => {
        if (t.length > 2) words.add(t.toLowerCase());
      });
    }
  }
  return [...words].slice(0, 6);
}

async function enrichDesignWithSparql(design) {
  if (!portalLive) return design;
  const target = document.getElementById("sparql-target").value;
  const isLocal = target === "local";
  const keywords = keywordsFromDesign(design);
  const label = keywords[0] || "product";

  const queries = [];
  if (isLocal && daemonReachable) {
    queries.push({
      endpoint: "local",
      query: `SELECT ?s ?p ?o WHERE { ?s ?p ?o . FILTER(CONTAINS(LCASE(STR(?o)), "${label}")) } LIMIT 15`,
    });
  } else if (!isLocal) {
    queries.push({
      endpoint: target,
      query: `SELECT ?item ?itemLabel WHERE {
        ?item rdfs:label ?itemLabel .
        FILTER(CONTAINS(LCASE(STR(?itemLabel)), "${label}"))
      } LIMIT 10`,
    });
  }

  for (const q of queries) {
    try {
      logSparql(`Querying ${q.endpoint === "local" ? "daemon" : q.endpoint}…`);
      const bindings = await runSparqlQuery(
        q.query,
        q.endpoint === "local" ? "local" : "remote",
        q.endpoint === "local" ? undefined : q.endpoint
      );
      design.sparql_context.push({
        endpoint: q.endpoint,
        query: q.query,
        bindings,
      });
      const rows = bindings?.results?.bindings?.length || 0;
      logSparql(`→ ${rows} bindings`);
      if (rows > 0) {
        design.explanations.push(
          `SPARQL enrichment: ${rows} related triples from ${q.endpoint === "local" ? "local graph" : "remote endpoint"}.`
        );
      }
    } catch (e) {
      logSparql(`✗ ${e.message}`);
    }
  }
  return design;
}

// ─── Tensor layout (JS fallback mirrors design_encode.rs) ────────────────────

function layoutTensors(design) {
  const total = design.parts.length;
  const positions = design.parts.map((p, i) => {
    if (p.pos) return p.pos;
    const t = i / Math.max(total, 1);
    const angle = t * Math.PI * 2;
    const r = 4 + total * 0.15;
    return [r * Math.cos(angle), i * 0.35 - total * 0.15, r * Math.sin(angle)];
  });

  const nodes = design.parts.map((p, i) => {
    const [x, y, z] = positions[i];
    return {
      id: p.id,
      label: p.label || p.id,
      x: x / 10,
      y: y / 10,
      z: z / 10,
      alpha: p.intensity ?? 0.65,
      q: p.installer && p.installer !== "user" ? 0 : 0.12,
      state: p.state,
      kind: "part",
    };
  });

  for (const rel of design.relations) {
    const fi = design.parts.findIndex((p) => p.id === rel.from);
    const ti = design.parts.findIndex((p) => p.id === rel.to);
    if (fi < 0 || ti < 0) continue;
    const a = positions[fi];
    const b = positions[ti];
    nodes.push({
      id: `${rel.from}-${rel.type}-${rel.to}`,
      label: rel.type,
      x: (a[0] + b[0]) / 20,
      y: (a[1] + b[1]) / 20 + 0.025,
      z: (a[2] + b[2]) / 20,
      alpha: 0.5,
      q: 0.18,
      state: "default",
      kind: "relation",
    });
  }

  const edges = design.relations.map((r) => ({ from: r.from, to: r.to, type: r.type }));
  return { nodes, edges, positions };
}

// ─── Canvas projection (Qualia Portal–style 10D preview) ───────────────────

function projectNode(node, w, h, time) {
  const yaw = time * 0.15;
  const cos = Math.cos(yaw);
  const sin = Math.sin(yaw);
  const x = node.x * cos - node.z * sin;
  const z = node.x * sin + node.z * cos;
  const depth = 1.6 + z;
  const scale = 220 / depth;
  const px = w * 0.5 + x * scale;
  const py = h * 0.45 - node.y * scale + Math.sin(time + node.alpha * 4) * 3;
  const radius = (node.kind === "relation" ? 4 : 8 + node.alpha * 10) / depth;
  return { px, py, radius, depth, node };
}

function paintDesignFrame() {
  const c = canvas();
  if (!c || !currentDesign) return;
  const ctx = c.getContext("2d");
  const w = c.width;
  const h = c.height;
  const { nodes, edges } = layoutTensors(currentDesign);

  const grad = ctx.createLinearGradient(0, 0, w, h);
  grad.addColorStop(0, "#0c1824");
  grad.addColorStop(1, "#060a10");
  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, w, h);

  for (let i = 0; i < 40; i++) {
    const px = ((i * 97) % w) + Math.sin(animTime + i) * 12;
    const py = ((i * 53) % h) + Math.cos(animTime * 0.7 + i) * 8;
    ctx.fillStyle = `rgba(52, 211, 153, ${0.03 + (i % 5) * 0.01})`;
    ctx.beginPath();
    ctx.arc(px, py, 1.2, 0, Math.PI * 2);
    ctx.fill();
  }

  const projected = nodes.map((n) => projectNode(n, w, h, animTime));
  const byId = Object.fromEntries(projected.map((p) => [p.node.id, p]));

  ctx.lineWidth = 1;
  for (const e of edges) {
    const a = byId[e.from];
    const b = byId[e.to];
    if (!a || !b) continue;
    ctx.strokeStyle = "rgba(52, 211, 153, 0.35)";
    ctx.beginPath();
    ctx.moveTo(a.px, a.py);
    ctx.lineTo(b.px, b.py);
    ctx.stroke();
  }

  projected.sort((a, b) => a.depth - b.depth);
  for (const p of projected) {
    const n = p.node;
    const hue =
      n.state === "alert" ? [239, 68, 68] : n.state === "highlighted" ? [245, 158, 11] : [52, 211, 153];
    const alpha = 0.35 + n.alpha * 0.55;
    ctx.fillStyle = `rgba(${hue[0]},${hue[1]},${hue[2]},${alpha})`;
    ctx.beginPath();
    ctx.arc(p.px, p.py, p.radius, 0, Math.PI * 2);
    ctx.fill();
    if (n.q > 0.1) {
      ctx.strokeStyle = `rgba(147, 197, 253, ${0.4 + n.q})`;
      ctx.beginPath();
      ctx.arc(p.px, p.py, p.radius + 4, 0, Math.PI * 2);
      ctx.stroke();
    }
    ctx.fillStyle = "rgba(232,238,247,0.85)";
    ctx.font = "11px system-ui";
    ctx.fillText(n.label, p.px + p.radius + 4, p.py + 3);
  }

  if (hud()) {
    hud().innerHTML = [
      `<strong>${currentDesign.title}</strong>`,
      `${currentDesign.parts.length} parts · ${currentDesign.relations.length} relations`,
      `${nodes.length} tensor nodes · 10D preview [x,y,z + q,α]`,
      portalLive ? (daemonReachable ? "SPARQL: local graph available" : "SPARQL: portal only") : "Save locally · connect :8080 for SPARQL",
    ].join("<br>");
  }
}

function startRenderLoop() {
  if (rafId) cancelAnimationFrame(rafId);
  const loop = () => {
    animTime += 0.016;
    paintDesignFrame();
    rafId = requestAnimationFrame(loop);
  };
  loop();
}

function resizeCanvas() {
  const c = canvas();
  const wrap = document.getElementById("design-canvas-wrap");
  if (!c || !wrap) return;
  const rect = wrap.getBoundingClientRect();
  c.width = Math.max(320, Math.floor(rect.width));
  c.height = Math.max(280, Math.floor(rect.height));
}

// ─── Asset recommendations (native :8080 API) ───────────────────────────────

function inferDomains(text) {
  const lower = (text || "").toLowerCase();
  const rules = [
    ["product", ["design", "product", "assembly", "module", "switch", "socket", "device"]],
    ["electrical", ["electric", "power", "mains", "wiring", "powerpoint"]],
    ["iot", ["sensor", "wifi", "smart", "mcu", "home"]],
    ["health", ["medical", "anatomy", "clinical", "dicom"]],
    ["legal", ["contract", "rights", "policy"]],
    ["geography", ["map", "location", "geo", "building"]],
    ["linguistics", ["word", "language", "lexicon"]],
  ];
  const out = [];
  for (const [domain, kws] of rules) {
    if (kws.some((k) => lower.includes(k))) out.push(domain);
  }
  return out.length ? out : ["general"];
}

function renderAssets(rec) {
  if (!rec) return;
  const llmUl = document.getElementById("asset-llms");
  const ontUl = document.getElementById("asset-ontologies");
  llmUl.innerHTML = "";
  ontUl.innerHTML = "";
  const tier =
    typeof rec.device?.tier === "string" ? rec.device.tier : rec.device?.tier?.label || "unknown";
  const ram = rec.device?.ram_gb?.toFixed?.(1) ?? rec.device?.ram_gb ?? "?";
  document.getElementById("device-summary").textContent =
    `Tier: ${tier} · ${ram} GB RAM (native) · domains: ${(rec.inferred_domains || []).join(", ")}`;

  for (const a of rec.llms || []) {
    const li = document.createElement("li");
    li.className = "asset-item";
    const installed = a.already_installed ? " ✓ installed" : "";
    li.innerHTML = `<div><strong>${a.name}</strong>${installed}</div><div class="meta">${a.reason} · ~${a.ram_estimate_mb || a.size_mb} MB · <code>${a.install?.cli_hint || ""}</code></div>`;
    llmUl.appendChild(li);
  }
  for (const a of rec.ontologies || []) {
    const li = document.createElement("li");
    li.className = "asset-item";
    const installed = a.already_installed;
    const btn =
      portalLive && !installed && a.install?.kind === "ontology_catalog_import"
        ? `<button type="button" data-ont="${a.id}">Enqueue install</button>`
        : installed
          ? `<span class="meta">Already installed</span>`
          : "";
    li.innerHTML = `<div><strong>${a.name}</strong></div><div class="meta">${a.reason} · ${a.size_mb?.toFixed?.(1) ?? a.size_mb} MB ${btn}</div>`;
    ontUl.appendChild(li);
    const b = li.querySelector("button[data-ont]");
    if (b) {
      b.onclick = async () => {
        try {
          const res = await fetch("/api/assets/enqueue", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ kind: "ontology_catalog_import", ontology_id: a.id }),
          });
          if (!res.ok) throw new Error(await res.text());
          b.textContent = "Queued ✓";
          b.className = "installed";
        } catch (e) {
          b.textContent = "Failed";
          console.warn(e);
        }
      };
    }
  }
}

async function refreshRecommendations(prompt) {
  if (!portalLive) {
    document.getElementById("device-summary").textContent =
      "Portal offline — open docs/design-studio.html for browser catalog scoring.";
    return;
  }
  try {
    const res = await fetch("/api/assets/recommend", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        device: {},
        design: { prompt: prompt || "", domains: inferDomains(prompt), keywords: [] },
      }),
    });
    if (!res.ok) throw new Error(await res.text());
    lastRecommendations = await res.json();
    renderAssets(lastRecommendations);
  } catch (e) {
    document.getElementById("device-summary").textContent = `Recommendations failed: ${e.message}`;
  }
}

// ─── Jobs persistence ────────────────────────────────────────────────────────

function loadJobs() {
  try {
    jobs = JSON.parse(localStorage.getItem(STORAGE_KEY) || "[]");
  } catch {
    jobs = [];
  }
  renderJobList();
}

function saveJobs() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(jobs));
  renderJobList();
}

function renderJobList() {
  const ul = document.getElementById("job-list");
  ul.innerHTML = "";
  if (!jobs.length) {
    ul.innerHTML = "<li class='meta'>No saved designs yet.</li>";
    return;
  }
  for (const job of jobs) {
    const li = document.createElement("li");
    li.className = "job-item" + (job.id === activeJobId ? " active" : "");
    li.innerHTML = `<div>${job.design.title}</div><div class="meta">${new Date(job.savedAt).toLocaleString()} · ${job.design.parts.length} parts</div>`;
    li.onclick = () => loadJob(job.id);
    ul.appendChild(li);
  }
}

function saveCurrentJob() {
  if (!currentDesign) return;
  const id = activeJobId || `job-${Date.now()}`;
  const entry = { id, savedAt: new Date().toISOString(), design: currentDesign, assets: lastRecommendations };
  const idx = jobs.findIndex((j) => j.id === id);
  if (idx >= 0) jobs[idx] = entry;
  else jobs.unshift(entry);
  if (jobs.length > 48) jobs.length = 48;
  activeJobId = id;
  saveJobs();
  document.getElementById("save-msg").textContent = "Saved.";
  setTimeout(() => (document.getElementById("save-msg").textContent = ""), 2000);
}

function loadJob(id) {
  const job = jobs.find((j) => j.id === id);
  if (!job) return;
  activeJobId = id;
  currentDesign = job.design;
  document.getElementById("design-prompt").value = job.design.prompt || "";
  if (job.assets) {
    lastRecommendations = job.assets;
    renderAssets(job.assets);
  }
  updateGraphPreview();
  renderJobList();
}

function updateGraphPreview() {
  const pre = document.getElementById("graph-preview");
  if (!currentDesign) {
    pre.textContent = "";
    return;
  }
  pre.textContent = JSON.stringify(
    {
      title: currentDesign.title,
      parts: currentDesign.parts.map((p) => ({ id: p.id, role: p.role, installer: p.installer })),
      relations: currentDesign.relations,
    },
    null,
    2
  );
}

// ─── WASM (optional) ─────────────────────────────────────────────────────────

async function tryWasmEncode(design) {
  if (typeof design_encode_wasm !== "function") return null;
  try {
    return design_encode_wasm(JSON.stringify(design));
  } catch (e) {
    console.warn("WASM encode", e);
    return null;
  }
}

// ─── Actions ─────────────────────────────────────────────────────────────────

async function generateFromPrompt() {
  const prompt = document.getElementById("design-prompt").value.trim();
  if (!prompt) return;
  let design = parsePromptToDesign(prompt);
  if (document.getElementById("sparql-on-generate").checked && portalLive) {
    design = await enrichDesignWithSparql(design);
  }
  currentDesign = design;
  activeJobId = null;
  updateGraphPreview();
  await refreshRecommendations(prompt);
  const wasm = await tryWasmEncode(design);
  if (wasm) logSparql(`WASM: ${wasm.tensor_count} tensor nodes baked`);
}

async function enrichOnly() {
  if (!currentDesign) {
    await generateFromPrompt();
    return;
  }
  currentDesign = await enrichDesignWithSparql(currentDesign);
  updateGraphPreview();
}

function newBlank() {
  currentDesign = null;
  activeJobId = null;
  document.getElementById("design-prompt").value = "";
  updateGraphPreview();
}

function init() {
  loadJobs();
  probePortal().then(() => refreshRecommendations(""));
  setInterval(probePortal, 15000);
  resizeCanvas();
  window.addEventListener("resize", resizeCanvas);
  startRenderLoop();

  document.getElementById("btn-generate").onclick = () => generateFromPrompt();
  document.getElementById("btn-enrich").onclick = () => enrichOnly();
  document.getElementById("btn-save").onclick = () => saveCurrentJob();
  document.getElementById("btn-new").onclick = () => newBlank();
  document.getElementById("btn-assets").onclick = () =>
    refreshRecommendations(document.getElementById("design-prompt").value.trim());

  document.getElementById("design-prompt").addEventListener("keydown", (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") generateFromPrompt();
  });
}

document.addEventListener("DOMContentLoaded", init);