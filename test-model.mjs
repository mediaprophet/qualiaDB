import { chromium } from 'playwright';
import fs from 'fs';

const URL = 'http://localhost:8888/llmdemo/index.html';
const TIMEOUT = 120000;

async function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

async function testModel(modelValue, modelName) {
    const browser = await chromium.launch({
        headless: false,
        args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--disable-gpu-sandbox', '--js-flags=--max-old-space-size=4096']
    });
    const context = await browser.newContext();
    const page = await context.newPage();

    const logs = [];
    const errors = [];
    page.on('console', msg => {
        logs.push(`[${msg.type()}] ${msg.text()}`);
        console.log(`[${msg.type()}] ${msg.text()}`);
    });
    page.on('pageerror', err => {
        errors.push(err.message);
        console.log(`[PAGE ERROR] ${err.message}`);
    });

    console.log(`\n========== Testing ${modelName} ==========`);
    await page.goto(URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForFunction(() => {
        const el = document.getElementById('btnLoadModel');
        return el && !el.disabled;
    }, { timeout: 30000 }).catch(() => {});

    await page.selectOption('#modelSelect', modelValue);
    await sleep(500);

    console.log('Loading model...');
    await page.click('#btnLoadModel');
    await page.waitForFunction(() => {
        const el = document.getElementById('btnGenerate');
        return el && !el.disabled;
    }, { timeout: TIMEOUT }).catch(() => {});

    const status = await page.evaluate(() => document.getElementById('sysStatus')?.textContent || '');
    console.log('Status:', status);

    console.log('Generating...');
    await page.click('#btnGenerate');

    for (let i = 0; i < 60; i++) {
        await sleep(2000);
        const term = await page.evaluate(() => document.getElementById('terminalOutput')?.textContent || '');
        const gen = await page.evaluate(() => document.getElementById('genOutput')?.textContent || '');
        const btnDisabled = await page.evaluate(() => document.getElementById('btnGenerate')?.disabled);
        if (term.includes('Done:') || term.includes('Inference error') || (!btnDisabled && i > 2)) {
            break;
        }
        if (i % 5 === 0) {
            console.log(`  [${i*2}s] gen: "${gen.substring(0, 100)}"`);
        }
    }

    const finalTerm = await page.evaluate(() => document.getElementById('terminalOutput')?.textContent || '');
    const finalGen = await page.evaluate(() => document.getElementById('genOutput')?.textContent || '');
    const finalTps = await page.evaluate(() => document.getElementById('tps')?.textContent || '');
    console.log(`\n=== ${modelName} RESULT ===`);
    console.log('Generated:', finalGen);
    console.log('TPS:', finalTps);
    console.log('Errors:', errors);

    // Extract just the inference-related terminal lines
    const termLines = finalTerm.split('\n').filter(l => l.includes('Generating') || l.includes('Done') || l.includes('error') || l.includes('prefill'));
    console.log('Key terminal:', termLines);

    await browser.close();
    return { output: finalGen, tps: finalTps, errors, logs };
}

const model = process.argv[2] || 'qwen2.5-0.5b';
const name = process.argv[3] || 'Qwen2.5-0.5B-Instruct-Q4_K_M';
const result = await testModel(model, name);
fs.writeFileSync('single-test-result.json', JSON.stringify(result, null, 2));
