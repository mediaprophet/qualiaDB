#!/usr/bin/env node
// Walk rustdoc JSON and emit a Vibe *backlog*, not bindings.
// rustdoc lists types; Vibe binds operations. Filter to free functions +
// inherent methods that look callable (no Self-only constructors).

import { readFileSync } from "node:fs";

const path = process.argv[2];
if (!path) {
  console.error("usage: node vibe-coverage-from-rustdoc.mjs <qualia_core_db.json>");
  process.exit(2);
}

const doc = JSON.parse(readFileSync(path, "utf8"));
const index = doc.index || {};
const paths = doc.paths || {};

const skip = [
  "tests",
  "bench",
  "fuzz",
  "stub",
  "mock",
];

function keepName(name) {
  const n = name.toLowerCase();
  return !skip.some((s) => n.includes(s));
}

const fns = [];
for (const [id, item] of Object.entries(index)) {
  if (!item || typeof item !== "object") continue;
  const name = item.name;
  if (!name || !keepName(String(name))) continue;
  const kind = item.inner && typeof item.inner === "object"
    ? Object.keys(item.inner)[0]
    : item.kind;
  if (kind !== "function" && kind !== "method") continue;
  const p = paths[id];
  const cratePath = Array.isArray(p?.path) ? p.path.join("::") : name;
  fns.push(cratePath);
}

fns.sort();
const roots = new Map();
for (const p of fns) {
  const root = p.split("::")[1] || p.split("::")[0] || "crate";
  roots.set(root, (roots.get(root) || 0) + 1);
}

console.log(`# rustdoc function backlog (${fns.length} names)`);
console.log("");
console.log("Use this to pick the *next* invoke file, not to auto-generate wrappers.");
console.log("Desktop Vibe → native host. WASM subset is whatever compiles under wasm-ontology / wasm-scientific.");
console.log("");
console.log("| module | public fn/method count |");
console.log("|---|---|");
for (const [k, n] of [...roots.entries()].sort((a, b) => b[1] - a[1])) {
  console.log(`| \`${k}\` | ${n} |`);
}
console.log("");
console.log("## first 80 names");
for (const p of fns.slice(0, 80)) {
  console.log(`- \`${p}\``);
}
