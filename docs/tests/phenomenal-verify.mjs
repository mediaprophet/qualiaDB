#!/usr/bin/env node
/**
 * PR-C11b — Phenomenal viewport CI checklist.
 *
 * Orchestrates Rust contract tests (WGSL compile, bindings, PGA oracle, VramLedger,
 * tensor binary layout) and optional WASM API surface checks after portal build.
 *
 * Usage:
 *   node docs/tests/phenomenal-verify.mjs
 *   node docs/tests/phenomenal-verify.mjs --wasm-api docs/pkg/qualia/qualia.d.ts
 */
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '../..');

const WASM_NAV_EXPORTS = [
    'select_node_at',
    'poll_selected_node',
    'navigate_to_node',
    'collapse_node_q',
    'observe_node_at',
];

function runStep(name, cmd, args, opts = {}) {
    console.log(`[phenomenal] ${name}…`);
    const result = spawnSync(cmd, args, {
        cwd: repoRoot,
        stdio: 'inherit',
        shell: process.platform === 'win32',
        ...opts,
    });
    if (result.status !== 0) {
        console.error(`[phenomenal] FAIL ${name} (exit ${result.status ?? 'signal'})`);
        process.exit(result.status || 1);
    }
    console.log(`[phenomenal] OK ${name}`);
}

function checkWasmApi(dtsPath) {
    if (!existsSync(dtsPath)) {
        console.warn(`[phenomenal] SKIP wasm-api (missing ${dtsPath})`);
        return;
    }
    const src = readFileSync(dtsPath, 'utf8');
    const missing = WASM_NAV_EXPORTS.filter((fn) => !src.includes(fn));
    if (missing.length) {
        console.error(`[phenomenal] FAIL wasm-api missing exports: ${missing.join(', ')}`);
        process.exit(1);
    }
    console.log(`[phenomenal] OK wasm-api (${WASM_NAV_EXPORTS.length} navigation exports)`);
}

const wasmApiArg = process.argv.indexOf('--wasm-api');
const wasmApiPath =
    wasmApiArg >= 0
        ? resolve(process.argv[wasmApiArg + 1] ?? '')
        : resolve(repoRoot, 'docs/pkg/qualia/qualia.d.ts');

runStep('wasm32-shader-smoke', 'cargo', [
    'check',
    '--target',
    'wasm32-unknown-unknown',
    '-p',
    'qualia-core-db',
    '--no-default-features',
    '--features',
    'portal',
]);

runStep('contract-tests', 'cargo', [
    'test',
    '-p',
    'qualia-core-db',
    'phenomenal_contract',
    '--lib',
]);

runStep('pga-oracle', 'cargo', ['test', '-p', 'qualia-core-db', 'portal_pga', '--lib']);

runStep('ambient-draw-step', 'cargo', [
    'test',
    '-p',
    'qualia-core-db',
    'ambient_draw_instant_step_by_mode',
    '--lib',
    '--',
    '--exact',
]);

checkWasmApi(wasmApiPath);
console.log('[phenomenal] all checks passed');