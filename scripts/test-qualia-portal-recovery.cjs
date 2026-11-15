// Run only after acquiring the shared GPU lane. Requires a fresh `portal` WASM build.
// WASM_ASSET_DIR selects its source-stamped pkg; device-harness alone lacks QualiaPortal.
// Canonical stale-readback refusal is covered separately by test-qualia-browser-render-lifecycle.cjs.
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { chromium } = require('playwright');
const root = path.resolve(__dirname, '..');
const assets = path.resolve(process.env.WASM_ASSET_DIR || path.join(root,
  'target/inference-migration-qualification/noninference-wasm-20261104-05/pkg'));
const output = path.resolve(process.env.ARTIFACT_DIR || path.join(root, 'target/qualia-portal-recovery'));
const MAX_RECEIPT = 1024 * 1024, MAX_ASSETS = 64 * 1024 * 1024;
const receipt = { schema: 'qualia.portal-dom-recovery.v1', startedUtc: new Date().toISOString(),
  scope: 'Actual production Portal host, owned WebGPU device loss, DOM canvas replacement and retained scene rendering. Canonical stale callback refusal requires the separate render-lifecycle component receipt.',
  checks: [], console: [], assets: {}, sources: {} };
const digest = data => crypto.createHash('sha256').update(data).digest('hex');
const sources = ['scripts/test-qualia-portal-recovery.cjs', 'docs/js/qualia-shell.js',
  'crates/qualia-core-db/src/render/portal/mod.rs', 'crates/qualia-core-db/src/render/portal/recovery.rs',
  'crates/qualia-core-db/src/render/portal/scene_mesh.rs',
  'crates/qualia-core-db/src/render/gpu/portal_gpu/constructors.rs',
  'crates/qualia-core-db/src/container_10d/header.rs', 'crates/qualia-core-db/src/container_10d/section.rs',
  'crates/qualia-core-db/src/container_10d/mesh_section.rs'];

// Fixed 4-vertex/4-face tetrahedron: readable legacy-v1 .10d envelope, one QuantizedMesh
// section. Test vector follows container_10d/{header,section,mesh_section,integrity}.rs.
function tetrahedron() {
  const bytes = Buffer.alloc(176), payload = bytes.subarray(88);
  function crc(data) {
    let c = 0xffffffff;
    for (const b of data) { c ^= b; for (let i = 0; i < 8; i++) c = (c >>> 1) ^ ((c & 1) ? 0x82f63b78 : 0); }
    return (~c) >>> 0;
  }
  bytes.write('10d\0', 0, 'ascii'); bytes.writeUInt16LE(1, 4);
  Buffer.from([1, 1, 1, 2, 2, 2, 2, 2, 4, 2]).copy(bytes, 8);
  [[0, 1, 1016], [1, 2, 56], [2, 3, 56], [255, 4, 0]].forEach(([v, k, mask], i) => {
    bytes[20 + i * 8] = v; bytes[21 + i * 8] = k; bytes.writeUInt16LE(mask, 22 + i * 8);
  });
  bytes.writeUInt32LE(64, 56); bytes.writeUInt32LE(1, 60);
  bytes[64] = 1; bytes[65] = 1; bytes.writeUInt32LE(88, 68); bytes.writeUInt32LE(88, 72);
  payload.writeUInt16LE(1, 0); payload.writeUInt32LE(4, 4); payload.writeUInt32LE(4, 8);
  for (let i = 0; i < 3; i++) { payload.writeFloatLE(-1, 12 + i * 4); payload.writeFloatLE(1, 24 + i * 4); }
  [0, 0, 0, 65535, 0, 65535, 0, 65535, 65535, 65535, 65535, 0]
    .forEach((v, i) => payload.writeUInt16LE(v, 40 + i * 2));
  [0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3].forEach((v, i) => payload.writeUInt16LE(v, 64 + i * 2));
  bytes.writeUInt32LE(crc(payload), 84); bytes.writeUInt32LE(crc(bytes), 52);
  return bytes;
}

(async () => {
  let browser, server, watchdog;
  try {
    const files = new Map([
      ['/pkg/qualia/qualia.js', path.join(assets, 'qualia.js')],
      ['/pkg/qualia/qualia_bg.wasm', path.join(assets, 'qualia_bg.wasm')],
      ...['qualia-shell.js', 'ambient-viz.js', 'qualia-debug.js', 'wasm-fetch.js']
        .map(name => ['/js/' + name, path.join(root, 'docs/js', name)]),
    ]);
    const immutable = new Map();
    let total = 0;
    for (const [route, file] of files) {
      const size = fs.statSync(file).size;
      if (size > MAX_ASSETS - total) throw new Error('Asset snapshot exceeds 64 MiB');
      total += size;
      const data = fs.readFileSync(file);
      immutable.set(route, data);
      receipt.assets[route] = { file: path.relative(root, file), bytes: data.length, sha256: digest(data) };
    }
    const glue = immutable.get('/pkg/qualia/qualia.js').toString('utf8');
    for (const api of ['export class QualiaPortal', 'export function portal_init_webgpu', 'export function probe_portal_graphics']) {
      if (!glue.includes(api)) throw new Error('Missing Portal API: ' + api + '; build with portal feature');
    }
    for (const file of sources) receipt.sources[file] = digest(fs.readFileSync(path.join(root, file)));
    receipt.sourceCorrespondence = 'Source hashes describe inspection at run start; pair with the selected build provenance to prove artifact correspondence.';
    immutable.set('/fixture.10d', tetrahedron());
    receipt.assets['/fixture.10d'] = { bytes: 176, sha256: digest(immutable.get('/fixture.10d')) };
    server = http.createServer((req, res) => {
      res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
      res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
      const route = new URL(req.url, 'http://localhost').pathname;
      if (route === '/') {
        res.setHeader('Content-Type', 'text/html');
        return res.end('<!doctype html><title>Portal recovery fixture</title><canvas id="portal" style="width:320px;height:240px" width="320" height="240"></canvas>');
      }
      if (route === '/favicon.ico') { res.statusCode = 204; return res.end(); }
      const data = immutable.get(route);
      if (!data || req.method !== 'GET') { res.statusCode = 404; return res.end(); }
      res.setHeader('Content-Type', route.endsWith('.wasm') ? 'application/wasm' : route.endsWith('.js') ? 'text/javascript' : 'application/octet-stream');
      res.setHeader('Content-Length', data.length); res.end(data);
    });
    server.requestTimeout = 10000; server.headersTimeout = 10000; server.maxConnections = 12;
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH ||
      'C:/Users/Admin/AppData/Local/ms-playwright/chromium-1234/chrome-win64/chrome.exe',
      headless: true, timeout: 30000, args: ['--enable-unsafe-webgpu'] });
    receipt.browserVersion = browser.version();
    watchdog = setTimeout(() => { receipt.timeout = true; void browser.close(); }, 120000);
    const page = await browser.newPage({ viewport: { width: 640, height: 480 }, deviceScaleFactor: 1 });
    page.setDefaultTimeout(45000);
    const log = text => { if (receipt.console.length < 64) receipt.console.push(text.slice(0, 2048)); };
    page.on('console', message => log(message.type() + ': ' + message.text()));
    const pageErrors = [];
    page.on('pageerror', error => {
      const message = String(error).slice(0, 2048);
      if (pageErrors.length < 32) pageErrors.push(message);
      log(message);
    });
    await page.goto('http://127.0.0.1:' + server.address().port, { timeout: 30000 });
    let result;
    try { result = await page.evaluate(async () => {
      const check = (condition, message) => { if (!condition) throw new Error(message); };
      const devices = [], adapters = [], gpuErrors = [];
      window.__portalRecoveryGpuErrors = gpuErrors;
      const original = GPUAdapter.prototype.requestDevice;
      GPUAdapter.prototype.requestDevice = async function (descriptor) {
        const device = await original.call(this, descriptor);
        if (descriptor?.label === 'qualia-portal-gpu') {
          check(devices.length < 8, 'Unexpected device retry count');
          devices.push(device);
          device.addEventListener('uncapturederror', event => {
            if (gpuErrors.length < 32) gpuErrors.push(String(event.error?.message || event.error).slice(0, 2048));
          });
          const info = this.info;
          adapters.push(info ? { vendor: info.vendor, architecture: info.architecture, device: info.device, description: info.description } : { unavailable: true });
        }
        return device;
      };
      const shell = await import('/js/qualia-shell.js');
      let frames = 0, replacements = 0, latest;
      const wait = async predicate => {
        const start = performance.now();
        while (!predicate()) {
          check(performance.now() - start < 30000, 'Portal recovery stage exceeded 30s');
          await new Promise(resolve => setTimeout(resolve, 20));
        }
      };
      try {
        const loaded = await shell.loadQualiaPortal(document.querySelector('canvas'), {
          anatomyBackend: 'webgpu', allowWebGl2: false, requireBodyRenderer: true,
        });
        check(loaded.portal && loaded.renderer === 'webgpu', 'Portal must start on WebGPU');
        const portal = loaded.portal, oldCanvas = loaded.canvas;
        latest = oldCanvas;
        const geometry = new Uint8Array(await (await fetch('/fixture.10d')).arrayBuffer());
        const upload = portal.load_scene_assets([{ bytes: geometry, r: 0.9, g: 0.2, b: 0.1, a: 1, id: 'recovery-tetrahedron' }]);
        shell.startPortalLoop(oldCanvas, () => { frames++; }, (next, previous) => {
          check(previous === latest && !previous.isConnected && next.isConnected, 'Owned canvas replacement invalid');
          replacements++; latest = next;
        });
        await wait(() => portal.scene_render_receipt().success);
        await devices.at(-1).queue.onSubmittedWorkDone();
        await new Promise(resolve => setTimeout(resolve, 0));
        check(gpuErrors.length === 0, 'Initial Portal GPU errors: ' + JSON.stringify(gpuErrors));
        const before = portal.scene_render_receipt(), generation = portal.etalon_session_generation();
        check(before.renderer === 'webgpu' && before.vertex_count === 4 && before.index_count === 12, 'Initial geometry receipt mismatch');
        const oldDevice = devices.at(-1), deviceCount = devices.length;
        check(oldDevice, 'Owned Portal device was not captured');
        oldDevice.destroy();
        const loss = await oldDevice.lost;
        await wait(() => window.__qualiaGraphicsRecovery?.attempt === 1 && !portal.graphics_recovery_requested());
        const framesAfterRecovery = frames;
        const presentedAfterRecovery = portal.scene_render_receipt().frames_presented;
        await wait(() => frames > framesAfterRecovery + 2 && portal.scene_render_receipt().frames_presented > presentedAfterRecovery);
        const after = portal.scene_render_receipt();
        await devices.at(-1).queue.onSubmittedWorkDone();
        await new Promise(resolve => setTimeout(resolve, 0));
        check(gpuErrors.length === 0, 'Portal GPU errors: ' + JSON.stringify(gpuErrors));
        check(window.__qualiaGraphicsRecovery.backend === 'webgpu', 'Recovery fell back from WebGPU');
        check(replacements === 1 && latest !== oldCanvas && latest.id === oldCanvas.id && latest.width === oldCanvas.width && latest.height === oldCanvas.height, 'Canvas identity/dimensions not preserved');
        check(devices.length === deviceCount + 1 && devices.at(-1) !== oldDevice, 'New owned device missing');
        check(portal.etalon_session_generation() > generation, 'Steward generation did not advance');
        check(after.success && after.renderer === 'webgpu' && after.vertex_count === 4 && after.index_count === 12, 'Retained geometry not rendered after recovery');
        return { passed: true, upload, before, after, generationBefore: generation,
          generationAfter: portal.etalon_session_generation(), recovery: window.__qualiaGraphicsRecovery,
          loss: { reason: loss.reason, message: loss.message }, replacements, frames,
          deviceCount: devices.length, adapters, gpuErrors, userAgent: navigator.userAgent, crossOriginIsolated };
      } finally { shell.stopPortalLoop(); GPUAdapter.prototype.requestDevice = original; }
    }); } finally {
      receipt.pageErrors = pageErrors;
      receipt.gpuErrors = await page.evaluate(() => window.__portalRecoveryGpuErrors || []).catch(() => ['GPU error capture unavailable after browser/page closure']);
    }
    if (pageErrors.length) throw new Error('Unexpected browser page errors: ' + JSON.stringify(pageErrors));
    receipt.checks.push({ name: 'Actual Portal device loss and DOM recovery with retained geometry', ...result });
    receipt.outcome = 'passed';
  } catch (error) { receipt.outcome = 'failed'; receipt.error = String(error).slice(0, 4096); process.exitCode = 1; }
  finally {
    clearTimeout(watchdog);
    if (browser) await browser.close().catch(() => {});
    if (server) { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
    receipt.completedUtc = new Date().toISOString();
    const encoded = JSON.stringify(receipt, null, 2);
    if (Buffer.byteLength(encoded) > MAX_RECEIPT) throw new Error('Receipt exceeds 1 MiB');
    fs.mkdirSync(output, { recursive: true });
    fs.writeFileSync(path.join(output, '.qualia-portal-recovery'), 'qualia.portal-dom-recovery.v1\n');
    fs.writeFileSync(path.join(output, 'portal-recovery.json'), encoded);
    console.log(receipt.outcome + ': ' + path.join(output, 'portal-recovery.json'));
  }
})();
