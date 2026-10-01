/**
 * Browser VibeScript measurement harness.
 *
 * Soft-rise Present purpose clocks (hero): checked cell, CompiledCell.compile,
 * run() ×2 on the same handle. Structural edit without host rebuild stays
 * quiet in the harness until Present — not a soft-rise hero card.
 * CompiledCell.run also appears as a labelled binding footnote.
 */

import initVibe, {
    CompiledCell,
    apply_structural_edit,
    check_cell_src,
    decode_and_run,
    encode_cell_bytecode,
    eval_cell_src,
    run_cell_bytecode,
} from '../pkg/vibe/vibe_wasm.js';

const CELL_SOURCE = '= 1 + 2 * 3 - 4';
const EXPECTED_VALUE = 3;
const EDIT_SOURCE = 'fn main() -> i64 {\n  return 1;\n}\n';
const EDIT_JSON = JSON.stringify({ op: 'rename_item', index: 0, new_name: 'entry' });
const SAMPLE_COUNT = 11;
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

function valueOf(result) {
    if (!result || result.ok !== true) {
        throw new Error(`VibeScript execution failed: ${result?.error || 'unknown error'}`);
    }
    if (Number(result.value) !== EXPECTED_VALUE) {
        throw new Error(`VibeScript result did not match the expected value ${EXPECTED_VALUE}.`);
    }
}

function assertOk(result, label) {
    if (!result || result.ok !== true) {
        const detail = typeof result?.error === 'string'
            ? result.error
            : (result?.error?.message || JSON.stringify(result?.error) || 'unknown error');
        throw new Error(`${label} failed: ${detail}`);
    }
    return result;
}

function percentile(sorted, fraction) {
    return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * fraction))];
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

function measureWholeJob(label, runOnce) {
    for (let i = 0; i < 8; i++) runOnce();
    const samples = [];
    for (let i = 0; i < SAMPLE_COUNT; i++) {
        const started = performance.now();
        runOnce();
        samples.push(performance.now() - started);
    }
    return summarizeSamples(label, samples);
}

function measureBatch(label, run, batchSize) {
    for (let i = 0; i < 32; i++) run();
    const samples = [];
    for (let i = 0; i < SAMPLE_COUNT; i++) {
        const started = performance.now();
        for (let j = 0; j < batchSize; j++) run();
        samples.push((performance.now() - started) / batchSize);
    }
    return summarizeSamples(label, samples);
}

function purposeDemos() {
    const checked = measureWholeJob('checked cell', () => {
        assertOk(check_cell_src(CELL_SOURCE), 'checked cell');
    });

    const compile = measureWholeJob('CompiledCell.compile', () => {
        CompiledCell.compile(CELL_SOURCE);
    });

    const cell = CompiledCell.compile(CELL_SOURCE);
    valueOf(cell.run());
    const run = measureWholeJob('CompiledCell.run ×2', () => {
        valueOf(cell.run());
        valueOf(cell.run());
    });

    const edit = measureWholeJob('edit without host rebuild', () => {
        const result = assertOk(
            apply_structural_edit(EDIT_SOURCE, EDIT_JSON),
            'structural edit',
        );
        if (typeof result.source !== 'string' || !result.source.includes('entry')) {
            throw new Error('structural edit did not project the renamed item.');
        }
    });

    return { checked, compile, run, edit };
}

function compiledHandleFootnote(batchSize) {
    const setupStarted = performance.now();
    const cell = CompiledCell.compile(CELL_SOURCE);
    const setupMs = performance.now() - setupStarted;
    valueOf(cell.run());
    const run = measureBatch('CompiledCell.run (no re-decode)', () => valueOf(cell.run()), batchSize);
    return {
        setupMs,
        run,
        codeSize: cell.code_size,
        note: 'Binding footnote only: compile once, then run() without decode. Not a language-speed comparison to bare JavaScript.',
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
            vibe: summarizeSamples(`CompiledCell.run × ${workerCount} workers`, perCallWallTimes),
            kernel: summarizeSamples('slowest worker kernel time', perCallKernelTimes),
            setupMs: configured.reduce((sum, result) => sum + result.setupMs, 0),
            wasmMemoryBytes: configured.reduce((sum, result) => sum + (result.wasmMemoryBytes || 0), 0),
            workerCount,
            workloadLabel: first.workloadLabel,
            note: `${first.note} Aggregate timing includes worker scheduling; no shared-memory WASM-thread claim.`,
        };
    } finally {
        workers.forEach((worker) => worker.terminate());
    }
}

/**
 * Run purpose demos (hero) plus an optional compiled-handle footnote.
 * Does not crown bare JavaScript arithmetic against unequal WASM jobs.
 */
export async function runVibeScriptVsV8Live(workloadName = 'bytecode', iterations = 1000, execution = 'single') {
    const batchSize = Math.max(1, Math.min(Number(iterations) || 1000, 10000));
    const workerCount = workerCountFor(execution);
    await ensureWasm();

    const purposes = purposeDemos();
    let binding = null;
    let kernel = null;
    let executionMode = 'single-threaded browser WASM';
    let note = 'Purpose demos clock whole jobs on the shipped binding. Binding cost is a footnote after CompiledCell exists.';

    if (workerCount > 0) {
        const parallel = await measureWorkerInstances(workloadName, batchSize, workerCount);
        binding = {
            setupMs: parallel.setupMs,
            run: parallel.vibe,
            note: parallel.note,
        };
        kernel = parallel.kernel;
        executionMode = `${workerCount} independent browser workers; one WASM instance per worker`;
        note = parallel.note;
    } else if (workloadName === 'source') {
        const run = measureBatch(
            'parse + compile + bytecode run',
            () => valueOf(run_cell_bytecode(CELL_SOURCE)),
            batchSize,
        );
        binding = {
            setupMs: 0,
            run,
            note: 'Source path includes parse and compile on every call.',
        };
    } else if (workloadName === 'ast') {
        const run = measureBatch(
            'parse + check + AST evaluation',
            () => valueOf(eval_cell_src(CELL_SOURCE)),
            batchSize,
        );
        binding = {
            setupMs: 0,
            run,
            note: 'AST evaluation path; not a bytecode hot loop.',
        };
    } else if (workloadName === 'decode') {
        const setupStarted = performance.now();
        const bytes = encode_cell_bytecode(CELL_SOURCE);
        if (!(bytes instanceof Uint8Array)) {
            throw new Error('VibeScript bytecode encoding did not produce a byte array.');
        }
        const setupMs = performance.now() - setupStarted;
        const run = measureBatch(
            'decode_and_run (compat; re-decodes)',
            () => valueOf(decode_and_run(bytes)),
            batchSize,
        );
        binding = {
            setupMs,
            run,
            note: 'Compat path retained for decode_and_run. Prefer CompiledCell for repeated runs.',
        };
    } else {
        binding = compiledHandleFootnote(batchSize);
    }

    return {
        workload: workloadName,
        workloadLabel: binding?.run?.label || 'purpose demos',
        iterations: batchSize,
        samples: SAMPLE_COUNT,
        purposes,
        binding,
        // Keep shape fields for older page controllers; do not treat as JS race.
        vibe: binding?.run
            ? { ...binding.run, fuel: null, heapBytes: null }
            : null,
        v8: null,
        setupMs: binding?.setupMs ?? 0,
        wasmMemoryBytes: wasmMemory?.buffer?.byteLength ?? null,
        throughputRatio: null,
        executionMode,
        workerCount: Math.max(workerCount, 1),
        kernel,
        note,
    };
}
