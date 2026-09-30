import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const page = readFileSync(new URL('../benchmark.html', import.meta.url), 'utf8');
const harness = readFileSync(new URL('../js/vibescript-bench.js', import.meta.url), 'utf8');
const worker = readFileSync(new URL('../js/vibescript-bench-worker.js', import.meta.url), 'utf8');
const shared = readFileSync(new URL('../js/benchmark-shared.js', import.meta.url), 'utf8');
const geometry = readFileSync(new URL('../js/geometry-bench.js', import.meta.url), 'utf8');
const geometryParallel = readFileSync(new URL('../js/geometry-parallel-bench.js', import.meta.url), 'utf8');
const geometryWorker = readFileSync(new URL('../js/geometry-bench-worker.js', import.meta.url), 'utf8');

assert.match(harness, /\.\.\/pkg\/vibe\/vibe_wasm\.js/);
assert.match(harness, /decode_and_run/);
assert.match(harness, /run_cell_bytecode/);
assert.match(harness, /eval_cell_src/);
assert.match(harness, /vibescript-bench-worker\.js/);
assert.match(worker, /vibe_wasm\.js/);
assert.match(worker, /one WASM instance per worker|independent WASM instances/);
assert.doesNotMatch(harness, /class VibeScriptVM/);
assert.doesNotMatch(harness, /BigInt64Array/);

assert.match(page, /browser single thread/);
assert.match(page, /independent WASM workers/);
assert.match(page, /Native Verification Required/);
assert.match(page, /Native Tensor Benchmark Required/);
assert.match(page, /Browser result boundary/);
assert.match(page, /cannot benchmark native Rust performance/);
assert.match(shared, /GPU probe \/ use/);
assert.match(shared, /accelerator_used_for_this_suite/);
assert.match(page, /Dense Geometry Scene Preview/);
assert.match(page, /browser Canvas scene preview/);
assert.match(geometry, /scenePoints/);
assert.match(geometry, /Math\.min\(count, 3000\)/);
assert.doesNotMatch(page, /res\.opsPerSec \* 1\.75/);
assert.match(page, /Parallel-model boundary/);
assert.match(page, /independent geometry workers/);
assert.match(geometryParallel, /runGeometryWithTopology/);
assert.match(geometryParallel, /geometry-bench-worker\.js/);
assert.match(geometryWorker, /runGeometryLive/);
assert.match(geometry, /sampleVolumetricSDF/);
assert.doesNotMatch(page, /Simulated results/);
assert.doesNotMatch(page, /Simulate zero heap allocation/);

console.log('VibeScript browser benchmark integrity tests passed.');
