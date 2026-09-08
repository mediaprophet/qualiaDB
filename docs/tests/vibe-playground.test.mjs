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

assert.match(benches, /let mut a = 0/);
assert.match(benches, /let mut s = 0/);

console.log('Vibe playground contract tests passed.');
