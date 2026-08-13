import assert from 'node:assert/strict';
import fs from 'node:fs';

const html = fs.readFileSync(new URL('../online-llm-demo.html', import.meta.url), 'utf8');
const wasmPackage = JSON.parse(
  fs.readFileSync(new URL('../playground/package.json', import.meta.url), 'utf8'),
);
const optionPattern = /<option\s+value="([^"]+)"[^>]*data-name="([^"]+)"[^>]*>/g;
const options = [...html.matchAll(optionPattern)];

assert.equal(options.length, 1, 'the public catalogue must contain only certified browser models');
assert.equal(
  options[0][1],
  'https://huggingface.co/HuggingFaceTB/SmolLM2-360M-Instruct-GGUF/resolve/main/smollm2-360m-instruct-q8_0.gguf',
);
assert.match(options[0][2], /SmolLM2-360M.*Q8_0/);
assert.match(html, /data-bytes="386404992"/);
assert.match(html, /data-sha256="48ab3034d0dd401fbc721eb1df3217902fee7dab9078992d66431f09b7750201"/);
assert.match(html, /data-cache-key="smollm2-360m-instruct-q8_0\.browser-decode-r2\.p64"/);
assert.match(html, /data-legacy-cache-key="smollm2-360m-instruct-q8_0\.gguf"/);
assert.match(html, /clearOpfsModel/);
assert.match(html, /recoverVerifiedModel/);
assert.match(html, /Clear cached Q8 model &amp; reload/);
assert.doesNotMatch(html, /releases\/download\/v0\.0\.24\/.*\.(p64|gguf)/);
assert.doesNotMatch(html, /value="models\/.*\.gguf"/);
assert.match(html, /inferWasmAsyncMeasured/);
assert.match(html, /returned no visible text/);
assert.match(html, /qualia_core_db\.js\?v=0\.0\.29-mobile-performance1/);
assert.match(html, /qualia_core_db_bg\.wasm\?v=0\.0\.29-mobile-performance1/);
assert.equal(wasmPackage.version, '0.0.29');

console.log('Online LLM demo catalogue tests passed.');
