// Lazy WASM initialisation - loads once, caches, shares across all test suites.
// Import path is relative to this file (docs/tests/), so playground is ../playground/.

let _mod = null;
let _initPromise = null;
let _coverage = null;
let _wasmVersion = null;

const EXPECTED_WASM_EXPORTS = [
    'compile_query_to_json',
    'execute_ntriples_query',
    'align_sequences_wasm',
    'validate_fasta_wasm',
    'compute_framingham_risk_wasm',
    'validate_fhir_observation_wasm',
    'check_drug_interactions_wasm',
    'compute_molecular_descriptors_wasm',
    'evaluate_lipinski_wasm',
    'detect_functional_groups_wasm',
    'compute_reaction_metrics_wasm',
    'compute_thermochemistry_wasm',
    'validate_shacl_constraint_wasm',
    'run_semantic_simulation',
    'predict_receptor_binding_wasm',
    'compute_storage_profile_wasm',
    'estimate_storage_size_wasm',
    'list_storage_tiers_wasm',
    'recommend_storage_tier_wasm',
    'list_resource_plugins_wasm',
    'load_resource_catalog_wasm',
    'catalog_summary_wasm',
    'search_resource_catalog_wasm',
];

function summarizeCoverage(mod) {
    const available = [];
    const missing = [];

    for (const name of EXPECTED_WASM_EXPORTS) {
        if (typeof mod?.[name] === 'function') available.push(name);
        else missing.push(name);
    }

    return {
        expected: EXPECTED_WASM_EXPORTS.length,
        available: available.length,
        missing,
        ready: available.length > 0,
        partial: available.length > 0 && missing.length > 0,
    };
}

export async function loadWasm() {
    if (_mod) return _mod;
    if (_initPromise) return _initPromise;

    _initPromise = (async () => {
        try {
            const module = await import('../playground/qualia_core_db.js');
            const response = await fetch('../playground/qualia_core_db_bg.wasm');
            const total = parseInt(response.headers.get('content-length'), 10) || 465124;
            let loaded = 0;
            const badge = document.getElementById('wasm-badge');

            const { readable, writable } = new TransformStream({
                transform(chunk, controller) {
                    loaded += chunk.length;
                    if (badge) {
                        const pct = Math.min(99, Math.round((loaded / total) * 100));
                        badge.textContent = `Loading WASM ${pct}%`;
                    }
                    controller.enqueue(chunk);
                }
            });
            response.body.pipeTo(writable);

            const trackedResponse = new Response(readable, { headers: response.headers });
            await module.default(trackedResponse);
            _mod = module;
            _wasmVersion = typeof module.get_engine_version === 'function'
                ? module.get_engine_version()
                : null;
            _coverage = summarizeCoverage(module);
        } catch (e) {
            console.warn('[wasm-loader] WASM init failed:', e.message);
            _mod = {};
            _coverage = summarizeCoverage(_mod);
        }
        return _mod;
    })();

    return _initPromise;
}

export async function getWasmCoverage() {
    if (_coverage) return _coverage;
    const mod = await loadWasm();
    if (!_coverage) _coverage = summarizeCoverage(mod);
    return _coverage;
}

export async function getWasmVersion() {
    if (_wasmVersion) return _wasmVersion;
    const mod = await loadWasm();
    if (!_wasmVersion && typeof mod?.get_engine_version === 'function') {
        _wasmVersion = mod.get_engine_version();
    }
    return _wasmVersion;
}

// Convenience: call fn(mod) only if fn exists in the module, else skip.
export function wasmFn(mod, name) {
    const fn = mod[name];
    if (typeof fn !== 'function') return null;
    return fn;
}
