import assert from 'node:assert/strict';
import fs from 'node:fs';

const { fetchWasmBinary, withAssetVersion, isTransientHttpStatus, WASM_ASSET_VERSION } =
    await import('../js/wasm-fetch.js');

assert.equal(WASM_ASSET_VERSION, '0.0.35-wasm-retry1');
assert.equal(isTransientHttpStatus(503), true);
assert.equal(isTransientHttpStatus(404), false);

const versioned = withAssetVersion('https://example.test/playground/qualia_core_db_bg.wasm');
assert.equal(
    versioned.href,
    'https://example.test/playground/qualia_core_db_bg.wasm?v=0.0.35-wasm-retry1',
);
const pinned = withAssetVersion(
    'https://example.test/playground/qualia_core_db_bg.wasm?v=already',
);
assert.equal(pinned.searchParams.get('v'), 'already');

const originalFetch = globalThis.fetch;
const calls = [];
globalThis.fetch = async (url) => {
    calls.push(String(url));
    const status = calls.length < 3 ? 503 : 200;
    return {
        ok: status >= 200 && status < 300,
        status,
    };
};
try {
    const resp = await fetchWasmBinary(
        'https://example.test/playground/qualia_core_db_bg.wasm',
        { attempts: 5, delayMs: 0 },
    );
    assert.equal(resp.status, 200);
    assert.equal(calls.length, 3);
    assert.match(calls[0], new RegExp(`\\?v=${WASM_ASSET_VERSION}$`));
} finally {
    globalThis.fetch = originalFetch;
}

const four = [];
globalThis.fetch = async () => {
    four.push(1);
    return { ok: false, status: 404 };
};
try {
    await fetchWasmBinary('https://example.test/missing.wasm', { attempts: 5, delayMs: 0 });
    assert.fail('404 should not retry to success');
} catch (err) {
    assert.equal(err.status, 404);
    assert.equal(four.length, 1);
} finally {
    globalThis.fetch = originalFetch;
}

const exhausted = [];
globalThis.fetch = async () => {
    exhausted.push(1);
    return { ok: false, status: 503 };
};
try {
    await fetchWasmBinary('https://example.test/playground/qualia_core_db_bg.wasm', {
        attempts: 4,
        delayMs: 0,
    });
    assert.fail('persistent 503 should throw');
} catch (err) {
    assert.equal(err.status, 503);
    assert.equal(exhausted.length, 4);
    assert.match(err.message, /WASM fetch failed: 503/);
} finally {
    globalThis.fetch = originalFetch;
}

const net = [];
globalThis.fetch = async () => {
    net.push(1);
    if (net.length < 2) throw new TypeError('Failed to fetch');
    return { ok: true, status: 200 };
};
try {
    const resp = await fetchWasmBinary('https://example.test/playground/qualia_core_db_bg.wasm', {
        attempts: 3,
        delayMs: 0,
    });
    assert.equal(resp.status, 200);
    assert.equal(net.length, 2);
} finally {
    globalThis.fetch = originalFetch;
}

const abort = [];
globalThis.fetch = async () => {
    abort.push(1);
    const err = new Error('aborted');
    err.name = 'AbortError';
    throw err;
};
try {
    await fetchWasmBinary('https://example.test/playground/qualia_core_db_bg.wasm', {
        attempts: 5,
        delayMs: 0,
    });
    assert.fail('AbortError should not be swallowed');
} catch (err) {
    assert.equal(err.name, 'AbortError');
    assert.equal(abort.length, 1);
} finally {
    globalThis.fetch = originalFetch;
}

const files = [
    '../benchmark.html',
    '../zero-heap-compliance.html',
    '../compute-engine.html',
    '../scientific-computing.html',
    '../science-playground.html',
    '../js/qualia-wasm-runtime.js',
    '../js/qualia-shell.js',
    '../js/logic-showcase-app.js',
    '../js/spatial-demo.js',
    '../playground/playground-app.js',
    '../playground/wordnet-demo.js',
];
for (const rel of files) {
    const text = fs.readFileSync(new URL(rel, import.meta.url), 'utf8');
    assert.match(text, /fetchWasmBinary/, `${rel} must use fetchWasmBinary`);
    assert.doesNotMatch(
        text,
        /qualia_core_db_bg\.wasm['"`].*cache:\s*['"]no-store['"]/,
        `${rel} must not fetch playground WASM with cache:no-store`,
    );
}

const bench = fs.readFileSync(new URL('../benchmark.html', import.meta.url), 'utf8');
assert.match(bench, /retryWasmEngine/);
assert.match(bench, /dismissWasmError/);
assert.match(bench, /Show CI suite anyway/);
assert.doesNotMatch(bench, /button onclick="location\.reload\(\)"/);
assert.match(bench, /loadCiSuite\(\)/);

console.log('WASM fetch retry + Pages loader wiring passed.');
