import { chromium } from 'playwright';
import { spawn } from 'child_process';
import { setTimeout as sleep } from 'timers/promises';
import fs from 'fs';

const URL = 'http://localhost:8888/llmdemo/index.html';
const TIMEOUT = 180000;
const OUTPUT_FILE = 'test-wasm-results.json';

const MODELS = [
    { value: 'smollm2-360m', label: 'SmolLM2-360M-Instruct (Q8_0)' },
    { value: 'qwen2.5-0.5b', label: 'Qwen2.5-0.5B-Instruct (Q4_K_M)' },
    { value: 'qwen3-0.6b', label: 'Qwen3-0.6B (Q4_K_M)' },
    { value: 'llama-3.2-1b', label: 'Llama-3.2-1B-Instruct (Q4_K_M)' },
];

async function run() {
    const allResults = {
        timestamp: new Date().toISOString(),
        url: URL,
        models: {},
    };

    const chromePath = 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
    const userProfileDir = 'C:\\temp\\chrome-wasm-test';
    const debugPort = 9333;

    // Kill only our dedicated test Chrome instance
    try {
        const { execSync } = await import('child_process');
        execSync('wmic process where "CommandLine like \'%chrome-wasm-test%\'" call terminate', { stdio: 'ignore' });
    } catch {}
    await sleep(1000);

    const chromeProc = spawn(chromePath, [
        `--remote-debugging-port=${debugPort}`,
        `--user-data-dir=${userProfileDir}`,
        '--enable-unsafe-webgpu',
        '--enable-features=WebGPU',
        '--use-gl=angle',
        '--use-angle=d3d11',
        '--ignore-gpu-blocklist',
        '--enable-gpu',
        '--no-first-run',
        '--no-default-browser-check',
        '--disable-popup-blocking',
        '--window-size=1280,900',
    ], { detached: true, stdio: 'ignore' });

    await sleep(3000);
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);

    for (const model of MODELS) {
        console.log(`\n========== Testing ${model.label} ==========`);
        const result = await testModel(browser, model);
        allResults.models[model.value] = result;
        fs.writeFileSync(OUTPUT_FILE, JSON.stringify(allResults, null, 2));
        console.log(`  Output: ${JSON.stringify(result.generation?.output?.substring(0, 120))}`);
        console.log(`  TPS: ${result.generation?.tps}`);
        console.log(`  Artifacts: ${result.generation?.hasUnicodeArtifacts}`);
    }

    console.log(`\nAll results written to ${OUTPUT_FILE}`);
    await browser.close();
    try { process.kill(chromeProc.pid); } catch {}
}

async function testModel(browser, model) {
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });

    const result = {
        model: model.label,
        consoleLogs: [],
        pageErrors: [],
        phases: {},
        generation: null,
    };

    page.on('console', msg => {
        result.consoleLogs.push({ type: msg.type(), text: msg.text() });
    });
    page.on('pageerror', err => {
        result.pageErrors.push(err.message);
    });

    // Phase 1: Navigate + WASM init
    console.log('  Phase 1: Navigate + WASM init...');
    await page.goto(URL, { waitUntil: 'networkidle', timeout: 30000 });

    await page.waitForFunction(() => {
        const el = document.getElementById('btnLoadModel');
        return el && !el.disabled;
    }, { timeout: 30000 }).catch(() => {});

    result.phases.init = {
        status: await page.evaluate(() => document.getElementById('sysStatus')?.textContent || ''),
        loadButtonEnabled: await page.evaluate(() => !document.getElementById('btnLoadModel')?.disabled),
    };

    if (!result.phases.init.loadButtonEnabled) {
        result.error = 'Load Model button never enabled';
        await page.close();
        return result;
    }

    // Select the model from dropdown
    console.log(`  Selecting model: ${model.value}`);
    await page.selectOption('#modelSelect', model.value);
    await sleep(500);

    // Phase 2: Load model
    console.log('  Phase 2: Loading model...');
    await page.click('#btnLoadModel');

    let modelLoaded = false;
    try {
        await page.waitForFunction(() => {
            const el = document.getElementById('btnGenerate');
            return el && !el.disabled;
        }, { timeout: TIMEOUT });
        modelLoaded = true;
    } catch (e) {
        result.error = 'Model load timeout';
    }

    result.phases.modelLoad = {
        success: modelLoaded,
        status: await page.evaluate(() => document.getElementById('sysStatus')?.textContent || ''),
        terminal: await page.evaluate(() => document.getElementById('terminalOutput')?.textContent || ''),
    };
    console.log(`  Model loaded: ${modelLoaded}`);

    if (!modelLoaded) {
        await page.close();
        return result;
    }

    // Phase 3: Generate
    console.log('  Phase 3: Generating...');
    const promptValue = await page.evaluate(() => document.getElementById('promptInput')?.value || '');
    await page.click('#btnGenerate');

    let waitSecs = 0;
    const maxWait = 60;
    while (waitSecs < maxWait) {
        await sleep(2000);
        waitSecs += 2;
        const termText = await page.evaluate(() => document.getElementById('terminalOutput')?.textContent || '');
        if (termText.includes('Done:')) break;
        if (termText.includes('Inference error')) break;
        const genEnabled = await page.evaluate(() => !document.getElementById('btnGenerate')?.disabled);
        if (genEnabled && waitSecs > 4) break;
    }

    const genOutput = await page.evaluate(() => document.getElementById('genOutput')?.textContent || '');
    const tpsText = await page.evaluate(() => document.getElementById('tps')?.textContent || '');
    const terminalFinal = await page.evaluate(() => document.getElementById('terminalOutput')?.textContent || '');

    const tokens = genOutput.split(/\s+/).filter(Boolean);
    const uniqueTokens = new Set(tokens);
    const hasArtifacts = /[\u0100-\u017F]/.test(genOutput);

    result.generation = {
        prompt: promptValue,
        output: genOutput,
        tps: tpsText,
        waitSeconds: waitSecs,
        terminal: terminalFinal,
        tokenCount: tokens.length,
        uniqueTokens: uniqueTokens.size,
        repetitionRatio: tokens.length > 0 ? 1 - (uniqueTokens.size / tokens.length) : 0,
        hasUnicodeArtifacts: hasArtifacts,
    };

    if (hasArtifacts) {
        const artifacts = [...genOutput].filter(c => c.codePointAt(0) >= 0x80 && c.codePointAt(0) <= 0x17F);
        result.generation.artifacts = [...new Set(artifacts)].map(c => ({
            char: c,
            codePoint: 'U+' + c.codePointAt(0).toString(16).toUpperCase().padStart(4, '0'),
        }));
    }

    // Collect unique errors/warnings
    const errs = result.consoleLogs.filter(l => l.type === 'error' || l.type === 'warning');
    const seen = new Set();
    result.uniqueErrors = [];
    for (const l of errs) {
        const key = l.text.substring(0, 120);
        if (!seen.has(key)) {
            seen.add(key);
            result.uniqueErrors.push(l);
        }
    }

    await page.close();
    return result;
}

run().catch(e => {
    console.error('Test failed:', e);
    process.exit(1);
});
