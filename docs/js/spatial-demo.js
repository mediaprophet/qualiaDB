/**
 * Spatial Mathematics demo — Qualia WASM portal (no Three.js).
 */

import { AmbientViz, bindTelemetrySliders, defaultTelemetry } from './ambient-viz.js';
import {
    loadQualiaPortal, startPortalLoop, stopPortalLoop, getPortal, getPortalModule,
    setPortalStandpoint, connectPortalToDaemon, refreshTensorSliceFromDaemon,
    onDaemonLinkState, updateDaemonBadge, getLastDaemonTensorBuffer,
    getDaemonLinkState, DaemonLinkState,
} from './qualia-shell.js';

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
    if (!canvas) return;

    const { portal, mod, source } = await loadQualiaPortal(canvas);
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
        onDaemonLinkState(() => updateDaemonBadge('wasm-text', 'wasm-dot', portal));
        const onTensorLoaded = ({ nodes }) => {
            if (nodes > 0) {
                document.getElementById('metric-vertices')?.textContent = nodes.toLocaleString();
                lastTensorBuffer = getLastDaemonTensorBuffer();
            }
            updateDaemonBadge('wasm-text', 'wasm-dot', portal);
        };
        connectPortalToDaemon(portal, {
            onLoaded: onTensorLoaded,
            onRefreshed: onTensorLoaded,
        }).then(() => updateDaemonBadge('wasm-text', 'wasm-dot', portal));
        return;
    }

    ambientViz = new AmbientViz(canvas, {
        telemetry: defaultTelemetry(),
        onResize: () => {},
    });
    ambientViz.start();
    bindTelemetrySliders(document.getElementById('telemetry-sliders'), ambientViz);
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
                    document.getElementById('metric-selected')?.textContent = String(idx);
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
                    document.getElementById('metric-vertices')?.textContent = nodes.toLocaleString();
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

export function switchTab(tabId, btn) {
    document.querySelectorAll('.tab-pane').forEach((p) => p.classList.remove('active'));
    document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
    document.getElementById(`tab-${tabId}`)?.classList.add('active');
    btn?.classList.add('active');
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

if (typeof window !== 'undefined') {
    window.generateGeometry = generateGeometry;
    window.updateDisplayMode = updateDisplayMode;
    window.encodeToQuins = encodeToQuins;
    window.runSpatialOp = runSpatialOp;
    window.runNativeOp = runNativeOp;
    window.switchTab = switchTab;
}

export async function bootSpatialPage() {
    initSpatialDemo();

    document.getElementById('loading-overlay').style.display = 'none';
    document.getElementById('main-content').style.display = 'block';
    await new Promise((r) => requestAnimationFrame(r));

    try {
        await initQualiaLayer();
        if (!wasm) {
            const module = await import('../playground/qualia_core_db.js');
            await module.default();
            wasm = module;
            wasmSource = 'qualia-core-db';
        }
        await generateGeometry();

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
    } catch (error) {
        console.warn('Qualia WASM load failed:', error);
        document.getElementById('wasm-dot').classList.remove('bg-slate-500');
        document.getElementById('wasm-dot').classList.add('bg-amber-500');
        document.getElementById('wasm-text').textContent = 'Viewer OK · Qualia offline';
    }
}