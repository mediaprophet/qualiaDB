import { chromium } from 'playwright';
import fs from 'fs';

const URL = 'http://localhost:8888/llmdemo/index.html';
const TIMEOUT = 120000;

async function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

async function main() {
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

    console.log('Navigating...');
    await page.goto(URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForFunction(() => {
        const el = document.getElementById('btnLoadModel');
        return el && !el.disabled;
    }, { timeout: 30000 }).catch(() => {});

    // Select SmolLM2
    await page.selectOption('#modelSelect', 'smollm2-360m');
    await sleep(500);

    console.log('Loading model...');
    await page.click('#btnLoadModel');
    await page.waitForFunction(() => {
        const el = document.getElementById('btnGenerate');
        return el && !el.disabled;
    }, { timeout: TIMEOUT }).catch(() => {});

    const status = await page.evaluate(() => document.getElementById('sysStatus')?.textContent || '');
    const terminal = await page.evaluate(() => document.getElementById('terminalOutput')?.textContent || '');
    console.log('Status:', status);
    console.log('Terminal:', terminal);

    console.log('Generating...');
    await page.click('#btnGenerate');

    // Wait and poll
    for (let i = 0; i < 60; i++) {
        await sleep(2000);
        const term = await page.evaluate(() => document.getElementById('terminalOutput')?.textContent || '');
        const gen = await page.evaluate(() => document.getElementById('genOutput')?.textContent || '');
        const btnDisabled = await page.evaluate(() => document.getElementById('btnGenerate')?.disabled);
        if (term.includes('Done:') || term.includes('Inference error') || (!btnDisabled && i > 2)) {
            console.log(`\n--- Final Output ---`);
            console.log('Terminal:', term);
            console.log('Generated:', gen);
            break;
        }
        if (i % 5 === 0) {
            console.log(`  [${i*2}s] terminal: ${term.substring(term.length-200)}...`);
            console.log(`  [${i*2}s] gen: "${gen.substring(0, 100)}"`);
        }
    }

    const finalTerm = await page.evaluate(() => document.getElementById('terminalOutput')?.textContent || '');
    const finalGen = await page.evaluate(() => document.getElementById('genOutput')?.textContent || '');
    const finalTps = await page.evaluate(() => document.getElementById('tps')?.textContent || '');
    console.log('\n=== FINAL ===');
    console.log('Terminal:', finalTerm);
    console.log('Generated:', finalGen);
    console.log('TPS:', finalTps);
    console.log('Errors:', errors);

    fs.writeFileSync('single-test-result.json', JSON.stringify({ logs, errors, terminal: finalTerm, output: finalGen, tps: finalTps }, null, 2));
    await browser.close();
}

main().catch(e => { console.error(e); process.exit(1); });
