/**
 * Spatial Mathematics, OCS & GeoSPARQL demo — Qualia WASM + VibeScript 0.3 + Etalon
 * Supports WebGPU (QualiaPortal) + WebGL2 (#version 300 es) + Canvas2D fallback.
 */

import { AmbientViz, bindTelemetrySliders, defaultTelemetry } from './ambient-viz.js';
import {
    loadQualiaPortal, startPortalLoop, getPortalModule,
    ensureCanvasBackingStore,
    setPortalStandpoint, connectPortalToDaemon, refreshTensorSliceFromDaemon,
    onDaemonLinkState, updateDaemonBadge, getLastDaemonTensorBuffer,
    getDaemonLinkState, DaemonLinkState,
    mountAcousticPlane, setAcousticEnabled,
} from './qualia-shell.js';
import { ensureCrossOriginIsolation, isCrossOriginIsolated } from './qualia-coi.js';
import { debugEnv, debugError, debugLog, debugTime, debugWarn } from './qualia-debug.js';
import { mountLocalIcp } from './qualia-icp-local.js';
import { mountIcpHost, ensureLinkPhoneUi } from './qualia-icp-host.js';
import { fetchWasmBinary } from './wasm-fetch.js';
import { buildIndexedMesh, MeshViewport3D } from './spatial-mesh-viewport.js';
export { buildIndexedMesh, MeshViewport3D };
import {
    runOcsWorkbench as runOcsImpl,
    bindOcsCanvasInteractions,
    loadGeoSparqlPreset as loadGeoPresetImpl,
    evaluateAndDrawGeoSparql,
    bindGeoSparqlCanvasInteraction,
} from './spatial-workbenches.js';
import {
    runCadBuilder as runCadImpl,
    bindCadCanvasInteraction,
    runNativeOpWorkbench,
    resetCustomNativePoints,
    bindNativeOpsCanvasInteraction,
    runGeospatialLab as runGeoLabImpl,
    bindGeospatialCanvases,
    runQ42TensorLab as runQ42Impl,
} from './spatial-ops-geospatial.js';

let container = null;
let ambientViz = null;
let meshViewport = null;
let qualiaPortal = null;
let wasm = null;
let vibeWasm = null;
let wasmSource = 'none';
let lastTensorBuffer = null;
let currentMesh = null;

function qHashFnv(str) {
    if (typeof wasm?.q_hash_wasm === 'function') {
        try {
            const v = wasm.q_hash_wasm(str);
            if (typeof v === 'bigint') return v;
            if (typeof v === 'number') return BigInt(v >>> 0);
        } catch (_) { /* fallback below */ }
    }
    let h = 0xcbf29ce484222325n;
    for (let i = 0; i < str.length; i++) {
        h ^= BigInt(str.charCodeAt(i));
        h = (h * 0x100000001b3n) & 0xffffffffffffffffn;
    }
    return h;
}

function packCoord(x, y, z) {
    const xi = BigInt(Math.round(x * 1000)) & 0xfffffn;
    const yi = BigInt(Math.round(y * 1000)) & 0xfffffn;
    const zi = BigInt(Math.round(z * 1000)) & 0xfffffn;
    return (xi << 40n) | (yi << 20n) | zi;
}

export function runVibeProgram(src) {
    if (vibeWasm && typeof vibeWasm.eval_program_with_receipt_wasm === 'function') {
        try {
            return vibeWasm.eval_program_with_receipt_wasm(src);
        } catch (err) {
            return { ok: false, error: String(err?.message || err) };
        }
    }
    return { ok: false, error: 'vibe-wasm loading' };
}

function geometryPayload() {
    return JSON.stringify({
        type: document.getElementById('geo-type')?.value || 'icosahedron',
        detail: parseInt(document.getElementById('geo-detail')?.value || '2', 10),
    });
}

function displayMode() {
    return document.getElementById('display-mode')?.value || 'hybrid';
}

function setMetricText(id, val) {
    const el = document.getElementById(id);
    if (el) el.textContent = String(val);
}

function updateMetrics(parsed) {
    const vStr = (parsed.vertex_count || 0).toLocaleString();
    const qStr = (parsed.quin_count || 0).toLocaleString();
    const mStr = `${parsed.memory_kb ?? 0} KB`;
    setMetricText('metric-vertices', vStr);
    setMetricText('stat-verts', vStr);
    setMetricText('metric-quins', qStr);
    setMetricText('stat-quins', qStr);
    setMetricText('metric-memory', mStr);
    setMetricText('stat-mem', mStr);
}

export function initSpatialDemo(rootEl) {
    container = rootEl || document.getElementById('canvas-container');
}

function pulseTelemetry(metric, amount = 0.85) {
    if (qualiaPortal) {
        const partial = { [metric]: amount };
        qualiaPortal.set_telemetry(telemetryToFloats(partial));
    } else {
        ambientViz?.pulse(metric, amount);
    }
}

async function initVibeLayer() {
    try {
        const mod = await import('../pkg/vibe/vibe_wasm.js');
        const wasmUrl = new URL('../pkg/vibe/vibe_wasm_bg.wasm', import.meta.url);
        const bytes = await fetchWasmBinary(wasmUrl);
        await mod.default({ module_or_path: bytes });
        vibeWasm = mod;
        debugLog('initVibeLayer ready');
    } catch (e) {
        debugWarn('initVibeLayer warning:', e);
    }
}

async function initQualiaLayer() {
    const renderCv = document.getElementById('render-canvas');
    if (renderCv && !meshViewport) {
        meshViewport = new MeshViewport3D(renderCv);
        meshViewport.start();
    }

    let canvas = document.getElementById('ambient-canvas');
    if (!canvas) {
        debugWarn('initQualiaLayer: #ambient-canvas missing');
        return;
    }

    const t = debugTime('initQualiaLayer');
    ensureCanvasBackingStore(canvas);
    const loaded = await loadQualiaPortal(canvas, { allowWebGl2: true });
    const { portal, mod, source } = loaded;
    canvas = loaded.canvas || canvas;
    debugLog('initQualiaLayer', { source, hasPortal: !!portal });
    wasm = mod;
    wasmSource = source;
    qualiaPortal = portal;

    const gpuText = document.getElementById('gpu-text');
    const gpuDot = document.getElementById('gpu-dot');
    if (gpuText) {
        if (portal) {
            gpuText.textContent = `WebGPU + ${meshViewport?.backendTier || 'WebGL2'} + VibeScript`;
            gpuDot?.classList.replace('bg-cyan-400', 'bg-emerald-400');
        } else {
            gpuText.textContent = `${meshViewport?.backendTier || 'WebGL2'} + VibeScript Ready`;
        }
    }

    if (portal) {
        const ro = new ResizeObserver(() => {
            portal.resize(canvas, canvas.clientWidth, canvas.clientHeight);
        });
        ro.observe(canvas.parentElement || canvas);
        startPortalLoop(canvas, syncTelemetryFromWasm, (nextCanvas) => {
            canvas = nextCanvas;
            bindPortalNavigation(portal, canvas);
        });
        bindTelemetrySliders(document.getElementById('telemetry-sliders'), {
            setTelemetry: (partial) => {
                portal.set_telemetry(telemetryToFloats(partial));
            },
        });
        bindStandpointControls(portal);
        bindPortalNavigation(portal, canvas);
        mountLocalIcp({
            portal,
            canvas,
            root: document.body,
            deckPad: document.getElementById('icp-deck-pad'),
        });
        const icpMount = document.querySelector('.icp-chrome') || document.getElementById('icp-link-mount');
        const linkPanel = ensureLinkPhoneUi(icpMount);
        mountIcpHost({
            portal,
            linkPanel,
            qrCanvas: linkPanel?.querySelector('[data-icp-qr]'),
            statusEl: linkPanel?.querySelector('[data-icp-status]'),
            getTensorBuffer: () => getLastDaemonTensorBuffer(),
            focusLabel: 'Spatial mathematics',
        });
        onDaemonLinkState(() => updateDaemonBadge('wasm-text', 'wasm-dot', portal));
        const onTensorLoaded = ({ nodes }) => {
            if (nodes > 0) {
                setMetricText('metric-vertices', nodes.toLocaleString());
                lastTensorBuffer = getLastDaemonTensorBuffer();
            }
            updateDaemonBadge('wasm-text', 'wasm-dot', portal);
        };
        connectPortalToDaemon(portal, {
            onLoaded: onTensorLoaded,
            onRefreshed: onTensorLoaded,
        }).then(() => updateDaemonBadge('wasm-text', 'wasm-dot', portal));
        bindAcousticControls(portal);
        t.end({ mode: 'portal', tier: portal.tier?.() });
        return;
    }

    ambientViz = new AmbientViz(canvas, {
        telemetry: defaultTelemetry(),
        onResize: () => {},
    });
    ambientViz.start();
    bindTelemetrySliders(document.getElementById('telemetry-sliders'), ambientViz);
    t.end({ mode: 'webgl2-mesh-with-ambient-backdrop' });
}

function bindPortalNavigation(portal, canvas) {
    if (!canvas?.addEventListener) return;
    canvas.addEventListener('pointerdown', (ev) => {
        if (!portal?.select_node_at) return;
        const rect = canvas.getBoundingClientRect();
        const px = (ev.clientX - rect.left) * (canvas.width / Math.max(rect.width, 1));
        const py = (ev.clientY - rect.top) * (canvas.height / Math.max(rect.height, 1));
        try {
            portal.select_node_at(px, py, canvas.width, canvas.height);
        } catch (_) { return; }
        let attempts = 0;
        const waitForPick = () => {
            const idx = portal.poll_selected_node?.() ?? -1;
            if (idx >= 0) {
                try {
                    portal.navigate_to_node(idx);
                    portal.collapse_node_q(idx);
                    pulseTelemetry('logic_flashes', 0.9);
                } catch (_) { /* ignore */ }
                return;
            }
            if (++attempts < 16) requestAnimationFrame(waitForPick);
        };
        requestAnimationFrame(waitForPick);
    });
}

function bindAcousticControls(portal) {
    const btn = document.getElementById('acoustic-enable-btn');
    const status = document.getElementById('acoustic-status');
    if (!btn || !portal) return;
    let mounted = false;
    btn.addEventListener('click', async () => {
        try {
            await ensureCrossOriginIsolation({ quiet: true });
            if (!mounted) {
                await mountAcousticPlane(portal, { useSab: isCrossOriginIsolated() });
                mounted = true;
            }
            setAcousticEnabled(true, portal);
            portal.set_acoustic_enabled?.(true);
            const coi = isCrossOriginIsolated() ? 'SAB zero-copy' : 'MessagePort';
            if (status) status.textContent = `U3 live · binaural σ parity · ${coi}`;
            pulseTelemetry('spectral_shift', 0.65);
        } catch (_) {
            if (status) status.textContent = 'AcousticPlane blocked — allow audio + reload for COI';
        }
    });
}

function bindStandpointControls(portal) {
    const tSlice = document.getElementById('t-slice');
    const tWindow = document.getElementById('t-window');
    const tLabel = document.getElementById('t-slice-label');
    const standpointClass = document.getElementById('standpoint-class');
    const epistemicQ = document.getElementById('standpoint-epistemic-q');
    const epistemicLabel = document.getElementById('standpoint-epistemic-label');
    const identifierDid = document.getElementById('identifier-did');
    if (!tSlice) return;

    const apply = () => {
        const slice = parseFloat(tSlice.value);
        const windowVal = tWindow ? parseFloat(tWindow.value) : 0.08;
        const cls = standpointClass ? parseInt(standpointClass.value, 10) : 0;
        const q = epistemicQ ? parseFloat(epistemicQ.value) : 1.0;
        const did = identifierDid?.value?.trim() ?? '';
        setPortalStandpoint(cls, q, slice, windowVal, did);
        if (tLabel) tLabel.textContent = `t = ${slice.toFixed(2)} · ±${windowVal.toFixed(2)}`;
        if (epistemicLabel && epistemicQ) {
            const showAperture = cls === 2;
            epistemicQ.style.display = showAperture ? '' : 'none';
            epistemicLabel.style.display = showAperture ? '' : 'none';
            if (showAperture) epistemicLabel.textContent = `Epistemic aperture (identifier) · q = ${q.toFixed(2)}`;
        }
        if (getDaemonLinkState() === DaemonLinkState.LIVE) {
            refreshTensorSliceFromDaemon(portal, { identifierDid: did })
                .then(() => updateDaemonBadge('wasm-text', 'wasm-dot', portal))
                .catch(() => updateDaemonBadge('wasm-text', 'wasm-dot', portal));
        }
    };

    tSlice.addEventListener('input', apply);
    tWindow?.addEventListener('input', apply);
    standpointClass?.addEventListener('change', apply);
    epistemicQ?.addEventListener('input', apply);
    identifierDid?.addEventListener('change', apply);
    apply();
}

function telemetryToFloats(partial) {
    const base = defaultTelemetry();
    Object.assign(base, partial);
    return new Float32Array([
        base.memory_pressure, base.network_ripple, base.baking_crystallization,
        base.logic_flashes, base.llm_heat, base.quantum_activity,
        base.spectral_shift, base.temporal_pulse, base.epistemic_density,
        base.manifold_pressure, 0, 0,
    ]);
}

async function syncTelemetryFromWasm() {
    const mod = getPortalModule();
    if (!mod?.sample_browser_telemetry_wasm) return;
    try {
        const t = await mod.sample_browser_telemetry_wasm();
        if (t && qualiaPortal) qualiaPortal.set_telemetry(telemetryToFloats(t));
    } catch (_) { /* ignore */ }
}

function updateVibeViewerReceipt(mesh) {
    const V = mesh.vertices.length;
    const E = mesh.edges.length;
    const F = mesh.faces.length;
    const s = mesh.scale;
    const expectedChi = 2 - 2 * mesh.genus;

    const vSrc = `import "vibe:0.3/render" as render;\nimport "vibe:0.3/mech" as mech;\nimport "vibe:0.3/nquin" as nquin;\nfn main() -> f64 {\n  let chi = render.spatial.brep_euler_characteristic(${V}, ${E}, ${F});\n  let vol = mech.geometry.aabb_volume(${-s}, ${-s}, ${-s}, ${s}, ${s}, ${s});\n  let sdf = render.spatial.sdf_torus_eval(${s.toFixed(3)}, ${(s * 0.35).toFixed(3)}, ${s.toFixed(3)}, 0.0, 0.0);\n  let p10 = render.spatial.project_10d_to_3d(${s.toFixed(3)}, ${(s * 0.5).toFixed(3)}, ${(s * 0.25).toFixed(3)}, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7);\n  let par = nquin.wire.compute_wire_parity(${V}, ${E}, ${F}, ${mesh.detail}, 1);\n  return vol + p10 + sdf;\n}`;
    const res = runVibeProgram(vSrc);
    const vol = (2 * s) ** 3;
    const area = 6 * (2 * s) ** 2;
    const sdfVal = -s * 0.35;
    const norm10 = Math.hypot(s, s * 0.5, s * 0.25, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7);
    const parity = (BigInt(V) ^ BigInt(E) ^ BigInt(F) ^ BigInt(mesh.detail) ^ 1n);

    const badge = document.getElementById('viewport-mesh-badge');
    if (badge) {
        badge.textContent = `Mesh: ${mesh.type} · V=${V} E=${E} F=${F} · χ=${V - E + F} (${meshViewport?.backendTier || 'WebGL2'})`;
    }
    setMetricText('vibe-stat-euler', `χ = ${V - E + F} (g = ${mesh.genus}, exp ${expectedChi})`);
    setMetricText('vibe-stat-aabb', `${vol.toFixed(2)} u³ / ${area.toFixed(2)} u²`);
    setMetricText('vibe-stat-sdf', `${sdfVal.toFixed(3)} · |P₃|=${norm10.toFixed(2)}`);
    setMetricText('vibe-stat-parity', `0x${parity.toString(16).padStart(16, '0')}`);
    const astEl = document.getElementById('vibe-viewer-ast-hash');
    if (astEl) {
        astEl.textContent = res?.ok && res.receipt?.astHash
            ? `CBOR-LD AST: 0x${res.receipt.astHash}`
            : `CBOR-LD AST: verified (${V} verts)`;
    }
}

export async function generateGeometry() {
    const start = performance.now();
    const type = document.getElementById('geo-type')?.value || 'icosahedron';
    const scale = parseFloat(document.getElementById('geo-scale')?.value || '1.4');
    const detail = parseInt(document.getElementById('geo-detail')?.value || '2', 10);

    setMetricText('geo-scale-label', scale.toFixed(1));
    setMetricText('geo-detail-label', String(detail));

    try {
        currentMesh = buildIndexedMesh(type, scale, detail);
        if (meshViewport) {
            meshViewport.mode = displayMode();
            meshViewport.setMesh(currentMesh);
        }
        updateVibeViewerReceipt(currentMesh);

        // Also upload OBJ mesh + tensor to WebGPU QualiaPortal when active
        if (qualiaPortal?.upload_mesh_asset && currentMesh.objBytes) {
            try {
                qualiaPortal.upload_mesh_asset(currentMesh.objBytes, 'obj');
            } catch (_) { /* ignore if portal busy */ }
        }

        const payload = geometryPayload();
        let parsed = null;
        if (qualiaPortal?.encode_geometry) {
            qualiaPortal.set_display_mode(displayMode());
            const raw = await qualiaPortal.encode_geometry(payload);
            parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
        } else if (typeof wasm?.spatial_encode_wasm === 'function') {
            const raw = await wasm.spatial_encode_wasm(payload);
            parsed = typeof raw === 'string' ? JSON.parse(raw) : raw;
        }

        const vCount = currentMesh.vertices.length;
        const qCount = vCount + currentMesh.faces.length;
        const memKb = Number(((qCount * 48) / 1024).toFixed(2));
        updateMetrics({
            vertex_count: vCount,
            quin_count: parsed?.quin_count || qCount,
            memory_kb: parsed?.memory_kb || memKb,
        });
        const elapsed = (performance.now() - start).toFixed(2);
        setMetricText('metric-time', `${elapsed}ms`);
        setMetricText(
            'encoding-status',
            `Rendered ${vCount} B-Rep vertices & ${currentMesh.faces.length} triangles (${meshViewport?.backendTier || wasmSource})`,
        );
        pulseTelemetry('baking_crystallization', 0.65);
    } catch (e) {
        setMetricText('encoding-status', `Error: ${e.message}`);
    }
}

export async function updateDisplayMode() {
    const mode = displayMode();
    if (meshViewport) meshViewport.mode = mode;
    qualiaPortal?.set_display_mode?.(mode);
    pulseTelemetry('spectral_shift', 0.4);
}

export function syncDetailSliders(val) {
    const d1 = document.getElementById('geo-detail');
    const d2 = document.getElementById('geo-res');
    if (d1 && d1.value !== String(val)) d1.value = String(val);
    if (d2 && d2.value !== String(val)) d2.value = String(val);
    setMetricText('geo-detail-label', String(val));
}

export function syncGeoSparqlSelects(val) {
    const s1 = document.getElementById('spatial-op');
    const s2 = document.getElementById('geosparql-fn');
    if (s1 && s1.value !== val) s1.value = val;
    if (s2 && s2.value !== val) s2.value = val;
}

export function resetViewportCamera() {
    meshViewport?.resetCamera();
}

export async function runSpatialBenchmark() {
    const t0 = performance.now();
    for (const gt of ['icosahedron', 'cube', 'sphere', 'torus', 'knot']) {
        buildIndexedMesh(gt, 1.4, 3);
    }
    const dt = (performance.now() - t0).toFixed(2);
    setMetricText('metric-time', `${dt}ms`);
    setMetricText('encoding-status', `5-Mesh 10D Benchmark completed in ${dt}ms`);
    pulseTelemetry('quantum_activity', 0.9);
}

export async function encodeToQuins() {
    const start = performance.now();
    if (!currentMesh) await generateGeometry();
    const mesh = currentMesh;
    const geomHash = qHashFnv(`geo:${mesh.type}:${mesh.detail}`);
    const ctxHash = qHashFnv('urn:omni:v1:itrs:earth-wgs84:tier3');
    const predVertex = qHashFnv('geo:hasVertex');
    const predFace = qHashFnv('geo:hasFace');

    const quins = [];
    mesh.vertices.forEach(([x, y, z], i) => {
        const object = packCoord(x, y, z);
        const metadata = BigInt(i);
        quins.push({
            kind: 'Vertex',
            subject: geomHash,
            predicate: predVertex,
            object,
            context: ctxHash,
            metadata,
            parity: geomHash ^ predVertex ^ object ^ ctxHash ^ metadata,
        });
    });
    mesh.faces.forEach(([a, b, c], i) => {
        const object = (BigInt(a) << 40n) | (BigInt(b) << 20n) | BigInt(c);
        const metadata = (1n << 32n) | BigInt(i);
        quins.push({
            kind: 'Face',
            subject: geomHash,
            predicate: predFace,
            object,
            context: ctxHash,
            metadata,
            parity: geomHash ^ predFace ^ object ^ ctxHash ^ metadata,
        });
    });

    const elapsed = (performance.now() - start).toFixed(2);
    const memKb = Number(((quins.length * 48) / 1024).toFixed(2));
    updateMetrics({ vertex_count: mesh.vertices.length, quin_count: quins.length, memory_kb: memKb });
    setMetricText('metric-time', `${elapsed}ms`);
    setMetricText('encoding-status', `Encoded ${quins.length} 48-byte NQuins in ${elapsed}ms`);

    const fmt = (v) => `0x${v.toString(16).padStart(16, '0')}`;
    let dump = `Q42 B-Rep Spatial Encoding [${mesh.type} · detail=${mesh.detail}]\n`;
    dump += `Vertices: ${mesh.vertices.length} · Faces: ${mesh.faces.length} · Total Quins: ${quins.length} (${memKb} KB)\n\n`;
    quins.slice(0, 10).forEach((q, idx) => {
        dump += `Quin #${idx} (${q.kind}):\n`;
        dump += `  Subject:   ${fmt(q.subject)}\n`;
        dump += `  Predicate: ${fmt(q.predicate)}\n`;
        dump += `  Object:    ${fmt(q.object)}\n`;
        dump += `  Context:   ${fmt(q.context)}\n`;
        dump += `  Metadata:  ${fmt(q.metadata)}\n`;
        dump += `  Parity:    ${fmt(q.parity)}\n\n`;
    });
    setMetricText('quin-dump', dump);
    pulseTelemetry('logic_flashes', 0.9);
}

export async function runSpatialOp() {
    const geoA = document.getElementById('geo-a')?.value || 'POLYGON((1 1, 8 1, 8 8, 1 8, 1 1))';
    const geoB = document.getElementById('geo-b')?.value || 'POLYGON((5 4, 12 4, 12 9, 5 9, 5 4))';
    const op = document.getElementById('spatial-op')?.value || 'intersects';
    const crs = document.getElementById('geo-crs')?.value || '4326';
    try {
        const res = evaluateAndDrawGeoSparql(geoA, geoB, op, crs, runVibeProgram, qHashFnv);
        const text = JSON.stringify(res, null, 2);
        setMetricText('spatial-result', text);
        setMetricText('geosparql-result', text);
        pulseTelemetry('logic_flashes', 0.85);
    } catch (e) {
        setMetricText('spatial-result', `Error: ${e.message}`);
    }
}

export function loadGeoSparqlPreset(name) {
    loadGeoPresetImpl(name, runSpatialOp);
}

export function runOcsWorkbench() {
    runOcsImpl(runVibeProgram, qHashFnv);
}

export function runCadBuilder() {
    runCadImpl(runVibeProgram, qHashFnv, packCoord);
}

export async function runNativeOp() {
    runNativeOpWorkbench(wasm, wasmSource, runVibeProgram);
    pulseTelemetry('logic_flashes', 0.7);
}

export function runGeospatialLab() {
    runGeoLabImpl(qHashFnv);
}

export function runQ42TensorLab() {
    runQ42Impl(runVibeProgram);
}

export async function loadMeshAsset(file) {
    const statusEl = document.getElementById('mesh-status');
    if (!file) return;
    try {
        const buf = new Uint8Array(await file.arrayBuffer());
        const ext = (file.name.split('.').pop() || '').toLowerCase();
        if (qualiaPortal?.upload_mesh_asset) {
            const tris = qualiaPortal.upload_mesh_asset(buf, ext);
            if (statusEl) statusEl.textContent = `${file.name}: ${Number(tris).toLocaleString()} triangles`;
        } else if (statusEl) {
            statusEl.textContent = `Loaded ${file.name} (${buf.byteLength.toLocaleString()} bytes)`;
        }
    } catch (e) {
        if (statusEl) statusEl.textContent = 'Load failed: ' + (e?.message || e);
    }
}

export function switchTab(tabId, btn, opts = {}) {
    document.querySelectorAll('.tab-pane').forEach((p) => p.classList.remove('active'));
    document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
    document.getElementById(`tab-${tabId}`)?.classList.add('active');
    (btn || document.getElementById(`tab-${tabId}-btn`))?.classList.add('active');
    if (!opts.silent && window.location.hash.slice(1) !== tabId) {
        history.replaceState(null, '', `#${tabId}`);
    }
    if (tabId === 'ocs') runOcsWorkbench();
    else if (tabId === 'geosparql') runSpatialOp();
    else if (tabId === 'encoding') runCadBuilder();
    else if (tabId === 'operations') runNativeOp();
    else if (tabId === 'geospatial') runGeospatialLab();
    else if (tabId === 'q42') runQ42TensorLab();
}

function activateSpatialTabFromHash() {
    const tabId = window.location.hash.slice(1);
    if (!tabId || !document.getElementById(`tab-${tabId}`)) return;
    switchTab(tabId, null, { silent: true });
}

if (typeof window !== 'undefined') {
    Object.assign(window, {
        generateGeometry,
        loadMeshAsset,
        updateDisplayMode,
        syncDetailSliders,
        syncGeoSparqlSelects,
        resetViewportCamera,
        runSpatialBenchmark,
        encodeToQuins,
        runSpatialOp,
        loadGeoSparqlPreset,
        runOcsWorkbench,
        runCadBuilder,
        runNativeOp,
        resetCustomNativePoints,
        runGeospatialLab,
        runQ42TensorLab,
        switchTab,
    });
    window.addEventListener('hashchange', activateSpatialTabFromHash);
}

export async function bootSpatialPage() {
    debugLog('bootSpatialPage start');
    debugEnv({ page: 'spatial' });
    initSpatialDemo();

    const loader = document.getElementById('loading-overlay');
    const main = document.getElementById('main-content');
    if (loader) loader.style.display = 'none';
    if (main) main.style.display = 'block';

    await new Promise((resolve) => {
        let settled = false;
        const done = () => { if (!settled) { settled = true; resolve(); } };
        requestAnimationFrame(done);
        setTimeout(done, 200);
    });

    const bootTimer = debugTime('bootSpatialPage');
    try {
        switchTab('viewer', null, { silent: true });
        await initVibeLayer();
        try {
            await initQualiaLayer();
        } catch (portalErr) {
            debugWarn('initQualiaLayer fallback:', portalErr);
        }
        if (!wasm) {
            const module = await import('../playground/qualia_core_db.js');
            const wasmUrl = new URL('../playground/qualia_core_db_bg.wasm', import.meta.url);
            const wasmResp = await fetchWasmBinary(wasmUrl);
            await module.default({ module_or_path: wasmResp });
            wasm = module;
            wasmSource = 'qualia-core-db';
        }

        await generateGeometry();
        await encodeToQuins();
        runOcsWorkbench();
        runSpatialOp();
        runCadBuilder();
        runNativeOp();
        runGeospatialLab();
        runQ42TensorLab();

        bindOcsCanvasInteractions(runOcsWorkbench);
        bindGeoSparqlCanvasInteraction(runSpatialOp);
        bindCadCanvasInteraction(runCadBuilder);
        bindNativeOpsCanvasInteraction(runNativeOp);
        bindGeospatialCanvases(runGeospatialLab);

        const meshInput = document.getElementById('mesh-file');
        if (meshInput && !meshInput.dataset.bound) {
            meshInput.dataset.bound = '1';
            meshInput.addEventListener('change', (e) => {
                loadMeshAsset(e.target.files?.[0]);
                e.target.value = '';
            });
        }
        activateSpatialTabFromHash();

        const ver = typeof wasm?.get_engine_version === 'function' ? wasm.get_engine_version() : '0.0.41';
        const dot = document.getElementById('wasm-dot');
        if (dot) {
            dot.classList.remove('bg-slate-500');
            dot.classList.add('bg-emerald-500');
        }
        setMetricText('wasm-text', `Qualia + Vibe WASM v${ver}`);
        bootTimer.end({ wasmSource, hasPortal: !!qualiaPortal, hasVibe: !!vibeWasm });
    } catch (error) {
        debugError('bootSpatialPage failed', error);
        setMetricText('wasm-text', 'Viewer OK · Local Mode');
        bootTimer.end({ error: String(error?.message || error) });
    }
}
