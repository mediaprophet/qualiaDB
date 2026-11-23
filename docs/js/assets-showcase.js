/**
 * Hypermedia Asset Repositories & QualiaPortal WASM Streaming Showcase
 * For https://mediaprophet.github.io/qualiadb/assets-showcase.html
 *
 * Streams .10d v2 Holographic manifolds, LODs, Rhizosphere/X-Ray layers,
 * .q42 semantic volumes, and .ttl RDF ontologies from the 53 dedicated
 * intenteco (Biosphere) and qualia-poiesis (Made-World) repositories
 * directly into the QualiaPortal WebGPU / WebGL2 / Canvas2D WASM runtime.
 */

import { defaultTelemetry } from './ambient-viz.js';
import { ensureCrossOriginIsolation } from './qualia-coi.js';
import { debugEnv, debugError, debugLog, debugTime, debugWarn } from './qualia-debug.js';
import { loadQualiaPortal, startPortalLoop, ensureCanvasBackingStore } from './qualia-shell.js';

const CATALOG_URL = 'data/assets-catalog.json';

let catalog = null;
let allAssets = [];
let filteredAssets = [];
let selectedAsset = null;
let selectedVariantId = null;
let activeViewMode = '10d'; // '10d' | 'rdf' | 'q42'
let streamSourcePolicy = 'auto'; // 'auto' | 'github' | 'mirror'

let qualiaPortal = null;
let portalMod = null;
let portalRenderer = null;
let activeCanvas = null;
let portalReady = false;

// Orbit camera state
const cam = {
    yaw: 0.72,
    pitch: 0.28,
    zoom: 4.2,
    tx: 0.0,
    ty: 0.5,
    tz: 0.0,
    defaultYaw: 0.72,
    defaultPitch: 0.28,
    defaultZoom: 4.2,
    defaultTx: 0.0,
    defaultTy: 0.5,
    defaultTz: 0.0,
};

let turntableActive = false;
let sceneComposerMode = false;
let composedSceneAssets = []; // Array<{ slug, title, bytes, r, g, b, a }>
let activeAbortController = null;

const $ = (id) => document.getElementById(id);

function formatBytes(bytes) {
    if (!Number.isFinite(bytes) || bytes < 0) return '—';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

async function computeSha256Hex(uint8) {
    try {
        const digest = await crypto.subtle.digest('SHA-256', uint8);
        return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0')).join('');
    } catch (_) {
        return null;
    }
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

function applyCameraToPortal() {
    if (!qualiaPortal) return;
    try {
        if (typeof qualiaPortal.set_camera_target === 'function') {
            qualiaPortal.set_camera_target(cam.yaw, cam.pitch, cam.zoom, cam.tx, cam.ty, cam.tz);
        } else if (typeof qualiaPortal.set_camera === 'function') {
            qualiaPortal.set_camera(cam.yaw, cam.pitch, cam.zoom);
        }
    } catch (e) {
        debugWarn('applyCameraToPortal failed', e);
    }
}

/**
 * QualiaPortal.load_10d normalizes the uploaded mesh into the portal's canonical
 * orbit viewport frame (centred around the origin, ~1.6-2.2 units radius) unless
 * set_preserve_authored_frame(true) is called. We keep orbit-frame normalization
 * so every asset (from a 12 mm ant to a 65 m Eucalyptus regnans) fills the viewport
 * cleanly at zoom ~3.4, while showing its true SI metric bounds in the HUD.
 */
function resetCameraForAsset(asset) {
    cam.defaultYaw = 0.72;
    cam.defaultPitch = 0.26;
    cam.defaultZoom = 3.4;
    cam.defaultTx = 0.0;
    cam.defaultTy = 0.0;
    cam.defaultTz = 0.0;

    cam.yaw = cam.defaultYaw;
    cam.pitch = cam.defaultPitch;
    cam.zoom = cam.defaultZoom;
    cam.tx = cam.defaultTx;
    cam.ty = cam.defaultTy;
    cam.tz = cam.defaultTz;
    applyCameraToPortal();
}

function bindCanvasControls(canvas) {
    if (!canvas || canvas.__showcaseControlsBound) return;
    canvas.__showcaseControlsBound = true;

    let dragging = false;
    let panMode = false;
    let lastX = 0;
    let lastY = 0;

    canvas.addEventListener('contextmenu', (e) => e.preventDefault());

    canvas.addEventListener('pointerdown', (e) => {
        dragging = true;
        panMode = e.button === 2 || e.shiftKey;
        lastX = e.clientX;
        lastY = e.clientY;
        try {
            canvas.setPointerCapture(e.pointerId);
        } catch (_) { /* ignore */ }
    });

    canvas.addEventListener('pointermove', (e) => {
        if (!dragging) return;
        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX;
        lastY = e.clientY;

        if (panMode) {
            const panScale = Math.max(0.0015, cam.zoom * 0.0018);
            const cosY = Math.cos(cam.yaw);
            const sinY = Math.sin(cam.yaw);
            cam.tx -= (dx * cosY) * panScale;
            cam.tz -= (dx * sinY) * panScale;
            cam.ty += dy * panScale;
        } else {
            cam.yaw += dx * 0.0085;
            cam.pitch = Math.max(-1.42, Math.min(1.42, cam.pitch + dy * 0.0085));
        }
        applyCameraToPortal();
    });

    const stopDrag = (e) => {
        if (!dragging) return;
        dragging = false;
        try {
            canvas.releasePointerCapture(e.pointerId);
        } catch (_) { /* ignore */ }
    };
    canvas.addEventListener('pointerup', stopDrag);
    canvas.addEventListener('pointercancel', stopDrag);

    canvas.addEventListener(
        'wheel',
        (e) => {
            e.preventDefault();
            const factor = e.deltaY > 0 ? 1.11 : 0.9;
            cam.zoom = Math.max(0.65, Math.min(28.0, cam.zoom * factor));
            applyCameraToPortal();
        },
        { passive: false },
    );
}

/**
 * Stream binary payload with chunk-by-chunk progress and candidate URL failover.
 */
async function streamBinaryWithFallback(candidates, { expectedBytes, expectedSha256, onProgress, signal } = {}) {
    let lastError = null;

    for (const cand of candidates) {
        if (!cand || !cand.url) continue;
        try {
            onProgress?.({
                phase: 'connecting',
                sourceLabel: cand.label,
                url: cand.url,
                loaded: 0,
                total: expectedBytes || 0,
                chunks: 0,
                pct: 0,
            });

            const t0 = performance.now();
            const response = await fetch(cand.url, { signal, credentials: 'omit' });
            if (!response.ok) {
                throw new Error(`HTTP ${response.status} from ${cand.label}`);
            }

            const headerLen = Number(response.headers.get('content-length')) || 0;
            const total = headerLen || expectedBytes || 0;

            let bytes;
            let chunkCount = 0;

            if (response.body?.getReader) {
                const reader = response.body.getReader();
                const chunks = [];
                let received = 0;

                while (true) {
                    const { done, value } = await reader.read();
                    if (done) break;
                    chunks.push(value);
                    received += value.byteLength;
                    chunkCount += 1;
                    const elapsedSec = Math.max(0.001, (performance.now() - t0) / 1000);
                    const rateBps = received / elapsedSec;
                    const pct = total ? Math.min(100, (received / total) * 100) : 50;
                    onProgress?.({
                        phase: 'streaming',
                        sourceLabel: cand.label,
                        url: cand.url,
                        loaded: received,
                        total,
                        chunks: chunkCount,
                        rateBps,
                        pct,
                    });
                }

                bytes = new Uint8Array(received);
                let offset = 0;
                for (const c of chunks) {
                    bytes.set(c, offset);
                    offset += c.byteLength;
                }
            } else {
                const buf = await response.arrayBuffer();
                bytes = new Uint8Array(buf);
                chunkCount = 1;
            }

            const elapsedMs = performance.now() - t0;
            const actualSha = await computeSha256Hex(bytes);
            const shaVerified = expectedSha256 ? actualSha === expectedSha256.toLowerCase() : true;

            onProgress?.({
                phase: 'complete',
                sourceLabel: cand.label,
                url: cand.url,
                loaded: bytes.byteLength,
                total: bytes.byteLength,
                chunks: chunkCount,
                elapsedMs,
                sha256: actualSha,
                shaVerified,
                pct: 100,
            });

            return {
                bytes,
                sourceLabel: cand.label,
                url: cand.url,
                chunks: chunkCount,
                elapsedMs,
                sha256: actualSha,
                shaVerified,
            };
        } catch (err) {
            if (err?.name === 'AbortError') throw err;
            lastError = err;
            debugWarn(`Stream candidate ${cand.label} (${cand.url}) failed:`, err.message);
        }
    }

    throw lastError || new Error('No streaming endpoint succeeded for this asset.');
}

function buildStreamCandidates(item) {
    if (!item) return [];
    const mirror = item.stream_url ? { label: 'Pages Same-Origin Stream Mirror', url: item.stream_url } : null;
    const jsdelivr = item.jsdelivr_url ? { label: 'GitHub Repo CDN (jsDelivr)', url: item.jsdelivr_url } : null;
    const rawGh = item.raw_github_url ? { label: 'GitHub Raw Stream (main)', url: item.raw_github_url } : null;

    if (streamSourcePolicy === 'github') {
        return [jsdelivr, rawGh, mirror].filter(Boolean);
    }
    if (streamSourcePolicy === 'mirror') {
        return [mirror, jsdelivr, rawGh].filter(Boolean);
    }
    // 'auto': prefer local mirror on Pages for instant zero-404 delivery, then GitHub CDN/raw
    return [mirror, jsdelivr, rawGh].filter(Boolean);
}

function updateStreamProgressUi(prog) {
    const bar = $('stream-progress-bar');
    const label = $('stream-progress-label');
    const meta = $('stream-progress-meta');

    if (bar && prog.pct != null) {
        bar.style.width = `${Math.max(4, Math.min(100, prog.pct))}%`;
    }
    if (label) {
        if (prog.phase === 'connecting') {
            label.textContent = `Connecting to ${prog.sourceLabel}…`;
        } else if (prog.phase === 'streaming') {
            const rateStr = prog.rateBps ? ` · ${formatBytes(prog.rateBps)}/s` : '';
            label.textContent = `Streaming via ${prog.sourceLabel}: ${formatBytes(prog.loaded)} / ${formatBytes(prog.total || prog.loaded)} (${prog.chunks} chunks${rateStr})`;
        } else if (prog.phase === 'complete') {
            const shaTag = prog.shaVerified ? 'SHA-256 verified ✓' : 'SHA-256 unverified';
            label.textContent = `Streamed ${formatBytes(prog.loaded)} in ${prog.elapsedMs.toFixed(0)} ms (${prog.chunks} chunks · ${prog.sourceLabel} · ${shaTag})`;
        }
    }
    if (meta && prog.sha256) {
        meta.textContent = `sha256:${prog.sha256.slice(0, 16)}…`;
        meta.title = prog.sha256;
    }
}

function domainPaletteRgba(asset, idx = 0) {
    if (asset.org === 'intenteco') {
        if (asset.slug === 'homo-sapiens') return [0.38, 0.78, 0.95, 1.0];
        if (asset.category_short === 'Fauna') return [0.92, 0.64, 0.28, 1.0];
        return [0.22, 0.82, 0.54, 1.0];
    }
    const palette = [
        [0.35, 0.72, 0.96, 1.0],
        [0.95, 0.65, 0.24, 1.0],
        [0.68, 0.52, 0.96, 1.0],
    ];
    return palette[idx % palette.length];
}

async function streamVariantIntoPortal(asset, variantId) {
    if (!asset) return;
    if (activeAbortController) {
        activeAbortController.abort();
    }
    activeAbortController = new AbortController();
    const signal = activeAbortController.signal;

    const variants = asset.ten_d_variants || [];
    const variant =
        variants.find((v) => v.id === variantId) ||
        variants.find((v) => v.id === asset.default_variant_id) ||
        variants.find((v) => v.stream_url) ||
        variants[0];

    if (!variant) {
        setHudStatus(`No .10d container found for ${asset.title}`, 'warn');
        return;
    }

    selectedVariantId = variant.id;
    activeViewMode = '10d';
    renderVariantPills(asset);

    const candidates = buildStreamCandidates(variant);
    try {
        const result = await streamBinaryWithFallback(candidates, {
            expectedBytes: variant.bytes,
            expectedSha256: variant.sha256,
            onProgress: updateStreamProgressUi,
            signal,
        });

        if (!qualiaPortal) {
            setHudStatus('QualiaPortal WASM not armed; binary stream verified in browser memory.', 'warn');
            return;
        }

        qualiaPortal.set_ambient_enabled?.(false);
        qualiaPortal.set_preserve_authored_frame?.(false);

        let receipt = null;
        const tUpload0 = performance.now();

        if (sceneComposerMode) {
            const rgba = domainPaletteRgba(asset, composedSceneAssets.length);
            const existingIdx = composedSceneAssets.findIndex((x) => x.slug === asset.slug);
            const entry = {
                slug: asset.slug,
                title: asset.title,
                bytes: result.bytes,
                r: rgba[0],
                g: rgba[1],
                b: rgba[2],
                a: rgba[3],
            };
            if (existingIdx >= 0) composedSceneAssets[existingIdx] = entry;
            else composedSceneAssets.push(entry);

            if (typeof qualiaPortal.load_scene_assets === 'function') {
                receipt = qualiaPortal.load_scene_assets(composedSceneAssets);
            } else if (typeof qualiaPortal.load_assets_colored === 'function') {
                receipt = qualiaPortal.load_assets_colored(composedSceneAssets);
            } else {
                receipt = qualiaPortal.load_10d(result.bytes);
            }
            updateSceneComposerTray();
        } else {
            composedSceneAssets = [];
            updateSceneComposerTray();
            if (typeof qualiaPortal.load_10d === 'function') {
                receipt = qualiaPortal.load_10d(result.bytes);
            }
        }

        const uploadMs = performance.now() - tUpload0;
        resetCameraForAsset(asset);
        qualiaPortal.set_telemetry?.(
            telemetryToFloats({
                network_ripple: 0.72,
                baking_crystallization: 0.64,
                epistemic_density: 0.88,
            }),
        );

        const verts = receipt?.vertex_count ?? asset.geometry?.vertices ?? '—';
        const tris = receipt?.triangle_count ?? receipt?.total_triangles ?? asset.geometry?.triangles ?? '—';
        const tier = qualiaPortal.tier?.() ?? receipt?.tier ?? 2;

        updateViewportMetrics({
            mode: sceneComposerMode ? `Multi-Asset Scene (${composedSceneAssets.length})` : `10D v2 (${variant.label})`,
            vertices: verts,
            triangles: tris,
            bytes: result.bytes.byteLength,
            tier,
            uploadMs,
            sha256: result.sha256,
            sourceLabel: result.sourceLabel,
        });

        renderInspectorDetails(asset, variant, receipt, result);
    } catch (err) {
        if (err?.name === 'AbortError') return;
        debugError('streamVariantIntoPortal failed', err);
        setHudStatus(`Stream failed: ${err.message}`, 'error');
    }
}

async function streamOntologyHolographIntoPortal(asset) {
    if (!asset?.ontology_ttl) {
        setHudStatus('No Turtle/N3 ontology file available for this asset.', 'warn');
        return;
    }
    if (activeAbortController) activeAbortController.abort();
    activeAbortController = new AbortController();

    activeViewMode = 'rdf';
    renderVariantPills(asset);

    const candidates = buildStreamCandidates(asset.ontology_ttl);
    try {
        const result = await streamBinaryWithFallback(candidates, {
            expectedBytes: asset.ontology_ttl.bytes,
            onProgress: updateStreamProgressUi,
            signal: activeAbortController.signal,
        });

        const ttlText = new TextDecoder('utf-8').decode(result.bytes);
        let receipt = null;
        const t0 = performance.now();

        if (qualiaPortal && typeof qualiaPortal.load_rdf_graph_text === 'function') {
            receipt = qualiaPortal.load_rdf_graph_text(ttlText, 'text/turtle');
        }
        const uploadMs = performance.now() - t0;

        updateViewportMetrics({
            mode: '3D RDF Ontology Holograph',
            vertices: receipt?.node_count ?? '—',
            triangles: receipt?.triple_count ?? '—',
            bytes: result.bytes.byteLength,
            tier: qualiaPortal?.tier?.() ?? 2,
            uploadMs,
            sha256: result.sha256,
            sourceLabel: result.sourceLabel,
            isRdf: true,
        });

        renderOntologyCodePreview(asset, ttlText, receipt);
    } catch (err) {
        if (err?.name === 'AbortError') return;
        setHudStatus(`Ontology stream failed: ${err.message}`, 'error');
    }
}

async function streamQ42VolumeIntoPortal(asset, q42Entry) {
    const q42 = q42Entry || (asset?.q42_volumes || [])[0];
    if (!q42) {
        setHudStatus('No .q42 volume found for this asset.', 'warn');
        return;
    }
    if (activeAbortController) activeAbortController.abort();
    activeAbortController = new AbortController();

    activeViewMode = 'q42';
    renderVariantPills(asset);

    const candidates = buildStreamCandidates(q42);
    try {
        const result = await streamBinaryWithFallback(candidates, {
            expectedBytes: q42.bytes,
            expectedSha256: q42.sha256,
            onProgress: updateStreamProgressUi,
            signal: activeAbortController.signal,
        });

        let parsed = null;
        const t0 = performance.now();
        if (qualiaPortal && typeof qualiaPortal.load_q42 === 'function') {
            try {
                parsed = qualiaPortal.load_q42(result.bytes);
            } catch (e) {
                parsed = { status: 'admitted_binary_q42', note: String(e?.message || e) };
            }
        }
        const uploadMs = performance.now() - t0;

        // Inspect magic & header from raw .q42 bytes
        const magic = new TextDecoder('utf-8', { fatal: false }).decode(result.bytes.slice(0, 4)).replace(/\0/g, '\\0');
        const summary = {
            filename: q42.filename,
            magic,
            bytes: result.bytes.byteLength,
            sha256: result.sha256,
            sha_verified: result.shaVerified,
            portal_load_q42_ms: Number(uploadMs.toFixed(2)),
            portal_receipt: parsed,
        };

        const codeBox = $('inspector-code-box');
        if (codeBox) {
            codeBox.textContent = JSON.stringify(summary, null, 2);
        }
        setHudStatus(`Admitted ${q42.filename} (${formatBytes(result.bytes.byteLength)}) into QualiaPortal.load_q42`, 'ok');
    } catch (err) {
        if (err?.name === 'AbortError') return;
        setHudStatus(`.q42 stream failed: ${err.message}`, 'error');
    }
}

async function handleLocalFileUpload(file) {
    if (!file || !qualiaPortal) return;
    const buf = await file.arrayBuffer();
    const bytes = new Uint8Array(buf);
    const sha = await computeSha256Hex(bytes);
    const name = file.name.toLowerCase();
    const t0 = performance.now();
    let receipt = null;
    let modeLabel = file.name;

    try {
        if (name.endsWith('.10d')) {
            receipt = qualiaPortal.load_10d(bytes);
            modeLabel = `Local .10d · ${file.name}`;
        } else if (name.endsWith('.hmc')) {
            let manifest = [];
            if (typeof qualiaPortal.pack_manifest === 'function') {
                try {
                    manifest = qualiaPortal.pack_manifest(bytes) || [];
                } catch (_) { /* fallback */ }
            }
            if (Array.isArray(manifest) && manifest.length > 0 && typeof qualiaPortal.load_hmc_mesh === 'function') {
                receipt = qualiaPortal.load_hmc_mesh(bytes, manifest[0].key);
                modeLabel = `Local .hmc (${manifest.length} entries) · ${manifest[0].key}`;
            } else if (typeof qualiaPortal.load_body_from_qualia_bundle === 'function') {
                receipt = qualiaPortal.load_body_from_qualia_bundle(bytes);
                modeLabel = `Local .hmc bundle · ${file.name}`;
            }
        } else if (name.endsWith('.q42')) {
            receipt = qualiaPortal.load_q42(bytes);
            modeLabel = `Local .q42 · ${file.name}`;
        } else if (name.endsWith('.emf')) {
            receipt = qualiaPortal.load_emf(bytes);
            modeLabel = `Local .emf field · ${file.name}`;
        } else if (name.endsWith('.ttl') || name.endsWith('.n3')) {
            const txt = new TextDecoder().decode(bytes);
            receipt = qualiaPortal.load_rdf_graph_text(txt, 'text/turtle');
            modeLabel = `Local RDF Holograph · ${file.name}`;
        } else if (typeof qualiaPortal.upload_mesh_asset === 'function') {
            const ext = name.split('.').pop() || '';
            const tris = qualiaPortal.upload_mesh_asset(bytes, ext);
            receipt = { triangle_count: tris };
            modeLabel = `Local 3D Mesh · ${file.name}`;
        }

        const uploadMs = performance.now() - t0;
        updateStreamProgressUi({
            phase: 'complete',
            sourceLabel: 'Local File Drop',
            loaded: bytes.byteLength,
            total: bytes.byteLength,
            chunks: 1,
            elapsedMs: uploadMs,
            sha256: sha,
            shaVerified: true,
            pct: 100,
        });
        updateViewportMetrics({
            mode: modeLabel,
            vertices: receipt?.vertex_count ?? receipt?.node_count ?? '—',
            triangles: receipt?.triangle_count ?? receipt?.total_triangles ?? receipt?.triple_count ?? '—',
            bytes: bytes.byteLength,
            tier: qualiaPortal.tier?.() ?? 2,
            uploadMs,
            sha256: sha,
            sourceLabel: 'Local File Drop',
        });
        const codeBox = $('inspector-code-box');
        if (codeBox) {
            codeBox.textContent = JSON.stringify({ file: file.name, bytes: bytes.byteLength, sha256: sha, receipt }, null, 2);
        }
    } catch (e) {
        setHudStatus(`Local file load error: ${e?.message || e}`, 'error');
    }
}

function setHudStatus(text, level = 'info') {
    const el = $('canvas-hud-status');
    if (!el) return;
    el.textContent = text;
    el.className =
        level === 'error'
            ? 'text-xs text-rose-300 font-medium'
            : level === 'warn'
              ? 'text-xs text-amber-300 font-medium'
              : 'text-xs text-emerald-300 font-medium';
}

function updateViewportMetrics({ mode, vertices, triangles, bytes, tier, uploadMs, sha256, sourceLabel, isRdf }) {
    const tierNames = ['T0 Canvas2D', 'T1 WebGL2 / Tensor', 'T2 WebGPU Etalon'];
    const tierStr = tierNames[tier] || `Tier ${tier}`;

    if ($('metric-mode')) $('metric-mode').textContent = mode || '10D v2';
    if ($('metric-verts')) {
        $('metric-verts-label').textContent = isRdf ? 'Graph Nodes' : 'Vertices';
        $('metric-verts').textContent = typeof vertices === 'number' ? vertices.toLocaleString() : String(vertices);
    }
    if ($('metric-tris')) {
        $('metric-tris-label').textContent = isRdf ? 'RDF Triples' : 'Triangles';
        $('metric-tris').textContent = typeof triangles === 'number' ? triangles.toLocaleString() : String(triangles);
    }
    if ($('metric-size')) $('metric-size').textContent = formatBytes(bytes);
    if ($('metric-tier')) $('metric-tier').textContent = `${tierStr} (${uploadMs.toFixed(1)} ms)`;

    setHudStatus(
        `${selectedAsset?.title || 'Asset'} · ${mode} · ${sourceLabel}`,
        'ok',
    );
}

function renderVariantPills(asset) {
    const container = $('variant-pills');
    if (!container || !asset) return;

    const pills = [];
    for (const v of asset.ten_d_variants || []) {
        const isCurrent = activeViewMode === '10d' && selectedVariantId === v.id;
        const badge = v.stream_url ? 'Stream Ready' : 'GitHub Raw';
        pills.push(`
            <button type="button"
                data-variant-id="${v.id}"
                class="variant-pill px-3 py-1.5 rounded-xl text-xs font-medium border transition flex items-center gap-1.5 ${
                    isCurrent
                        ? 'bg-emerald-500/20 text-emerald-300 border-emerald-500/50 shadow-sm'
                        : 'bg-slate-900/80 text-slate-300 border-white/10 hover:border-emerald-400/40 hover:text-white'
                }">
                <i class="fa-solid fa-cube text-[11px]"></i>
                <span>${v.label}</span>
                <span class="text-[10px] opacity-65 font-mono">(${formatBytes(v.bytes)})</span>
                <span class="text-[9px] px-1.5 py-0.5 rounded bg-black/40 ${v.stream_url ? 'text-cyan-300' : 'text-amber-300'}">${badge}</span>
            </button>
        `);
    }

    if (asset.ontology_ttl) {
        const isRdf = activeViewMode === 'rdf';
        pills.push(`
            <button type="button"
                data-action="stream-rdf"
                class="variant-pill px-3 py-1.5 rounded-xl text-xs font-medium border transition flex items-center gap-1.5 ${
                    isRdf
                        ? 'bg-violet-500/25 text-violet-200 border-violet-400/50'
                        : 'bg-slate-900/80 text-violet-300 border-violet-500/25 hover:border-violet-400/50'
                }">
                <i class="fa-solid fa-project-diagram text-[11px]"></i>
                <span>3D RDF Ontology Holograph</span>
                <span class="text-[10px] opacity-65 font-mono">(${formatBytes(asset.ontology_ttl.bytes)})</span>
            </button>
        `);
    }

    if ((asset.q42_volumes || []).length > 0) {
        const q0 = asset.q42_volumes[0];
        const isQ42 = activeViewMode === 'q42';
        pills.push(`
            <button type="button"
                data-action="stream-q42"
                class="variant-pill px-3 py-1.5 rounded-xl text-xs font-medium border transition flex items-center gap-1.5 ${
                    isQ42
                        ? 'bg-cyan-500/25 text-cyan-200 border-cyan-400/50'
                        : 'bg-slate-900/80 text-cyan-300 border-cyan-500/25 hover:border-cyan-400/50'
                }">
                <i class="fa-solid fa-database text-[11px]"></i>
                <span>Inspect .q42 Volume (${q0.filename})</span>
                <span class="text-[10px] opacity-65 font-mono">(${formatBytes(q0.bytes)})</span>
            </button>
        `);
    }

    container.innerHTML = pills.join('');

    container.querySelectorAll('[data-variant-id]').forEach((btn) => {
        btn.addEventListener('click', () => {
            void streamVariantIntoPortal(asset, btn.getAttribute('data-variant-id'));
        });
    });

    container.querySelector('[data-action="stream-rdf"]')?.addEventListener('click', () => {
        void streamOntologyHolographIntoPortal(asset);
    });

    container.querySelector('[data-action="stream-q42"]')?.addEventListener('click', () => {
        void streamQ42VolumeIntoPortal(asset);
    });
}

function renderOntologyCodePreview(asset, ttlText, receipt) {
    const codeBox = $('inspector-code-box');
    if (!codeBox) return;
    const header = [
        `# ${asset.title} (${asset.did})`,
        `# Repository: ${asset.repository?.repo_url}`,
        `# QualiaPortal.load_rdf_graph_text receipt: ${JSON.stringify(receipt || {})}`,
        '',
    ].join('\n');
    codeBox.textContent = header + ttlText.slice(0, 6000);
}

function renderInspectorDetails(asset, activeVariant, portalReceipt, streamResult) {
    if (!asset) return;

    if ($('detail-title')) $('detail-title').textContent = asset.title;
    if ($('detail-brief')) $('detail-brief').textContent = asset.brief;
    if ($('detail-did')) $('detail-did').textContent = asset.did;
    if ($('detail-work-packet')) $('detail-work-packet').textContent = asset.work_packet;
    if ($('detail-licence')) $('detail-licence').textContent = asset.licence_policy;
    if ($('detail-domain-badge')) {
        $('detail-domain-badge').textContent = asset.domain_label;
        $('detail-domain-badge').className =
            asset.org === 'intenteco'
                ? 'px-2.5 py-1 rounded-full text-xs font-semibold bg-emerald-500/15 text-emerald-300 border border-emerald-500/30'
                : 'px-2.5 py-1 rounded-full text-xs font-semibold bg-cyan-500/15 text-cyan-300 border border-cyan-500/30';
    }

    // Dedicated GitHub repository link
    const repoLink = $('detail-repo-link');
    if (repoLink && asset.repository) {
        repoLink.href = asset.repository.repo_url;
        repoLink.querySelector('.repo-label').textContent = `${asset.org}/${asset.slug}`;
    }

    const commitBadge = $('detail-repo-commit');
    if (commitBadge && asset.repository) {
        commitBadge.textContent = asset.repository.head_commit
            ? `HEAD ${asset.repository.head_commit} (${asset.repository.default_branch})`
            : `git initialized · origin/${asset.repository.default_branch}`;
    }

    // Preview thumbnail & spectral slice
    const previewImg = $('detail-preview-img');
    if (previewImg) {
        if (asset.preview_image) {
            previewImg.src = asset.preview_image;
            previewImg.alt = `${asset.title} turnaround preview`;
            previewImg.parentElement?.classList.remove('hidden');
        } else {
            previewImg.parentElement?.classList.add('hidden');
        }
    }

    const spectralImg = $('detail-spectral-img');
    if (spectralImg) {
        if (asset.spectral_image) {
            spectralImg.src = asset.spectral_image;
            spectralImg.alt = `${asset.title} spectral / field slice`;
            spectralImg.parentElement?.classList.remove('hidden');
        } else {
            spectralImg.parentElement?.classList.add('hidden');
        }
    }

    // 10D Coordinates & SI Bounds
    const coords = asset.coordinates_10d || [];
    const coordLabels = ['q', 'v', 'w', 'x (lat)', 'y (lon)', 'z (alt m)', 't (epoch)', 'α', 'μ', 'σ'];
    const coordsEl = $('detail-10d-coords');
    if (coordsEl) {
        coordsEl.innerHTML = coords
            .map(
                (val, idx) => `
                <div class="p-2 rounded-lg bg-black/40 border border-white/10">
                    <div class="text-[10px] text-white/45 font-mono">${coordLabels[idx] || `x${idx}`}</div>
                    <div class="text-xs font-mono text-cyan-300 font-semibold truncate">${Number(val).toFixed(idx === 6 ? 0 : 3)}</div>
                </div>`,
            )
            .join('');
    }

    // Container inventory table (.hmc, .10d, .q42, .m64, .emf, .qac)
    const tableBody = $('detail-files-tbody');
    if (tableBody) {
        const rows = [];

        if (asset.hmc_container) {
            const h = asset.hmc_container;
            rows.push({
                kind: 'HMC (QBDL v1)',
                badgeClass: 'bg-amber-500/15 text-amber-300 border-amber-500/30',
                name: h.filename,
                bytes: h.bytes,
                sha256: h.sha256,
                url: h.raw_github_url,
                streamType: 'hmc',
            });
        }

        for (const v of asset.ten_d_variants || []) {
            rows.push({
                kind: `10D v2 (${v.id})`,
                badgeClass: 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30',
                name: v.filename,
                bytes: v.bytes,
                sha256: v.sha256,
                url: v.stream_url || v.raw_github_url,
                variantId: v.id,
                streamType: '10d',
            });
        }

        for (const q of asset.q42_volumes || []) {
            rows.push({
                kind: 'Q42 Volume',
                badgeClass: 'bg-cyan-500/15 text-cyan-300 border-cyan-500/30',
                name: q.filename,
                bytes: q.bytes,
                sha256: q.sha256,
                url: q.stream_url || q.raw_github_url,
                streamType: 'q42',
                q42Entry: q,
            });
        }

        for (const m of asset.m64_containers || []) {
            rows.push({
                kind: 'M64 Matter',
                badgeClass: 'bg-violet-500/15 text-violet-300 border-violet-500/30',
                name: m.filename,
                bytes: m.bytes,
                sha256: m.sha256,
                url: m.stream_url,
                streamType: 'm64',
            });
        }

        for (const e of asset.emf_fields || []) {
            rows.push({
                kind: 'EMF64 Field',
                badgeClass: 'bg-rose-500/15 text-rose-300 border-rose-500/30',
                name: e.filename,
                bytes: e.bytes,
                sha256: e.sha256,
                url: e.raw_github_url,
                streamType: 'emf',
            });
        }

        for (const a of asset.qac_acoustics || []) {
            rows.push({
                kind: 'QAC Audio',
                badgeClass: 'bg-sky-500/15 text-sky-300 border-sky-500/30',
                name: a.filename,
                bytes: a.bytes,
                sha256: a.sha256,
                url: a.raw_github_url,
                streamType: 'qac',
            });
        }

        tableBody.innerHTML = rows
            .map(
                (r, i) => `
                <tr class="border-b border-white/5 hover:bg-white/[0.03]">
                    <td class="py-2 px-2.5"><span class="px-2 py-0.5 rounded text-[10px] font-mono border ${r.badgeClass}">${r.kind}</span></td>
                    <td class="py-2 px-2.5 font-mono text-xs text-white/90">${r.name}</td>
                    <td class="py-2 px-2.5 font-mono text-xs text-white/60 text-right">${formatBytes(r.bytes)}</td>
                    <td class="py-2 px-2.5 font-mono text-[11px] text-white/45" title="${r.sha256 || ''}">${r.sha256 ? `${r.sha256.slice(0, 12)}…` : '—'}</td>
                    <td class="py-2 px-2.5 text-right">
                        ${
                            r.streamType === '10d'
                                ? `<button type="button" data-row-variant="${r.variantId}" class="px-2.5 py-1 rounded-lg bg-emerald-500/15 text-emerald-300 border border-emerald-500/30 text-[11px] hover:bg-emerald-500/25">Stream 10D</button>`
                                : r.streamType === 'q42'
                                  ? `<button type="button" data-row-q42="${i}" class="px-2.5 py-1 rounded-lg bg-cyan-500/15 text-cyan-300 border border-cyan-500/30 text-[11px] hover:bg-cyan-500/25">Load Q42</button>`
                                  : `<a href="${r.url}" target="_blank" rel="noopener" class="px-2.5 py-1 rounded-lg bg-white/5 text-white/70 border border-white/10 text-[11px] hover:text-white">Fetch ↗</a>`
                        }
                    </td>
                </tr>`,
            )
            .join('');

        tableBody.querySelectorAll('[data-row-variant]').forEach((btn) => {
            btn.addEventListener('click', () => {
                void streamVariantIntoPortal(asset, btn.getAttribute('data-row-variant'));
            });
        });

        tableBody.querySelectorAll('[data-row-q42]').forEach((btn) => {
            btn.addEventListener('click', () => {
                const rowIdx = Number(btn.getAttribute('data-row-q42'));
                void streamQ42VolumeIntoPortal(asset, rows[rowIdx]?.q42Entry);
            });
        });
    }

    // Live WASM Receipt & Bounds JSON
    const codeBox = $('inspector-code-box');
    if (codeBox) {
        const receiptPayload = {
            asset: asset.slug,
            did: asset.did,
            repository: asset.repository?.repo_url,
            streamed_variant: activeVariant
                ? {
                      id: activeVariant.id,
                      filename: activeVariant.filename,
                      bytes: activeVariant.bytes,
                      sha256: activeVariant.sha256,
                      sha256_verified: streamResult?.shaVerified ?? true,
                      via: streamResult?.sourceLabel ?? 'Pages Same-Origin Stream Mirror',
                  }
                : null,
            qualia_portal_wasm_receipt: portalReceipt || {
                vertex_count: asset.geometry?.vertices,
                triangle_count: asset.geometry?.triangles,
                tier: qualiaPortal?.tier?.() ?? 2,
            },
            geometry_bounds_si_m: {
                bounds_min: asset.geometry?.bounds_min,
                bounds_max: asset.geometry?.bounds_max,
                dimensions_si: asset.dimensions_si,
            },
            multi_scale_strata: `L0..L6 (${asset.strata_count} computational strata, Holographic 10D v2 HOLO 0x0306)`,
        };
        codeBox.textContent = JSON.stringify(receiptPayload, null, 2);
    }
}

function updateSceneComposerTray() {
    const tray = $('scene-composer-tray');
    const countEl = $('scene-composer-count');
    if (countEl) countEl.textContent = String(composedSceneAssets.length);
    if (!tray) return;

    if (!sceneComposerMode || composedSceneAssets.length === 0) {
        tray.classList.add('hidden');
        return;
    }
    tray.classList.remove('hidden');
    tray.innerHTML = composedSceneAssets
        .map(
            (item) => `
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
                <span>${item.title}</span>
                <span class="text-[10px] font-mono opacity-65">(${formatBytes(item.bytes.byteLength)})</span>
            </span>`,
        )
        .join('');
}

function renderAssetRoster() {
    const listEl = $('asset-roster-list');
    const countEl = $('roster-count');
    if (!listEl) return;

    if (countEl) {
        countEl.textContent = `Showing ${filteredAssets.length} of ${allAssets.length} repositories`;
    }

    if (filteredAssets.length === 0) {
        listEl.innerHTML = `<div class="p-6 text-center text-white/50 text-sm">No asset repositories match your filter.</div>`;
        return;
    }

    listEl.innerHTML = filteredAssets
        .map((a) => {
            const isSelected = selectedAsset?.slug === a.slug;
            const isEco = a.org === 'intenteco';
            const defaultVar =
                (a.ten_d_variants || []).find((v) => v.id === a.default_variant_id) ||
                (a.ten_d_variants || [])[0];
            const streamBytes = defaultVar ? formatBytes(defaultVar.bytes) : '—';
            const hmcBytes = a.hmc_container ? formatBytes(a.hmc_container.bytes) : '—';
            const tris = (a.geometry?.triangles || 0).toLocaleString();

            return `
            <article
                data-asset-slug="${a.slug}"
                class="asset-card cursor-pointer rounded-2xl p-3.5 border transition flex gap-3.5 items-start ${
                    isSelected
                        ? 'bg-emerald-500/15 border-emerald-400/60 shadow-lg shadow-emerald-950/50'
                        : 'bg-slate-900/65 border-white/10 hover:border-white/25 hover:bg-slate-900/90'
                }">
                <div class="w-16 h-16 rounded-xl bg-black/60 border border-white/10 shrink-0 overflow-hidden flex items-center justify-center">
                    ${
                        a.preview_image
                            ? `<img src="${a.preview_image}" alt="${a.title}" loading="lazy" class="w-full h-full object-cover">`
                            : `<i class="fa-solid ${isEco ? 'fa-seedling text-emerald-400' : 'fa-cubes text-cyan-400'} text-xl"></i>`
                    }
                </div>
                <div class="min-w-0 flex-1">
                    <div class="flex items-center justify-between gap-2 mb-1">
                        <span class="px-2 py-0.5 rounded-md text-[10px] font-semibold ${
                            isEco
                                ? 'bg-emerald-500/15 text-emerald-300 border border-emerald-500/30'
                                : 'bg-cyan-500/15 text-cyan-300 border border-cyan-500/30'
                        }">${a.org}</span>
                        <span class="text-[10px] font-mono text-white/45">${a.category_short}</span>
                    </div>
                    <h3 class="text-sm font-semibold text-white truncate">${a.title}</h3>
                    <p class="text-xs text-white/55 font-mono truncate">${a.slug}</p>
                    <div class="flex flex-wrap items-center gap-2 mt-2 text-[11px] text-white/60 font-mono">
                        <span><i class="fa-solid fa-bolt text-emerald-400 mr-1"></i>10D: ${streamBytes}</span>
                        <span>·</span>
                        <span>HMC: ${hmcBytes}</span>
                        <span>·</span>
                        <span>${tris} tris</span>
                    </div>
                </div>
            </article>`;
        })
        .join('');

    listEl.querySelectorAll('[data-asset-slug]').forEach((card) => {
        card.addEventListener('click', () => {
            const slug = card.getAttribute('data-asset-slug');
            const asset = allAssets.find((x) => x.slug === slug);
            if (asset) {
                selectAsset(asset);
            }
        });
    });
}

function applyFilters() {
    const q = ($('filter-search')?.value || '').trim().toLowerCase();
    const orgFilter = $('filter-org')?.value || 'all';
    const catFilter = $('filter-category')?.value || 'all';

    filteredAssets = allAssets.filter((a) => {
        if (orgFilter !== 'all' && a.org !== orgFilter) return false;
        if (catFilter !== 'all' && a.category_short !== catFilter) return false;
        if (q) {
            const hay = `${a.slug} ${a.title} ${a.brief} ${a.did} ${a.work_packet} ${a.category}`.toLowerCase();
            if (!hay.includes(q)) return false;
        }
        return true;
    });

    renderAssetRoster();
}

function selectAsset(asset) {
    selectedAsset = asset;
    selectedVariantId = asset.default_variant_id;
    renderAssetRoster();
    void streamVariantIntoPortal(asset, selectedVariantId);
}

async function initPortalViewport() {
    ensureCrossOriginIsolation({ quiet: true }).catch(() => {});
    let canvas = $('portal-showcase-canvas');
    const wrap = $('portal-canvas-wrap');
    if (!canvas) throw new Error('#portal-showcase-canvas missing');

    ensureCanvasBackingStore(canvas, wrap?.clientWidth || 720, wrap?.clientHeight || 480);

    const t = debugTime('initPortalViewport');
    const loaded = await loadQualiaPortal(canvas, { allowWebGl2: true });
    canvas = loaded.canvas || canvas;
    activeCanvas = canvas;
    qualiaPortal = loaded.portal;
    portalMod = loaded.mod;
    portalRenderer = loaded.renderer;

    const badgeDot = $('portal-wasm-dot');
    const badgeText = $('portal-wasm-text');

    if (qualiaPortal) {
        portalReady = true;
        qualiaPortal.set_display_mode?.('hybrid');
        qualiaPortal.set_ambient_enabled?.(false);
        qualiaPortal.set_sky_preset?.(1); // Daylight default

        const ro = new ResizeObserver(() => {
            if (!activeCanvas || !qualiaPortal) return;
            const w = Math.max(wrap?.clientWidth || 640, 320);
            const h = Math.max(wrap?.clientHeight || 460, 280);
            ensureCanvasBackingStore(activeCanvas, w, h);
            qualiaPortal.resize?.(activeCanvas, activeCanvas.width, activeCanvas.height);
        });
        if (wrap) ro.observe(wrap);

        bindCanvasControls(activeCanvas);
        startPortalLoop(
            activeCanvas,
            () => {},
            (nextCanvas) => {
                activeCanvas = nextCanvas;
                bindCanvasControls(activeCanvas);
            },
        );

        const tier = qualiaPortal.tier?.() ?? 2;
        const rName = portalRenderer ? portalRenderer.toUpperCase() : `Tier ${tier}`;
        if (badgeDot) badgeDot.className = 'w-2 h-2 rounded-full bg-emerald-400 animate-pulse';
        if (badgeText) badgeText.textContent = `QualiaPortal WASM · ${rName}`;
        t.end({ tier, renderer: portalRenderer });
    } else {
        if (badgeDot) badgeDot.className = 'w-2 h-2 rounded-full bg-amber-400';
        if (badgeText) badgeText.textContent = 'Qualia Engine WASM (Headless)';
    }
}

function wireToolbarEvents() {
    $('filter-search')?.addEventListener('input', applyFilters);
    $('filter-org')?.addEventListener('change', applyFilters);
    $('filter-category')?.addEventListener('change', applyFilters);

    $('stream-source-select')?.addEventListener('change', (e) => {
        streamSourcePolicy = e.target.value || 'auto';
        if (selectedAsset) {
            void streamVariantIntoPortal(selectedAsset, selectedVariantId);
        }
    });

    $('btn-reset-camera')?.addEventListener('click', () => {
        if (selectedAsset) resetCameraForAsset(selectedAsset);
    });

    $('btn-turntable')?.addEventListener('click', () => {
        if (!qualiaPortal) return;
        turntableActive = !turntableActive;
        const btn = $('btn-turntable');
        if (turntableActive) {
            qualiaPortal.animate_artefact?.('revolute', 0, 1, 0, 0.55);
            btn?.classList.add('bg-emerald-500/25', 'text-emerald-300', 'border-emerald-400/50');
        } else {
            qualiaPortal.stop_artefact_animation?.();
            btn?.classList.remove('bg-emerald-500/25', 'text-emerald-300', 'border-emerald-400/50');
        }
    });

    $('select-sky-preset')?.addEventListener('change', (e) => {
        const preset = Number(e.target.value) || 0;
        qualiaPortal?.set_sky_preset?.(preset);
    });

    $('select-display-mode')?.addEventListener('change', (e) => {
        const mode = e.target.value || 'hybrid';
        qualiaPortal?.set_display_mode?.(mode);
        if (mode === 'tensor') {
            qualiaPortal?.set_ambient_enabled?.(true);
            qualiaPortal?.set_tensor_projection_enabled?.(true);
        } else {
            qualiaPortal?.set_ambient_enabled?.(false);
        }
    });

    $('btn-scene-composer')?.addEventListener('click', () => {
        sceneComposerMode = !sceneComposerMode;
        const btn = $('btn-scene-composer');
        if (sceneComposerMode) {
            btn?.classList.add('bg-cyan-500/25', 'text-cyan-200', 'border-cyan-400/60');
            setHudStatus('Multi-Asset Scene Composer active: click any asset in the roster to stream & add it to the current scene.', 'ok');
        } else {
            composedSceneAssets = [];
            updateSceneComposerTray();
            btn?.classList.remove('bg-cyan-500/25', 'text-cyan-200', 'border-cyan-400/60');
            if (selectedAsset) void streamVariantIntoPortal(selectedAsset, selectedVariantId);
        }
    });

    $('btn-clear-scene')?.addEventListener('click', () => {
        composedSceneAssets = [];
        updateSceneComposerTray();
        if (selectedAsset) void streamVariantIntoPortal(selectedAsset, selectedVariantId);
    });

    const fileInput = $('local-asset-file');
    fileInput?.addEventListener('change', (e) => {
        const file = e.target.files?.[0];
        if (file) void handleLocalFileUpload(file);
    });

    const wrap = $('portal-canvas-wrap');
    if (wrap) {
        wrap.addEventListener('dragover', (e) => {
            e.preventDefault();
            wrap.classList.add('ring-2', 'ring-emerald-400');
        });
        wrap.addEventListener('dragleave', () => {
            wrap.classList.remove('ring-2', 'ring-emerald-400');
        });
        wrap.addEventListener('drop', (e) => {
            e.preventDefault();
            wrap.classList.remove('ring-2', 'ring-emerald-400');
            const file = e.dataTransfer?.files?.[0];
            if (file) void handleLocalFileUpload(file);
        });
    }
}

async function boot() {
    debugEnv({ page: 'assets-showcase' });
    wireToolbarEvents();

    // Load catalog and initialize QualiaPortal in parallel
    const [catData] = await Promise.all([
        fetch(CATALOG_URL).then((r) => {
            if (!r.ok) throw new Error(`Failed to fetch ${CATALOG_URL}: HTTP ${r.status}`);
            return r.json();
        }),
        initPortalViewport().catch((err) => {
            debugError('QualiaPortal init warning:', err);
            setHudStatus(`QualiaPortal initialization note: ${err.message}`, 'warn');
        }),
    ]);

    catalog = catData;
    allAssets = catalog.assets || [];
    filteredAssets = [...allAssets];

    // Populate top summary counters
    if ($('summary-total-repos')) $('summary-total-repos').textContent = String(catalog.summary?.total_assets || allAssets.length);
    if ($('summary-eco-repos')) $('summary-eco-repos').textContent = String(catalog.summary?.intenteco_biosphere_count || 39);
    if ($('summary-poiesis-repos')) $('summary-poiesis-repos').textContent = String(catalog.summary?.qualia_poiesis_made_world_count || 14);
    if ($('summary-total-tris')) $('summary-total-tris').textContent = (catalog.summary?.total_triangles || 0).toLocaleString();
    if ($('summary-total-hmc')) $('summary-total-hmc').textContent = formatBytes(catalog.summary?.total_hmc_bytes || 0);

    renderAssetRoster();

    // Check URL query param `?asset=<slug>` or default to rover-field-utility-4wd / acacia-pycnantha
    const params = new URLSearchParams(location.search);
    const requestedSlug = params.get('asset');
    const initialAsset =
        allAssets.find((a) => a.slug === requestedSlug) ||
        allAssets.find((a) => a.slug === 'rover-field-utility-4wd') ||
        allAssets.find((a) => a.slug === 'acacia-pycnantha') ||
        allAssets[0];

    if (initialAsset) {
        selectAsset(initialAsset);
    }
}

document.addEventListener('DOMContentLoaded', boot);
