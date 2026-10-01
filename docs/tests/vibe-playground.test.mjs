import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(new URL('../vibe/playground.html', import.meta.url), 'utf8');
const showcase = readFileSync(new URL('../vibe/showcase.html', import.meta.url), 'utf8');
const benches = readFileSync(new URL('../vibe/benchmarks.html', import.meta.url), 'utf8');

assert.match(html, /eval_program_src\(src\)/);
assert.doesNotMatch(html, /run_program_bytecode\(src,/);
assert.match(html, /diagnose_src/);
assert.match(html, /host_version/);
assert.match(html, /Cosmic geodesy/);
assert.match(html, /HID\.poll/);
assert.match(html, /pg-diagnose/);
assert.match(html, /let mut a = 0/);
assert.match(html, /let mut s = 0/);
assert.match(html, /oklch\(0\.72, 0\.12, 230\.0\)/);
assert.doesNotMatch(html, /return glow;/);
assert.doesNotMatch(html, /\[\^'\*/);
assert.doesNotMatch(html, /'\^'\*/);
assert.match(html, /pkg\/vibe\/vibe_wasm\.js/);

assert.match(showcase, /eval_program_src\(demo\.code\)/);
assert.match(showcase, /let mut a = 0/);
assert.match(showcase, /using Cosmic/);
assert.match(showcase, /effect fn main\(\) -> Record/);
assert.doesNotMatch(showcase, /return glow;/);

assert.match(benches, /runPresentJobs|Present purpose clocks/);
assert.match(benches, /CompiledCell\.compile|run\(\) ×2/);
assert.doesNotMatch(benches, /data-job="hostAsk"/);

const present = readFileSync(new URL('../js/vibescript-present.js', import.meta.url), 'utf8');
assert.match(present, /softRisePresentCards/);
assert.match(present, /PRESENT_STAGGER_MS\s*=\s*180/);
assert.match(present, /prefersReducedMotion|prefers-reduced-motion/);
assert.doesNotMatch(present, /hostAsk|ASK_SOURCE/);

console.log('Vibe playground contract tests passed.');
