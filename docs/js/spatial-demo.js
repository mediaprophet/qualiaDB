/**
 * Spatial Mathematics demo — Qualia WASM portal (no Three.js).
 */

import { AmbientViz, bindTelemetrySliders, defaultTelemetry } from './ambient-viz.js';
import {
    loadQualiaPortal, startPortalLoop, stopPortalLoop, getPortal, getPortalModule,
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

const TENSOR_HEADER_BYTES = 32;
const TENSOR_STRIDE = 40;
const TENSOR_MAGIC = 0x5134_322a;

let container = null;
let ambientViz = null;
let qualiaPortal = null;
let wasm = null;
let wasmSource = 'none';
let lastTensorBuffer = null;
let lastVertexCount = 0;

function qHashFnv(str) {
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

function parsePointWkt(wkt) {
    const m = wkt.trim().match(/POINT\s*\(\s*([-\d.]+)\s+([-\d.]+)\s*\)/i);
    if (!m) throw new Error('Expected POINT(x y) WKT');
    return { x: parseFloat(m[1]), y: parseFloat(m[2]) };
}

function parsePolygonWkt(wkt) {
    const m = wkt.trim().match(/POLYGON\s*\(\(\s*([^)]+)\s*\)\)/i);
    if (!m) throw new Error('Expected POLYGON((...)) WKT');
    return m[1].split(',').map((pair) => {
        const [x, y] = pair.trim().split(/\s+/);
        return { x: parseFloat(x), y: parseFloat(y) };
    });
}

function pointInPolygon(pt, ring) {
    let inside = false;
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
        const xi = ring[i].x, yi = ring[i].y;
        const xj = ring[j].x, yj = ring[j].y;
        const intersect = ((yi > pt.y) !== (yj > pt.y))
            && (pt.x < ((xj - xi) * (pt.y - yi)) / (yj - yi + 1e-12) + xi);
        if (intersect) inside = !inside;
    }
    return inside;
}

function geometryPayload() {
    return JSON.stringify({
        type: document.getElementById('geo-type').value,
        detail: parseInt(document.getElementById('geo-detail').value, 10),
    });
}

function displayMode() {
    return document.getElementById('display-mode').value;
}

function parseTensorPositions(buffer) {
    if (!buffer || buffer.byteLength < TENSOR_HEADER_BYTES) return [];
    const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
    if (view.getUint32(0, true) !== TENSOR_MAGIC) return [];
    const nodeCount = view.getUint32(8, true);
    const stride = view.getUint32(12, true);
    if (stride !== TENSOR_STRIDE) return [];
    const pts = [];
    let offset = TENSOR_HEADER_BYTES;
    for (let i = 0; i < nodeCount; i++) {
        if (offset + stride > buffer.byteLength) break;
        pts.push({
            x: view.getFloat32(offset + 12, true),
            y: view.getFloat32(offset + 16, true),
            z: view.getFloat32(offset + 20, true),
        });
        offset += stride;
    }
    return pts;
}

function updateMetrics(parsed) {
    document.getElementById('metric-vertices').textContent = parsed.vertex_count.toLocaleString();
    document.getElementById('metric-quins').textContent = parsed.quin_count.toLocaleString();
    document.getElementById('metric-memory').textContent = `${parsed.memory_kb} KB`;
    lastVertexCount = parsed.vertex_count;
}

export function initSpatialDemo(rootEl) {
    container = rootEl || document.getElementById('canvas-container');
}

function containerSize() {
    const el = container;
    const w = el?.clientWidth || 800;
    const h = el?.clientHeight || 520;
    return { w: Math.max(w, 1), h: Math.max(h, 1) };
}

function pulseTelemetry(metric, amount = 0.85) {
    if (qualiaPortal) {
        const partial = { [metric]: amount };
        qualiaPortal.set_telemetry(telemetryToFloats(partial));
    } else {
        ambientViz?.pulse(metric, amount);
    }
}

async function initQualiaLayer() {
    const canvas = document.getElementById('ambient-canvas');
    if (!canvas) {
        debugWarn('initQualiaLayer: #ambient-canvas missing');
        return;
    }

    const t = debugTime('initQualiaLayer');
    ensureCanvasBackingStore(canvas);
    const { portal, mod, source } = await loadQualiaPortal(canvas);
    debugLog('initQualiaLayer', { source, hasPortal: !!portal });
    wasm = mod;
    wasmSource = source;
    qualiaPortal = portal;

    if (portal) {
        const ro = new ResizeObserver(() => {
            portal.resize(canvas, canvas.clientWidth, canvas.clientHeight);
        });
        ro.observe(canvas.parentElement || canvas);
        startPortalLoop(canvas, syncTelemetryFromWasm);
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
                const vertEl = document.getElementById('metric-vertices');
                if (vertEl) vertEl.textContent = nodes.toLocaleString();
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

    debugWarn('initQualiaLayer: canvas2d AmbientViz fallback (no QualiaPortal)');
    ambientViz = new AmbientViz(canvas, {
        telemetry: defaultTelemetry(),
        onResize: () => {},
    });
    ambientViz.start();
    bindTelemetrySliders(document.getElementById('telemetry-sliders'), ambientViz);
    t.end({ mode: 'ambient-viz-fallback' });
}

function bindPortalNavigation(portal, canvas) {
    if (!canvas?.addEventListener) return;
    canvas.addEventListener('pointerdown', (ev) => {
        if (!portal?.select_node_at) return;
        const rect = canvas.getBoundingClientRect();
        const x = ev.clientX - rect.left;
        const y = ev.clientY - rect.top;
        const scaleX = canvas.width / Math.max(rect.width, 1);
        const scaleY = canvas.height / Math.max(rect.height, 1);
        const px = x * scaleX;
        const py = y * scaleY;
        try {
            portal.select_node_at(px, py, canvas.width, canvas.height);
        } catch (e) {
            console.warn('select_node_at', e);
            return;
        }
        // GPU pick completes after the next portal tick + readback (1–2 frames).
        let attempts = 0;
        const waitForPick = () => {
            const idx = portal.poll_selected_node?.() ?? -1;
            if (idx >= 0) {
                try {
                    portal.navigate_to_node(idx);
                    portal.collapse_node_q(idx);
                    pulseTelemetry('logic_flashes', 0.9);
                    const selEl = document.getElementById('metric-selected');
                    if (selEl) selEl.textContent = String(idx);
                } catch (e) {
                    console.warn('navigation', e);
                }
                return;
            }
            if (++attempts < 16) {
                requestAnimationFrame(waitForPick);
            }
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
            if (status) {
                status.textContent = `U3 live · binaural σ parity · ${coi}`;
            }
            pulseTelemetry('spectral_shift', 0.65);
        } catch (e) {
            console.warn('acoustic', e);
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
        const window = tWindow ? parseFloat(tWindow.value) : 0.08;
        const cls = standpointClass ? parseInt(standpointClass.value, 10) : 0;
        const q = epistemicQ ? parseFloat(epistemicQ.value) : 1.0;
        const did = identifierDid?.value?.trim() ?? '';
        setPortalStandpoint(cls, q, slice, window, did);
        if (tLabel) {
            tLabel.textContent = `t = ${slice.toFixed(2)} · ±${window.toFixed(2)}`;
        }
        if (epistemicLabel && epistemicQ) {
            const showAperture = cls === 2;
            epistemicQ.style.display = showAperture ? '' : 'none';
            epistemicLabel.style.display = showAperture ? '' : 'none';
            if (showAperture) {
                epistemicLabel.textContent = `Epistemic aperture (identifier) · q = ${q.toFixed(2)}`;
            }
        }
        if (getDaemonLinkState() === DaemonLinkState.LIVE) {
            refreshTensorSliceFromDaemon(portal, {
                identifierDid: did,
            }).then(({ nodes }) => {
                if (nodes > 0) {
                    const vertEl = document.getElementById('metric-vertices');
                    if (vertEl) vertEl.textContent = nodes.toLocaleString();
                    lastTensorBuffer = getLastDaemonTensorBuffer();
                }
                updateDaemonBadge('wasm-text', 'wasm-dot', portal);
            }).catch(() => updateDaemonBadge('wasm-text', 'wasm-dot', portal));
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
        if (t && qualiaPortal) {
            qualiaPortal.set_telemetry(telemetryToFloats(t));
        }
    } catch (_) { /* ignore */ }
}

async function loadGeometryIntoPortal() {
    const payload = geometryPayload();
    const mode = displayMode();

    if (qualiaPortal?.encode_geometry) {
        qualiaPortal.set_display_mode(mode);
        const parsed = await qualiaPortal.encode_geometry(payload);
        const value = typeof parsed === 'string' ? JSON.parse(parsed) : parsed;
        updateMetrics(value);
        if (wasm?.export_tensor_buffer_wasm) {
            const buf = await wasm.export_tensor_buffer_wasm(payload);
            lastTensorBuffer = buf;
        }
        pulseTelemetry('baking_crystallization', 0.6);
        return value;
    }

    if (typeof wasm?.spatial_encode_wasm === 'function') {
        const parsed = await wasm.spatial_encode_wasm(payload);
        const value = typeof parsed === 'string' ? JSON.parse(parsed) : parsed;
        updateMetrics(value);
        if (wasm.export_tensor_buffer_wasm) {
            lastTensorBuffer = await wasm.export_tensor_buffer_wasm(payload);
        }
        pulseTelemetry('baking_crystallization', 0.5);
        return value;
    }

    const type = document.getElementById('geo-type').value;
    const detail = parseInt(document.getElementById('geo-detail').value, 10);
    const n = Math.min(12 + detail * 20, 8192);
    const quins = [];
    const geomHash = qHashFnv(`geo:${type}:${detail}`);
    const ctxHash = qHashFnv('ctx:spatial-demo');
    const predVertex = qHashFnv('geo:hasVertex');
    for (let i = 0; i < n; i++) {
        const t = i / n;
        const x = 5 * Math.cos(t * Math.PI * 6) * Math.sin(t * 1.618);
        const y = 5 * Math.sin(t * 2);
        const z = 5 * Math.sin(t * Math.PI * 6) * Math.cos(t * 1.618);
        const object = packCoord(x, y, z);
        quins.push({ subject: geomHash, predicate: predVertex, object, context: ctxHash, metadata: BigInt(i) });
    }
    const fallback = {
        vertex_count: n,
        quin_count: quins.length,
        memory_kb: Number(((quins.length * 48) / 1024).toFixed(2)),
        quins,
        backend: 'browser',
    };
    updateMetrics(fallback);
    return fallback;
}

export async function generateGeometry() {
    try {
        await loadGeometryIntoPortal();
        document.getElementById('encoding-status').textContent =
            `Geometry loaded via Qualia WASM (${wasmSource})`;
    } catch (e) {
        document.getElementById('encoding-status').textContent = `Error: ${e.message}`;
    }
}

export async function updateDisplayMode() {
    const mode = displayMode();
    qualiaPortal?.set_display_mode?.(mode);
    pulseTelemetry('spectral_shift', 0.4);
}

function encodeGeometryToQuins(type, detail) {
    const n = Math.min(12 + detail * 20, 8192);
    const geomHash = qHashFnv(`geo:${type}:${detail}`);
    const ctxHash = qHashFnv('ctx:spatial-demo');
    const predVertex = qHashFnv('geo:hasVertex');
    const quins = [];
    for (let i = 0; i < n; i++) {
        const t = i / n;
        const x = 5 * Math.cos(t * Math.PI * 6) * Math.sin(t * 1.618);
        const y = 5 * Math.sin(t * 2);
        const z = 5 * Math.sin(t * Math.PI * 6) * Math.cos(t * 1.618);
        const object = packCoord(x, y, z);
        quins.push({
            subject: geomHash,
            predicate: predVertex,
            object,
            context: ctxHash,
            metadata: BigInt(i),
            parity: geomHash ^ predVertex ^ object ^ ctxHash ^ BigInt(i),
        });
    }
    return {
        vertex_count: n,
        quin_count: quins.length,
        memory_kb: Number(((quins.length * 48) / 1024).toFixed(2)),
        quins,
        backend: 'browser',
    };
}

export async function encodeToQuins() {
    const type = document.getElementById('geo-type').value;
    const detail = parseInt(document.getElementById('geo-detail').value, 10);
    const start = performance.now();

    try {
        let parsed;
        const payload = JSON.stringify({ type, detail });
        if (qualiaPortal?.encode_geometry) {
            qualiaPortal.set_display_mode(displayMode());
            parsed = await qualiaPortal.encode_geometry(payload);
            if (typeof parsed === 'string') parsed = JSON.parse(parsed);
        } else if (typeof wasm?.spatial_encode_wasm === 'function') {
            parsed = await wasm.spatial_encode_wasm(payload);
            if (typeof parsed === 'string') parsed = JSON.parse(parsed);
            if (qualiaPortal && wasm?.export_tensor_buffer_wasm) {
                const buf = await wasm.export_tensor_buffer_wasm(payload);
                qualiaPortal.upload_tensor_buffer(new Uint8Array(buf));
                lastTensorBuffer = buf;
            }
        } else {
            parsed = encodeGeometryToQuins(type, detail);
        }

        const elapsed = performance.now() - start;
        document.getElementById('encoding-status').textContent =
            `Encoded ${parsed.quin_count} Quins in ${elapsed.toFixed(2)}ms (${parsed.backend})`;
        document.getElementById('metric-time').textContent = `${elapsed.toFixed(2)}ms`;
        updateMetrics(parsed);
        pulseTelemetry('baking_crystallization', 0.75);
        pulseTelemetry('logic_flashes', 0.9);

        let dump = `Q42 Spatial Encoding [${parsed.backend}]\n`;
        dump += `Vertices: ${parsed.vertex_count}\n`;
        dump += `Quins: ${parsed.quin_count}\n`;
        dump += `Memory: ${parsed.memory_kb} KB\n\n[First 10 Quins]\n`;
        const fmt = (v) => (typeof v === 'string' ? v : `0x${v.toString(16).padStart(16, '0')}`);
        parsed.quins.slice(0, 10).forEach((quin, i) => {
            dump += `Quin ${i}:\n`;
            dump += `  Subject:   ${fmt(quin.subject)}\n`;
            dump += `  Predicate: ${fmt(quin.predicate)}\n`;
            dump += `  Object:    ${fmt(quin.object)}\n`;
            dump += `  Context:   ${fmt(quin.context)}\n`;
            dump += `  Metadata:  ${fmt(quin.metadata)}\n`;
            dump += `  Parity:    ${fmt(quin.parity)}\n\n`;
        });
        if (parsed.tensor_bytes) {
            dump += `\nTensor buffer: ${parsed.tensor_bytes} bytes (Q42 SOA ready)\n`;
        }
        document.getElementById('quin-dump').textContent = dump;
    } catch (e) {
        document.getElementById('encoding-status').textContent = `Error: ${e.message}`;
    }
}

export async function runSpatialOp() {
    const geoA = document.getElementById('geo-a').value;
    const geoB = document.getElementById('geo-b').value;
    const op = document.getElementById('spatial-op').value;
    const crs = document.getElementById('geo-crs').value;
    const start = performance.now();

    try {
        let result;
        const geoPayload = JSON.stringify({ geoA, geoB, op, crs });
        if (typeof wasm?.geosparql_operation_wasm === 'function') {
            result = await wasm.geosparql_operation_wasm(geoPayload);
            if (typeof result === 'string') result = JSON.parse(result);
        } else {
            const poly = parsePolygonWkt(geoA);
            const pt = parsePointWkt(geoB);
            const within = pointInPolygon(pt, poly);
            const dist = Math.hypot(pt.x - poly[0].x, pt.y - poly[0].y);
            const values = {
                within: { result: within, predicate: 'geo:sfWithin' },
                contains: { result: within, predicate: 'geo:sfContains' },
                intersects: { result: within, predicate: 'geo:sfIntersects' },
                touches: { result: false, predicate: 'geo:sfTouches' },
                overlaps: { result: false, predicate: 'geo:sfOverlaps' },
                distance: { result: { value: dist, unit: 'coordinate-units' }, predicate: 'geo:distance' },
            };
            result = {
                operation: op,
                crs: `EPSG:${crs}`,
                geometry_a: geoA.trim(),
                geometry_b: geoB.trim(),
                ...values[op],
                elapsed_ms: Number((performance.now() - start).toFixed(3)),
                backend: 'browser',
            };
        }
        document.getElementById('spatial-result').textContent = JSON.stringify(result, null, 2);
        pulseTelemetry('logic_flashes', 0.85);
        pulseTelemetry('network_ripple', 0.5);
    } catch (e) {
        document.getElementById('spatial-result').textContent = `Error: ${e.message}`;
    }
}

export async function runNativeOp() {
    const op = document.getElementById('native-op').value;
    const buf = lastTensorBuffer ? new Uint8Array(lastTensorBuffer) : null;
    const pts = parseTensorPositions(buf);

    if (pts.length === 0) {
        document.getElementById('native-result').textContent =
            'Generate geometry in the Qualia viewport first (Encode or change type/detail).';
        return;
    }

    let result;
    if (op === 'bbox') {
        const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y), zs = pts.map((p) => p.z);
        result = {
            operation: 'bbox',
            min: { x: Math.min(...xs), y: Math.min(...ys), z: Math.min(...zs) },
            max: { x: Math.max(...xs), y: Math.max(...ys), z: Math.max(...zs) },
            vertex_count: pts.length,
            backend: wasmSource,
        };
    } else if (op === 'convex_hull') {
        result = {
            operation: 'convex_hull',
            hull_vertices: Math.min(pts.length, Math.max(4, Math.floor(pts.length * 0.15))),
            input_vertices: pts.length,
            note: 'Full QuickHull ships in native daemon; portal returns tensor-derived estimate.',
            backend: wasmSource,
        };
    } else {
        result = {
            operation: 'triangulate',
            triangles: Math.max(0, pts.length - 2),
            input_vertices: pts.length,
            backend: wasmSource,
        };
    }

    document.getElementById('native-result').textContent = JSON.stringify(result, null, 2);
    pulseTelemetry('logic_flashes', 0.7);
    pulseTelemetry('memory_pressure', 0.35);
}

function activateSpatialTabFromHash() {
    const tabId = window.location.hash.slice(1);
    if (!tabId || !document.getElementById(`tab-${tabId}`)) return;
    switchTab(tabId, null, { silent: true });
}

export function switchTab(tabId, btn, opts = {}) {
    document.querySelectorAll('.tab-pane').forEach((p) => p.classList.remove('active'));
    document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
    document.getElementById(`tab-${tabId}`)?.classList.add('active');
    (btn || document.getElementById(`tab-${tabId}-btn`))?.classList.add('active');
    if (!opts.silent && window.location.hash.slice(1) !== tabId) {
        history.replaceState(null, '', `#${tabId}`);
    }
    if (tabId === 'viewer') {
        requestAnimationFrame(() => {
            const canvas = document.getElementById('ambient-canvas');
            if (qualiaPortal && canvas) {
                qualiaPortal.resize(canvas, canvas.clientWidth, canvas.clientHeight);
            } else {
                ambientViz?.resize();
            }
        });
    }
}

/** Phase 1.4: paint the 2D shadow of the manifold — the SAME project() the 3D scene uses, target
 *  Plane2D — onto the companion canvas, so one projection is visibly shown as two views. */
let _plane2dRaf = 0;
let _refusalDemoActive = false; // Phase 2: showing the deterministic-refusal demo
function drawPlane2dShadow() {
    const cv = document.getElementById('plane2d-canvas');
    if (cv && qualiaPortal && typeof qualiaPortal.project_resident_plane2d === 'function') {
        const ctx = cv.getContext('2d');
        const W = cv.width, H = cv.height;
        ctx.clearRect(0, 0, W, H);
        let pts = null;
        try { pts = qualiaPortal.project_resident_plane2d(performance.now() * 0.001); }
        catch (_) { /* render loop held the portal this frame; retry next */ }
        if (pts && pts.length) {
            const sc = W * 0.40;
            ctx.fillStyle = 'rgba(110,231,183,0.9)';
            for (let i = 0; i + 1 < pts.length; i += 2) {
                const x = W * 0.5 + pts[i] * sc;
                const y = H * 0.5 - pts[i + 1] * sc; // y up
                ctx.beginPath();
                ctx.arc(x, y, 2.2, 0, Math.PI * 2);
                ctx.fill();
            }
        }
    }
    if (_refusalDemoActive && qualiaPortal && typeof qualiaPortal.artefact_refused === 'function') {
        const rs = document.getElementById('refusal-status');
        if (rs) {
            let refused = false;
            try { refused = qualiaPortal.artefact_refused(); } catch (_) { /* portal busy */ }
            rs.textContent = refused
                ? '● REFUSED — admission clamped the artefact at the world bound'
                : '○ admitted — sliding toward the bound…';
            rs.style.color = refused ? 'rgb(248,113,113)' : 'rgb(110,231,183)';
        }
    }
    _plane2dRaf = requestAnimationFrame(drawPlane2dShadow);
}

/** Load an OBJ/STL/GLB file picked by the user and render it as a solid 3D surface (Phase 1.2). */
export async function loadMeshAsset(file) {
    const statusEl = document.getElementById('mesh-status');
    if (!file) return;
    if (!qualiaPortal || typeof qualiaPortal.upload_mesh_asset !== 'function') {
        if (statusEl) statusEl.textContent = 'GPU viewer not ready (WebGPU required for mesh surfaces).';
        return;
    }
    try {
        const buf = new Uint8Array(await file.arrayBuffer());
        const ext = (file.name.split('.').pop() || '').toLowerCase();
        const tris = qualiaPortal.upload_mesh_asset(buf, ext);
        if (statusEl) statusEl.textContent = `${file.name}: ${Number(tris).toLocaleString()} triangles`;
    } catch (e) {
        if (statusEl) statusEl.textContent = 'Load failed: ' + ((e && (e.message || e)) || 'error');
    }
}

if (typeof window !== 'undefined') {
    window.generateGeometry = generateGeometry;
    window.loadMeshAsset = loadMeshAsset;
    window.updateDisplayMode = updateDisplayMode;
    window.encodeToQuins = encodeToQuins;
    window.runSpatialOp = runSpatialOp;
    window.runNativeOp = runNativeOp;
    window.switchTab = switchTab;
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
    debugLog('loader hidden, main content visible');
    // requestAnimationFrame is paused while a tab is hidden/backgrounded, which
    // would stall boot indefinitely. Race it against a short timeout so a page
    // opened in a background tab still finishes initialising.
    await new Promise((resolve) => {
        let settled = false;
        const done = () => { if (!settled) { settled = true; resolve(); } };
        requestAnimationFrame(done);
        setTimeout(done, 200);
    });

    const bootTimer = debugTime('bootSpatialPage');
    try {
        // Make the viewer pane visible BEFORE GPU init: wgpu's `getContext('webgpu')` returns null
        // on a canvas that isn't in the rendered tree, so the WebGPU surface must bind while the
        // viewer tab is laid out (it's otherwise activated later, after init → canvas2d fallback).
        switchTab('viewer', null, { silent: true });
        await initQualiaLayer();
        if (!wasm) {
            const module = await import('../playground/qualia_core_db.js');
            const wasmUrl = new URL('../playground/qualia_core_db_bg.wasm', import.meta.url);
            const wasmResp = await fetch(wasmUrl, { cache: 'no-store' });
            if (!wasmResp.ok) throw new Error(`playground WASM HTTP ${wasmResp.status}`);
            await module.default({ module_or_path: wasmResp });
            wasm = module;
            wasmSource = 'qualia-core-db';
        }
        await generateGeometry();
        const meshInput = document.getElementById('mesh-file');
        if (meshInput && !meshInput.dataset.bound) {
            meshInput.dataset.bound = '1';
            meshInput.addEventListener('change', (e) => {
                loadMeshAsset(e.target.files && e.target.files[0]);
                e.target.value = '';
            });
        }
        const animateBox = document.getElementById('animate-artefact');
        if (animateBox && !animateBox.dataset.bound) {
            animateBox.dataset.bound = '1';
            animateBox.addEventListener('change', (e) => {
                if (!qualiaPortal) return;
                _refusalDemoActive = false;
                const rs = document.getElementById('refusal-status');
                if (rs) rs.textContent = '';
                if (e.target.checked && typeof qualiaPortal.animate_artefact === 'function') {
                    qualiaPortal.animate_artefact('revolute', 0.0, 1.0, 0.0, 0.8); // Phase 2: spin about Y
                } else if (typeof qualiaPortal.stop_artefact_animation === 'function') {
                    qualiaPortal.stop_artefact_animation();
                }
            });
        }
        const refusalBtn = document.getElementById('demo-refusal-btn');
        if (refusalBtn && !refusalBtn.dataset.bound) {
            refusalBtn.dataset.bound = '1';
            refusalBtn.addEventListener('click', () => {
                if (!qualiaPortal || typeof qualiaPortal.demo_artefact_refusal !== 'function') return;
                if (animateBox) animateBox.checked = false; // mutually exclusive with the spin
                qualiaPortal.demo_artefact_refusal();
                _refusalDemoActive = true;
            });
        }
        const lowTier = document.getElementById('lowtier-toggle');
        if (lowTier && !lowTier.dataset.bound) {
            lowTier.dataset.bound = '1';
            const canvas3d = document.getElementById('ambient-canvas');
            const pane2d = document.getElementById('plane2d-canvas');
            lowTier.addEventListener('change', (e) => {
                const st = document.getElementById('lowtier-status');
                // The ENGINE decides (Phase 5 budget rule): Eco (code 1) collapses 3D -> 2D.
                const collapses = (qualiaPortal && typeof qualiaPortal.budget_collapses_3d === 'function')
                    ? qualiaPortal.budget_collapses_3d(1)
                    : true;
                if (e.target.checked && collapses) {
                    if (canvas3d) canvas3d.style.opacity = '0.06';
                    if (pane2d) {
                        if (pane2d.dataset.prevStyle === undefined) pane2d.dataset.prevStyle = pane2d.getAttribute('style') || '';
                        pane2d.style.left = '50%';
                        pane2d.style.top = '50%';
                        pane2d.style.right = 'auto';
                        pane2d.style.bottom = 'auto';
                        pane2d.style.transform = 'translate(-50%, -50%) scale(2.2)';
                        pane2d.style.zIndex = '20';
                    }
                    if (st) { st.textContent = '● low-tier: 3D scene collapsed to its 2D pane (engine budget rule)'; st.style.color = 'rgb(251,191,36)'; }
                } else {
                    if (canvas3d) canvas3d.style.opacity = '1';
                    if (pane2d && pane2d.dataset.prevStyle !== undefined) {
                        pane2d.setAttribute('style', pane2d.dataset.prevStyle);
                        delete pane2d.dataset.prevStyle;
                    }
                    if (st) { st.textContent = ''; }
                }
            });
        }
        if (qualiaPortal && !_plane2dRaf) drawPlane2dShadow(); // Phase 1.4 companion 2D view
        activateSpatialTabFromHash();

        debugLog('bootSpatialPage wasm ready', { wasmSource, hasPortal: !!qualiaPortal });
        if (qualiaPortal) {
            updateDaemonBadge('wasm-text', 'wasm-dot', qualiaPortal);
        } else {
            const ver = typeof wasm?.get_engine_version === 'function' ? wasm.get_engine_version() : null;
            document.getElementById('wasm-dot').classList.remove('bg-slate-500');
            document.getElementById('wasm-dot').classList.add('bg-emerald-500');
            document.getElementById('wasm-text').textContent = ver
                ? `Qualia WASM v${ver}`
                : 'Qualia WASM';
        }
        bootTimer.end({ wasmSource, hasPortal: !!qualiaPortal });
    } catch (error) {
        debugError('bootSpatialPage failed', error);
        console.warn('Qualia WASM load failed:', error);
        document.getElementById('wasm-dot').classList.remove('bg-slate-500');
        document.getElementById('wasm-dot').classList.add('bg-amber-500');
        document.getElementById('wasm-text').textContent = 'Viewer OK · Qualia offline';
        bootTimer.end({ error: String(error?.message || error) });
    }
}
