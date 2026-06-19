// Smoke-check the API Explorer after catalog.js edits: page loads, catalog.js parses (module
// import resolves), the new Phase 5 LLM entries render, no page errors.
import { chromium } from 'playwright';
import { createServer } from 'http';
import { readFileSync, statSync, existsSync } from 'fs';
import { join, extname } from 'path';
import { fileURLToPath } from 'url';

const ROOT = join(fileURLToPath(new URL('.', import.meta.url)), '..', 'docs');
const PORT = 8793;
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.css': 'text/css' };
const server = createServer((req, res) => {
  let p = decodeURIComponent(new URL(req.url, `http://127.0.0.1:${PORT}`).pathname);
  if (p === '/' || p === '/api-explorer/') p = '/api-explorer/index.html';
  const file = join(ROOT, p.replace(/^\//, '').replace(/\//g, '\\'));
  if (!file.startsWith(ROOT) || !existsSync(file)) { res.writeHead(404); res.end('nf'); return; }
  res.writeHead(200, { 'Content-Type': MIME[extname(file)] || 'application/octet-stream', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Content-Length': String(statSync(file).size) });
  res.end(readFileSync(file));
});
await new Promise((r) => server.listen(PORT, '127.0.0.1', r));
const browser = await chromium.launch({ channel: 'chrome', headless: true, args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan'] });
const page = await browser.newPage();
const errs = [];
page.on('pageerror', (e) => errs.push(e.message));
page.on('console', (m) => { if (m.type() === 'error') errs.push('[console.error] ' + m.text()); });
await page.goto(`http://127.0.0.1:${PORT}/api-explorer/`, { waitUntil: 'networkidle' });
await page.waitForTimeout(1500);
const body = await page.locator('body').innerText();
const has = (s) => body.includes(s);
console.log('catalog rendered (has "initialize_webgpu_engine"):', has('initialize_webgpu_engine'));
console.log('new entry compileGgufToQ42:', has('compileGgufToQ42'));
console.log('new entry q42FormatVersion:', has('q42FormatVersion'));
console.log('new entry inferWasmStreaming:', has('inferWasmStreaming'));
console.log('header v0.0.18:', has('v0.0.18'));
// pageerrors filtered: ignore benign WebGPU-limits-shim/adapter notices
const real = errs.filter((e) => !/limits-shim|requestAdapter|WebGPU|Failed to load resource/i.test(e));
console.log('real page errors:', real.length ? real.slice(0, 6).join(' | ') : '(none)');
await browser.close();
server.close();
process.exit(has('compileGgufToQ42') && has('initialize_webgpu_engine') && real.length === 0 ? 0 : 2);
