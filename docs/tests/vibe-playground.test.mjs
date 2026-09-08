import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(new URL('../vibe/playground.html', import.meta.url), 'utf8');

assert.match(html, /diagnose_src/);
assert.match(html, /host_version/);
assert.match(html, /Cosmic geodesy/);
assert.match(html, /HID\.poll/);
assert.match(html, /pg-diagnose/);
assert.doesNotMatch(html, /\[\^'\*/);
assert.doesNotMatch(html, /'\^'\*/);
assert.match(html, /pkg\/vibe\/vibe_wasm\.js/);

console.log('Vibe playground contract tests passed.');
