#!/usr/bin/env node
/**
 * Phase 7.1 — assert Qualia portal WASM size budget.
 * Usage: node docs/tests/wasm-size-check.mjs [path/to/qualia_bg.wasm]
 */
import { readFileSync, existsSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const defaultPath = resolve(__dirname, '../pkg/qualia/qualia_bg.wasm');
const wasmPath = process.argv[2] ? resolve(process.argv[2]) : defaultPath;

const MAX_RAW_BYTES = 2 * 1024 * 1024;   // 2 MB raw (portal slim target)
const MAX_GZIP_BYTES = 800 * 1024;       // 800 KB gzip (well under 8 MB CI ceiling)

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