/**
 * Soft-rise Present jobs the browser binding already runs.
 * Checked cell · CompiledCell.compile · run() ×2 on the same handle.
 * Arithmetic `= 1 + 2 * 3 - 4` is quiet interim payload for those jobs.
 */

import initVibe, {
    CompiledCell,
    check_cell_src,
} from '../pkg/vibe/vibe_wasm.js';

/** Quiet interim cell payload — exercises check / compile / run. */
export const CELL_SOURCE = '= 1 + 2 * 3 - 4';
export const EXPECTED_CELL_VALUE = 3;

/** Stagger between Present card soft-rises (ms). Skipped when reduced-motion. */
export const PRESENT_STAGGER_MS = 180;

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

export function prefersReducedMotion() {
    try {
        return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    } catch (_) {
        return false;
    }
}

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Soft-rise paint for Present cards: opacity + slight rise, ~180 ms between
 * check → compile → run(). Reduced motion: no stagger, no motion class.
 * Expects three cards only (data-job checked | compile | run).
 */
export async function softRisePresentCards(root, jobs, options = {}) {
    const scope = root || document;
    const reduced = options.forceReduced ?? prefersReducedMotion();
    const staggerMs = reduced ? 0 : (options.staggerMs ?? PRESENT_STAGGER_MS);
    const order = options.order || ['checked', 'compile', 'run'];
    const byId = new Map((jobs || []).map((job) => [job.id, job]));

    for (let i = 0; i < order.length; i++) {
        const id = order[i];
        const job = byId.get(id);
        const card = scope.querySelector(`[data-job="${id}"]`);
        if (!card) continue;

        card.classList.remove('is-wait', 'is-fail', 'is-rise');
        // Retrigger CSS soft-rise when motion is allowed.
        void card.offsetWidth;

        if (!job) {
            card.classList.add('is-wait');
            const meter = card.querySelector('.meter');
            if (meter) meter.textContent = 'warming…';
            continue;
        }

        if (!job.ok) card.classList.add('is-fail');
        const meter = card.querySelector('.meter');
        if (meter) {
            meter.textContent = job.ok
                ? `${Number(job.ms).toFixed(2)} ms · ${job.detail}`
                : (job.detail || 'failed');
        }
        if (!reduced) card.classList.add('is-rise');

        if (staggerMs > 0 && i < order.length - 1) {
            await sleep(staggerMs);
        }
    }
}

/**
 * Run the three soft-rise Present jobs once.
 */
export async function runPresentJobs(options = {}) {
    const cellSrc = options.cellSource || CELL_SOURCE;
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

    // 3. Run twice without re-decode
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

    return {
        live: true,
        jobs,
        note: 'Present · check · compile · run ×2 on the shipped vibe-wasm binding.',
    };
}
