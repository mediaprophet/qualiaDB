import { boundedText, validateManifest, validateCases } from './report.js';
import { probeWebGpuAdapters } from './webgpu-diagnostics.js';

let exports;
let phase = 'load';
let assetsVerified = false;
const send = (type, value) => postMessage({ type, value });
let logCount = 0;
for (const method of ['log', 'warn', 'error']) {
  console[method] = (...args) => {
    if (logCount++ < 64) send('log', { level: method, text: boundedText(args.slice(0, 8).map(x => boundedText(x, 256)).join(' ')) });
  };
}

async function readBounded(url, limit, expected) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 90000);
  try {
    const response = await fetch(url, { cache: 'no-store', credentials: 'omit', signal: controller.signal, redirect: 'error' });
    if (!response.ok || !response.body) throw new Error(`Asset HTTP ${response.status}`);
    const declared = Number(response.headers.get('content-length'));
    if (declared > limit) throw new Error('Asset exceeds byte budget');
    const reader = response.body.getReader();
    const parts = [];
    let size = 0;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > limit) { await reader.cancel(); throw new Error('Asset exceeds byte budget'); }
      parts.push(value);
    }
    if (expected !== undefined && size !== expected) throw new Error('Asset byte count mismatch');
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const part of parts) { bytes.set(part, offset); offset += part.byteLength; }
    return bytes;
  } finally { clearTimeout(timer); }
}

async function loadAsset(asset) {
  const bytes = await readBounded(asset.url, asset.bytes, asset.bytes);
  if (!self.isSecureContext) return bytes;
  const digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), x => x.toString(16).padStart(2, '0')).join('');
  if (digest !== asset.sha256) throw new Error('Asset SHA256 mismatch');
  return bytes;
}

async function load(manifestUrl) {
  if (self.isSecureContext && !globalThis.crypto?.subtle) throw new Error('Secure context has no SHA256 implementation');
  const raw = await readBounded(manifestUrl, 64 * 1024);
  const build = validateManifest(JSON.parse(new TextDecoder().decode(raw)), manifestUrl);
  send('manifest', build);
  const js = await loadAsset(build.assets.portal_js);
  const wasm = await loadAsset(build.assets.portal_wasm);
  assetsVerified = self.isSecureContext;
  const source = new TextDecoder('utf-8', { fatal: true }).decode(js);
  if (/\b(?:import|export)\s+(?:[^;\n]*\sfrom\s*)?['"]|\bimport\s*\(/m.test(source)) {
    throw new Error('Verified glue must be a self-contained module');
  }
  const blobUrl = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
  try {
    exports = await import(blobUrl);
    if (typeof exports.default !== 'function'
      || typeof exports.run_etalon_vibe_cpu_harness !== 'function'
      || typeof exports.run_etalon_vibe_gpu_harness !== 'function') throw new Error('Harness exports missing from diagnostics build');
    await exports.default({ module_or_path: wasm });
  } finally { URL.revokeObjectURL(blobUrl); }
  phase = 'cpu';
  send('ready', {
    assets_verified: assetsVerified, build,
    build_attribution: assetsVerified ? 'asset bytes match manifest SHA256; source attribution as declared in build record'
      : 'manifest declared, unverified over insecure HTTP',
  });
}

self.onmessage = async ({ data }) => {
  try {
    if (data?.type === 'load' && !exports && phase === 'load') await load(data.manifestUrl);
    else if (data?.type === 'cpu' && phase === 'cpu') {
      phase = 'running_cpu';
      send('cpu', validateCases(exports.run_etalon_vibe_cpu_harness(), 'cpu'));
      phase = 'gpu';
    } else if (data?.type === 'gpu' && phase === 'gpu') {
      phase = 'running_gpu';
      send('gpu', assetsVerified ? validateCases(await exports.run_etalon_vibe_gpu_harness(), 'gpu') : [{
        name: 'etalon_browser_compute_readback', execution_class: 'gpu', outcome: 'unsupported',
        details: { reason: 'Browser WebGPU requires a secure context; use HTTPS or localhost', gpu_execution: false, execution: 'not_run' },
      }]);
      phase = 'done';
    } else if (data?.type === 'adapter-diagnostics' && phase === 'done') {
      phase = 'adapter_diagnostics';
      send('adapter-diagnostics', await probeWebGpuAdapters(navigator, 'worker'));
      phase = 'done';
    } else throw new Error('Invalid harness phase');
  } catch (error) {
    send('failure', { phase, message: boundedText(error?.stack ?? error) });
  }
};
self.onunhandledrejection = event => send('failure', { phase, message: boundedText(event.reason) });
