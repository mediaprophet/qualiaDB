// HTTP client for the Qualia native daemon (localhost:4242).
// Mirrors the fetch logic in playground/playground.js.

const DEFAULT_BASE  = 'http://127.0.0.1:4242';
const DEFAULT_TOKEN_KEY = 'qualia_x_token';

function makeSignal(ms) {
    if (typeof AbortSignal.timeout === 'function') return AbortSignal.timeout(ms);
    const c = new AbortController();
    setTimeout(() => c.abort(), ms);
    return c.signal;
}

export class NativeClient {
    constructor(base = DEFAULT_BASE, token = '') {
        this.base  = base;
        this.token = token || (typeof localStorage !== 'undefined'
            ? (localStorage.getItem(DEFAULT_TOKEN_KEY) || '')
            : '');
    }

    _headers(extra = {}) {
        const h = { 'Content-Type': 'application/json', ...extra };
        if (this.token) h['X-Qualia-Token'] = this.token;
        return h;
    }

    async health(timeoutMs = 1500) {
        const r = await fetch(`${this.base}/health`, { signal: makeSignal(timeoutMs) });
        const body = await r.json();
        return { ok: r.ok, status: r.status, body };
    }

    async query(query, format = 'json-ld', timeoutMs = 5000) {
        const accept = format === 'n-triples'
            ? 'application/n-triples'
            : 'application/ld+json';
        const r = await fetch(`${this.base}/query`, {
            method: 'POST',
            headers: this._headers({ Accept: accept }),
            signal: makeSignal(timeoutMs),
            body: JSON.stringify({ query, format }),
        });
        const computeCost = r.headers.get('X-Qualia-Compute-Cost');
        let body = null;
        try { body = await r.json(); } catch (_) {}
        return { ok: r.ok, status: r.status, body, computeCost };
    }

    async queryText(query, timeoutMs = 5000) {
        const r = await fetch(`${this.base}/query`, {
            method: 'POST',
            headers: this._headers({ Accept: 'application/n-triples' }),
            signal: makeSignal(timeoutMs),
            body: JSON.stringify({ query, format: 'n-triples' }),
        });
        const computeCost = r.headers.get('X-Qualia-Compute-Cost');
        const text = await r.text();
        return { ok: r.ok, status: r.status, text, computeCost };
    }
}

// ─── Mode detection ───────────────────────────────────────────────────────────

export async function detectModes() {
    const result = {
        wasm:     false,
        native:   false,
        token:    '',
        isMobile: /Mobi|Android|iPhone|iPad/i.test(navigator.userAgent),
        daemonVersion: null,
    };

    // Detect WASM
    try {
        const { loadWasm } = await import('./wasm-loader.js');
        const mod = await loadWasm();
        result.wasm = typeof mod.execute_ntriples_query === 'function'
            || typeof mod.compile_query_to_json === 'function';
    } catch (_) {}

    // Detect native daemon
    try {
        const token = typeof localStorage !== 'undefined'
            ? (localStorage.getItem(DEFAULT_TOKEN_KEY) || '')
            : '';
        result.token = token;
        const resp = await fetch(`${DEFAULT_BASE}/health`, { signal: makeSignal(800) });
        if (resp.ok) {
            result.native = true;
            const body = await resp.json();
            result.daemonVersion = body.version || null;
        }
    } catch (_) {}

    return result;
}
