#!/usr/bin/env node
/**
 * Headless test runner for docs/tests — no browser required.
 *
 * Usage:
 *   node docs/tests/run-headless.mjs [--mode logic|wasm|native|both]
 *
 * Modes:
 *   logic  — JS modality reference suites only (default, CI-safe)
 *   wasm   — logic + WASM export tests (loads docs/playground binary)
 *   native — logic + localhost:4242 daemon tests (skips when offline)
 *   both   — logic + WASM + native + comparison
 */

import { TestRunner } from './test-runner.js';
import { loadWasm } from './wasm-loader.js';
import { NativeClient, detectModes } from './native-client.js';
import { registerSuites } from './suite-registry.js';

const mode = process.argv.includes('--mode')
    ? process.argv[process.argv.indexOf('--mode') + 1]
    : 'logic';

const ctx = { mode, wasm: null, native: null, isMobile: false };

function buildRunner(runMode) {
    const r = new TestRunner();
    registerSuites(r, { ...ctx, mode: runMode }, runMode);
    return r;
}

const failures = [];

async function prepareContext() {
    if (mode === 'wasm' || mode === 'both') {
        process.stdout.write('Loading WASM… ');
        ctx.wasm = await loadWasm();
        const ver = typeof ctx.wasm?.get_engine_version === 'function'
            ? ctx.wasm.get_engine_version()
            : '?';
        console.log(`ok (engine v${ver})`);
    }

    if (mode === 'native' || mode === 'both') {
        const detected = await detectModes();
        if (detected.native) {
            ctx.native = new NativeClient('http://127.0.0.1:4242', detected.token);
            console.log(`Daemon online v${detected.daemonVersion ?? '?'}`);
        } else {
            console.log('Daemon offline — native suites will fail or skip');
        }
    }
}

async function main() {
    const runMode = ['logic', 'wasm', 'native', 'both'].includes(mode) ? mode : 'logic';
    await prepareContext();

    const runner = buildRunner(runMode);
    let passed = 0;
    let failed = 0;
    let skipped = 0;
    const skips = [];

    await runner.run(evt => {
        if (evt.type === 'pass') {
            passed++;
            process.stdout.write('.');
        } else if (evt.type === 'fail') {
            failed++;
            failures.push({
                suite: evt.suite?.name,
                test: evt.name,
                error: evt.error?.message || String(evt.error),
            });
            process.stdout.write('F');
        } else if (evt.type === 'skip') {
            skipped++;
            skips.push({ suite: evt.suite?.name, test: evt.name, reason: evt.reason });
            process.stdout.write('s');
        }
    });

    console.log('\n');
    console.log(`Mode:    ${runMode}`);
    console.log(`Passed:  ${passed}   (executed at least one assertion)`);
    console.log(`Skipped: ${skipped}   (asserted nothing — daemon offline / WASM export absent / not run)`);
    console.log(`Failed:  ${failed}`);
    console.log(`Total:   ${passed + failed + skipped}`);

    if (skipped && process.argv.includes('--show-skips')) {
        console.log('\nSkipped:');
        for (const s of skips) console.log(`  ${s.suite} › ${s.test}  — ${s.reason}`);
    }

    if (failures.length) {
        console.log('\nFailures:');
        for (const f of failures) {
            console.log(`  ${f.suite} › ${f.test}`);
            console.log(`    ${f.error}`);
        }
        process.exit(1);
    }
}

main().catch(e => {
    console.error(e);
    process.exit(1);
});