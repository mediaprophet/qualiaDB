/**
 * Browser VibeScript measurement harness.
 *
 * This module runs the shipped `vibe_wasm` binding. It deliberately does not
 * emulate VibeScript in JavaScript or claim browser threading, native SIMD,
 * Super-Quin throughput, or allocation guarantees that this harness cannot
 * measure. The JavaScript baseline is a semantically equivalent arithmetic
 * calculation; it is a reference point, not a claim of like-for-like engines.
 */

import initVibe, {
    decode_and_run,
    encode_cell_bytecode,
    eval_cell_src,
    run_cell_bytecode,
} from '../pkg/vibe/vibe_wasm.js';

const CELL_SOURCE = '= 1 + 2 * 3 - 4';
const EXPECTED_VALUE = 3;
const SAMPLE_COUNT = 15;
let wasmReady;
let wasmMemory;

async function ensureWasm() {
    if (!wasmReady) {
        wasmReady = initVibe().then((instance) => {
            wasmMemory = instance.memory;
            return instance;
        });
    }
    return wasmReady;
}

function directJavaScript() {
    return 1 + 2 * 3 - 4;
}

function valueOf(result) {
    if (!result || result.ok !== true) {
        throw new Error(`VibeScript execution failed: ${result?.error || 'unknown error'}`);
    }
    if (Number(result.value) !== EXPECTED_VALUE) {
        throw new Error(`VibeScript result did not match the expected value ${EXPECTED_VALUE}.`);
    }
}

function percentile(sorted, fraction) {
    return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * fraction))];
}

function measureBatch(run, batchSize) {
    const started = performance.now();
    for (let i = 0; i < batchSize; i++) run();
    return performance.now() - started;
}

function measure(label, run, batchSize) {
    for (let i = 0; i < 32; i++) run();
    const samples = [];
    for (let i = 0; i < SAMPLE_COUNT; i++) {
        samples.push(measureBatch(run, batchSize) / batchSize);
    }
    const sorted = [...samples].sort((a, b) => a - b);
    const mean = sorted.reduce((sum, value) => sum + value, 0) / sorted.length;
    return {
        label,
        times: samples,
        mean,
        p50: percentile(sorted, 0.5),
        p95: percentile(sorted, 0.95),
        opsPerSec: Math.round(1000 / Math.max(mean, Number.EPSILON)),
    };
}

function summarizeSamples(label, samples) {
    const sorted = [...samples].sort((a, b) => a - b);
    const mean = sorted.reduce((sum, value) => sum + value, 0) / sorted.length;
    return {
        label,
        times: samples,
        mean,
        p50: percentile(sorted, 0.5),
        p95: percentile(sorted, 0.95),
        opsPerSec: Math.round(1000 / Math.max(mean, Number.EPSILON)),
    };
}

function workloadFor(name) {
    if (name === 'source') {
        return {
            label: 'parse + compile + bytecode run',
            setupMs: 0,
            vibe: () => valueOf(run_cell_bytecode(CELL_SOURCE)),
            js: directJavaScript,
            note: 'Measures the public source-to-bytecode API on every call; parsing and compilation are intentionally included.',
        };
    }
    if (name === 'ast') {
        return {
            label: 'parse + check + AST evaluation',
            setupMs: 0,
            vibe: () => valueOf(eval_cell_src(CELL_SOURCE)),
            js: directJavaScript,
            note: 'Measures the public AST evaluation API on every call; it is not a bytecode hot-loop benchmark.',
        };
    }

    const setupStarted = performance.now();
    const bytes = encode_cell_bytecode(CELL_SOURCE);
    if (!(bytes instanceof Uint8Array)) {
        throw new Error('VibeScript bytecode encoding did not produce a byte array.');
    }
    const setupMs = performance.now() - setupStarted;
    return {
        label: 'encoded bytecode decode + run',
        setupMs,
        vibe: () => valueOf(decode_and_run(bytes)),
        js: directJavaScript,
        note: 'Compilation is measured separately. Each timed VibeScript call decodes and runs the same shipped VBC1 bytecode because the current browser binding exposes decode-and-run, not a persistent VM handle.',
    };
}

function workerCountFor(execution) {
    return execution === 'workers-2' ? 2 : execution === 'workers-4' ? 4 : 0;
}

function callWorker(worker, message) {
    return new Promise((resolve, reject) => {
        const onMessage = (event) => {
            cleanup();
            if (event.data?.error) reject(new Error(event.data.error));
            else resolve(event.data);
        };
        const onError = (event) => {
            cleanup();
            reject(event.error || new Error(event.message || 'VibeScript worker failed.'));
        };
        const cleanup = () => {
            worker.removeEventListener('message', onMessage);
            worker.removeEventListener('error', onError);
        };
        worker.addEventListener('message', onMessage);
        worker.addEventListener('error', onError);
        worker.postMessage(message);
    });
}

async function measureWorkerInstances(workloadName, batchSize, workerCount) {
    const workers = Array.from({ length: workerCount }, () => new Worker(
        new URL('./vibescript-bench-worker.js', import.meta.url),
        { type: 'module', name: 'vibescript-wasm-bench' },
    ));
    try {
        const configured = await Promise.all(workers.map((worker) => callWorker(worker, {
            type: 'configure', workloadName,
        })));
        const perCallWallTimes = [];
        const perCallKernelTimes = [];
        for (let sample = 0; sample < SAMPLE_COUNT; sample++) {
            const started = performance.now();
            const results = await Promise.all(workers.map((worker) => callWorker(worker, {
                type: 'sample', batchSize,
            })));
            const calls = batchSize * workerCount;
            perCallWallTimes.push((performance.now() - started) / calls);
            perCallKernelTimes.push(Math.max(...results.map((result) => result.elapsedMs)) / batchSize);
        }
        const first = configured[0];
        return {
            vibe: summarizeSamples(`VibeScript WASM (${workerCount} independent workers)`, perCallWallTimes),
            kernel: summarizeSamples('slowest worker kernel time', perCallKernelTimes),
            setupMs: configured.reduce((sum, result) => sum + result.setupMs, 0),
            wasmMemoryBytes: configured.reduce((sum, result) => sum + (result.wasmMemoryBytes || 0), 0),
            workerCount,
            workloadLabel: first.workloadLabel,
            note: `${first.note} Aggregate timing includes worker scheduling and message delivery; no shared-memory or WASM-thread claim is made.`,
        };
    } finally {
        workers.forEach((worker) => worker.terminate());
    }
}

/**
 * Run a reproducible browser comparison against the real WASM binding.
 * All browser modes are single-threaded by design in the current Vibe build.
 */
export async function runVibeScriptVsV8Live(workloadName = 'bytecode', iterations = 1000, execution = 'single') {
    const batchSize = Math.max(1, Math.min(Number(iterations) || 1000, 10000));
    const workerCount = workerCountFor(execution);
    let vibe, kernel = null, setupMs, wasmMemoryBytes, workloadLabel, note, js;
    if (workerCount > 0) {
        const parallel = await measureWorkerInstances(workloadName, batchSize, workerCount);
        ({ vibe, kernel, setupMs, wasmMemoryBytes, workloadLabel, note } = parallel);
        js = directJavaScript;
    } else {
        await ensureWasm();
        const workload = workloadFor(workloadName);
        vibe = measure('VibeScript WASM', workload.vibe, batchSize);
        setupMs = workload.setupMs;
        wasmMemoryBytes = wasmMemory?.buffer?.byteLength ?? null;
        workloadLabel = workload.label;
        note = workload.note;
        js = workload.js;
    }
    const v8 = measure('JavaScript baseline', js, batchSize);

    return {
        workload: workloadName,
        workloadLabel,
        iterations: batchSize,
        samples: SAMPLE_COUNT,
        vibe: { ...vibe, fuel: null, heapBytes: null },
        v8: { ...v8, heapBytes: null, gcJitterMs: +(v8.p95 - v8.p50).toFixed(6) },
        setupMs,
        wasmMemoryBytes,
        throughputRatio: +(vibe.opsPerSec / Math.max(v8.opsPerSec, 1)).toFixed(4),
        executionMode: workerCount > 0
            ? `${workerCount} independent browser workers; one WASM instance per worker`
            : 'single-threaded browser WASM',
        workerCount: Math.max(workerCount, 1),
        kernel,
        note,
    };
}
