/**
 * Durable fetch for the Qualia WASM binary on GitHub Pages.
 *
 * Pages/Fastly returns 502/503/504 on a cold 4 MiB object when the
 * request is `cache: 'no-store'` (forces origin every time). Use a
 * versioned URL so the browser and CDN can keep the 600s Pages cache,
 * and retry transient statuses / network errors.
 */

export const WASM_ASSET_VERSION = '0.0.34-wasm-retry1';

const TRANSIENT_STATUS = new Set([408, 425, 429, 500, 502, 503, 504]);

function pageBase() {
    if (typeof location !== 'undefined' && location.href) return location.href;
    return 'https://mediaprophet.github.io/qualiaDB/';
}

/** Resolve `url` and add `?v=` when the caller did not already pin one. */
export function withAssetVersion(url, version = WASM_ASSET_VERSION) {
    const resolved = url instanceof URL ? new URL(url.href) : new URL(String(url), pageBase());
    if (!resolved.searchParams.has('v')) {
        resolved.searchParams.set('v', version);
    }
    return resolved;
}

export function isTransientHttpStatus(status) {
    return TRANSIENT_STATUS.has(Number(status));
}

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

function retryDelayMs(attempt, opts) {
    const spec = opts.delayMs;
    if (typeof spec === 'function') return Number(spec(attempt)) || 0;
    if (spec != null) return Number(spec) || 0;
    return Math.min(4000, 250 * (2 ** (attempt - 1)));
}

function asFetchError(err, url) {
    if (err && typeof err === 'object' && err.status) {
        if (!err.url) err.url = url;
        return err;
    }
    const wrapped = new Error(err?.message || String(err));
    wrapped.url = url;
    if (err?.name) wrapped.cause = err;
    return wrapped;
}

/**
 * Fetch a WASM (or other large Pages) binary with version pin + transient retry.
 * Does not pass `cache: 'no-store'` — that is what produced the live 503.
 *
 * @param {string|URL} url
 * @param {object} [opts]
 * @param {number} [opts.attempts=5]
 * @param {string} [opts.version]
 * @param {AbortSignal} [opts.signal]
 * @param {(attempt: number, attempts: number, lastError: Error|null) => void} [opts.onAttempt]
 * @param {number|((attempt: number) => number)} [opts.delayMs]
 * @returns {Promise<Response>}
 */
export async function fetchWasmBinary(url, opts = {}) {
    const attempts = Math.max(1, opts.attempts ?? 5);
    const version = opts.version ?? WASM_ASSET_VERSION;
    const signal = opts.signal;
    const onAttempt = opts.onAttempt;
    const target = withAssetVersion(url, version);
    let lastError = null;

    for (let attempt = 1; attempt <= attempts; attempt++) {
        onAttempt?.(attempt, attempts, lastError);
        try {
            const response = await fetch(target.href, { signal });
            if (response.ok) return response;
            lastError = Object.assign(new Error(`WASM fetch failed: ${response.status}`), {
                status: response.status,
                url: target.href,
            });
            if (!isTransientHttpStatus(response.status) || attempt === attempts) {
                throw lastError;
            }
        } catch (err) {
            if (err?.name === 'AbortError') throw err;
            if (err?.status && !isTransientHttpStatus(err.status)) throw err;
            lastError = asFetchError(err, target.href);
            if (attempt === attempts) throw lastError;
        }
        const delay = retryDelayMs(attempt, opts);
        if (delay > 0) await sleep(delay);
    }
    throw lastError || new Error(`WASM fetch failed: ${target.href}`);
}
