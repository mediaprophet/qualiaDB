/**
 * Comunica SPARQL benchmark — Qualia-DB comparative harness.
 *
 * Loads the synthetic N-Triples graph into an N3.js in-memory store,
 * then measures point lookup, two-hop traversal, and predicate filter
 * via Comunica's QueryEngine.  Results are written as JSON to stdout.
 *
 * Usage:  node bench.mjs [n_triples]
 * Memory: run with  node --max-old-space-size=512 bench.mjs  to enforce 512 MB ceiling.
 */

import { QueryEngine } from "@comunica/query-sparql";
import { Store, Parser } from "n3";
import fs from "node:fs";

const N       = parseInt(process.argv[2] ?? "10000", 10);
const WARMUP  = 10;
const SAMPLES = 50;
const NT_PATH = process.env.QUALIA_BENCH_NT_PATH || null;
const QUERIES = (() => {
    try {
        return JSON.parse(process.env.QUALIA_BENCH_QUERIES_JSON || "{}");
    } catch {
        return {};
    }
})();

// ── Synthetic N-Triples dataset (same structure as common.py) ─────────────────
function generateNT(n) {
    const lines = [];
    for (let i = 0; i < n; i++) {
        const p = i % 5;
        const o = (i * 13) % n;
        lines.push(`<http://q.test/s/${i}> <http://q.test/p/${p}> <http://q.test/o/${o}> .`);
    }
    return lines.join("\n");
}

// ── Timing helpers ────────────────────────────────────────────────────────────
async function latencyStatsMs(fn, warmup = WARMUP, samples = SAMPLES) {
    for (let i = 0; i < warmup; i++) await fn();
    const times = [];
    for (let i = 0; i < samples; i++) {
        const t0 = process.hrtime.bigint();
        await fn();
        times.push(Number(process.hrtime.bigint() - t0) / 1e6);
    }
    times.sort((a, b) => a - b);
    const mean = times.reduce((s, t) => s + t, 0) / times.length;
    const r = (x) => Math.round(x * 1e4) / 1e4;
    return {
        min:  r(times[0]),
        max:  r(times[times.length - 1]),
        mean: r(mean),
        p50:  r(times[Math.floor(times.length * 0.50)]),
        p95:  r(times[Math.floor(times.length * 0.95)]),
        p99:  r(times[Math.floor(times.length * 0.99)]),
        samples, warmup_samples: warmup, unit: "milliseconds",
    };
}

// ── SPARQL queries (identical logical operations to Oxigraph runner) ──────────
const POINT_Q  = "SELECT * WHERE { <http://q.test/s/0> ?p ?o }";
const TWOHOP_Q = "SELECT * WHERE { <http://q.test/s/0> ?p1 ?b . ?b ?p2 ?o . } LIMIT 1";
const FILTER_Q = "SELECT * WHERE { ?s <http://q.test/p/0> ?o } LIMIT 100";

function loadNT() {
    if (NT_PATH) {
        return fs.readFileSync(NT_PATH, "utf8");
    }
    return generateNT(N);
}

async function drainStream(stream) {
    const results = [];
    for await (const binding of stream) { results.push(binding); }
    return results;
}

// ── Main ──────────────────────────────────────────────────────────────────────
(async () => {
    const result = { engine: "comunica", n_triples: N };

    // Ingestion — parse N-Triples into N3.js Store
    const t0 = process.hrtime.bigint();
    let store;
    try {
        store = new Store();
        const parser = new Parser({ format: "N-Triples" });
        store.addQuads(parser.parse(loadNT()));
        result.ingestion_ms = Math.round(Number(process.hrtime.bigint() - t0) / 1e4) / 100;
        result.quad_count = store.size;
    } catch (err) {
        result.error = `ingestion failed: ${err.message}`;
        process.stdout.write(JSON.stringify(result, null, 2) + "\n");
        process.exit(0);
    }

    const engine = new QueryEngine();

    // Point lookup
    try {
        result.point = await latencyStatsMs(async () => {
            const stream = await engine.queryBindings(QUERIES.point || POINT_Q, { sources: [store] });
            await drainStream(stream);
        });
    } catch (err) {
        result.point = `ERROR: ${err.message}`;
    }

    // Two-hop traversal
    try {
        result.twohop = await latencyStatsMs(async () => {
            const stream = await engine.queryBindings(QUERIES.twohop || TWOHOP_Q, { sources: [store] });
            await drainStream(stream);
        });
    } catch (err) {
        result.twohop = `ERROR: ${err.message}`;
    }

    // Predicate filter scan
    try {
        result.filter = await latencyStatsMs(async () => {
            const stream = await engine.queryBindings(QUERIES.filter || FILTER_Q, { sources: [store] });
            await drainStream(stream);
        });
    } catch (err) {
        result.filter = `ERROR: ${err.message}`;
    }

    process.stdout.write(JSON.stringify(result, null, 2) + "\n");
})();
