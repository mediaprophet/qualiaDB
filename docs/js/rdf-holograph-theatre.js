/**
 * Cinematic RDF holograph theatre. Mobile requires WebGL2; desktop prefers WebGPU
 * then WebGL2. Canvas2D is fail-closed — it is not a holograph path.
 * HUD: minimized glyph by default; h cycles, Escape recedes.
 */
import { loadQualiaPortal, startPortalLoop, ensureCanvasBackingStore } from "./qualia-shell.js";

const SAMPLES = {
    foaf: `@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix ex: <http://ex/> .
ex:alice foaf:knows ex:bob .
ex:bob foaf:knows ex:carol .
ex:carol foaf:knows ex:alice .
ex:alice foaf:name "Alice" .
ex:bob foaf:name "Bob" .
ex:carol foaf:name "Carol" .
`,
    org: `@prefix org: <http://www.w3.org/ns/org#> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix ex: <http://ex/> .
ex:webizen org:hasMember ex:alice, ex:bob, ex:carol .
ex:alice org:reportsTo ex:dana .
ex:bob org:reportsTo ex:dana .
ex:carol org:reportsTo ex:eve .
ex:dana org:headOf ex:webizen .
ex:lab org:linkedTo ex:webizen .
`,
    star: `@prefix ex: <http://ex/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:nucleus rdf:type ex:Core .
ex:nucleus ex:emits ex:photon .
ex:nucleus ex:binds ex:electron .
ex:nucleus ex:binds ex:proton .
ex:nucleus ex:binds ex:neutron .
ex:photon ex:interferesWith ex:electron .
ex:proton ex:attracts ex:electron .
ex:neutron ex:stabilizes ex:proton .
`,
};

const theatre = document.getElementById("theatre");
const canvas = document.getElementById("render-canvas");
const statusEl = document.getElementById("status");
const backendEl = document.getElementById("backend");
const inspectorEl = document.getElementById("inspector");
const sourceEl = document.getElementById("rdf-source");
const formatEl = document.getElementById("rdf-format");
const failEl = document.getElementById("fail-closed");
const dropVeil = document.getElementById("drop-veil");
const telemNodes = document.getElementById("telem-nodes");
const telemTriples = document.getElementById("telem-triples");
const telemW = document.getElementById("telem-w");
const searchEl = document.getElementById("search");

{
    const sampleKey = new URLSearchParams(location.search).get("sample");
    sourceEl.value = (sampleKey && SAMPLES[sampleKey]) || SAMPLES.foaf;
}

const HUD_MODES = ["is-min", "is-compact", "is-inspect"];
let hudMode = "is-min";

function setHud(mode) {
    hudMode = mode;
    HUD_MODES.forEach((cls) => theatre.classList.toggle(cls, cls === mode));
}

function cycleHud() {
    const order = ["is-min", "is-compact", "is-inspect"];
    setHud(order[(order.indexOf(hudMode) + 1) % order.length]);
}

document.getElementById("hud-glyph").addEventListener("click", () => setHud("is-compact"));
document.getElementById("hud-expand").addEventListener("click", () => setHud("is-inspect"));
document.getElementById("hud-min-a").addEventListener("click", () => setHud("is-min"));
document.getElementById("hud-min-b").addEventListener("click", () => setHud("is-min"));
window.addEventListener("keydown", (e) => {
    if (e.target && (e.target.tagName === "TEXTAREA" || e.target.tagName === "INPUT")) {
        if (e.key === "Escape") {
            e.target.blur();
            setHud("is-min");
        }
        return;
    }
    if (e.key === "h" || e.key === "H") cycleHud();
    if (e.key === "Escape") setHud("is-min");
});

const isMobile = window.matchMedia("(pointer: coarse)").matches
    || /Mobi|Android|iPhone|iPad/i.test(navigator.userAgent);
const forceBackend = new URLSearchParams(location.search).get("backend");

function failClosed(message) {
    failEl.classList.add("show");
    failEl.querySelector("div").textContent = message;
    statusEl.textContent = message;
}

function setStatus(text) {
    statusEl.textContent = text;
}

let portal = null;
let labels = [];
let wAngle = 0;
let vibeSource = "";
const labelField = document.getElementById("label-field");

function labelText(l) {
    return (l.name || l.id || "").toString();
}

function visibleLabels() {
    return labels.filter((l) => l.kind !== "plane" && l.kind !== "predicate");
}

function applyQueryLight(filter) {
    if (!portal || typeof portal.select_holograph_index !== "function") return;
    const q = (filter || "").trim().toLowerCase();
    if (!q) {
        portal.select_holograph_index(-1);
        return;
    }
    const hit = labels.findIndex((l) => {
        if (l.kind === "plane") return false;
        const id = String(l.id || "").toLowerCase();
        const name = String(l.name || "").toLowerCase();
        const kind = String(l.kind || "").toLowerCase();
        return kind.includes(q) || id.includes(q) || name.includes(q);
    });
    portal.select_holograph_index(hit);
}

function renderInspector(filter) {
    const q = (filter || "").trim().toLowerCase();
    const rows = visibleLabels().filter((l) => {
        if (!q) return true;
        const id = String(l.id || "");
        const name = String(l.name || "");
        const kind = String(l.kind || "");
        return kind.toLowerCase().includes(q)
            || id.toLowerCase().includes(q)
            || name.toLowerCase().includes(q);
    });
    inspectorEl.textContent = rows.slice(0, 12).map((l) => `${l.kind} ${labelText(l)}`).join("\n")
        || (q ? "no match — graph recedes" : "");
    applyQueryLight(q);
}

function paintOverlays() {
    if (!labelField || !portal || typeof portal.holograph_label_overlays !== "function") return;
    const chips = portal.holograph_label_overlays(canvas.width, canvas.height) || [];
    const cssW = Math.max(canvas.clientWidth || canvas.width, 1);
    const cssH = Math.max(canvas.clientHeight || canvas.height, 1);
    const sx = cssW / Math.max(canvas.width, 1);
    const sy = cssH / Math.max(canvas.height, 1);
    while (labelField.childElementCount > chips.length) {
        labelField.removeChild(labelField.lastChild);
    }
    for (let i = 0; i < chips.length; i += 1) {
        let el = labelField.children[i];
        if (!el) {
            el = document.createElement("div");
            el.className = "label-chip";
            labelField.appendChild(el);
        }
        const kind = chips[i].kind || "instance";
        el.className = `label-chip is-${kind}`;
        el.textContent = chips[i].name || "";
        el.style.left = `${chips[i].x * sx}px`;
        el.style.top = `${chips[i].y * sy}px`;
    }
}

function projectGraph() {
    if (!portal) {
        setStatus("portal not armed");
        return;
    }
    if (typeof portal.load_rdf_graph_text !== "function"
        && typeof portal.load_rdf_graph_vibe !== "function"
        && typeof portal.load_rdf_graph !== "function") {
        failClosed("This QualiaPortal WASM build has no load_rdf_graph. Rebuild docs/pkg/qualia.");
        return;
    }
    const payload = sourceEl.value;
    const contentType = formatEl.value;
    try {
        const receipt = typeof portal.load_rdf_graph_vibe === "function"
            ? portal.load_rdf_graph_vibe(payload, contentType, vibeSource)
            : (typeof portal.load_rdf_graph_text === "function"
                ? portal.load_rdf_graph_text(payload, contentType)
                : portal.load_rdf_graph(new TextEncoder().encode(payload), contentType));
        labels = receipt.labels || [];
        telemNodes.textContent = `nodes ${receipt.node_count}`;
        telemTriples.textContent = `triples ${receipt.triple_count}`;
        const layout = receipt.layout ? ` · ${receipt.layout}` : "";
        backendEl.textContent = `backend: ${receipt.backend}${receipt.truncated ? " · truncated" : ""}${layout}`;
        setStatus(`projected ${receipt.node_count} holographic entities`);
        renderInspector(searchEl.value);
        paintOverlays();
    } catch (err) {
        setStatus(String(err && err.message ? err.message : err));
    }
}

searchEl.addEventListener("input", () => {
    renderInspector(searchEl.value);
    if (searchEl.value.trim() && hudMode === "is-min") setHud("is-compact");
});

async function boot() {
    const probe = document.createElement("canvas");
    const webgl2Ok = !!probe.getContext("webgl2");
    if (isMobile && !webgl2Ok) {
        failClosed("Holographic theatre requires WebGL2");
        return;
    }
    ensureCanvasBackingStore(canvas);

    const loaded = await loadQualiaPortal(canvas, {
        allowWebGl2: true,
        anatomyBackend: forceBackend === "webgl2" || isMobile ? "webgl2" : undefined,
        requireBodyRenderer: true,
    });
    portal = loaded.portal;
    const renderer = loaded.renderer || "unknown";
    if (!portal) {
        failClosed("QualiaPortal failed to construct");
        return;
    }
    if (renderer !== "webgl2" && renderer !== "webgpu") {
        failClosed("Holographic theatre requires WebGL2");
        return;
    }
    backendEl.textContent = `backend: ${renderer}`;
    try {
        const vibeResp = await fetch(new URL("../vibe/cinema-graph.vibe", import.meta.url), { cache: "no-store" });
        if (vibeResp.ok) vibeSource = await vibeResp.text();
    } catch (_err) {
        vibeSource = "";
    }
    startPortalLoop(loaded.canvas || canvas, () => {
        wAngle = (wAngle + 0.00055) % (Math.PI * 2);
        telemW.textContent = `W ${wAngle.toFixed(2)}`;
        paintOverlays();
    });
    projectGraph();
}

let pointerDown = null;
canvas.addEventListener("pointerdown", (e) => {
    pointerDown = { x: e.clientX, y: e.clientY };
});
canvas.addEventListener("pointerup", (e) => {
    const start = pointerDown;
    pointerDown = null;
    if (!start || !portal || typeof portal.pick_holograph_at !== "function") return;
    const dx = e.clientX - start.x;
    const dy = e.clientY - start.y;
    if (dx * dx + dy * dy > 36) return;
    const r = canvas.getBoundingClientRect();
    const x = (e.clientX - r.left) * (canvas.width / Math.max(r.width, 1));
    const y = (e.clientY - r.top) * (canvas.height / Math.max(r.height, 1));
    const idx = portal.pick_holograph_at(x, y, canvas.width, canvas.height);
    if (idx >= 0 && hudMode === "is-min") setHud("is-compact");
    renderInspector(searchEl.value);
});

document.getElementById("load-btn").addEventListener("click", projectGraph);
document.querySelectorAll("[data-sample]").forEach((btn) => {
    btn.addEventListener("click", () => {
        sourceEl.value = SAMPLES[btn.dataset.sample] || SAMPLES.foaf;
        projectGraph();
    });
});

["dragenter", "dragover"].forEach((ev) => {
    theatreListener(ev, (e) => {
        e.preventDefault();
        dropVeil.classList.add("active");
    });
});
["dragleave", "drop"].forEach((ev) => {
    theatreListener(ev, (e) => {
        e.preventDefault();
        dropVeil.classList.remove("active");
    });
});
theatreListener("drop", async (e) => {
    const file = e.dataTransfer?.files?.[0];
    if (!file) return;
    const text = await file.text();
    sourceEl.value = text;
    const name = file.name.toLowerCase();
    if (name.endsWith(".nt")) formatEl.value = "application/n-triples";
    else if (name.endsWith(".nq")) formatEl.value = "application/n-quads";
    else if (name.endsWith(".jsonld") || name.endsWith(".json")) formatEl.value = "application/ld+json";
    else formatEl.value = "text/turtle";
    setHud("is-inspect");
    projectGraph();
});

function theatreListener(type, handler) {
    document.getElementById("theatre").addEventListener(type, handler);
}

boot().catch((err) => failClosed(String(err && err.message ? err.message : err)));
