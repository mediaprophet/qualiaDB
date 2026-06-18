#!/usr/bin/env node
// Stamp a content-hash `?v=` query onto shared static assets across every HTML
// page, so a fresh deploy never serves a stale browser-cached copy. The links
// have no hash of their own, so a returning visitor can otherwise keep an old
// `tailwind-built.css` (this is exactly what caused the "WASM Engine Required"
// overlay to stay stuck after a clean rebuild — see WASM_PAGES_AUDIT §1/§4.2).
//
// Intended to run at BUILD time against the published output tree (e.g. `_site`)
// so the committed source HTML stays diff-clean. Idempotent: re-running replaces
// any existing `?v=` it added.
//
//   node docs/scripts/stamp-asset-versions.mjs <target-dir>
//
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';

const target = process.argv[2];
if (!target) {
  console.error('usage: stamp-asset-versions.mjs <target-dir>  (e.g. _site)');
  process.exit(2);
}
const root = resolve(target);

// Assets to version, relative to `root`. Each is content-hashed; every HTML
// reference to it (with any number of `../` prefixes) gets `?v=<hash>` appended.
const ASSETS = ['css/tailwind-built.css'];

const SKIP_DIRS = new Set(['rustdoc', 'node_modules']);

function shortHash(absPath) {
  return createHash('sha256').update(readFileSync(absPath)).digest('hex').slice(0, 10);
}

function listHtml(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    if (SKIP_DIRS.has(name) || name.startsWith('.')) continue;
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) out.push(...listHtml(p));
    else if (name.endsWith('.html')) out.push(p);
  }
  return out;
}

function escapeRe(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

const hashes = {};
for (const a of ASSETS) {
  const abs = join(root, a);
  if (existsSync(abs)) hashes[a] = shortHash(abs);
  else console.warn(`stamp-asset-versions: asset not found, skipping: ${a}`);
}

let edits = 0;
for (const file of listHtml(root)) {
  let html = readFileSync(file, 'utf8');
  let changed = false;
  for (const [asset, hash] of Object.entries(hashes)) {
    // (href|src)="<../ prefixes>css/tailwind-built.css[?existing-query]"
    const re = new RegExp(
      `((?:href|src)=")((?:\\.\\./)*)${escapeRe(asset)}(?:\\?[^"]*)?(")`,
      'g',
    );
    html = html.replace(re, (_m, pre, prefix, post) => {
      changed = true;
      return `${pre}${prefix}${asset}?v=${hash}${post}`;
    });
  }
  if (changed) {
    writeFileSync(file, html);
    edits++;
  }
}

console.log(`stamp-asset-versions: ${edits} HTML file(s) updated under ${root}`);
for (const [a, h] of Object.entries(hashes)) console.log(`  ${a} -> ?v=${h}`);
