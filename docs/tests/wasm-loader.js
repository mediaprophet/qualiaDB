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
            const response = await fetch('../playground/qualia_core_db_bg.wasm');
            const total = parseInt(response.headers.get('content-length'), 10) || 465124;
            let loaded = 0;
            const reader = response.body.getReader();
            const chunks = [];
            const badge = document.getElementById('wasm-badge');
            
            while(true) {
                const { done, value } = await reader.read();
                if (done) break;
                chunks.push(value);
                loaded += value.length;
                if (badge) {
                    const pct = Math.round((loaded / total) * 100);
                    badge.textContent = `… Loading WASM ${pct}%`;
                }
            }
            
            const buf = new Uint8Array(loaded);
            let pos = 0;
            for (const c of chunks) { buf.set(c, pos); pos += c.length; }
            await module.default(buf.buffer); // call init()
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
