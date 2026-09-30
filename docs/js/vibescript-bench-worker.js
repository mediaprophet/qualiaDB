/**
 * Worker-owned VibeScript WASM benchmark instance.
 *
 * Each worker loads the shipped binding independently. This is browser-level
 * parallelism through independent WASM instances, not shared-memory WASM
 * threading.
 */
import initVibe, {
    decode_and_run,
    encode_cell_bytecode,
    eval_cell_src,
    run_cell_bytecode,
} from '../pkg/vibe/vibe_wasm.js';

const CELL_SOURCE = '= 1 + 2 * 3 - 4';
const EXPECTED_VALUE = 3;
let wasmMemory;
let runner;

function assertValue(result) {
    if (!result || result.ok !== true) {
        throw new Error(`VibeScript worker execution failed: ${result?.error || 'unknown error'}`);
    }
    if (Number(result.value) !== EXPECTED_VALUE) {
        throw new Error('VibeScript worker returned an unexpected arithmetic result.');
    }
}

async function configure(workloadName) {
    const instance = await initVibe();
    wasmMemory = instance.memory;
    let setupMs = 0;
    let workloadLabel;
    let note;

    if (workloadName === 'source') {
        runner = () => assertValue(run_cell_bytecode(CELL_SOURCE));
        workloadLabel = 'parse + compile + bytecode run';
        note = 'Measures the public source-to-bytecode API on every call; parsing and compilation are intentionally included.';
    } else if (workloadName === 'ast') {
        runner = () => assertValue(eval_cell_src(CELL_SOURCE));
        workloadLabel = 'parse + check + AST evaluation';
        note = 'Measures the public AST evaluation API on every call; it is not a bytecode hot-loop benchmark.';
    } else {
        const started = performance.now();
        const bytes = encode_cell_bytecode(CELL_SOURCE);
        setupMs = performance.now() - started;
        if (!(bytes instanceof Uint8Array)) throw new Error('VibeScript bytecode encoding did not produce a byte array.');
        runner = () => assertValue(decode_and_run(bytes));
        workloadLabel = 'encoded bytecode decode + run';
        note = 'Compilation is measured separately. Each timed call decodes and runs the same shipped VBC1 bytecode.';
    }

    for (let i = 0; i < 32; i++) runner();
    return { setupMs, workloadLabel, note, wasmMemoryBytes: wasmMemory?.buffer?.byteLength ?? null };
}

self.onmessage = async (event) => {
    try {
        const { type, workloadName, batchSize } = event.data || {};
        if (type === 'configure') {
            self.postMessage({ type: 'configured', ...(await configure(workloadName)) });
            return;
        }
        if (type === 'sample') {
            if (!runner) throw new Error('Worker was sampled before configuration.');
            const started = performance.now();
            for (let i = 0; i < batchSize; i++) runner();
            self.postMessage({ type: 'sample', elapsedMs: performance.now() - started });
            return;
        }
        throw new Error(`Unsupported worker message: ${type}`);
    } catch (error) {
        self.postMessage({ error: error?.message || String(error) });
    }
};
