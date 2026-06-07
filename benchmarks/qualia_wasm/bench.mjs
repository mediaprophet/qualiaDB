/**
 * Qualia core WASM benchmark — comparative harness.
 *
 * Loads synthetic N-Triples into flat QualiaQuin bytes (48 B each), then measures
 * point / filter via execute_ntriples_query and two-hop via the same subject
 * adjacency index used by q42_comparative_bench.
 *
 * Usage: node --max-old-space-size=512 bench.mjs [n_triples]
 */

import { readFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const __dir = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dir, "..", "..");
const PLAYGROUND = process.env.QUALIA_WASM_PLAYGROUND || join(ROOT, "docs", "playground");

const N = parseInt(process.argv[2] ?? "10000", 10);
const WARMUP = 10;
const SAMPLES = 30;
const NT_PATH = process.env.QUALIA_BENCH_NT_PATH || null;
const QUERIES = (() => {
    try {
        return JSON.parse(process.env.QUALIA_BENCH_QUERIES_JSON || "{}");
    } catch {
        return {};
    }
})();

const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const MASK64 = 0xffffffffffffffffn;
const OBJECT_HASH_MASK = 0x0fffffffffffffffn;

function qHash(s) {
    let hash = FNV_OFFSET;
    for (const b of new TextEncoder().encode(s)) {
        hash ^= BigInt(b);
        hash = (hash * FNV_PRIME) & MASK64;
    }
    return hash;
}

function hashToken(token) {
    if (token.startsWith("<") && token.endsWith(">")) {
        return qHash(token.slice(1, -1));
    }
    if (token.startsWith('"')) {
        const bytes = new TextEncoder().encode(token);
        let i = 1;
        while (i < bytes.length) {
            if (bytes[i] === 0x5c) { i += 2; continue; }
            if (bytes[i] === 0x22) break;
            i += 1;
        }
        return qHash(token.slice(1, i));
    }
    return qHash(token);
}

function generateNT(n) {
    const lines = [];
    for (let i = 0; i < n; i++) {
        const p = i % 5;
        const o = (i * 13) % n;
        lines.push(`<http://q.test/s/${i}> <http://q.test/p/${p}> <http://q.test/o/${o}> .`);
    }
    return lines.join("\n");
}

function encodeQuin(subject, predicate, object) {
    const buf = new Uint8Array(48);
    const dv = new DataView(buf.buffer);
    dv.setBigUint64(0, subject, true);
    dv.setBigUint64(8, predicate, true);
    dv.setBigUint64(16, object, true);
    dv.setBigUint64(24, 0n, true);
    dv.setBigUint64(32, 0n, true);
    dv.setBigUint64(40, 0n, true);
    return buf;
}

function parseNT(text) {
    const quins = [];
    const index = new Map();
    for (const line of text.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith("#")) continue;
        const parts = trimmed.replace(/\s+\.\s*$/, "").split(/\s+/);
        if (parts.length < 3) continue;
        const s = hashToken(parts[0]);
        const p = hashToken(parts[1]);
        const o = hashToken(parts[2]) & OBJECT_HASH_MASK;
        const chunk = encodeQuin(s, p, o);
        quins.push(chunk);
        let bucket = index.get(s);
        if (!bucket) {
            bucket = [];
            index.set(s, bucket);
        }
        bucket.push(o);
    }
    const total = quins.length * 48;
    const db = new Uint8Array(total);
    let off = 0;
    for (const q of quins) {
        db.set(q, off);
        off += 48;
    }
    return { db, index, quinCount: quins.length };
}

function latencyStatsMs(fn, warmup = WARMUP, samples = SAMPLES) {
    for (let i = 0; i < warmup; i++) fn();
    const times = [];
    for (let i = 0; i < samples; i++) {
        const t0 = process.hrtime.bigint();
        fn();
        times.push(Number(process.hrtime.bigint() - t0) / 1e6);
    }
    times.sort((a, b) => a - b);
    const mean = times.reduce((s, t) => s + t, 0) / times.length;
    const r = (x) => Math.round(x * 1e4) / 1e4;
    return {
        min: r(times[0]),
        max: r(times[times.length - 1]),
        mean: r(mean),
        p50: r(times[Math.floor(times.length * 0.5)]),
        p95: r(times[Math.floor(times.length * 0.95)]),
        p99: r(times[Math.floor(times.length * 0.99)]),
        samples,
        warmup_samples: warmup,
        unit: "milliseconds",
    };
}

async function loadWasm() {
    const jsPath = join(PLAYGROUND, "qualia_core_db.js");
    const wasmPath = join(PLAYGROUND, "qualia_core_db_bg.wasm");
    if (!existsSync(jsPath) || !existsSync(wasmPath)) {
        throw new Error(
            `WASM artifacts missing under ${PLAYGROUND} — run wasm-pack build or CI Pages workflow`
        );
    }
    const mod = await import(pathToFileURL(jsPath).href);
    const wasmBytes = readFileSync(wasmPath);
    await mod.default(wasmBytes);
    return mod;
}

function loadNT() {
    if (NT_PATH) return readFileSync(NT_PATH, "utf8");
    return generateNT(N);
}

(async () => {
    const result = {
        engine: "qualia_wasm",
        n_triples: N,
        measurement_path: "wasm_node_in_process",
    };

    let wasm;
    try {
        wasm = await loadWasm();
        if (typeof wasm.get_engine_version === "function") {
            result.engine_version = wasm.get_engine_version();
        }
    } catch (err) {
        result.error = err.message;
        process.stdout.write(JSON.stringify(result, null, 2) + "\n");
        process.exit(0);
    }

    const pointSubject = QUERIES.point_subject || "http://q.test/s/0";
    const twohopStart = QUERIES.twohop_start || pointSubject;
    const filterPredicate = QUERIES.filter_predicate || "http://q.test/p/0";

    const pointPattern = `<${pointSubject}> ?p ?o .`;
    const filterPattern = `?s <${filterPredicate}> ?o .`;

    const t0 = process.hrtime.bigint();
    let db;
    let index;
    try {
        const parsed = parseNT(loadNT());
        db = parsed.db;
        index = parsed.index;
        result.ingestion_ms = Math.round(Number(process.hrtime.bigint() - t0) / 1e4) / 100;
        result.quin_count = parsed.quinCount;
        result.note =
            "Point/filter via execute_ntriples_query on flat quins; two-hop uses subject adjacency index (same logical ops as q42_comparative_bench).";
    } catch (err) {
        result.error = `ingestion failed: ${err.message}`;
        process.stdout.write(JSON.stringify(result, null, 2) + "\n");
        process.exit(0);
    }

    const exec = wasm.execute_ntriples_query;

    try {
        result.point = latencyStatsMs(() => {
            exec(pointPattern, db, 256);
        });
    } catch (err) {
        result.point = `ERROR: ${err.message}`;
    }

    try {
        const startHash = qHash(twohopStart);
        result.twohop = latencyStatsMs(() => {
            const outs = index.get(startHash);
            if (!outs) return;
            for (const obj of outs) {
                const next = index.get(obj);
                if (next && next.length) return;
            }
        });
    } catch (err) {
        result.twohop = `ERROR: ${err.message}`;
    }

    try {
        result.filter = latencyStatsMs(() => {
            exec(filterPattern, db, 256);
        });
    } catch (err) {
        result.filter = `ERROR: ${err.message}`;
    }

    result.peak_rss_mb =
        Math.round((process.memoryUsage().rss / (1024 * 1024)) * 100) / 100;

    process.stdout.write(JSON.stringify(result, null, 2) + "\n");
})();
