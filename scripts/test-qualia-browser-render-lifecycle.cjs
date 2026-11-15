// Requires existing Playwright and a device-harness WASM build. No model/assets downloaded.
// Run after obtaining the shared GPU lane. NO_SAB=1 selects nonisolated transfer mode.
const http = require('node:http'), fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { chromium } = require('playwright');
const root = path.resolve(__dirname, '..');
const assets = path.resolve(process.env.WASM_ASSET_DIR || path.join(root, 'docs/pkg/device-harness'));
const basename = process.env.WASM_BASENAME || 'qualia';
const out = path.resolve(process.env.ARTIFACT_DIR || path.join(root, 'target/qualia-browser-runtime'));
const files = new Map([
  ['/qualia.js', path.join(assets, basename + '.js')],
  ['/' + basename + '_bg.wasm', path.join(assets, basename + '_bg.wasm')],
  ['/transport.js', path.join(root, 'coordination/worktrees/etalon-migration/crates/etalon-wasm/js/worker_transport.js')],
  ['/worker.mjs', path.join(__dirname, 'fixtures/render-lifecycle-worker.mjs')],
]);
const receipt = { schema: 'qualia.browser-render-lifecycle.v1', startedUtc: new Date().toISOString(),
  expectedTransport: process.env.NO_SAB ? 'transferable' : 'shared', checks: [], console: [],
  scope: 'Canonical render/picking/compute lifecycle through actual worker transport; independent native OffscreenCanvas ownership transfer and canonical stewardship admission; GPU renders to offscreen texture, not transferred canvas; Portal DOM recovery not exercised',
  artifacts: Object.fromEntries([...files.values(), __filename,
    path.join(root, 'crates/qualia-core-db/src/etalon_vibe_browser/render_lifecycle.rs'),
    path.join(root, 'coordination/worktrees/etalon-migration/crates/etalon-render/src/picking_readback.rs'),
    path.join(root, 'coordination/worktrees/etalon-migration/crates/etalon-render/src/compute_readback.rs'),
    path.join(root, 'coordination/worktrees/etalon-migration/crates/etalon-wasm/src/worker_steward.rs'),
    path.join(root, 'coordination/worktrees/etalon-migration/crates/etalon-wasm/src/readback.rs')].map(file => [path.relative(root, file),
    crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex')])) };
const server = http.createServer((req, res) => {
  if (!process.env.NO_SAB) {
    res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
    res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
  }
  const route = new URL(req.url, 'http://localhost').pathname;
  if (route === '/') { res.setHeader('Content-Type', 'text/html'); return res.end('<!doctype html><title>Render lifecycle fixture</title>'); }
  if (route === '/favicon.ico') { res.statusCode = 204; return res.end(); }
  const file = files.get(route);
  if (!file) { res.statusCode = 404; return res.end(); }
  res.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
  res.setHeader('Content-Length', fs.statSync(file).size);
  fs.createReadStream(file).pipe(res);
});
(async () => {
  let browser;
  try {
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH ||
      'C:/Users/Admin/AppData/Local/ms-playwright/chromium-1234/chrome-win64/chrome.exe',
      headless: true, args: ['--enable-unsafe-webgpu'] });
    const page = await browser.newPage();
    page.setDefaultTimeout(180000);
    page.on('pageerror', error => { if (receipt.console.length < 64) receipt.console.push(String(error).slice(0, 2048)); });
    await page.goto('http://127.0.0.1:' + server.address().port);
    const execution = page.evaluate(async () => {
      const { EtalonWorkerTransport } = await import('/transport.js');
      let id = 0;
      const checks = [];
      const canvasElement = document.createElement('canvas');
      const offscreen = canvasElement.transferControlToOffscreen();
      let duplicateRejected = false;
      try { canvasElement.transferControlToOffscreen(); } catch { duplicateRejected = true; }
      checks.push({ name: 'Native OffscreenCanvas exclusive transfer (independent of GPU render target)', passed: duplicateRejected });
      let canvas = offscreen;
      for (const action of ['cancel', 'teardown', 'device-loss']) {
        const worker = new Worker('/worker.mjs', { type: 'module' });
        const waiting = new Map();
        const transport = new EtalonWorkerTransport(worker, value => {
          const pending = waiting.get(value.id);
          if (!pending) return;
          waiting.delete(value.id);
          clearTimeout(pending.timer);
          value.error ? pending.reject(new Error(value.error)) : pending.resolve(value);
        }, true, 1024 * 1024);
        worker.onmessage = event => transport.handle(event.data);
        worker.onerror = event => {
          for (const pending of waiting.values()) { clearTimeout(pending.timer); pending.reject(new Error(event.message)); }
          waiting.clear();
        };
        const request = (command, payload = {}, transfer = []) => new Promise((resolve, reject) => {
          const requestId = ++id;
          const timer = setTimeout(() => { waiting.delete(requestId); reject(new Error(command + ' exceeded 45s')); }, 45000);
          waiting.set(requestId, { resolve, reject, timer });
          // Canvas is a distinct native ownership transfer. Lifecycle commands and
          // GPU outputs use canonical SAB/transferable message transport.
          try {
            if (transfer.length) worker.postMessage({ id: requestId, command, ...payload }, transfer);
            else transport.send({ id: requestId, command, ...payload });
          }
          catch (error) { clearTimeout(timer); waiting.delete(requestId); reject(error); }
        });
        try {
          const attached = await request('attach', { canvas }, [canvas]);
          const response = await request('run', { action });
          const expectedShared = crossOriginIsolated;
          checks.push({ name: action, passed: attached.attached && attached.shared === expectedShared &&
            response.shared === expectedShared && response.result.outcome === 'passed', ...response });
          const detached = await request('detach');
          canvas = detached.canvas;
          checks.push({ name: action + ' actual canvas returned for replacement worker',
            passed: canvas instanceof OffscreenCanvas && canvas.width === 300 });
        } finally { worker.terminate(); for (const pending of waiting.values()) clearTimeout(pending.timer); }
      }
      return { crossOriginIsolated, checks };
    });
    let timer;
    try {
      const observed = await Promise.race([execution, new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error('Lifecycle fixture exceeded 180s')), 180000);
      })]);
      receipt.crossOriginIsolated = observed.crossOriginIsolated;
      receipt.checks = observed.checks;
      if (receipt.crossOriginIsolated !== !process.env.NO_SAB) throw new Error('Unexpected transport isolation');
      if (receipt.checks.some(check => !check.passed)) throw new Error('Lifecycle assertion failed or unsupported; inspect explicit outcome');
    } finally { clearTimeout(timer); }
  } catch (error) { receipt.error = String(error); process.exitCode = 1; }
  finally {
    if (browser) await browser.close();
    server.close();
    receipt.completedUtc = new Date().toISOString();
    if (receipt.console.length) process.exitCode = 1;
    const bytes = Buffer.from(JSON.stringify(receipt, null, 2));
    if (bytes.length > 1024 * 1024) throw new Error('Receipt exceeds 1MiB');
    fs.mkdirSync(out, { recursive: true });
    const file = path.join(out, 'browser-render-lifecycle-' + (process.env.NO_SAB ? 'transferable' : 'shared') + '.json');
    fs.writeFileSync(file, bytes);
    console.log(file);
  }
})();
