/**
 * ICP pairing session — QR payload, storage, minimal canvas QR render.
 */

export const PAIRING_STORAGE_KEY = 'qualia-icp-pairing.v1';
export const PAIRING_VERSION = 1;
export const DEFAULT_PAIR_TTL_SEC = 3600;
export const DEFAULT_RELAY_PORT = 4242;

/** LAN-reachable relay base (uses page hostname so phones can reach the daemon). */
export function defaultRelayBase(port = DEFAULT_RELAY_PORT) {
    if (typeof window === 'undefined') return `http://127.0.0.1:${port}`;
    const host = window.location.hostname || '127.0.0.1';
    return `http://${host}:${port}`;
}

export function randomHex(bytes = 16) {
    const buf = new Uint8Array(bytes);
    crypto.getRandomValues(buf);
    return Array.from(buf, (b) => b.toString(16).padStart(2, '0')).join('');
}

export function randomSessionId() {
    return `icp-${randomHex(12)}`;
}

/**
 * @param {object} [opts]
 * @param {string} [opts.relayBase]
 * @param {string} [opts.origin]
 * @param {number} [opts.ttlSec]
 */
export function createConsoleSession(opts = {}) {
    const origin = opts.origin || (typeof window !== 'undefined' ? window.location.origin : '');
    const relay = (opts.relayBase || defaultRelayBase()).replace(/\/+$/, '');
    const sessionId = opts.sessionId || randomSessionId();
    const expUnix = Math.floor(Date.now() / 1000) + (opts.ttlSec || DEFAULT_PAIR_TTL_SEC);
    return {
        v: PAIRING_VERSION,
        origin,
        relay,
        session_id: sessionId,
        desktop_pubkey: opts.desktopPubkey || '',
        exp_unix: expUnix,
        capabilities: opts.capabilities || ['remote', 'context_push', 'vault_sync'],
        role: 'desktop',
    };
}

/**
 * @param {string|object} input
 * @returns {object|null}
 */
export function parsePairingPayload(input) {
    let obj = input;
    if (typeof input === 'string') {
        const trimmed = input.trim();
        if (!trimmed) return null;
        try {
            obj = JSON.parse(trimmed);
        } catch {
            return null;
        }
    }
    if (!obj || typeof obj !== 'object') return null;
    if (!obj.session_id || !obj.relay) return null;
    if (obj.v && obj.v !== PAIRING_VERSION) return null;
    if (obj.exp_unix && obj.exp_unix < Math.floor(Date.now() / 1000)) return null;
    return {
        v: obj.v || PAIRING_VERSION,
        origin: obj.origin || '',
        relay: String(obj.relay).replace(/\/+$/, ''),
        session_id: String(obj.session_id),
        desktop_pubkey: obj.desktop_pubkey || '',
        exp_unix: obj.exp_unix || 0,
        capabilities: obj.capabilities || ['remote'],
        role: obj.role || 'phone',
    };
}

export function savePairing(payload) {
    const json = JSON.stringify(payload);
    try {
        sessionStorage.setItem(PAIRING_STORAGE_KEY, json);
    } catch {
        // ignore quota errors
    }
    try {
        localStorage.setItem(PAIRING_STORAGE_KEY, json);
    } catch {
        // ignore
    }
}

export function loadPairing() {
    const raw = sessionStorage.getItem(PAIRING_STORAGE_KEY)
        || localStorage.getItem(PAIRING_STORAGE_KEY);
    if (!raw) return null;
    return parsePairingPayload(raw);
}

export function clearPairing() {
    sessionStorage.removeItem(PAIRING_STORAGE_KEY);
    localStorage.removeItem(PAIRING_STORAGE_KEY);
}

export function pairingToJson(payload) {
    return JSON.stringify(payload);
}

/**
 * Minimal QR on canvas via external image API (offline fallback: JSON text).
 * @param {HTMLCanvasElement} canvas
 * @param {object} payload
 */
export function renderPairingQr(canvas, payload) {
    if (!canvas) return;
    const json = pairingToJson(payload);
    const size = Math.min(canvas.width || 200, canvas.height || 200, 220);
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
        ctx.fillStyle = '#ffffff';
        ctx.fillRect(0, 0, size, size);
        ctx.drawImage(img, 0, 0, size, size);
    };
    img.onerror = () => {
        ctx.fillStyle = '#0a0e17';
        ctx.fillRect(0, 0, size, size);
        ctx.fillStyle = '#34d399';
        ctx.font = '10px monospace';
        ctx.textAlign = 'center';
        const lines = [
            'ICP Pair',
            payload.session_id.slice(0, 12) + '…',
            'Paste JSON on phone',
        ];
        lines.forEach((line, i) => ctx.fillText(line, size / 2, 24 + i * 14));
    };
    const encoded = encodeURIComponent(json.slice(0, 1800));
    img.src = `https://api.qrserver.com/v1/create-qr-code/?size=${size}x${size}&data=${encoded}`;
}

/**
 * @param {object} opts
 * @param {HTMLVideoElement} opts.video
 * @param {(payload: object) => void} opts.onPair
 */
export async function startBarcodePairing(opts) {
    const { video, onPair } = opts;
    if (!('BarcodeDetector' in window)) {
        return { supported: false, stop: () => {} };
    }
    const detector = new window.BarcodeDetector({ formats: ['qr_code'] });
    const stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: 'environment' },
        audio: false,
    });
    video.srcObject = stream;
    await video.play();

    let stopped = false;
    const tick = async () => {
        if (stopped) return;
        try {
            const codes = await detector.detect(video);
            for (const code of codes) {
                const payload = parsePairingPayload(code.rawValue);
                if (payload) {
                    savePairing(payload);
                    onPair(payload);
                    stopped = true;
                    stream.getTracks().forEach((t) => t.stop());
                    return;
                }
            }
        } catch {
            // camera frame not ready
        }
        if (!stopped) requestAnimationFrame(tick);
    };
    tick();

    return {
        supported: true,
        stop: () => {
            stopped = true;
            stream.getTracks().forEach((t) => t.stop());
        },
    };
}