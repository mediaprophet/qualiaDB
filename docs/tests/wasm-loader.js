// Lazy WASM initialisation — loads once, caches, shares across all test suites.
// Import path is relative to this file (docs/tests/), so playground is ../playground/.

let _mod = null;
let _initPromise = null;

export async function loadWasm() {
    if (_mod) return _mod;
    if (_initPromise) return _initPromise;

    _initPromise = (async () => {
        try {
            const module = await import('../playground/qualia_core_db.js');
            await module.default(); // call init()
            _mod = module;
        } catch (e) {
            console.warn('[wasm-loader] WASM init failed:', e.message);
            _mod = {};  // empty — tests will see missing functions and skip
        }
        return _mod;
    })();

    return _initPromise;
}

// Convenience: call fn(mod) only if fn exists in the module, else skip.
export function wasmFn(mod, name) {
    const fn = mod[name];
    if (typeof fn !== 'function') return null;
    return fn;
}
