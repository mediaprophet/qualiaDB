import http from 'node:http';
import https from 'node:https';
import { readFileSync, realpathSync, statSync } from 'node:fs';
import { open, realpath } from 'node:fs/promises';
import { networkInterfaces } from 'node:os';
import { dirname, extname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { pipeline } from 'node:stream/promises';

const docsRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../docs');
const HARNESS_PAGE = '/playground/etalon-vibe-harness.html';
const BUILD_MANIFEST = '/playground/device-harness-build.json';
const HARNESS_FILES = [HARNESS_PAGE, BUILD_MANIFEST,
  '/js/device-harness/main.js', '/js/device-harness/worker.js', '/js/device-harness/report.js',
  '/js/device-harness/email.js'];
const MIME = new Map([
  ['.html', 'text/html; charset=utf-8'], ['.js', 'text/javascript; charset=utf-8'],
  ['.mjs', 'text/javascript; charset=utf-8'], ['.wasm', 'application/wasm'],
  ['.json', 'application/json; charset=utf-8'], ['.css', 'text/css; charset=utf-8'],
  ['.svg', 'image/svg+xml'], ['.png', 'image/png'], ['.jpg', 'image/jpeg'],
  ['.ico', 'image/x-icon'], ['.md', 'text/plain; charset=utf-8'],
]);

function inside(root, target) {
  const part = relative(root, target);
  return part === '' || (!isAbsolute(part) && part !== '..' && !part.startsWith(`..${sep}`));
}

export function harnessAllowPaths(root) {
  root = realpathSync.native(root);
  const manifestPath = realpathSync.native(resolve(root, `.${BUILD_MANIFEST}`));
  if (relative(resolve(root, `.${BUILD_MANIFEST}`), manifestPath) !== ''
      || statSync(manifestPath).size > 64 * 1024) {
    throw new Error('Harness manifest is outside docs or exceeds 64 KiB');
  }
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  if (manifest.schema_version !== 1 || !manifest.assets
      || Object.keys(manifest.assets).sort().join(',') !== 'portal_js,portal_wasm') {
    throw new Error('Harness manifest must identify exactly two portal assets');
  }
  const paths = new Set(HARNESS_FILES);
  const publicPackageRoot = realpathSync.native(resolve(root, 'pkg'));
  if (relative(resolve(root, 'pkg'), publicPackageRoot) !== '') throw new Error('Public package root is redirected');
  for (const [name, file] of [['portal_js', 'qualia.js'], ['portal_wasm', 'qualia_bg.wasm']]) {
    const url = manifest.assets[name]?.url;
    if (typeof url !== 'string' || url.length > 512 || !url.startsWith('../pkg/')) {
      throw new Error('Harness asset must be a local docs/pkg URL');
    }
    const segments = url.slice('../pkg/'.length).split('/');
    if (segments.pop() !== file || segments.some(segment => !/^[a-zA-Z0-9_-]+$/.test(segment))) {
      throw new Error('Harness asset path contains invalid segments or filename');
    }
    const path = `/pkg/${[...segments, file].join('/')}`;
    const expectedAsset = resolve(root, `.${path}`);
    const canonicalAsset = realpathSync.native(expectedAsset);
    if (!inside(publicPackageRoot, canonicalAsset) || relative(expectedAsset, canonicalAsset) !== ''
        || !statSync(canonicalAsset).isFile()) {
      throw new Error('Harness asset is outside public docs/pkg');
    }
    paths.add(path);
  }
  return paths;
}

export function parseOptions(args) {
  const options = { host: '0.0.0.0', port: 8080 };
  for (let index = 0; index < args.length; index += 2) {
    const name = args[index];
    if (!['--host', '--port', '--cert', '--key'].includes(name) || !args[index + 1]) {
      throw new Error('Usage: node scripts/serve-device-harness.mjs [--host ADDRESS] [--port PORT] [--cert PEM --key PEM]');
    }
    options[name.slice(2)] = args[index + 1];
  }
  options.port = Number(options.port);
  if (!Number.isInteger(options.port) || options.port < 1 || options.port > 65535) {
    throw new Error('Port must be an integer from 1 to 65535');
  }
  if (Boolean(options.cert) !== Boolean(options.key)) {
    throw new Error('HTTPS requires both --cert and --key');
  }
  return options;
}

export function createDocsServer({ root = docsRoot, cert, key, allowPaths, maxFileBytes = 128 * 1024 * 1024 } = {}) {
  if (Boolean(cert) !== Boolean(key)) throw new Error('HTTPS requires both cert and key');
  const canonicalRoot = realpathSync.native(root);
  if (!statSync(canonicalRoot).isDirectory()) throw new Error('Docs root is not a directory');
  const allowed = allowPaths ? new Set(allowPaths) : harnessAllowPaths(canonicalRoot);
  let active = 0;
  const handler = async (request, response) => {
    response.setHeader('X-Content-Type-Options', 'nosniff');
    response.setHeader('Cache-Control', 'no-store');
    const reject = (status, message) => {
      response.statusCode = status;
      response.setHeader('Content-Type', 'text/plain; charset=utf-8');
      response.setHeader('Connection', 'close');
      response.end(request.method === 'HEAD' ? undefined : message);
    };
    if (!['GET', 'HEAD'].includes(request.method)) {
      response.setHeader('Allow', 'GET, HEAD');
      return reject(405, 'Only GET and HEAD are supported');
    }
    if (request.headers['transfer-encoding'] || Number(request.headers['content-length'] || 0) !== 0) {
      return reject(413, 'Request bodies are not accepted');
    }
    if (!request.url || Buffer.byteLength(request.url) > 4096) return reject(414, 'Request path is too long');
    if (active >= 32) return reject(503, 'Too many active requests');
    active += 1;
    let file;
    try {
      let path = decodeURIComponent(request.url.split(/[?#]/, 1)[0]);
      const segments = path.split('/').filter(Boolean);
      if (!path.startsWith('/') || /[\\\0:%]/.test(path)
          || segments.some(segment => segment.startsWith('.'))) {
        return reject(400, 'Invalid document path');
      }
      if (path === '/') path = HARNESS_PAGE;
      if (!allowed.has(path)) return reject(403, 'Only device-harness files are served');
      let candidate = resolve(canonicalRoot, `.${path}`);
      if (!inside(canonicalRoot, candidate)) return reject(403, 'Path outside docs');
      const expectedPath = candidate;
      candidate = await realpath(candidate);
      if (relative(expectedPath, candidate) !== '') return reject(403, 'Redirected file paths are not served');
      if (!inside(canonicalRoot, candidate)) return reject(403, 'Path outside docs');
      if (statSync(candidate).isDirectory()) return reject(403, 'Directory requests are not supported');
      file = await open(candidate, 'r');
      const metadata = await file.stat();
      if (!metadata.isFile()) return reject(404, 'Document not found');
      if (metadata.size > maxFileBytes) return reject(413, 'Document exceeds the file-size budget');
      response.setHeader('Content-Type', MIME.get(extname(candidate).toLowerCase()) || 'application/octet-stream');
      response.setHeader('Content-Length', metadata.size);
      if (request.method === 'HEAD') response.end();
      else await pipeline(file.createReadStream({ autoClose: false }), response);
    } catch (error) {
      if (!response.headersSent) {
        const status = error instanceof URIError ? 400 : ['ENOENT', 'ENOTDIR'].includes(error.code) ? 404 : 403;
        reject(status, status === 404 ? 'Document not found' : 'Document request rejected');
      } else response.destroy();
    } finally {
      await file?.close();
      active -= 1;
    }
  };
  let server;
  if (cert) {
    for (const path of [cert, key]) {
      if (statSync(path).size > 1024 * 1024) throw new Error('TLS file exceeds the one MiB budget');
    }
    server = https.createServer({ cert: readFileSync(cert), key: readFileSync(key), maxHeaderSize: 8192 }, handler);
  } else server = http.createServer({ maxHeaderSize: 8192 }, handler);
  server.maxHeadersCount = 32;
  server.maxConnections = 64;
  server.headersTimeout = 5000;
  server.requestTimeout = 10000;
  server.keepAliveTimeout = 1000;
  server.setTimeout(15000, socket => socket.destroy());
  return server;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const options = parseOptions(process.argv.slice(2));
    const server = createDocsServer(options);
    server.on('error', error => { console.error(error.message); process.exitCode = 1; });
    server.listen(options.port, options.host, () => {
      const protocol = options.cert ? 'https' : 'http';
      const addresses = new Set();
      if (['0.0.0.0', '::'].includes(options.host)) {
        addresses.add('127.0.0.1');
        for (const entries of Object.values(networkInterfaces())) {
          for (const address of entries || []) {
            if (!address.internal && address.family === 'IPv4') addresses.add(address.address);
          }
        }
      } else addresses.add(options.host);
      console.log('Serving only the device-harness page, manifest, four modules and two manifest-selected portal assets. Stop with Ctrl+C.');
      for (const address of addresses) {
        const host = address.includes(':') ? `[${address}]` : address;
        console.log(`${protocol}://${host}:${options.port}/playground/etalon-vibe-harness.html`);
      }
      if (!options.cert) console.log('Phone LAN HTTP is not a secure context: use CPU-only checks; WebGPU needs trusted HTTPS.');
      else console.log('Each phone must trust this certificate and its URL hostname/IP for a WebGPU secure context.');
    });
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
