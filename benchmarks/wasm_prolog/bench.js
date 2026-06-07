/**
 * WASM-Prolog (tau-prolog) benchmark — Qualia-DB comparative harness.
 *
 * Translates the synthetic N-Triples graph into Prolog facts, consults them
 * into a tau-prolog session, then measures point lookup, two-hop path query,
 * and predicate filter.  This directly benchmarks Prolog's backtracking-based
 * search against Qualia-DB's O(1) FNV-indexed hash lookup.
 *
 * Usage:  node bench.js [n_triples]
 * Memory: run with  node --max-old-space-size=512 bench.js  for 512 MB ceiling.
 */

"use strict";

const fs = require("fs");
const pl = require("tau-prolog");

const N       = parseInt(process.argv[2] || "10000", 10);
const WARMUP  = 5;   // fewer warmup/samples — Prolog is slower by design
const SAMPLES = 20;
const NT_PATH = process.env.QUALIA_BENCH_NT_PATH || null;
const QUERIES = (() => {
    try {
        return JSON.parse(process.env.QUALIA_BENCH_QUERIES_JSON || "{}");
    } catch {
        return {};
    }
})();

// ── Generate Prolog facts matching the synthetic N-Triples dataset ────────────
// triple(SubjectIdx, PredicateIdx, ObjectIdx).
// We use integer indices instead of full URIs for performance (URI atoms are
// expensive in Prolog; the lookup semantics are equivalent).
function generateFacts(n) {
    const lines = [];
    for (let i = 0; i < n; i++) {
        const p = i % 5;
        const o = (i * 13) % n;
        lines.push(`triple(s${i}, p${p}, o${o}).`);
    }
    // Two-hop rule: path(X, Z) :- triple(X, _, Y), triple(Y, _, Z).
    lines.push("two_hop(X, Z) :- triple(X, _, Y), triple(Y, _, Z).");
    return lines.join("\n");
}

function loadFactsFromNt(path) {
    const text = fs.readFileSync(path, "utf8");
    const lines = [];
    const tokenIds = new Map();
    let nextId = 0;

    const intern = (token) => {
        if (!tokenIds.has(token)) {
            tokenIds.set(token, `n${nextId++}`);
        }
        return tokenIds.get(token);
    };

    for (const line of text.split(/\r?\n/)) {
        const match = line.match(/^\s*<([^>]+)>\s+<([^>]+)>\s+(.+?)\s+\.\s*$/);
        if (!match) continue;
        const [, subject, predicate, object] = match;
        const s = intern(`<${subject}>`);
        const p = intern(`<${predicate}>`);
        const o = intern(object.trim());
        lines.push(`triple(${s}, ${p}, ${o}).`);
    }

    lines.push("two_hop(X, Z) :- triple(X, _, Y), triple(Y, _, Z).");
    return { facts: lines.join("\n"), tokenIds };
}

// ── Timing helper (synchronous — tau-prolog is a sync JS interpreter) ─────────
function timeMs(fn) {
    const t0 = process.hrtime.bigint();
    fn();
    return Number(process.hrtime.bigint() - t0) / 1e6;
}

function latencyStats(fn, warmup = WARMUP, samples = SAMPLES) {
    for (let i = 0; i < warmup; i++) fn();
    const times = Array.from({ length: samples }, () => timeMs(fn));
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

// ── Run a query and collect the first answer (synchronous) ────────────────────
function queryFirst(session, goal) {
    let answer = undefined;
    session.query(goal + ".");
    session.answer((a) => { answer = a; });
    return answer;
}

// ── Main ──────────────────────────────────────────────────────────────────────
const result = { engine: "wasm_prolog", n_triples: N };

// Ingestion — consult all facts into the tau-prolog session
const SESSION_LIMIT = Math.max(1_000_000, N * 20);
const session = pl.create(SESSION_LIMIT);

const t0 = process.hrtime.bigint();
let consultOk = false;
const loaded = NT_PATH ? loadFactsFromNt(NT_PATH) : { facts: generateFacts(N), tokenIds: null };
session.consult(loaded.facts, {
    success: () => { consultOk = true; },
    error:   (e) => { result.error = `consult failed: ${JSON.stringify(e)}`; },
});
result.ingestion_ms = Math.round(Number(process.hrtime.bigint() - t0) / 1e4) / 100;

if (!consultOk) {
    process.stdout.write(JSON.stringify(result, null, 2) + "\n");
    process.exit(0);
}

// Point lookup — find all predicates/objects for subject s0
const pointToken = loaded.tokenIds?.get(`<${QUERIES.point_subject}>`) || "s0";
const twohopToken = loaded.tokenIds?.get(`<${QUERIES.twohop_start}>`) || "s0";
const filterToken = loaded.tokenIds?.get(`<${QUERIES.filter_predicate}>`) || "p0";

result.point = latencyStats(() => queryFirst(session, `triple(${pointToken}, P, O)`));

// Two-hop traversal — first result of path(s0, Z)
result.twohop = latencyStats(() => queryFirst(session, `two_hop(${twohopToken}, Z)`));

// Predicate filter — all subjects with predicate p0
result.filter = latencyStats(() => queryFirst(session, `triple(S, ${filterToken}, O)`));

process.stdout.write(JSON.stringify(result, null, 2) + "\n");
