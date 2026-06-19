// Isolate the prefill(n>=40) path: load SmolLM2-360M (the proven model) via the SAME path
// online-llm-demo uses (compileGgufToQ42 -> initialize_webgpu_engine -> inferWasmStreaming) but with a
// LONG prompt that forces a full 40-token prefill chunk. Tells us whether the prefill bind bug is
// model-specific (1B only) or prompt-length-specific (any model, n>=40).
import { chromium } from 'playwright';
import { createServer } from 'http';
import { readFileSync, statSync, existsSync } from 'fs';
import { join, extname } from 'path';
import { fileURLToPath } from 'url';

const ROOT = join(fileURLToPath(new URL('.', import.meta.url)), '..', 'docs');
const PORT = Number(process.env.WASM_TEST_PORT || 8794);
const MODEL = process.env.MODEL || '/models/SmolLM2-360M-Instruct-Q4_K_M.gguf';
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.gguf': 'application/octet-stream', '.json': 'application/json' };

const TEST_HTML = `<!doctype html><meta charset=utf8><body><pre id=log></pre><script type="module">
const log = (m) => { document.getElementById('log').textContent += m + "\\n"; console.log(m); };
window.__done = false; window.__ok = false;
try {
  const mod = await import('/playground/qualia_core_db.js');
  await mod.default();
  mod.init_panic_hook?.();
  log('wasm ready; fetching model ${MODEL}');
  const buf = new Uint8Array(await (await fetch('${MODEL}')).arrayBuffer());
  log('gguf bytes: ' + buf.length);
  log('compiling GGUF -> .q42 …');
  const q42 = mod.compileGgufToQ42(buf, 14);
  log('.q42 bytes: ' + q42.length);
  log('initialize_webgpu_engine …');
  await mod.initialize_webgpu_engine(q42);
  log('engine ready: ' + (mod.isWebgpuEngineReady?.() ?? '?'));
  // LONG prompt — well over 40 tokens to force a full prefill chunk (n=40).
  const prompt = 'You are a helpful assistant. Please answer the following question clearly and concisely, '
    + 'using complete sentences and accurate facts about world geography and capital cities. '
    + 'Here is the question that I would like you to answer for me today: what is the capital city of France?';
  log('prompt words: ' + prompt.split(/\\s+/).length);
  let out = '';
  log('inferWasmStreaming …');
  await mod.inferWasmStreaming(prompt, (d) => { out += d; });
  log('OUTPUT: ' + out.slice(0, 200));
  window.__out = out;
  window.__ok = /paris/i.test(out);
} catch (e) { log('THREW: ' + (e && e.message || e)); window.__err = String(e && e.message || e); }
window.__done = true;
</script>`;

function serve(req, res) {
  let path = decodeURIComponent(new URL(req.url, `http://127.0.0.1:${PORT}`).pathname);
  if (path === '/_test') {
    res.writeHead(200, { 'Content-Type': 'text/html', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' });
    res.end(TEST_HTML); return;
  }
  const file = join(ROOT, path.replace(/^\//, '').replace(/\//g, '\\'));
  if (!file.startsWith(ROOT) || !existsSync(file)) { res.writeHead(404); res.end('nf'); return; }
  const ext = extname(file); const st = statSync(file);
  const h = { 'Content-Type': MIME[ext] || 'application/octet-stream', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Accept-Ranges': 'bytes' };
  const range = req.headers.range;
  if (range) {
    const m = /bytes=(\d+)-(\d*)/.exec(range);
    if (m) { const s = Number(m[1]); const e = m[2] ? Number(m[2]) : st.size - 1; const b = readFileSync(file);
      res.writeHead(206, { ...h, 'Content-Range': `bytes ${s}-${e}/${st.size}`, 'Content-Length': String(e - s + 1) }); res.end(b.subarray(s, e + 1)); return; }
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
await page.goto(`http://127.0.0.1:${PORT}/_test`, { waitUntil: 'load' });
await page.waitForFunction(() => window.__done === true, null, { timeout: 600000 });
const ok = await page.evaluate(() => window.__ok);
const err = await page.evaluate(() => window.__err || '');

console.log('\n=== CONSOLE (engine) ===');
console.log(logs.join('\n'));
console.log('\n=== prefill failures? ===', logs.filter((l) => /PREFILL chunk FAILED/i.test(l)).length);
console.log('=== binding/256 errors? ===', logs.filter((l) => /minimum binding size|Binding size|MC8GemmBGL|getMappedRange/i.test(l)).length);
console.log('=== threw:', err || '(none)');
console.log('=== Paris in output:', ok);

await browser.close();
server.close();
process.exit(ok ? 0 : 2);
