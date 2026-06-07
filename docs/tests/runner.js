// Mode-aware test orchestrator.
// Modes: 'wasm' | 'native' | 'both'

import { TestRunner } from './test-runner.js';
import { loadWasm, getWasmCoverage, getWasmVersion } from './wasm-loader.js';
import { NativeClient, detectModes } from './native-client.js';

// Suite imports

// Pure JS logic (all modes)
import { register as regPrimitives }     from './suites/primitives.js';
import { register as regEpistemic }      from './suites/modality-epistemic.js';
import { register as regLtl }            from './suites/modality-ltl.js';
import { register as regParaconsistent } from './suites/modality-paraconsistent.js';
import { register as regLinear }         from './suites/modality-linear.js';
import { register as regDialectical }    from './suites/modality-dialectical.js';
import { register as regSpatioTemporal } from './suites/modality-spatio-temporal.js';
import { register as regDl }             from './suites/modality-dl.js';
import { register as regAsp }            from './suites/modality-asp.js';
import { register as regProbabilistic }  from './suites/modality-probabilistic.js';
import { register as regCogAi }          from './suites/modality-cogai.js';

// WASM-backed (wasm + both)
import { register as regQueryEngine }    from './suites/wasm-query-engine.js';
import { register as regBioinformatics } from './suites/wasm-bioinformatics.js';
import { register as regClinical }       from './suites/wasm-clinical.js';
import { register as regChemistry }      from './suites/wasm-chemistry.js';
import { register as regEconomics }      from './suites/wasm-economics.js';
import { register as regShacl }          from './suites/wasm-shacl.js';
import { register as regGovernance }     from './suites/wasm-governance.js';
import { register as regWasmIngest }     from './suites/wasm-ingest.js';
import { register as regProfiles }       from './suites/wasm-profiles.js';
import { register as regResources }      from './suites/wasm-resources.js';

// Native-only (native + both)
import { register as regNativeDaemon }   from './suites/native-daemon.js';
import { register as regNativeQuery }    from './suites/native-query.js';
import { register as regNativeLive }     from './suites/native-live.js';

// Both-mode comparison (both only)
import { register as regComparison }     from './suites/native-comparison.js';

// App state

let appMode = 'wasm';
let detected = {
    wasm: false,
    native: false,
    isMobile: false,
    daemonVersion: null,
    token: '',
    wasmCoverage: null,
    wasmVersion: null,
};

// Shared test context - passed to every register() call
let ctx = { mode: 'wasm', wasm: null, native: null, isMobile: false };

// UI helpers

function esc(s) {
    return String(s)
        .replace(/&/g, '&amp;').replace(/</g, '&lt;')
        .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function $id(id) { return document.getElementById(id); }

function updateModeUI() {
    for (const btn of document.querySelectorAll('.mode-btn')) {
        btn.classList.toggle('active', btn.dataset.mode === appMode);
    }

    const nb = $id('native-badge');
    if (detected.native) {
        nb.className = 'mode-status online';
        nb.textContent = `Daemon v${detected.daemonVersion || '?'} online`;
    } else {
        nb.className = 'mode-status offline';
        nb.textContent = 'Daemon offline';
    }

    const wb = $id('wasm-badge');
    const coverage = detected.wasmCoverage;
    const ver = detected.wasmVersion ? ` v${detected.wasmVersion}` : '';
    if (detected.wasm && coverage?.partial) {
        wb.className = 'mode-status warn';
        wb.textContent = `Partial WASM${ver} (${coverage.available}/${coverage.expected} exports)`;
    } else if (detected.wasm) {
        wb.className = 'mode-status online';
        wb.textContent = `WASM${ver} ready`;
    } else {
        wb.className = 'mode-status loading';
        wb.textContent = 'Loading WASM';
    }

    $id('mobile-badge').style.display = detected.isMobile ? '' : 'none';

    document.querySelector('.mode-btn[data-mode="native"]')
        .classList.toggle('unavailable', !detected.native);
    document.querySelector('.mode-btn[data-mode="both"]')
        .classList.toggle('unavailable', !detected.native && !detected.wasm);

    const coverageNote = $id('wasm-coverage-note');
    if (!coverageNote) return;
    if (coverage?.partial) {
        const preview = coverage.missing.slice(0, 4).join(', ');
        const suffix = coverage.missing.length > 4 ? ', ...' : '';
        coverageNote.textContent = `Current WASM binary exposes ${coverage.available}/${coverage.expected} expected browser-test exports. Missing exports are skipped, so green results mean executable tests passed, not full feature coverage. Missing now: ${preview}${suffix}`;
        coverageNote.style.display = '';
    } else if (detected.wasm) {
        coverageNote.textContent = 'Current WASM binary exposes the expected browser-test surface for this page.';
        coverageNote.style.display = '';
    } else {
        coverageNote.style.display = 'none';
    }
}

// Suite list per mode

function buildRunner(mode) {
    const r = new TestRunner();
    const c = { ...ctx, mode };

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

    if (mode === 'wasm' || mode === 'both') {
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

    if (mode === 'native' || mode === 'both') {
        regNativeDaemon(r, c);
        regNativeQuery(r, c);
        regNativeLive(r, c);
    }

    if (mode === 'both') {
        regComparison(r, c);
    }

    return r;
}

// Result rendering

let _totPassed = 0, _totFailed = 0, _totTests = 0;
let _suiteEls = new Map();

function getOrCreateSuiteEl(name, category) {
    if (_suiteEls.has(name)) return _suiteEls.get(name);

    const container = $id('suite-list');
    const el = document.createElement('div');
    el.className = `suite-card cat-${category}`;
    el.innerHTML = `
        <div class="suite-header">
            <span class="suite-toggle">></span>
            <span class="suite-cat">${category}</span>
            <span class="suite-name">${esc(name)}</span>
            <span class="suite-stats">
                <span class="suite-pass">0 passed</span>
                <span class="suite-fail">0 failed</span>
            </span>
        </div>
        <ul class="suite-tests"></ul>`;

    el.querySelector('.suite-header').addEventListener('click', () => {
        el.classList.toggle('open');
        el.querySelector('.suite-toggle').textContent =
            el.classList.contains('open') ? 'v' : '>';
    });

    container.appendChild(el);
    const state = { el, passCount: 0, failCount: 0 };
    _suiteEls.set(name, state);
    return state;
}

function suiteCategory(name) {
    if (name.startsWith('Native:') || name.startsWith('Compare:')) return 'native';
    if (name.startsWith('WASM:')) return 'wasm';
    return 'logic';
}

function addTestRow(suiteName, testName, passed, error, ms) {
    const cat = suiteCategory(suiteName);
    const state = getOrCreateSuiteEl(suiteName, cat);
    const li = document.createElement('li');
    li.className = `test-row ${passed ? 'pass' : 'fail'}`;
    const errHtml = error
        ? `<div class="test-error">${esc(error.message || String(error))}</div>`
        : '';
    li.innerHTML = `
        <span class="test-icon">${passed ? 'OK' : 'X'}</span>
        <span class="test-name">${esc(testName)}</span>
        <span class="test-ms">${ms < 1 ? '<1' : Math.round(ms)}ms</span>
        ${errHtml}`;
    state.el.querySelector('.suite-tests').appendChild(li);

    if (passed) state.passCount++; else state.failCount++;
    state.el.querySelector('.suite-pass').textContent = `${state.passCount} passed`;
    state.el.querySelector('.suite-fail').textContent = `${state.failCount} failed`;

    if (!passed && !state.el.classList.contains('open')) {
        state.el.classList.add('open');
        state.el.querySelector('.suite-toggle').textContent = 'v';
    }
    state.el.querySelector('.suite-header').classList.toggle('has-failures', state.failCount > 0);
}

function updateSummary() {
    const pct = _totTests ? Math.round((_totPassed / _totTests) * 100) : 0;
    $id('summary-passed').textContent = _totPassed;
    $id('summary-failed').textContent = _totFailed;
    $id('summary-total').textContent = _totTests;
    $id('progress-bar').style.width = `${pct}%`;
    $id('progress-bar').className = `bar ${_totFailed > 0 ? 'fail' : 'pass'}`;
}

// Run

export async function runAll(mode = appMode) {
    appMode = mode;
    _totPassed = 0;
    _totFailed = 0;
    _totTests = 0;
    $id('suite-list').innerHTML = '';
    _suiteEls.clear();
    $id('status-label').textContent = `Running (${mode} mode)...`;
    $id('run-btn').disabled = true;

    const runner = buildRunner(mode);

    await runner.run(evt => {
        if (evt.type === 'pass') {
            _totPassed++;
            _totTests++;
            addTestRow(evt.suite.name, evt.name, true, null, evt.ms);
        } else if (evt.type === 'fail') {
            _totFailed++;
            _totTests++;
            addTestRow(evt.suite.name, evt.name, false, evt.error, evt.ms);
        }
        updateSummary();
    });

    $id('status-label').textContent = _totFailed === 0
        ? `All ${_totPassed} tests passed`
        : `${_totFailed} failed / ${_totTests} total`;

    const coverage = detected.wasmCoverage;
    if ((mode === 'wasm' || mode === 'both') && coverage?.partial && _totFailed === 0) {
        $id('status-label').textContent += ` - partial WASM build (${coverage.available}/${coverage.expected} exports)`;
    }

    $id('run-btn').disabled = false;
    updateModeUI();
}

// Boot

document.addEventListener('DOMContentLoaded', async () => {
    if (new URLSearchParams(window.location.search).get('manual') === '1') {
        window.MANUAL_TESTS = true;
    }
    const manualToggle = document.getElementById('manual-toggle');
    if (manualToggle) {
        manualToggle.checked = !!window.MANUAL_TESTS;
        manualToggle.addEventListener('change', () => {
            window.MANUAL_TESTS = manualToggle.checked;
        });
    }

    detectModes().then(async d => {
        detected = d;
        if (d.wasm) {
            try {
                detected.wasmCoverage = await getWasmCoverage();
                detected.wasmVersion = await getWasmVersion();
            } catch (_) {}
        }
        ctx.wasm = null;
        ctx.native = d.native ? new NativeClient('http://127.0.0.1:4242', d.token) : null;
        ctx.isMobile = d.isMobile;
        updateModeUI();
    });

    const wakeBtn = $id('wake-btn');
    if (wakeBtn) {
        wakeBtn.addEventListener('click', (e) => {
            e.preventDefault();
            window.location.href = 'qualia://start';
            setTimeout(() => detectModes().then(d => {
                detected = d;
                ctx.native = d.native ? new NativeClient('http://127.0.0.1:4242', d.token) : null;
                updateModeUI();
            }), 2500);
        });
    }

    for (const btn of document.querySelectorAll('.mode-btn')) {
        btn.addEventListener('click', () => {
            if (!btn.classList.contains('unavailable')) {
                runAll(btn.dataset.mode);
            }
        });
    }

    $id('run-btn').addEventListener('click', () => runAll(appMode));

    runAll('wasm');
});
