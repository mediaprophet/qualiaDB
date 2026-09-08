import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(fileURLToPath(new URL('../..', import.meta.url)));
const html = fs.readFileSync(path.join(root, 'docs/vibe/dev-docs.html'), 'utf8');
const json = JSON.parse(fs.readFileSync(path.join(root, 'docs/vibe/dev-docs.json'), 'utf8'));
const wasmLib = fs.readFileSync(path.join(root, 'crates/vibe-wasm/src/lib.rs'), 'utf8');

assert.match(html, /eval_program_src/);
assert.match(html, /effect fn main/);
assert.match(html, /LocalHost/);
assert.match(html, /trait Host/);
assert.match(html, /new URL\('dev-docs\.json'/);
assert.doesNotMatch(html, /fetch\("\{\{ '\/vibe\/dev-docs\.json' \| relative_url \}\}"\)/);
assert.match(html, /moduleFromHash/);
assert.match(html, /&gt;/);
assert.match(html, /a\.name === 'wasm'/);

const wasmMod = json.find((m) => m.name === 'wasm');
assert.ok(wasmMod, 'dev-docs.json must include module wasm');

const exportNames = [];
const wasmLines = wasmLib.split('\n');
for (let i = 0; i < wasmLines.length; i++) {
  if (!wasmLines[i].includes('#[wasm_bindgen]')) continue;
  for (let j = i + 1; j < Math.min(i + 6, wasmLines.length); j++) {
    const m = wasmLines[j].match(/^pub fn ([A-Za-z0-9_]+)/);
    if (m) {
      exportNames.push(m[1]);
      break;
    }
  }
}
assert.ok(exportNames.includes('eval_program_src'));
assert.ok(exportNames.length >= 20, `expected wasm-bindgen exports, got ${exportNames.length}`);

const jsonNames = new Set(wasmMod.items.map((item) => item.name));
for (const name of exportNames) {
  assert.ok(jsonNames.has(name), `dev-docs.json wasm module missing ${name}`);
}

const evalProgram = wasmMod.items.find((item) => item.name === 'eval_program_src');
assert.ok(evalProgram?.doc.includes('main'), 'eval_program_src must document calling main()');
assert.equal(
  evalProgram.line,
  wasmLines.findIndex((line) => line.includes('pub fn eval_program_src')) + 1,
);

const hostMod = json.find((m) => m.name === 'bind::host');
assert.ok(hostMod, 'dev-docs.json must include bind::host');
assert.ok(
  hostMod.items.some((item) => item.name === 'Host::time_now'),
  'Host trait methods must be indexed',
);

console.log(
  `Vibe dev-docs tests passed (${json.length} modules, wasm ${exportNames.length} exports).`,
);
