/**
 * Present VibeScript jobs the browser binding already runs.
 * Checked cell · CompiledCell.compile · run() without re-decode · host ask.
 * Arithmetic `= 1 + 2 * 3 - 4` is an interim payload only — not the product claim.
 */

import initVibe, {
    CompiledCell,
    check_cell_src,
    eval_cell_src,
} from '../pkg/vibe/vibe_wasm.js';

/** Interim cell payload — enough to exercise check / compile / run. */
export const CELL_SOURCE = '= 1 + 2 * 3 - 4';
export const EXPECTED_CELL_VALUE = 3;
export const ASK_SOURCE = '= graph? { ?s ?p ?o }';

let wasmReady;

export async function ensurePresentWasm() {
    if (!wasmReady) {
        wasmReady = initVibe();
    }
    return wasmReady;
}

function okResult(result, label) {
    if (!result || result.ok !== true) {
        const detail = typeof result?.error === 'string'
            ? result.error
            : (result?.error?.message || JSON.stringify(result?.error) || 'unknown');
        throw new Error(`${label}: ${detail}`);
    }
    return result;
}

/**
 * Run the four Present jobs once. Returns proud working status rows.
 */
export async function runPresentJobs(options = {}) {
    const cellSrc = options.cellSource || CELL_SOURCE;
    const askSrc = options.askSource || ASK_SOURCE;
    await ensurePresentWasm();

    const jobs = [];

    // 1. Checked cell
    {
        const t0 = performance.now();
        const checked = okResult(check_cell_src(cellSrc), 'checked cell');
        jobs.push({
            id: 'checked',
            label: 'Checked cell',
            claim: 'Type-check a cell before it runs',
            ok: true,
            ms: performance.now() - t0,
            detail: checked.ty ? `ty · ${checked.ty}` : 'cell ok',
        });
    }

    // 2. Compile once
    let cell;
    {
        const t0 = performance.now();
        cell = CompiledCell.compile(cellSrc);
        jobs.push({
            id: 'compile',
            label: 'CompiledCell.compile',
            claim: 'Compile once · keep the handle',
            ok: true,
            ms: performance.now() - t0,
            detail: `${cell.code_size} B bytecode · ${cell.constants} const`,
            handle: cell,
        });
    }

    // 3. Run again without re-decode
    {
        const t0 = performance.now();
        const first = okResult(cell.run(), 'CompiledCell.run');
        const second = okResult(cell.run(), 'CompiledCell.run (again)');
        const ms = performance.now() - t0;
        const value = first.value ?? second.value;
        jobs.push({
            id: 'run',
            label: 'CompiledCell.run',
            claim: 'Run again without re-decode',
            ok: true,
            ms,
            detail: `value · ${value} · two runs · same handle`,
            value,
        });
    }

    // 4. Host ask
    {
        const t0 = performance.now();
        const ask = okResult(eval_cell_src(askSrc), 'host ask');
        jobs.push({
            id: 'hostAsk',
            label: 'Host ask',
            claim: 'graph? ask through the binding',
            ok: true,
            ms: performance.now() - t0,
            detail: ask.value !== undefined ? `bindings · ${JSON.stringify(ask.value).slice(0, 72)}` : 'ask ok',
            value: ask.value,
        });
    }

    return {
        live: true,
        jobs,
        note: 'Present · these four jobs run on the shipped vibe-wasm binding.',
    };
}
