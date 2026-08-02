import assert from 'node:assert/strict';
import fs from 'node:fs';

const html = fs.readFileSync(new URL('../online-llm-demo.html', import.meta.url), 'utf8');
const glue = fs.readFileSync(new URL('../playground/qualia_core_db.js', import.meta.url), 'utf8');
const mobileLab = fs.readFileSync(new URL('../js/mobile-wasm-lab.js', import.meta.url), 'utf8');
const cpuWorkerClient = fs.readFileSync(new URL('../js/qualia-cpu-worker-client.js', import.meta.url), 'utf8');

assert.match(glue, /export function initializeCpuWasmEngine\(/);
assert.match(glue, /export function isWasmEngineReady\(/);
assert.match(glue, /export function getWasmBackend\(/);
assert.match(html, /getBrowserCapabilityReceipt/);
assert.match(html, /capabilities\.selection\.llm/);
assert.match(html, /selectedInferenceBackend !== 'webgpu'/);
assert.match(html, /Chrome is not exposing WebGPU; Qualia will use CPU-WASM/);
assert.match(html, /BACKEND_OVERRIDE === 'cpu'/);
assert.match(html, /BACKEND_OVERRIDE === 'cpu-wasm'/);
assert.match(html, /\? initialize_webgpu_engine\(modelBytes\)\s*: cpuWorker\.initialize\(modelBytes\)/);
assert.match(html, /cpuWorker\.infer\(prompt, maxTokens, onToken\)/);
assert.match(cpuWorkerClient, /type: 'module'/);
assert.doesNotMatch(html, /WebGPU is not available in this browser — model loading is disabled/);
assert.doesNotMatch(html, /WebGPU is required for Qualia native inference/);
assert.doesNotMatch(html, /Adapter blocked by Chrome/);
assert.match(mobileLab, /getBrowserCapabilityReceipt/);
assert.match(mobileLab, /capabilityReceipt/);
assert.doesNotMatch(mobileLab, /requestAdapter\(\{ powerPreference: 'high-performance' \}\)/);

console.log('Online LLM CPU-WASM fallback wiring passed.');
