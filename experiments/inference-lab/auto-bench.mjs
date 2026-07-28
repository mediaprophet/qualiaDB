#!/usr/bin/env node
/**
 * auto-bench.mjs — Automated WASM LLM benchmark runner.
 * Launches Chrome with WebGPU enabled, loads auto-bench.html,
 * captures results, and writes them to a JSON file.
 *
 * Usage: node experiments/inference-lab/auto-bench.mjs [--out results.json]
 */
import puppeteer from 'puppeteer';
import { writeFileSync, readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..', '..');
const SERVER_URL = 'http://localhost:8080/auto-bench.html';
const HISTORY_FILE = resolve(__dirname, 'bench-history.json');

const args = process.argv.slice(2);
const outIdx = args.indexOf('--out');
const outFile = outIdx >= 0 && args[outIdx + 1] ? args[outIdx + 1] : null;

function loadHistory() {
  if (existsSync(HISTORY_FILE)) {
    try { return JSON.parse(readFileSync(HISTORY_FILE, 'utf-8')); } catch { return []; }
  }
  return [];
}

function saveHistory(history) {
  writeFileSync(HISTORY_FILE, JSON.stringify(history, null, 2));
}

async function main() {
  console.log('Launching Chrome with WebGPU enabled…');

  const browser = await puppeteer.launch({
    head: false,
    args: [
      '--enable-unsafe-webgpu',
      '--ignore-gpu-blocklist',
      '--enable-features=Vulkan',
      '--disable-gpu-sandbox',
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--use-gl=angle',
      '--use-angle=d3d11',
      '--enable-features=Vulkan,SkiaVulkan,DawnUseDirect3D12',
    ],
  });

  const page = await browser.newPage();
  page.setDefaultTimeout(300000); // 5 min timeout

  // Capture console output — also write to log file for debugging
  const browserLog = [];
  page.on('console', (msg) => {
    const text = msg.text();
    browserLog.push(text);
    if (text.startsWith('BENCH_RESULT:')) {
      try {
        const data = JSON.parse(text.slice('BENCH_RESULT:'.length));
        if (data.status === 'complete') {
          console.log('\n=== BENCHMARK COMPLETE ===');
          console.log(JSON.stringify(data, null, 2));
        } else if (data.status?.includes('error')) {
          console.error(`ERROR: ${data.error || JSON.stringify(data)}`);
        } else {
          console.log(`[${data.status}] ${data.phase || ''} ${data.progress ? data.progress + '%' : ''}`);
        }
      } catch {}
    } else if (text.includes('Shader') || text.includes('WGSL') || text.includes('Pipeline') || text.includes('Invalid') || text.includes('validation') || text.includes('error')) {
      console.error(`[wgsl] ${text}`);
    } else {
      console.log(`[browser] ${text}`);
    }
  });

  page.on('pageerror', (err) => {
    console.error(`[page-error] ${err.message}`);
  });

  // Capture WGSL validation errors from browser
  page.on('requestfailed', (req) => {
    console.error(`[request-failed] ${req.url()} — ${req.failure()?.errorText}`);
  });

  console.log(`Navigating to ${SERVER_URL}…`);
  await page.goto(SERVER_URL, { waitUntil: 'domcontentloaded' });

  console.log('Waiting for benchmark to complete (up to 5 min)…');

  // Wait for window.__benchResult.status === 'complete'
  let result = null;
  const deadline = Date.now() + 300000;
  while (Date.now() < deadline) {
    result = await page.evaluate(() => window.__benchResult || null);
    if (result && result.status === 'complete') break;
    if (result && result.status === 'fatal_error') {
      console.error('Fatal error from page:', JSON.stringify(result, null, 2));
      break;
    }
    await new Promise(r => setTimeout(r, 2000));
  }

  await browser.close();

  // Always dump browser log for debugging
  const logFile = resolve(__dirname, 'auto-bench-browser.log');
  writeFileSync(logFile, browserLog.join('\n'));
  console.log(`Browser log saved to ${logFile}`);

  if (!result) {
    console.error('Timed out waiting for benchmark results.');
    process.exit(1);
  }

  if (result.status === 'fatal_error') {
    console.error('Benchmark failed:', result.error);
    if (outFile) writeFileSync(outFile, JSON.stringify(result, null, 2));
    process.exit(1);
  }

  // Save to history
  const history = loadHistory();
  const entry = {
    timestamp: result.timestamp || new Date().toISOString(),
    qualia: result.qualia,
    wllama: result.wllama,
    ratio: result.ratio,
    goal_achieved: result.goal_achieved,
  };
  history.push(entry);
  if (history.length > 100) history.shift();
  saveHistory(history);

  // Write to output file if specified
  if (outFile) {
    writeFileSync(outFile, JSON.stringify(entry, null, 2));
    console.log(`Results written to ${outFile}`);
  }

  // Print summary
  const qTps = result.qualia?.tokensPerSecond;
  const wTps = result.wllama?.tokensPerSecond;
  console.log('\n=== SUMMARY ===');
  console.log(`Qualia:  ${qTps ? qTps.toFixed(1) + ' tok/s' : 'FAILED'}`);
  console.log(`wllama:  ${wTps ? wTps.toFixed(1) + ' tok/s' : 'FAILED'}`);
  if (result.ratio) {
    console.log(`Ratio:   ${result.ratio.toFixed(2)}×`);
    console.log(`Goal:    ${result.goal_achieved ? 'ACHIEVED ✓' : 'PENDING — needs ' + Math.abs((result.ratio - 1) * 100).toFixed(0) + '% improvement'}`);
  }

  // Print history trend
  if (history.length > 1) {
    console.log('\n=== HISTORY TREND ===');
    history.forEach((h, i) => {
      const q = h.qualia?.tokensPerSecond;
      const w = h.wllama?.tokensPerSecond;
      const r = h.ratio;
      const date = new Date(h.timestamp).toLocaleString();
      const trend = i > 0 && q && history[i-1].qualia?.tokensPerSecond
        ? ` (${q > history[i-1].qualia.tokensPerSecond ? '+' : ''}${(q - history[i-1].qualia.tokensPerSecond).toFixed(1)} vs prev)`
        : '';
      console.log(`  #${i+1} ${date} — Qualia: ${q?.toFixed(1) || '—'} tok/s, wllama: ${w?.toFixed(1) || '—'} tok/s, ratio: ${r?.toFixed(2) || '—'}×${trend}`);
    });
  }

  process.exit(0);
}

main().catch(e => {
  console.error('Runner failed:', e);
  process.exit(1);
});
