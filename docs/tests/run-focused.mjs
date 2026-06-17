#!/usr/bin/env node
/** Run only the suites that were failing on GitHub Pages. */
import { TestRunner } from './test-runner.js';
import { loadWasm } from './wasm-loader.js';
import { register as regDiffusion } from './suites/modality-diffusion.js';
import { register as regDataFormats } from './suites/wasm-data-formats.js';
import { register as regRdfStar } from './suites/wasm-rdf-star.js';
import { register as regSolvers } from './suites/wasm-solvers.js';

const ctx = { mode: 'wasm', wasm: null, native: null, isMobile: false };
ctx.wasm = await loadWasm();
console.log('WASM loaded');

const runner = new TestRunner();
regDiffusion(runner, ctx);
regDataFormats(runner, ctx);
regRdfStar(runner, ctx);
regSolvers(runner, ctx);

const failures = [];
let passed = 0, failed = 0;

await runner.run(evt => {
    if (evt.type === 'pass') { passed++; process.stdout.write('.'); }
    else if (evt.type === 'fail') {
        failed++;
        failures.push({ test: evt.name, error: evt.error?.message || String(evt.error) });
        process.stdout.write('F');
    }
});

console.log(`\nPassed: ${passed}, Failed: ${failed}`);
if (failures.length) {
    console.log('\nFailures:');
    for (const f of failures) console.log(`  ${f.test}\n    ${f.error}`);
    process.exit(1);
}