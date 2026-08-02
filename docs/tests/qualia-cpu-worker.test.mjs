import assert from 'node:assert/strict';
import fs from 'node:fs';

const client = fs.readFileSync(new URL('../js/qualia-cpu-worker-client.js', import.meta.url), 'utf8');
const worker = fs.readFileSync(new URL('../js/qualia-cpu-worker.js', import.meta.url), 'utf8');
const demo = fs.readFileSync(new URL('../online-llm-demo.html', import.meta.url), 'utf8');

assert.match(client, /new Worker\(WORKER_URL, \{ type: 'module'/, 'CPU execution must use a module worker');
assert.match(client, /\[modelBuffer\]/, 'model bytes must transfer to the worker without a second JS copy');
assert.match(worker, /initializeCpuWasmEngine/, 'worker must own CPU-WASM initialization');
assert.match(worker, /inferWasmAsyncMeasured/, 'worker must own measured decode');
assert.match(worker, /getBrowserExecutionReceipt/, 'worker must return an execution receipt');
assert.match(worker, /executionHost: 'dedicated-worker'/, 'receipt must declare its worker host');
assert.match(worker, /wasmSimd128: true/, 'receipt must declare the SIMD build');
assert.match(worker, /packedQ8Gemv: true/, 'receipt must declare the packed Q8 kernel');
assert.match(demo, /cpuWorker\.initialize\(modelBytes\)/, 'demo must initialize CPU-WASM in the worker');
assert.match(demo, /cpuWorker\.infer\(prompt, maxTokens, onToken\)/, 'demo must decode in the worker');
assert.doesNotMatch(demo, /:\s*initializeCpuWasmEngine\(modelBytes\)/, 'demo must not initialize CPU-WASM on the UI thread');

console.log('Qualia CPU-WASM worker contract passed.');
