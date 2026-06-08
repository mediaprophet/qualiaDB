/**
 * Shared Qualia WASM runtime loader for docs pages.
 *
 * Usage:
 *   import { initQualiaWasm, getEngineVersion, getEngineInfo, bindEngineBadge }
 *     from './js/qualia-wasm-runtime.js';
 *
 *   const mod = await initQualiaWasm({ base: '.' });
 *   console.log(getEngineVersion(mod)); // "0.0.9"
 */

let _mod = null;
let _initPromise = null;
let _version = null;
let _info = null;

function resolvePaths(opts = {}) {
    const base = (opts.base ?? '.').replace(/\/?$/, '/');
    const jsUrl = opts.jsUrl ?? `${base}playground/qualia_core_db.js`;
    const wasmUrl = opts.wasmUrl ?? `${base}playground/qualia_core_db_bg.wasm`;
    return { jsUrl, wasmUrl };
}

/**
 * Lazy-load and initialise the wasm-pack build.
 * @param {object} [opts]
 * @param {string} [opts.base] — path prefix to docs root (e.g. '.' or '..')
 * @returns {Promise<object>} wasm-bindgen module exports
 */
export async function initQualiaWasm(opts = {}) {
    if (_mod) return _mod;
    if (_initPromise) return _initPromise;

    _initPromise = (async () => {
        try {
            const { jsUrl, wasmUrl } = resolvePaths(opts);
            const module = await import(jsUrl);
            const response = await fetch(wasmUrl);
            if (!response.ok) {
                throw new Error(`WASM fetch failed: ${response.status} ${wasmUrl}`);
            }
            await module.default(response);
            _mod = module;
            _version = readVersion(module);
            _info = readInfo(module);
        } catch (e) {
            console.warn('[qualia-wasm-runtime] init failed:', e.message);
            _mod = {};
        }
        return _mod;
    })();

    return _initPromise;
}

function readVersion(mod) {
    if (typeof mod?.get_engine_version === 'function') {
        return mod.get_engine_version();
    }
    return null;
}

function readInfo(mod) {
    if (typeof mod?.get_engine_info === 'function') {
        try {
            const raw = mod.get_engine_info();
            if (raw && typeof raw === 'object') return raw;
        } catch (_) { /* fall through */ }
    }
    const version = readVersion(mod);
    if (version) {
        return { version, engine: 'qualia-core-db', target: 'wasm32', capabilities: [] };
    }
    return null;
}

/** @returns {string|null} semver string from the loaded WASM module */
export function getEngineVersion(mod = _mod) {
    if (_version) return _version;
    if (mod && Object.keys(mod).length) {
        _version = readVersion(mod);
    }
    return _version;
}

/** @returns {object|null} { version, engine, target, capabilities } */
export function getEngineInfo(mod = _mod) {
    if (_info) return _info;
    if (mod && Object.keys(mod).length) {
        _info = readInfo(mod);
    }
    return _info;
}

/**
 * Populate a DOM element with the WASM engine version after load.
 * @param {string} elementId
 * @param {object} [opts] — passed to initQualiaWasm
 * @param {string} [prefix='WASM v'] — text before version
 */
export async function bindEngineBadge(elementId, opts = {}, prefix = 'WASM v') {
    const el = document.getElementById(elementId);
    if (!el) return null;
    el.textContent = 'Loading WASM…';
    const mod = await initQualiaWasm(opts);
    const ver = getEngineVersion(mod);
    if (ver) {
        el.textContent = `${prefix}${ver}`;
        el.dataset.qualiaEngineVersion = ver;
        el.classList.add('qualia-wasm-ready');
    } else {
        el.textContent = 'WASM unavailable';
        el.classList.add('qualia-wasm-offline');
    }
    return ver;
}

/** Reset cached module (for tests / hot reload). */
export function resetQualiaWasmCache() {
    _mod = null;
    _initPromise = null;
    _version = null;
    _info = null;
}
