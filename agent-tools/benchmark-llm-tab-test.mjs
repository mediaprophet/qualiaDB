// Verify benchmark.html's integrated "Browser LLM" tab: clicking it lazy-loads benchmarks.html
// into the iframe and that document mounts. Does NOT wait for full LLM init (heavy) — just that
// the integration wiring (tab activate → iframe src → embedded page loads) works.
import { chromium } from 'playwright';
import { createServer } from 'http';
import { readFileSync, statSync, existsSync } from 'fs';
import { join, extname } from 'path';
import { fileURLToPath } from 'url';

const ROOT = join(fileURLToPath(new URL('.', import.meta.url)), '..', 'docs');
const PORT = 8792;
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.css': 'text/css', '.gguf': 'application/octet-stream' };
const server = createServer((req, res) => {
  let p = decodeURIComponent(new URL(req.url, `http://127.0.0.1:${PORT}`).pathname);
  if (p === '/') p = '/benchmark.html';
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
await page.goto(`http://127.0.0.1:${PORT}/benchmark.html`, { waitUntil: 'networkidle' });

const hasBtn = await page.locator('#tab-llm-btn').count();
console.log('Browser LLM tab button present:', hasBtn === 1);
await page.click('#tab-llm-btn');
await page.waitForFunction(() => { const f = document.getElementById('llm-bench-frame'); return f && f.getAttribute('src'); }, null, { timeout: 10000 });
const src = await page.getAttribute('#llm-bench-frame', 'src');
console.log('iframe src after click:', src);
// Confirm the embedded benchmarks.html actually mounted (its <title>).
const frame = page.frames().find((f) => /benchmarks\.html/.test(f.url()));
let frameTitle = '(frame not found)';
if (frame) { try { await frame.waitForLoadState('domcontentloaded', { timeout: 20000 }); frameTitle = await frame.title(); } catch (e) { frameTitle = 'load error: ' + e.message; } }
console.log('embedded frame title:', frameTitle);
console.log('pane active:', await page.locator('#tab-llm.tab-pane.active').count() === 1);
console.log('pageerrors:', errs.length ? errs.slice(0, 5).join(' | ') : '(none)');
await browser.close();
server.close();
process.exit(hasBtn === 1 && src === 'benchmarks.html' && /LLM/i.test(frameTitle) ? 0 : 2);
