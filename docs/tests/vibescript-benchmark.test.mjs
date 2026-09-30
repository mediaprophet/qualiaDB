import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const page = readFileSync(new URL('../benchmark.html', import.meta.url), 'utf8');
const harness = readFileSync(new URL('../js/vibescript-bench.js', import.meta.url), 'utf8');
const worker = readFileSync(new URL('../js/vibescript-bench-worker.js', import.meta.url), 'utf8');
const shared = readFileSync(new URL('../js/benchmark-shared.js', import.meta.url), 'utf8');
const geometry = readFileSync(new URL('../js/geometry-bench.js', import.meta.url), 'utf8');
const geometryParallel = readFileSync(new URL('../js/geometry-parallel-bench.js', import.meta.url), 'utf8');
const geometryWorker = readFileSync(new URL('../js/geometry-bench-worker.js', import.meta.url), 'utf8');
const wasmDts = readFileSync(new URL('../pkg/vibe/vibe_wasm.d.ts', import.meta.url), 'utf8');

assert.match(harness, /\.\.\/pkg\/vibe\/vibe_wasm\.js/);
assert.match(harness, /CompiledCell/);
assert.match(harness, /check_cell_src/);
assert.match(harness, /apply_structural_edit/);
assert.match(harness, /purposeDemos|checked cell|CompiledCell\.compile/);
assert.match(harness, /vibescript-bench-worker\.js/);
assert.match(worker, /CompiledCell/);
assert.match(worker, /independent WASM instances|one WASM instance per worker|Independent instances/);
assert.doesNotMatch(harness, /class VibeScriptVM/);
assert.doesNotMatch(harness, /BigInt64Array/);
assert.doesNotMatch(harness, /directJavaScript|1 \+ 2 \* 3 - 4;\s*$/m);

assert.match(page, /purpose clocks|does the binding run/i);
assert.match(page, /Checked cell/);
assert.match(page, /Compile once|Run ×2/);
assert.doesNotMatch(page, /vibe-purpose-ask/);
assert.doesNotMatch(harness, /ASK_SOURCE|host ask \(graph\?\)/);
assert.match(page, /Edit without host rebuild/);
assert.match(page, /CompiledCell\.run/);
assert.match(page, /independent WASM workers/);
assert.match(page, /Native Verification Required/);
assert.match(page, /Native Tensor Benchmark Required/);
assert.doesNotMatch(page, /Browser result boundary/);
assert.doesNotMatch(page, /vs JavaScript Baseline/);
assert.doesNotMatch(page, /updateMetrics\([^)]*0,\s*24\)/);
assert.doesNotMatch(page, /honesty-banner/);
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

assert.match(wasmDts, /export class CompiledCell/);
assert.match(wasmDts, /run\(\): any/);
assert.match(wasmDts, /static compile\(src: string\): CompiledCell/);

console.log('VibeScript browser benchmark integrity tests passed.');
