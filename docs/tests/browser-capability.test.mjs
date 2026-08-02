import assert from 'node:assert/strict';
import {
  BROWSER_CAPABILITY_SCHEMA,
  probeBrowserCapabilities,
  recordBackendDeviceOutcome,
  selectBrowserBackends,
} from '../js/browser-capability.js';

const fakeDocument = (webgl2) => ({
  createElement: () => ({
    getContext: (kind) => kind === 'webgl2' ? webgl2 : null,
  }),
});

const fakeGl = {
  getExtension: () => null,
};

const nullAdapterReceipt = await probeBrowserCapabilities({
  navigatorObject: {
    hardwareConcurrency: 8,
    gpu: { requestAdapter: async () => null },
  },
  documentObject: fakeDocument(fakeGl),
  secureContext: true,
  crossOriginIsolated: true,
  Worker: class {},
  SharedArrayBuffer,
  engineVersion: 'test',
  now: 0,
});

assert.equal(nullAdapterReceipt.schema, BROWSER_CAPABILITY_SCHEMA);
assert.equal(nullAdapterReceipt.webgpu.apiPresent, true);
assert.equal(nullAdapterReceipt.webgpu.adapterAvailable, false);
assert.equal(nullAdapterReceipt.webgpu.state, 'adapter_unavailable');
assert.equal(nullAdapterReceipt.webgpu.attempts.length, 4);
assert.equal(nullAdapterReceipt.webgl2.available, true);
assert.equal(nullAdapterReceipt.selection.llm, 'cpu-wasm');
assert.equal(nullAdapterReceipt.selection.anatomy, 'webgl2');

const adapter = {
  info: { vendor: 'Qualia', architecture: 'test', device: 'fixture', description: 'fixture adapter' },
  limits: { maxBufferSize: 1024, maxComputeInvocationsPerWorkgroup: 256 },
  features: new Set(['shader-f16']),
};
const gpuReceipt = await probeBrowserCapabilities({
  navigatorObject: { gpu: { requestAdapter: async () => adapter } },
  documentObject: fakeDocument(null),
  now: 0,
});
assert.equal(gpuReceipt.webgpu.adapterAvailable, true);
assert.equal(gpuReceipt.webgpu.adapterAttempt, 'compatibility-default');
assert.equal(gpuReceipt.selection.llm, 'webgpu');
assert.equal(gpuReceipt.selection.anatomy, 'webgpu');

const unsupported = selectBrowserBackends({
  webgpu: { adapterAvailable: false, state: 'api_absent' },
  webgl2: { available: false },
  wasm: { available: false },
});
assert.deepEqual(unsupported, {
  llm: 'unsupported',
  llmReason: 'wasm_unavailable',
  anatomy: 'unsupported',
  anatomyReason: 'no_hardware_renderer',
});

recordBackendDeviceOutcome(gpuReceipt, 'anatomy', { backend: 'webgpu', state: 'available' });
assert.equal(gpuReceipt.webgpu.device.acquired, true);
assert.equal(gpuReceipt.anatomyBackend.backend, 'webgpu');

console.log('Browser capability contract tests passed.');
