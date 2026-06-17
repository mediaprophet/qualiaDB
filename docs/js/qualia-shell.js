/**
 * Thin loader for the Qualia WASM portal (Semantic Subjectivity Bifurcation Portal).
 */

import { defaultTelemetry } from './ambient-viz.js';

let portal = null;
let portalModule = null;
let rafId = null;

export const DAEMON_DEFAULT_PORT = 4242;
export const DAEMON_BASE = `http://127.0.0.1:${DAEMON_DEFAULT_PORT}`;

export const DaemonLinkState = {
    OFFLINE: 'offline',
    SLICE_UNAVAILABLE: 'slice_unavailable',
    AUTH_FAILED: 'auth_failed',
    LIVE: 'live',
};

const STANDPOINT_IDENTIFIER = 2;

const TENSOR_HEADER_BYTES = 32;
const TENSOR_MAGIC = 0x5134_322a;

let daemonLinkState = DaemonLinkState.OFFLINE;
const daemonLinkListeners = new Set();

let daemonEventSource = null;
let currentRevision = 0;
let refreshDebounceTimer = null;
const REFRESH_DEBOUNCE_MS = 250;
let sessionNonce = null;
let daemonDevMode = false;
const signingKeyCache = new Map();

/** Canonical f32 formatting — must match `daemon_tensor::format_canonical_f32`. */
function formatCanonicalF32(v) {
    const s = Number(v).toFixed(6);
    return s.replace(/\.?0+$/, '');
}

/** Canonical request string: `"{nonce}|{standpoint_class}|{t_slice}|{t_window}"` */
export function canonicalTensorSlicePayload(nonce, standpointClass, tSlice, tWindow) {
    return `${nonce}|${standpointClass}|${formatCanonicalF32(tSlice)}|${formatCanonicalF32(tWindow)}`;
}

export function generateSessionNonce() {
    const bytes = new Uint8Array(12);
    crypto.getRandomValues(bytes);
    return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

export function getSessionNonce() {
    if (!sessionNonce) {
        sessionNonce = generateSessionNonce();
    }
    return sessionNonce;
}

function bytesToHex(bytes) {
    return Array.from(new Uint8Array(bytes), (b) => b.toString(16).padStart(2, '0')).join('');
}

async function importEd25519SigningKey(seed32) {
    return crypto.subtle.importKey(
        'raw',
        seed32,
        { name: 'Ed25519' },
        false,
        ['sign'],
    );
}

/**
 * Resolve signing key for identifier DID. Dev daemon exposes derived seed via
 * `GET /tensor/dev-signing-key` (localhost pairing only).
 */
async function ensureIdentifierSigningKey(identifierDid, base, devMode) {
    if (!identifierDid) return null;
    const cacheKey = `${base}:${identifierDid}`;
    if (signingKeyCache.has(cacheKey)) {
        return signingKeyCache.get(cacheKey);
    }
    if (!devMode) {
        console.warn('Identifier signing requires dev daemon pairing or provisioned key');
        return null;
    }
    try {
        const res = await fetch(
            `${base}/tensor/dev-signing-key?identifier_did=${encodeURIComponent(identifierDid)}`,
            { signal: AbortSignal.timeout(4000) },
        );
        if (!res.ok) return null;
        const body = await res.json();
        const seedHex = body.signing_key_hex;
        if (!seedHex || seedHex.length !== 64) return null;
        const seed = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            seed[i] = parseInt(seedHex.slice(i * 2, i * 2 + 2), 16);
        }
        const key = await importEd25519SigningKey(seed);
        signingKeyCache.set(cacheKey, key);
        return key;
    } catch (e) {
        console.warn('Failed to provision identifier signing key:', e);
        return null;
    }
}

async function signTensorSliceRequest({
    identifierDid,
    standpointClass,
    tSlice,
    tWindow,
    nonce,
    base,
    devMode,
}) {
    const signingKey = await ensureIdentifierSigningKey(identifierDid, base, devMode);
    if (!signingKey) {
        throw new Error('identifier_signing_key_unavailable');
    }
    const canonical = canonicalTensorSlicePayload(nonce, standpointClass, tSlice, tWindow);
    const sig = await crypto.subtle.sign(
        'Ed25519',
        signingKey,
        new TextEncoder().encode(canonical),
    );
    return bytesToHex(sig);
}

export async function loadQualiaPortal(canvas) {
    try {
        const url = new URL('../pkg/qualia/qualia.js', import.meta.url);
        const mod = await import(url.href);
        await mod.default();
        if (typeof mod.init_panic_hook === 'function') {
            mod.init_panic_hook();
        }
        portalModule = mod;
        portal = new mod.QualiaPortal(canvas);
        return { portal, mod, source: 'qualia-portal' };
    } catch (e) {
        console.warn('Qualia portal pkg not found, falling back to qualia_core_db.wasm', e);
        const mod = await import('../playground/qualia_core_db.js');
        await mod.default();
        portalModule = mod;
        return { portal: null, mod, source: 'qualia-core-db' };
    }
}

export function startPortalLoop(canvas, onFrame) {
    if (!portal) return;
    let last = performance.now();
    const loop = (now) => {
        const dt = Math.min(now - last, 50);
        last = now;
        try {
            portal.tick(canvas, dt);
            onFrame?.(portal.tier(), dt);
        } catch (e) {
            console.error('QualiaPortal tick', e);
        }
        rafId = requestAnimationFrame(loop);
    };
    rafId = requestAnimationFrame(loop);
}

export function stopPortalLoop() {
    if (rafId) cancelAnimationFrame(rafId);
    rafId = null;
}

export function getPortal() {
    return portal;
}

export function getPortalModule() {
    return portalModule;
}

/** Human-Centric observer standpoint (decoupled from camera lens). */
export function setPortalStandpoint(
    standpointClass = 0,
    epistemicQ = 1.0,
    tSlice = 0.5,
    tWindow = 1.0,
    identifierDid = '',
) {
    portal?.set_standpoint?.(standpointClass, epistemicQ, tSlice, tWindow, identifierDid);
}

export function setPortalTemporalScrub(tSlice, tWindow = 0.08) {
    if (!portal?.set_standpoint) return;
    const cls = portal.standpoint_class?.() ?? 0;
    const q = portal.epistemic_q?.() ?? 1.0;
    portal.set_standpoint(cls, q, tSlice, tWindow, '');
}

export async function applyTelemetryFromWasm(portalInstance) {
    const mod = portalModule;
    if (!mod) return null;
    try {
        if (typeof mod.sample_browser_telemetry_wasm === 'function') {
            return mod.sample_browser_telemetry_wasm();
        }
        if (portalInstance?.sample_telemetry) {
            return portalInstance.sample_telemetry();
        }
    } catch (_) { /* ignore */ }
    return null;
}

function telemetryToFloats(partial) {
    const base = defaultTelemetry();
    const merged = { ...base, ...partial };
    return new Float32Array([
        merged.memory_pressure, merged.network_ripple, merged.baking_crystallization,
        merged.logic_flashes, merged.llm_heat, merged.quantum_activity,
        merged.spectral_shift, merged.temporal_pulse, merged.epistemic_density,
        merged.manifold_pressure, 0, 0,
    ]);
}

export function getDaemonLinkState() {
    return daemonLinkState;
}

export function onDaemonLinkState(listener) {
    daemonLinkListeners.add(listener);
    return () => daemonLinkListeners.delete(listener);
}

function setDaemonLinkState(state) {
    if (daemonLinkState === state) return;
    daemonLinkState = state;
    for (const fn of daemonLinkListeners) {
        try { fn(state); } catch (_) { /* ignore */ }
    }
}

function pulseNetworkRipple(portalInstance, amount = 0.85) {
    portalInstance?.set_telemetry?.(telemetryToFloats({ network_ripple: amount }));
}

function decayNetworkRipple(portalInstance) {
    pulseNetworkRipple(portalInstance, 0.06);
}

export function parseTensorNodeCount(arrayBuffer) {
    if (!arrayBuffer || arrayBuffer.byteLength < TENSOR_HEADER_BYTES) return 0;
    const view = new DataView(arrayBuffer);
    if (view.getUint32(0, true) !== TENSOR_MAGIC) return 0;
    return view.getUint32(8, true);
}

/** Probe daemon liveness (`GET /health`). */
export async function probeDaemonHealth(base = DAEMON_BASE, timeoutMs = 2000) {
    try {
        const res = await fetch(`${base}/health`, {
            method: 'GET',
            signal: AbortSignal.timeout(timeoutMs),
        });
        if (!res.ok) return null;
        return await res.json();
    } catch (_) {
        return null;
    }
}

/**
 * Fetch binary Tensor10D SOA (`GET /tensor/slice`) — ArrayBuffer for zero-copy WASM upload.
 */
export async function fetchTensorSliceFromDaemon(options = {}) {
    const {
        base = DAEMON_BASE,
        maxNodes = 12_000,
        tSlice = 0.5,
        tWindow = 1.0,
        standpointClass = 0,
        identifierDid = '',
        lane = standpointClass >= STANDPOINT_IDENTIFIER ? 'identifier' : 'commons',
        timeoutMs = 12_000,
        devMode = false,
    } = options;

    const nonce = options.sessionNonce ?? getSessionNonce();

    const params = new URLSearchParams({
        max_nodes: String(maxNodes),
        t_slice: String(tSlice),
        t_window: String(tWindow),
        lane,
    });

    const headers = {
        Accept: 'application/octet-stream',
        'X-Qualia-Standpoint-Class': String(standpointClass),
        'X-Qualia-T-Slice': String(tSlice),
        'X-Qualia-T-Window': String(tWindow),
        'X-Qualia-Lane': lane,
    };

    if (standpointClass >= STANDPOINT_IDENTIFIER) {
        const did = identifierDid?.trim();
        if (!did) {
            const err = new Error('identifier_did_required');
            err.status = 403;
            err.code = 'identifier_did_required';
            throw err;
        }
        headers['X-Qualia-Session-Nonce'] = nonce;
        headers['X-Qualia-Identifier-Did'] = did;
        headers['X-Qualia-Signature'] = await signTensorSliceRequest({
            identifierDid: did,
            standpointClass,
            tSlice,
            tWindow,
            nonce,
            base,
            devMode,
        });
    }

    const res = await fetch(`${base}/tensor/slice?${params}`, {
        method: 'GET',
        headers,
        signal: AbortSignal.timeout(timeoutMs),
    });
    if (!res.ok) {
        const err = new Error(`tensor/slice HTTP ${res.status}`);
        err.status = res.status;
        if (res.status === 403) {
            try {
                const body = await res.json();
                err.code = body.code ?? 'tensor_slice_auth_failed';
            } catch (_) {
                err.code = 'tensor_slice_auth_failed';
            }
        }
        throw err;
    }
    return res.arrayBuffer();
}

function handleTensorSliceAuthFailure(portalInstance, err) {
    setDaemonLinkState(DaemonLinkState.AUTH_FAILED);
    const tSlice = portalInstance?.t_slice?.() ?? 0.5;
    const tWindow = portalInstance?.t_window?.() ?? 1.0;
    setPortalStandpoint(0, 1.0, tSlice, tWindow, '');
    console.warn('Identifier tensor slice rejected — reset to Spectator:', err?.code ?? err);
}

/**
 * Re-fetch tensor slice and upload to portal (no health probe). Used by SSE revision sync.
 */
export async function refreshTensorSliceFromDaemon(portalInstance, options = {}) {
    const {
        base = DAEMON_BASE,
        maxNodes = 12_000,
        onRefreshed,
    } = options;

    if (!portalInstance?.upload_tensor_buffer) {
        return { nodes: 0, byteLength: 0 };
    }

    pulseNetworkRipple(portalInstance, 0.78);

    const tSlice = portalInstance?.t_slice?.() ?? 0.5;
    const tWindow = portalInstance?.t_window?.() ?? 1.0;
    const standpointClass = portalInstance?.standpoint_class?.() ?? 0;

    let buf;
    try {
        buf = await fetchTensorSliceFromDaemon({
            base,
            maxNodes,
            tSlice,
            tWindow,
            standpointClass,
            identifierDid: options.identifierDid,
            devMode: options.devMode ?? daemonDevMode,
        });
    } catch (e) {
        decayNetworkRipple(portalInstance);
        if (e?.status === 403) {
            handleTensorSliceAuthFailure(portalInstance, e);
        }
        throw e;
    }

    if (!buf || buf.byteLength < TENSOR_HEADER_BYTES) {
        decayNetworkRipple(portalInstance);
        return { nodes: 0, byteLength: 0 };
    }

    const nodes = parseTensorNodeCount(buf);
    portalInstance.upload_tensor_buffer(new Uint8Array(buf));
    lastTensorBufferFromDaemon = buf;
    decayNetworkRipple(portalInstance);
    onRefreshed?.({ nodes, byteLength: buf.byteLength });
    return { nodes, byteLength: buf.byteLength };
}

function debouncedRefreshTensorSlice(portalInstance, options = {}) {
    if (refreshDebounceTimer) clearTimeout(refreshDebounceTimer);
    refreshDebounceTimer = setTimeout(() => {
        refreshDebounceTimer = null;
        refreshTensorSliceFromDaemon(portalInstance, options).catch((e) => {
            if (e?.status !== 403) {
                console.warn('Daemon tensor refresh failed:', e);
            }
        });
    }, REFRESH_DEBOUNCE_MS);
}

/**
 * Subscribe to `GET /tensor/events` SSE; debounced re-fetch on graph revision bumps.
 */
export function startDaemonEventStream(portalInstance, options = {}) {
    stopDaemonEventStream();
    if (!portalInstance) return;

    const base = options.base ?? DAEMON_BASE;
    const url = `${base}/tensor/events`;

    try {
        daemonEventSource = new EventSource(url);
    } catch (e) {
        console.warn('EventSource unavailable:', e);
        return;
    }

    daemonEventSource.onmessage = (ev) => {
        try {
            const payload = JSON.parse(ev.data);
            const newRev = payload.revision;
            if (typeof newRev === 'number' && newRev > currentRevision) {
                currentRevision = newRev;
                debouncedRefreshTensorSlice(portalInstance, options);
            }
        } catch (_) { /* ignore malformed SSE payload */ }
    };

    daemonEventSource.onerror = () => {
        // EventSource reconnects automatically; no badge downgrade on transient drops.
    };
}

export function stopDaemonEventStream() {
    if (daemonEventSource) {
        daemonEventSource.close();
        daemonEventSource = null;
    }
    if (refreshDebounceTimer) {
        clearTimeout(refreshDebounceTimer);
        refreshDebounceTimer = null;
    }
}

export function getDaemonGraphRevision() {
    return currentRevision;
}

/**
 * Probe → ripple → binary fetch → `upload_tensor_buffer`. Badge state machine:
 * Offline → Slice unavailable → Live.
 */
export async function connectPortalToDaemon(portalInstance, options = {}) {
    const {
        base = DAEMON_BASE,
        maxNodes = 12_000,
        onLoaded,
    } = options;

    setDaemonLinkState(DaemonLinkState.OFFLINE);
    stopDaemonEventStream();

    const health = await probeDaemonHealth(base, options.healthTimeoutMs ?? 2000);
    if (!health) {
        setDaemonLinkState(DaemonLinkState.OFFLINE);
        return { state: DaemonLinkState.OFFLINE, health: null, nodes: 0 };
    }
    daemonDevMode = health.dev_mode === true;

    pulseNetworkRipple(portalInstance, 0.92);

    try {
        const tSlice = portalInstance?.t_slice?.() ?? 0.5;
        const tWindow = portalInstance?.t_window?.() ?? 1.0;
        const standpointClass = portalInstance?.standpoint_class?.() ?? 0;

        const identifierDid = options.identifierDid
            ?? document.getElementById('identifier-did')?.value?.trim()
            ?? '';
        const buf = await fetchTensorSliceFromDaemon({
            base,
            maxNodes,
            tSlice,
            tWindow,
            standpointClass,
            identifierDid,
            devMode: daemonDevMode,
        });

        if (!buf || buf.byteLength < TENSOR_HEADER_BYTES) {
            decayNetworkRipple(portalInstance);
            setDaemonLinkState(DaemonLinkState.SLICE_UNAVAILABLE);
            return { state: DaemonLinkState.SLICE_UNAVAILABLE, health, nodes: 0 };
        }

        const nodes = parseTensorNodeCount(buf);
        portalInstance.upload_tensor_buffer(new Uint8Array(buf));
        lastTensorBufferFromDaemon = buf;
        decayNetworkRipple(portalInstance);
        setDaemonLinkState(DaemonLinkState.LIVE);
        currentRevision = health.graph_revision ?? currentRevision;
        startDaemonEventStream(portalInstance, {
            base,
            maxNodes,
            onRefreshed: options.onRefreshed,
            identifierDid,
            devMode: daemonDevMode,
        });
        onLoaded?.({ nodes, byteLength: buf.byteLength, health });
        return { state: DaemonLinkState.LIVE, health, nodes };
    } catch (e) {
        decayNetworkRipple(portalInstance);
        if (e?.status === 403) {
            handleTensorSliceAuthFailure(portalInstance, e);
            return { state: DaemonLinkState.AUTH_FAILED, health, nodes: 0, error: e };
        }
        console.warn('Daemon tensor slice failed:', e);
        setDaemonLinkState(DaemonLinkState.SLICE_UNAVAILABLE);
        return { state: DaemonLinkState.SLICE_UNAVAILABLE, health, nodes: 0, error: e };
    }
}

let lastTensorBufferFromDaemon = null;

export function getLastDaemonTensorBuffer() {
    return lastTensorBufferFromDaemon;
}

export function daemonBadgeLabel(state, tier = 0) {
    const tierLabel = `T${tier}`;
    switch (state) {
        case DaemonLinkState.LIVE:
            return `Qualia WASM · ${tierLabel} · Live`;
        case DaemonLinkState.AUTH_FAILED:
            return `Qualia WASM · ${tierLabel} · Auth Failed`;
        case DaemonLinkState.SLICE_UNAVAILABLE:
            return `Qualia WASM · ${tierLabel} · Slice unavailable`;
        default:
            return `Qualia WASM · ${tierLabel} · Local Edge Only`;
    }
}

let acousticContext = null;
let acousticNode = null;
let acousticSab = null;
let acousticSyncTimer = null;

/**
 * Mount U3 AcousticPlane — binaural stereo + optional SAB zero-copy (COOP/COEP).
 */
export async function mountAcousticPlane(portalInstance = portal, options = {}) {
    const {
        workletUrl = new URL('./qualia-audio-worklet.js', import.meta.url).href,
        syncIntervalMs = 50,
        autoStart = true,
        useSab = true,
    } = options;

    if (!portalInstance) {
        console.warn('mountAcousticPlane: portal not loaded');
        return null;
    }

    const AudioCtx = window.AudioContext || window.webkitAudioContext;
    if (!AudioCtx) return null;

    if (!acousticContext) {
        acousticContext = new AudioCtx({ latencyHint: 'interactive' });
    }
    if (acousticContext.state === 'suspended' && autoStart) {
        await acousticContext.resume();
    }

    const sabOk = useSab && typeof portalInstance.create_acoustic_sab === 'function' && window.crossOriginIsolated;
    if (sabOk) {
        try {
            acousticSab = portalInstance.create_acoustic_sab();
        } catch (e) {
            console.warn('AcousticPlane SAB fallback to MessagePort', e);
            acousticSab = null;
        }
    }

    if (!acousticNode) {
        await acousticContext.audioWorklet.addModule(workletUrl);
        acousticNode = new AudioWorkletNode(acousticContext, 'qualia-acoustic', {
            numberOfOutputs: 1,
            outputChannelCount: [2],
            processorOptions: { sab: acousticSab },
        });
        acousticNode.connect(acousticContext.destination);
        if (acousticSab) {
            acousticNode.port.postMessage({ type: 'sab', buffer: acousticSab });
        }
    }

    let sidecarTick = 0;
    const sync = () => {
        try {
            if (acousticSab && typeof portalInstance.publish_acoustic_sab === 'function') {
                portalInstance.publish_acoustic_sab(acousticSab);
            } else if (typeof portalInstance.acoustic_uniform_floats === 'function') {
                const floats = portalInstance.acoustic_uniform_floats();
                acousticNode.port.postMessage({ type: 'uniform', floats });
            }
            // Refresh STFT preview bins every ~2s (sidecar bake is cold-path)
            if (typeof portalInstance.bake_stft_sidecar_demo === 'function' && (sidecarTick++ % 40) === 0) {
                const sidecar = portalInstance.bake_stft_sidecar_demo(32);
                if (sidecar?.length > 32) {
                    const view = new DataView(sidecar.buffer, sidecar.byteOffset, sidecar.byteLength);
                    const bins = new Float32Array(64);
                    const off = 32;
                    for (let i = 0; i < 64; i++) {
                        bins[i] = view.getFloat32(off + i * 4, true);
                    }
                    acousticNode.port.postMessage({ type: 'sidecar', bins });
                }
            }
            const pending = portalInstance.sonic_token_pending?.() ?? 0;
            if (pending > 0 && typeof portalInstance.drain_sonic_tokens === 'function') {
                const raw = portalInstance.drain_sonic_tokens(Math.min(pending, 16));
                const arr = [];
                for (let i = 0; i < (raw?.length ?? 0); i++) arr.push(Number(raw[i]));
                if (arr.length) acousticNode.port.postMessage({ type: 'tokens', raw: arr });
            }
        } catch (e) {
            console.warn('AcousticPlane sync', e);
        }
    };

    if (acousticSyncTimer) clearInterval(acousticSyncTimer);
    sync();
    acousticSyncTimer = setInterval(sync, syncIntervalMs);
    return { context: acousticContext, node: acousticNode, sab: acousticSab };
}

export function unmountAcousticPlane() {
    if (acousticSyncTimer) {
        clearInterval(acousticSyncTimer);
        acousticSyncTimer = null;
    }
    if (acousticNode) {
        acousticNode.disconnect();
        acousticNode = null;
    }
    if (acousticContext) {
        acousticContext.close().catch(() => {});
        acousticContext = null;
    }
    acousticSab = null;
}

export function setAcousticEnabled(enabled, portalInstance = portal) {
    portalInstance?.set_acoustic_enabled?.(enabled);
    acousticNode?.port.postMessage({ type: 'mute', mute: !enabled });
}

export function updateDaemonBadge(elementId = 'wasm-text', dotId = 'wasm-dot', portalInstance = portal) {
    const textEl = document.getElementById(elementId);
    const dotEl = document.getElementById(dotId);
    if (!textEl) return;

    const tier = portalInstance?.tier?.() ?? 0;
    textEl.textContent = daemonBadgeLabel(daemonLinkState, tier);

    if (dotEl) {
        dotEl.classList.remove('bg-slate-500', 'bg-emerald-500', 'bg-amber-500', 'bg-cyan-500', 'bg-red-500');
        if (daemonLinkState === DaemonLinkState.LIVE) {
            dotEl.classList.add('bg-emerald-500');
        } else if (daemonLinkState === DaemonLinkState.AUTH_FAILED) {
            dotEl.classList.add('bg-red-500');
        } else if (daemonLinkState === DaemonLinkState.SLICE_UNAVAILABLE) {
            dotEl.classList.add('bg-amber-500');
        } else {
            dotEl.classList.add('bg-slate-500');
        }
    }
}