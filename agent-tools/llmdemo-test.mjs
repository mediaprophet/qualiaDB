// Headless driver for docs/llmdemo/index.html — verifies the production demo boots from a
// (compiled+cached) .q42 and generates coherent text with a tok/s readout. Serves docs/ with
// COOP/COEP so the engine's WebGPU device + OPFS work without the coi-serviceworker reload.
import { chromium } from 'playwright';
import { createServer } from 'http';
import { readFileSync, statSync, existsSync } from 'fs';
import { join, extname } from 'path';
import { fileURLToPath } from 'url';

const ROOT = join(fileURLToPath(new URL('.', import.meta.url)), '..', 'docs');
const PORT = Number(process.env.WASM_TEST_PORT || 8791);
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.gguf': 'application/octet-stream', '.d.ts': 'text/plain', '.json': 'application/json' };

function serve(req, res) {
  const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
  let path = decodeURIComponent(url.pathname);
  if (path === '/') path = '/llmdemo/index.html';
  const file = join(ROOT, path.replace(/^\//, '').replace(/\//g, '\\'));
  if (!file.startsWith(ROOT) || !existsSync(file)) { res.writeHead(404); res.end('not found'); return; }
  const ext = extname(file);
  const st = statSync(file);
  const h = { 'Content-Type': MIME[ext] || 'application/octet-stream', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Accept-Ranges': 'bytes' };
  if (req.method === 'HEAD') { res.writeHead(200, { ...h, 'Content-Length': String(st.size) }); res.end(); return; }
  const range = req.headers.range;
  if (range) {
    const m = /bytes=(\d+)-(\d*)/.exec(range);
    if (m) {
      const start = Number(m[1]); const end = m[2] ? Number(m[2]) : st.size - 1;
      const buf = readFileSync(file);
      res.writeHead(206, { ...h, 'Content-Range': `bytes ${start}-${end}/${st.size}`, 'Content-Length': String(end - start + 1) });
      res.end(buf.subarray(start, end + 1)); return;
    }
  }
  res.writeHead(200, { ...h, 'Content-Length': String(st.size) }); res.end(readFileSync(file));
}

const server = createServer(serve);
await new Promise((r) => server.listen(PORT, '127.0.0.1', r));
const browser = await chromium.launch({ channel: 'chrome', headless: true, args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan'] });
const page = await browser.newPage();
const logs = [];
page.on('console', (m) => logs.push(m.text()));
page.on('pageerror', (e) => logs.push('[pageerror] ' + e.message));

await page.goto(`http://127.0.0.1:${PORT}/llmdemo/index.html`, { waitUntil: 'networkidle' });
// coi-serviceworker may reload once to install; wait for the load button to enable (WASM ready).
await page.waitForSelector('#btnLoadModel:not([disabled])', { timeout: 120000 });
console.log('WASM ready; clicking Load & Compile…');
await page.click('#btnLoadModel');
await page.waitForSelector('#btnGenerate:not([disabled])', { timeout: 600000 });
console.log('Engine resident; clicking Generate…');
await page.click('#btnGenerate');
await page.waitForFunction(() => document.getElementById('tps').textContent.trim().length > 0, null, { timeout: 600000 });

const out = (await page.locator('#genOutput').textContent()) || '';
const tps = (await page.locator('#tps').textContent()) || '';
const term = (await page.locator('#terminalOutput').textContent()) || '';
console.log('=== TPS ===');
console.log(tps);
console.log('=== OUTPUT (first 300) ===');
console.log(out.slice(0, 300));
console.log('=== TERMINAL TAIL ===');
console.log(term.split('\n').slice(-12).join('\n'));
const errs = logs.filter((l) => /error|Error|invalid|Invalid|does not match/.test(l));
console.log('=== CONSOLE ERRORS ===');
console.log(errs.length ? errs.slice(0, 10).join('\n') : '(none)');

await browser.close();
server.close();
process.exit(/Paris/i.test(out) ? 0 : 2);
