// Qualia-DB Benchmark Worker (module worker)
// Imports the real wasm-pack WASM build and runs timed iterations of
// compile_query_to_json using performance.now().
// No mocks. No hardcoded values. Every timing comes from a real call.

import init, { compile_query_to_json } from '../playground/qualia_core_db.js';

// Kick off WASM initialisation immediately when the worker module loads.
// BENCH messages await this promise before running, so the first bench
// does NOT pay cold-init cost unless the test type is 'coldstart'.
const wasmReady = init();

// ── Queries exercised per test type ──────────────────────────────────────────
// Each is a string or (i: number) => string.
// compile_query_to_json exercises the full QueryCompiler + SentinelCompiler
// pipeline: tokenisation → AST → FNV-hashed Quin plan → Sentinel bytecode.

const QUERIES = {
    point:     (i) => `SELECT * WHERE { <http://example.org/e${i % 1000}> ?p ?o }`,
    twohop:    (i) => `SELECT ?z WHERE { <http://example.org/e${i % 500}> <http://schema.org/knows> ?y . ?y <http://schema.org/knows> ?z }`,
    filter:    (i) => `SELECT ?s WHERE { ?s a <http://schema.org/Person> . FILTER(?s > <http://example.org/e${i}>) }`,
    ingestion: (i) => `INSERT DATA { <http://example.org/s${i}> <http://example.org/p${i % 5}> "${i}" }`,
    recursive: ()  => `@prefix : <http://example.org/> .\n{ :a :delegates :b . :b :delegates :c } => { :a :transitive-delegates :c } .`,
    coldstart: ()  => `SELECT * WHERE { ?s ?p ?o } LIMIT 1`,
    jitter:    (i) => `SELECT ?s WHERE { ?s <http://schema.org/name> "Entity${i % 200}" }`,
    crdt:      (i) => `SELECT ?s ?p ?o WHERE { ?s ?p ?o . FILTER(?s > <http://example.org/e${i % 100}>) }`,
    intercept: ()  => `@prefix q: <http://qualia-db.org/> .\n{ ?vec q:bound ?b . ?b q:exceeds q:local } => { ?vec q:clipped true } .`,
    escrow:    ()  => `@prefix : <http://example.org/> .\n{ ?asset :cost ?c . ?payment :amount ?a . ?a :gte ?c } => { ?asset :licenseShifted true } .`,
    provenance:(i) => `SELECT ?creator WHERE { <http://example.org/commit${i % 200}> <http://purl.org/dc/terms/creator> ?creator }`,
    nym:       (i) => `@prefix : <http://example.org/> .\n{ ?user :nym ?id . ?id :partition ${i % 16} } => { ?user :isolated true } .`,
};

// ── Statistics ────────────────────────────────────────────────────────────────

function stats(samples) {
    samples.sort((a, b) => a - b);
    const n = samples.length;
    const mean = samples.reduce((a, b) => a + b, 0) / n;
    const variance = samples.reduce((s, x) => s + (x - mean) ** 2, 0) / n;
    return {
        min:  +samples[0].toFixed(4),
        p50:  +samples[Math.floor(n * 0.50)].toFixed(4),
        p95:  +samples[Math.floor(n * 0.95)].toFixed(4),
        max:  +samples[n - 1].toFixed(4),
        mean: +mean.toFixed(4),
        stddev: +Math.sqrt(variance).toFixed(4),
        n,
    };
}

// ── Main bench runner ─────────────────────────────────────────────────────────

function runBench(testType, N) {
    const qFn = QUERIES[testType] ?? QUERIES.point;
    const samples = [];
    for (let i = 0; i < N; i++) {
        const q = typeof qFn === 'function' ? qFn(i) : qFn;
        const t0 = performance.now();
        compile_query_to_json(q);
        samples.push(performance.now() - t0);
    }
    return stats(samples);
}

// Cold-start: first call latency only (no warm-up).
function runColdStart() {
    const q = QUERIES.coldstart();
    const t0 = performance.now();
    compile_query_to_json(q);
    const first = performance.now() - t0;
    // Then measure warm (p50 of 20 subsequent calls).
    const warmSamples = [];
    for (let i = 0; i < 20; i++) {
        const t = performance.now();
        compile_query_to_json(q);
        warmSamples.push(performance.now() - t);
    }
    warmSamples.sort((a, b) => a - b);
    return {
        cold_ms: +first.toFixed(4),
        warm_p50_ms: +warmSamples[Math.floor(warmSamples.length * 0.50)].toFixed(4),
        n: 21,
    };
}

// Jitter: real stddev over 50 iterations.
function runJitter() {
    const s = stats(Array.from({ length: 50 }, (_, i) => {
        const t0 = performance.now();
        compile_query_to_json(QUERIES.jitter(i));
        return performance.now() - t0;
    }));
    return s;
}

// ── Message handler ───────────────────────────────────────────────────────────

self.onmessage = async (e) => {
    const { type, payload } = e.data;

    if (type === 'INIT') {
        try {
            await wasmReady;
            const smoke = compile_query_to_json('SELECT * WHERE { ?s ?p ?o } LIMIT 1');
            postMessage({ type: 'INIT_SUCCESS', smoke });
        } catch (err) {
            postMessage({ type: 'INIT_ERROR', error: String(err) });
        }
        return;
    }

    if (type === 'BENCH') {
        const { testType, iterations = 50 } = payload;
        try {
            await wasmReady;
            let result;
            if (testType === 'coldstart') {
                result = runColdStart();
            } else if (testType === 'jitter') {
                result = runJitter();
            } else {
                result = runBench(testType, iterations);
            }
            postMessage({ type: 'BENCH_RESULT', testType, result });
        } catch (err) {
            postMessage({ type: 'BENCH_ERROR', testType, error: String(err) });
        }
        return;
    }

    console.warn('[QualiaWorker] Unknown message type:', type);
};
