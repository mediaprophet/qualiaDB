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

import { register as regPrimitives } from './suites/primitives.js';
import { register as regEpistemic } from './suites/modality-epistemic.js';
import { register as regLtl } from './suites/modality-ltl.js';
import { register as regParaconsistent } from './suites/modality-paraconsistent.js';
import { register as regLinear } from './suites/modality-linear.js';
import { register as regDialectical } from './suites/modality-dialectical.js';
import { register as regSpatioTemporal } from './suites/modality-spatio-temporal.js';
import { register as regDl } from './suites/modality-dl.js';
import { register as regAsp } from './suites/modality-asp.js';
import { register as regProbabilistic } from './suites/modality-probabilistic.js';
import { register as regCogAi } from './suites/modality-cogai.js';
import { register as regAgency } from './suites/modality-agency.js';
import { register as regComorbidity } from './suites/modality-comorbidity.js';
import { register as regDicom } from './suites/modality-dicom.js';
import { register as regDeontic } from './suites/modality-deontic.js';
import { register as regOntology } from './suites/ontology-alignment.js';
import { register as regQueryEngine } from './suites/wasm-query-engine.js';
import { register as regBioinformatics } from './suites/wasm-bioinformatics.js';
import { register as regClinical } from './suites/wasm-clinical.js';
import { register as regChemistry } from './suites/wasm-chemistry.js';
import { register as regEconomics } from './suites/wasm-economics.js';
import { register as regShacl } from './suites/wasm-shacl.js';
import { register as regGovernance } from './suites/wasm-governance.js';
import { register as regWasmIngest } from './suites/wasm-ingest.js';
import { register as regProfiles } from './suites/wasm-profiles.js';
import { register as regResources } from './suites/wasm-resources.js';
import { register as regNativeDaemon } from './suites/native-daemon.js';
import { register as regNativeQuery } from './suites/native-query.js';
import { register as regNativeLive } from './suites/native-live.js';
import { register as regComparison } from './suites/native-comparison.js';

const mode = process.argv.includes('--mode')
    ? process.argv[process.argv.indexOf('--mode') + 1]
    : 'logic';

const ctx = { mode, wasm: null, native: null, isMobile: false };

function buildRunner(runMode) {
    const r = new TestRunner();
    const c = { ...ctx, mode: runMode };

    regPrimitives(r, c);
    regEpistemic(r, c);
    regLtl(r, c);
    regParaconsistent(r, c);
    regLinear(r, c);
    regDialectical(r, c);
    regSpatioTemporal(r, c);
    regDl(r, c);
    regAsp(r, c);
    regProbabilistic(r, c);
    regCogAi(r, c);
    regAgency(r, c);
    regComorbidity(r, c);
    regDicom(r, c);
    regDeontic(r, c);
    regOntology(r, c);

    if (runMode === 'wasm' || runMode === 'both') {
        regQueryEngine(r, c);
        regBioinformatics(r, c);
        regClinical(r, c);
        regChemistry(r, c);
        regEconomics(r, c);
        regShacl(r, c);
        regGovernance(r, c);
        regWasmIngest(r, c);
        regProfiles(r, c);
        regResources(r, c);
    }

    if (runMode === 'native' || runMode === 'both') {
        regNativeDaemon(r, c);
        regNativeQuery(r, c);
        regNativeLive(r, c);
    }

    if (runMode === 'both') {
        regComparison(r, c);
    }

    return r;
}

const failures = [];
const skipped = [];

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
        }
    });

    console.log('\n');
    console.log(`Mode: ${runMode}`);
    console.log(`Passed: ${passed}`);
    console.log(`Failed: ${failed}`);
    console.log(`Total:  ${passed + failed}`);

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
