import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
import { File } from 'node:buffer';
import { boundedText, validateManifest, validateCases, counts, serializeReport, REPORT_LIMIT } from '../js/device-harness/report.js';
import { reportEmailDraft, reportSharePayload, REPORT_FILENAME, MAILTO_LIMIT } from '../js/device-harness/email.js';
import { ADAPTER_DIAGNOSTICS_VERSION, probeWebGpuAdapters, probeWebGlRenderer, summarizeAdapterDiagnostics } from '../js/device-harness/webgpu-diagnostics.js';

const manifest = {
  schema_version: 1, qualia_revision: 'a'.repeat(40), etalon_revision: 'b'.repeat(40),
  dirty: false,
  assets: Object.fromEntries(['portal_js', 'portal_wasm'].map(key => [key, { url: `../pkg/${key}`, sha256: 'c'.repeat(64), bytes: 123 }])),
};
const base = 'https://example.test/project/playground/device-harness-build.json';
assert.equal(validateManifest(manifest, base).assets.portal_wasm.url, 'https://example.test/project/pkg/portal_wasm');
assert.throws(() => validateManifest({ ...manifest, qualia_revision: 'trunk' }, base));
assert.throws(() => validateManifest({ ...manifest, dirty: undefined }, base));
assert.throws(() => validateManifest({ ...manifest, assets: { ...manifest.assets, portal_js: { ...manifest.assets.portal_js, url: 'https://foreign.test/a' } } }, base));
assert.throws(() => validateManifest({ ...manifest, assets: { ...manifest.assets, portal_wasm: { ...manifest.assets.portal_wasm, bytes: 1024 ** 3 } } }, base));
const cases = [
  { name: 'canonical_vibe_graph_transaction', execution_class: 'cpu', outcome: 'passed' },
  { name: 'canonical_vibe_capability_rejection', execution_class: 'cpu', outcome: 'unsupported' },
];
const envelope = { schema_version: 1, scope: 'qualia-vibe-cpu', cases };
assert.deepEqual(validateCases(JSON.stringify(envelope), 'cpu'), cases);
assert.throws(() => validateCases(JSON.stringify({ ...envelope, cases: [] }), 'cpu'));
assert.throws(() => validateCases(JSON.stringify({ ...envelope, cases: [cases[0], cases[0]] }), 'cpu'));
assert.throws(() => validateCases(JSON.stringify({ ...envelope, scope: 'native-etalon' }), 'cpu'));
assert.throws(() => validateCases(JSON.stringify({ ...envelope, cases: cases.map(x => ({ ...x, execution_class: 'npu' })) }), 'cpu'));
assert.equal(counts(cases).cpu.passed, 1);
assert.equal(counts(cases).cpu.unsupported, 1);
assert.equal(counts(cases).npu.passed, 0);
assert.equal(boundedText('x'.repeat(4096)).length, 2048);
const oversized = serializeReport({ cases, logs: ['🙂'.repeat(REPORT_LIMIT)] });
assert.ok(new TextEncoder().encode(oversized).length < REPORT_LIMIT);
assert.equal(JSON.parse(oversized).outcome, 'failed');
assert.equal(JSON.parse(oversized).execution_counts.cpu.passed, 0);
console.log('Device harness report checks passed');

const uiSource = (await readFile(new URL('../js/device-harness/main.js', import.meta.url), 'utf8'))
  .replace(/^import .*;\r?\n/gm, '').replaceAll('import.meta.url', JSON.stringify('https://example.test/js/device-harness/main.js'));
function uiFixture(secure = true, navigatorOverrides = {}, adapterProbe = probeWebGpuAdapters) {
  const elements = new Map();
  const timers = new Map();
  const workers = [];
  const links = [];
  let nextTimer = 0;
  const element = id => {
    if (!elements.has(id)) elements.set(id, { disabled: false, textContent: '', handlers: {}, addEventListener(type, callback) { this.handlers[type] = callback; } });
    return elements.get(id);
  };
  class FakeWorker {
    constructor() { workers.push(this); this.requests = []; this.terminated = false; }
    postMessage(message) { this.requests.push(message); }
    terminate() { this.terminated = true; }
    deliver(type, value) { this.onmessage?.({ data: { type, value } }); }
  }
  vm.runInNewContext(uiSource, {
    boundedText, serializeReport, reportEmailDraft, reportSharePayload: report => reportSharePayload(report, File), REPORT_FILENAME,
    ADAPTER_DIAGNOSTICS_VERSION, probeWebGpuAdapters: adapterProbe, probeWebGlRenderer, summarizeAdapterDiagnostics,
    document: { getElementById: element, createElement() { const link = { click() { links.push(this.href); } }; return link; } },
    navigator: { userAgent: 'test browser', ...navigatorOverrides },
    isSecureContext: secure, crossOriginIsolated: false, globalThis: { crypto: secure ? { subtle: {} } : undefined },
    Worker: FakeWorker, WebAssembly: {}, URL, Blob, Date,
    setTimeout(callback, milliseconds) { const id = ++nextTimer; timers.set(id, { callback, milliseconds }); return id; },
    clearTimeout(id) { timers.delete(id); }, addEventListener() {},
  });
  return { element, workers, timers, links, receipt: () => JSON.parse(element('report').textContent) };
}
const timeoutUi = uiFixture();
timeoutUi.element('run').handlers.click();
const timedWorker = timeoutUi.workers[0];
assert.equal(timedWorker.requests[0].type, 'load');
timedWorker.deliver('ready', { build: manifest, assets_verified: true, build_attribution: 'verified' });
timedWorker.deliver('cpu', cases.map(x => ({ ...x, outcome: 'passed' })));
assert.equal(timedWorker.requests.at(-1).type, 'gpu');
const gpuDeadline = [...timeoutUi.timers.values()].find(x => x.milliseconds === 60000);
assert.ok(gpuDeadline);
gpuDeadline.callback();
assert.equal(timedWorker.terminated, true);
assert.equal(timeoutUi.receipt().outcome, 'failed');
assert.equal(timeoutUi.receipt().execution_counts.cpu.passed, 2);
assert.equal(timeoutUi.receipt().execution_counts.gpu.passed, 0);
assert.equal(timeoutUi.element('download').disabled, false);
assert.equal(timedWorker.onmessage, null);
const lanUi = uiFixture(false);
lanUi.element('run').handlers.click();
const lanWorker = lanUi.workers[0];
assert.ok(lanWorker, 'plain HTTP still starts CPU diagnostics');
lanWorker.deliver('ready', { build: manifest, assets_verified: false, build_attribution: 'manifest declared, unverified over insecure HTTP' });
lanWorker.deliver('cpu', cases.map(x => ({ ...x, outcome: 'passed' })));
lanWorker.deliver('gpu', [{ name: 'etalon_browser_compute_readback', execution_class: 'gpu', outcome: 'unsupported' }]);
assert.equal(lanUi.receipt().assets_verified, false);
assert.equal(lanUi.receipt().execution_counts.gpu.unsupported, 1);
assert.equal(lanUi.receipt().execution_counts.gpu.passed, 0);
assert.match(lanUi.element('status').textContent, /Limited HTTP/);
console.log('Device harness worker timeout and LAN UI checks passed');

const shareReport = { outcome: 'completed', assets_verified: true, build: manifest, cases, errors: [], logs: [] };
const payload = reportSharePayload(shareReport, File);
assert.equal(payload.files.length, 1);
assert.equal(payload.files[0].name, REPORT_FILENAME);
assert.equal(payload.files[0].type, 'application/json');
assert.equal(await payload.files[0].text(), serializeReport(shareReport));
assert.ok(payload.files[0].size <= REPORT_LIMIT);
const largePayload = reportSharePayload({ ...shareReport, logs: ['🙂'.repeat(REPORT_LIMIT)] }, File);
assert.ok(largePayload.files[0].size < REPORT_LIMIT);
assert.equal(JSON.parse(await largePayload.files[0].text()).outcome, 'failed');
const draft = reportEmailDraft(shareReport, 'me+device@example.test');
assert.ok(new TextEncoder().encode(draft.url).length <= MAILTO_LIMIT);
assert.ok(draft.url.startsWith('mailto:me%2Bdevice%40example.test?'));
assert.match(new URL(draft.url).searchParams.get('body'), /COMPACT SUMMARY ONLY/);
assert.match(draft.body, /attach it manually/);
const unavailableGpuDraft = reportEmailDraft({
  ...shareReport, browser: { webgpu_api: true, secure_context: true },
  cases: [{ name: 'etalon_browser_compute_readback', execution_class: 'gpu', outcome: 'unsupported',
    details: { reason: 'Browser returned no compatibility WebGPU adapter' } }],
});
assert.match(unavailableGpuDraft.body, /GPU reason: Browser returned no compatibility/);
assert.match(unavailableGpuDraft.body, /WebGPU API: true; secure context: true/);
assert.ok(new TextEncoder().encode(unavailableGpuDraft.url).length <= MAILTO_LIMIT);
assert.doesNotThrow(() => reportEmailDraft({ ...shareReport, cases: [{
  name: 'etalon_browser_compute_readback', execution_class: 'gpu', outcome: 'unsupported',
  details: { reason: '不可用🙂'.repeat(300) },
}] }));
assert.ok(!draft.body.includes(JSON.stringify(cases)));
assert.throws(() => reportEmailDraft(shareReport, 'me@example.test\r\nBcc:other@example.test'));
assert.throws(() => reportEmailDraft(shareReport, 'a'.repeat(255) + '@x'));
assert.throws(() => reportEmailDraft(shareReport, 'é'.repeat(250) + '@x'), /URL budget/);
assert.equal(shareReport.recipient, undefined);

const shared = [];
const shareUi = uiFixture(true, { canShare: value => value.files.length === 1, async share(value) { shared.push(value); } });
shareUi.element('run').handlers.click();
shareUi.element('cancel').handlers.click();
shareUi.element('email-recipient').value = 'private-self@example.test';
const reportBeforeShare = shareUi.element('report').textContent;
await shareUi.element('email').handlers.click();
assert.equal(shared.length, 1);
assert.equal(await shared[0].files[0].text(), reportBeforeShare);
assert.equal(shareUi.links.length, 0);
assert.ok(!shareUi.element('report').textContent.includes('private-self'));
assert.match(shareUi.element('status').textContent, /cannot confirm sending/);

const cancelShareUi = uiFixture(true, { canShare: () => true, async share() { throw Object.assign(new Error('cancelled'), { name: 'AbortError' }); } });
cancelShareUi.element('run').handlers.click();
cancelShareUi.element('cancel').handlers.click();
const cancelledReceipt = cancelShareUi.element('report').textContent;
await cancelShareUi.element('email').handlers.click();
assert.equal(cancelShareUi.element('report').textContent, cancelledReceipt);
assert.equal(cancelShareUi.receipt().outcome, 'cancelled');
assert.equal(cancelShareUi.element('download').disabled, false);
assert.equal(cancelShareUi.links.length, 0);
assert.match(cancelShareUi.element('status').textContent, /Sharing cancelled/);

const fallbackUi = uiFixture(true, { canShare: () => false });
fallbackUi.element('run').handlers.click();
fallbackUi.element('cancel').handlers.click();
fallbackUi.element('email-recipient').value = 'me+device@example.test';
await fallbackUi.element('email').handlers.click();
assert.equal(fallbackUi.links.length, 1);
assert.ok(fallbackUi.links[0].startsWith('mailto:me%2Bdevice%40example.test?'));
assert.ok(new TextEncoder().encode(fallbackUi.links[0]).length <= MAILTO_LIMIT);
assert.equal(fallbackUi.element('download').disabled, false);
assert.match(fallbackUi.element('status').textContent, /attach it manually/);
fallbackUi.element('email-recipient').value = 'é'.repeat(250) + '@x';
await fallbackUi.element('email').handlers.click();
assert.equal(fallbackUi.links.length, 1, 'oversized draft must not navigate or truncate');
assert.match(fallbackUi.element('status').textContent, /URL budget/);
assert.equal(fallbackUi.element('download').disabled, false);
console.log('Device harness file sharing, cancellation, and bounded email fallback checks passed');

const noApi = await probeWebGpuAdapters({}, 'worker');
assert.equal(noApi.api_available, false);
assert.deepEqual(noApi.attempts, []);
const requests = [];
const refused = await probeWebGpuAdapters({ gpu: { async requestAdapter(options) { requests.push(options); return null; } } }, 'worker');
assert.deepEqual(requests, [
  { featureLevel: 'core', forceFallbackAdapter: false },
  { featureLevel: 'compatibility', forceFallbackAdapter: false },
]);
assert.deepEqual(refused.attempts.map(x => x.outcome), ['null', 'null']);
const available = await probeWebGpuAdapters({ gpu: { async requestAdapter() {
  return { info: { vendor: 'example', description: 'x'.repeat(4096), isFallbackAdapter: false },
    features: new Set(['core-features-and-limits']), requestDevice() { assert.fail('Discovery must not request a device'); } };
} } }, 'window');
assert.equal(available.gpu_execution, false);
assert.equal(available.attempts[0].info.description.length, 256);
assert.equal(available.attempts[0].is_fallback_adapter, false);
assert.equal(available.attempts[0].core_features_and_limits, true);
const rejected = await probeWebGpuAdapters({ gpu: { requestAdapter() { throw new Error('x'.repeat(4096)); } } }, 'worker');
assert.equal(rejected.attempts[0].outcome, 'error');
assert.equal(rejected.attempts[0].error.length, 512);
let hungRequests = 0;
const hung = await probeWebGpuAdapters({ gpu: { requestAdapter() { hungRequests++; return new Promise(() => {}); } } }, 'worker', 5);
assert.equal(hung.attempts[0].outcome, 'timeout');
assert.equal(hungRequests, 1, 'Do not enqueue another request after a timeout');
assert.equal(summarizeAdapterDiagnostics({ window: refused, worker: refused }), 'browser_refused_core_and_compatibility_in_both_contexts');
assert.equal(summarizeAdapterDiagnostics({ window: available, worker: refused }), 'window_adapter_available_worker_unavailable');
assert.equal(summarizeAdapterDiagnostics({ window: refused, worker: available }), 'raw_worker_adapter_available_after_rust_refusal');
assert.equal(summarizeAdapterDiagnostics({ window: hung, worker: refused }), 'inconclusive');
const diagnosticDraft = reportEmailDraft({ ...shareReport,
  webgpu_diagnostics: { classification: 'browser_refused_core_and_compatibility_in_both_contexts',
    webgl_inventory: { powervr_25_1_string_observed: true } },
});
assert.match(diagnosticDraft.body, /Browser refused core and compatibility in page and worker/);
assert.match(diagnosticDraft.body, /WebGL string resembles PowerVR 25\.1/);
assert.ok(new TextEncoder().encode(diagnosticDraft.url).length <= MAILTO_LIMIT);
let releasedGl = false;
const webgl = probeWebGlRenderer({ createElement() { return { getContext() { return {
  RENDERER: 1, VERSION: 2,
  getExtension(name) { return name === 'WEBGL_lose_context' ? { loseContext() { releasedGl = true; } } : null; },
  getParameter(key) { return key === 1 ? 'PowerVR driver 25.1 build' : 'WebGL 1.0'; },
}; } }; } });
assert.equal(webgl.powervr_25_1_string_observed, true);
assert.equal(webgl.gpu_execution, false);
assert.equal(webgl.unmasked, false);
assert.equal(releasedGl, true);
assert.equal(probeWebGlRenderer({ createElement() { return { getContext() { return null; } }; } }).outcome, 'unavailable');

function startUnsupported(ui) {
  ui.element('run').handlers.click();
  const worker = ui.workers.at(-1);
  worker.deliver('ready', { build: manifest, assets_verified: true });
  worker.deliver('cpu', cases.map(x => ({ ...x, outcome: 'passed' })));
  worker.deliver('gpu', [{ name: 'etalon_browser_compute_readback', execution_class: 'gpu', outcome: 'unsupported', details: { gpu_execution: false } }]);
  return worker;
}
for (const [windowProbe, workerProbe, classification] of [
  [refused, refused, 'browser_refused_core_and_compatibility_in_both_contexts'],
  [available, refused, 'window_adapter_available_worker_unavailable'],
  [refused, available, 'raw_worker_adapter_available_after_rust_refusal'],
]) {
  const ui = uiFixture(true, {}, async () => windowProbe);
  const worker = startUnsupported(ui);
  assert.equal(worker.requests.at(-1).type, 'adapter-diagnostics');
  worker.deliver('adapter-diagnostics', workerProbe);
  await Promise.resolve();
  assert.equal(ui.receipt().webgpu_diagnostics.classification, classification);
  assert.equal(ui.receipt().execution_counts.gpu.passed, 0);
  assert.equal(ui.receipt().execution_counts.gpu.unsupported, 1);
  assert.equal(ui.receipt().outcome, 'completed');
  assert.equal(worker.terminated, true);
}
let resolveLate;
const diagnosticTimeoutUi = uiFixture(true, {}, () => new Promise(resolve => { resolveLate = resolve; }));
const diagnosticWorker = startUnsupported(diagnosticTimeoutUi);
const diagnosticDeadline = [...diagnosticTimeoutUi.timers.values()].find(x => x.milliseconds === 10000);
assert.ok(diagnosticDeadline);
diagnosticDeadline.callback();
assert.equal(diagnosticWorker.terminated, true);
assert.equal(diagnosticTimeoutUi.receipt().webgpu_diagnostics.classification, 'diagnostic_deadline_exceeded');
assert.equal(diagnosticTimeoutUi.receipt().execution_counts.cpu.passed, 2);
assert.equal(diagnosticTimeoutUi.receipt().execution_counts.gpu.unsupported, 1);
assert.equal(diagnosticTimeoutUi.receipt().outcome, 'completed');
diagnosticTimeoutUi.element('run').handlers.click();
resolveLate(available);
await Promise.resolve();
assert.equal(diagnosticTimeoutUi.receipt().webgpu_diagnostics, undefined, 'A late probe must not change a newer run');
diagnosticTimeoutUi.element('cancel').handlers.click();
console.log('Bounded WebGPU differential diagnostics and termination checks passed');
