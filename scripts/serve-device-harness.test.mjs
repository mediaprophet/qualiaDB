import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import http from 'node:http';
import { once } from 'node:events';
import { mkdtemp, mkdir, writeFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createDocsServer, harnessAllowPaths, parseOptions } from './serve-device-harness.mjs';

async function fixture(t, options = {}) {
  const owned = await mkdtemp(join(tmpdir(), 'qualia-device-server-'));
  await writeFile(join(owned, '.qualia-owned-run'), 'device-server-test');
  const root = join(owned, 'docs');
  await mkdir(root);
  await writeFile(join(owned, 'private.txt'), 'PRIVATE');
  await mkdir(join(root, 'playground'));
  await writeFile(join(root, 'playground', 'etalon-vibe-harness.html'), 'HOME');
  await writeFile(join(root, 'unlisted.html'), 'UNRELATED PRIVATE DOCS');
  await writeFile(join(root, 'test.wasm'), Buffer.from([0, 97, 115, 109]));
  await writeFile(join(root, 'test.js'), 'export const value = 1;');
  await mkdir(join(root, 'empty'));
  const server = createDocsServer({ root, allowPaths: ['/playground/etalon-vibe-harness.html', '/test.wasm', '/test.js', '/empty/', '/missing', '/outside/private.txt'], ...options });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  t.after(async () => {
    await new Promise((done, reject) => server.close(error => error ? reject(error) : done()));
    await rm(owned, { recursive: true, force: true });
  });
  return { port: server.address().port, root };
}

function request(port, path, method = 'GET', headers = {}) {
  return new Promise((done, reject) => {
    const req = http.request({ hostname: '127.0.0.1', port, path, method, headers }, response => {
      const chunks = [];
      response.on('data', chunk => chunks.push(chunk));
      response.on('end', () => done({ status: response.statusCode, headers: response.headers, body: Buffer.concat(chunks).toString() }));
    });
    req.on('error', reject);
    req.end();
  });
}

test('serves allowed harness files with correct WASM/JavaScript MIME and HEAD metadata', async t => {
  const { port } = await fixture(t);
  assert.equal((await request(port, '/')).body, 'HOME');
  assert.equal((await request(port, '/test.wasm')).headers['content-type'], 'application/wasm');
  assert.equal((await request(port, '/test.js')).headers['content-type'], 'text/javascript; charset=utf-8');
  const head = await request(port, '/test.wasm', 'HEAD');
  assert.equal(head.status, 200);
  assert.equal(head.headers['content-length'], '4');
  assert.equal(head.body, '');
});

test('rejects raw, encoded and double encoded traversal and Windows alternate paths', async t => {
  const { port } = await fixture(t);
  for (const path of ['/../private.txt', '/%2e%2e/private.txt', '/%252e%252e/private.txt',
    '/%2e%2e%2fprivate.txt', '/..%5cprivate.txt', '/C:%5cprivate.txt', '/index.html:stream', '/%00', '/%ZZ']) {
    const result = await request(port, path);
    assert.equal(result.status, 400, path);
    assert.ok(!result.body.includes('PRIVATE'));
  }
});

test('does not list directories or permit writes/request bodies', async t => {
  const { port } = await fixture(t);
  assert.equal((await request(port, '/empty/')).status, 403);
  assert.equal((await request(port, '/missing')).status, 404);
  for (const method of ['POST', 'PUT', 'DELETE', 'OPTIONS']) {
    const result = await request(port, '/', method);
    assert.equal(result.status, 405);
    assert.equal(result.headers.allow, 'GET, HEAD');
  }
  assert.equal((await request(port, '/', 'GET', { 'Content-Length': '1' })).status, 413);
});

test('rejects existing unlisted documents and root index', async t => {
  const { port, root } = await fixture(t);
  await writeFile(join(root, 'index.html'), 'OTHER INDEX');
  for (const path of ['/unlisted.html', '/index.html', '/playground/', '/device-harness-local-hosting.md']) {
    const response = await request(port, path);
    assert.equal(response.status, 403, path);
    assert.ok(!response.body.includes('UNRELATED PRIVATE DOCS'));
  }
  assert.equal((await request(port, '/')).body, 'HOME');
});

test('default allowlist contains exactly six harness files and two bounded manifest assets', async t => {
  const { root } = await fixture(t);
  await mkdir(join(root, 'js', 'device-harness'), { recursive: true });
  await writeFile(join(root, 'js', 'device-harness', 'email.js'), 'export const email = true;');
  await mkdir(join(root, 'pkg', 'device-harness'), { recursive: true });
  await writeFile(join(root, 'pkg', 'device-harness', 'qualia.js'), 'JS');
  await writeFile(join(root, 'pkg', 'device-harness', 'qualia_bg.wasm'), 'WASM');
  const manifestPath = join(root, 'playground', 'device-harness-build.json');
  const manifest = { schema_version: 1, assets: {
    portal_js: { url: '../pkg/device-harness/qualia.js' },
    portal_wasm: { url: '../pkg/device-harness/qualia_bg.wasm' },
  } };
  await writeFile(manifestPath, JSON.stringify(manifest));
  assert.deepEqual([...harnessAllowPaths(root)].sort(), [
    '/playground/etalon-vibe-harness.html', '/playground/device-harness-build.json',
    '/js/device-harness/main.js', '/js/device-harness/worker.js', '/js/device-harness/report.js',
    '/js/device-harness/email.js',
    '/pkg/device-harness/qualia.js', '/pkg/device-harness/qualia_bg.wasm',
  ].sort());
  const defaults = createDocsServer({ root });
  defaults.listen(0, '127.0.0.1');
  await once(defaults, 'listening');
  try {
    const port = defaults.address().port;
    assert.equal((await request(port, '/')).body, 'HOME');
    assert.equal((await request(port, '/unlisted.html')).status, 403);
    assert.equal((await request(port, '/pkg/device-harness/qualia_bg.wasm')).status, 200);
    assert.equal((await request(port, '/js/device-harness/email.js')).status, 200);
    assert.equal((await request(port, '/js/device-harness/unlisted.js')).status, 403);
    assert.equal((await request(port, '/pkg/device-harness/unlisted.wasm')).status, 403);
  } finally {
    await new Promise((done, reject) => defaults.close(error => error ? reject(error) : done()));
  }
  for (const url of ['../pkg/../unlisted.html', '../pkg/%2e%2e/qualia.js',
    '../pkg/a/../../qualia.js', '../pkg/device-harness/other.js', '/unlisted.html',
    'https://example.com/qualia.js', '../pkg/device-harness/qualia.js?secret']) {
    manifest.assets.portal_js.url = url;
    await writeFile(manifestPath, JSON.stringify(manifest));
    assert.throws(() => harnessAllowPaths(root), undefined, url);
  }
  await writeFile(manifestPath, ' '.repeat(64 * 1024 + 1));
  assert.throws(() => harnessAllowPaths(root), /64 KiB/);
});

test('enforces file and request URL budgets', async t => {
  const { port } = await fixture(t, { maxFileBytes: 4 });
  assert.equal((await request(port, '/test.js')).status, 413);
  assert.equal((await request(port, `/${'a'.repeat(4096)}`)).status, 414);
});

test('rejects a directory symlink escaping the docs root', async t => {
  const { port, root } = await fixture(t);
  await symlink(join(root, '..'), join(root, 'outside'), process.platform === 'win32' ? 'junction' : 'dir');
  const result = await request(port, '/outside/private.txt');
  assert.equal(result.status, 403);
  assert.ok(!result.body.includes('PRIVATE'));
});

test('an allowed path cannot redirect to an unrelated document inside docs', async t => {
  const { port, root } = await fixture(t);
  await mkdir(join(root, 'unrelated'));
  await writeFile(join(root, 'unrelated', 'private.txt'), 'UNRELATED PRIVATE DOCS');
  await symlink(join(root, 'unrelated'), join(root, 'outside'), process.platform === 'win32' ? 'junction' : 'dir');
  assert.equal((await request(port, '/outside/private.txt')).status, 403);
});

test('HTTPS rejects unusable explicit certificate/key material', async t => {
  const { root } = await fixture(t);
  const cert = join(root, '..', 'invalid-cert.pem');
  const key = join(root, '..', 'invalid-key.pem');
  await writeFile(cert, 'invalid certificate');
  await writeFile(key, 'invalid key');
  assert.throws(() => createDocsServer({ root, cert, key, allowPaths: [] }));
});

test('HTTPS is explicit and requires both local PEM files', () => {
  assert.deepEqual(parseOptions([]), { host: '0.0.0.0', port: 8080 });
  assert.deepEqual(parseOptions(['--cert', 'cert.pem', '--key', 'key.pem', '--port', '9443']),
    { host: '0.0.0.0', port: 9443, cert: 'cert.pem', key: 'key.pem' });
  assert.throws(() => parseOptions(['--cert', 'cert.pem']), /both/);
  assert.throws(() => createDocsServer({ key: 'key.pem' }), /both/);
  assert.throws(() => parseOptions(['--port', '0']), /Port/);
  assert.throws(() => parseOptions(['--root', '/']), /Usage/);
});
