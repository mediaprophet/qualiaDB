#!/usr/bin/env node
/**
 * Assert a WASM artifact against a size budget.
 *
 * Usage:
 *   node docs/tests/wasm-size-check.mjs [path] [maxRawBytes] [maxGzipBytes]
 *
 * Defaults match the GitHub Pages / release-wasm portal + playground sanity cap
 * (16 MiB raw / 4 MiB gzip). CI always passes explicit limits:
 *
 *   Ontology MCP  655360 / 204800     (640 KiB / 200 KiB)
 *   Portal        16777216 / 4194304  (16 MiB / 4 MiB)
 *   Playground    16777216 / 4194304  (16 MiB / 4 MiB)
 *
 * Ontology MCP stays the tight product budget. Portal / wasm-full are a sanity
 * cap for the full WASM-safe engine, not a slim viewport bundle.
 */
import { readFileSync, existsSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const defaultPath = resolve(__dirname, '../pkg/qualia/qualia_bg.wasm');
const wasmPath = process.argv[2] ? resolve(process.argv[2]) : defaultPath;

const MAX_RAW_BYTES = process.argv[3]
  ? Number(process.argv[3])
  : 16 * 1024 * 1024;
const MAX_GZIP_BYTES = process.argv[4]
  ? Number(process.argv[4])
  : 4 * 1024 * 1024;

if (!existsSync(wasmPath)) {
    console.error(`[wasm-size] missing: ${wasmPath}`);
    process.exit(1);
}

const raw = readFileSync(wasmPath);
const gz = gzipSync(raw);
const rawMb = (raw.length / (1024 * 1024)).toFixed(2);
const gzKb = (gz.length / 1024).toFixed(0);

console.log(`[wasm-size] raw=${raw.length} (${rawMb} MB) gzip=${gz.length} (${gzKb} KB)`);

let failed = false;
if (raw.length > MAX_RAW_BYTES) {
    console.error(`[wasm-size] FAIL raw ${raw.length} > ${MAX_RAW_BYTES}`);
    failed = true;
}
if (gz.length > MAX_GZIP_BYTES) {
    console.error(`[wasm-size] FAIL gzip ${gz.length} > ${MAX_GZIP_BYTES}`);
    failed = true;
}

if (failed) process.exit(1);
console.log('[wasm-size] OK');
