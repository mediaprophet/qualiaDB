/**
 * Regression: WordNet object hashes are delimiter-stripped at ingest.
 * Ontology streaming queries must try hashTokenVariants (quoted + stripped),
 * matching playground/wordnet-demo.js — otherwise lemma lookup returns 0 hits.
 *
 * Requires docs/data/wordnet/princeton.q42 (or docs/playground/wordnet.q42).
 *   node docs/tests/wordnet-hash-variants.test.mjs
 */
import { pathToFileURL } from 'node:url';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const docsRoot = path.resolve(__dirname, '..');
const repoRoot = path.resolve(docsRoot, '..');
const volumeRel = existsSync(path.join(docsRoot, 'data', 'wordnet', 'princeton.q42'))
    ? 'data/wordnet/princeton.q42'
    : existsSync(path.join(docsRoot, 'playground', 'wordnet.q42'))
        ? 'playground/wordnet.q42'
        : null;

if (!volumeRel) {
    console.error('SKIP: no princeton.q42 / wordnet.q42 — run scripts/fetch_wordnet_release.ps1');
    process.exit(0);
}

const MIME = {
    '.js': 'application/javascript',
    '.mjs': 'application/javascript',
    '.json': 'application/json',
    '.wasm': 'application/wasm',
    '.q42': 'application/octet-stream',
};

async function serveRange(req, res, filePath, body) {
    const range = req.headers.range;
    const type = MIME[path.extname(filePath).toLowerCase()] ?? 'application/octet-stream';
    if (!range) {
        res.writeHead(200, {
            'Content-Type': type,
            'Content-Length': body.length,
            'Accept-Ranges': 'bytes',
            'Access-Control-Allow-Origin': '*',
        });
        res.end(body);
        return;
    }
    const m = /^bytes=(\d+)-(\d*)$/.exec(range);
    if (!m) {
        res.writeHead(416);
        res.end();
        return;
    }
    const start = Number(m[1]);
    const end = m[2] ? Number(m[2]) : body.length - 1;
    const slice = body.subarray(start, end + 1);
    res.writeHead(206, {
        'Content-Type': type,
        'Content-Length': slice.length,
        'Content-Range': `bytes ${start}-${start + slice.length - 1}/${body.length}`,
        'Accept-Ranges': 'bytes',
        'Access-Control-Allow-Origin': '*',
    });
    res.end(slice);
}

const server = createServer(async (req, res) => {
    try {
        const urlPath = decodeURIComponent((req.url ?? '/').split('?')[0]);
        const filePath = path.normalize(path.join(docsRoot, urlPath.replace(/^\//, '')));
        if (!filePath.startsWith(docsRoot)) {
            res.writeHead(403);
            res.end('forbidden');
            return;
        }
        await serveRange(req, res, filePath, await readFile(filePath));
    } catch (e) {
        res.writeHead(404, { 'Content-Type': 'text/plain' });
        res.end(String(e.message ?? e));
    }
});

await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const { port } = server.address();
const base = `http://127.0.0.1:${port}`;

try {
    const { hashToken, hashTokenVariants } = await import(
        pathToFileURL(path.join(docsRoot, 'playground', 'hash.js')).href
    );
    const variants = hashTokenVariants('"dog"');
    if (variants.length < 2) {
        throw new Error('expected quoted+stripped dog hash variants');
    }
    const quoted = hashToken('"dog"');
    const stripped = hashToken('dog');
    if (quoted === stripped) {
        throw new Error('quoted and stripped dog hashes unexpectedly equal');
    }

    // Import ontology-engine from the HTTP origin so its relative playground/
    // and wasm URLs resolve against the same static host.
    const { OntologyEngine } = await import(`${base}/js/ontology-engine.js`);
    const eng = new OntologyEngine();
    await eng.init('wordnet');

    const qLegacyWouldMiss = eng.vfs.lookupBlocks(quoted);
    const qStripped = eng.vfs.lookupBlocks(stripped);
    if (!qStripped?.length) {
        throw new Error(`BIDX miss for stripped dog hash; quoted blocks=${JSON.stringify(qLegacyWouldMiss)}`);
    }

    const q = await eng.query('?s ?p "dog"', 20);
    if (q.matches.length < 1) {
        throw new Error(
            `OntologyEngine.query returned 0 dog matches ` +
            `(cycles=${q.vm_cycles}, quotedBlocks=${JSON.stringify(qLegacyWouldMiss)}, ` +
            `strippedBlocks=${JSON.stringify(qStripped)})`,
        );
    }

    const lookup = await eng.lookupEntity('dog');
    if (!lookup.found || !lookup.entities?.length) {
        throw new Error(`lookupEntity('dog') failed: ${JSON.stringify({
            found: lookup.found,
            entities: lookup.entities?.length ?? 0,
            matches: q.matches.length,
        })}`);
    }

    // Schema.org-style quoted hashes must still work when that is what is stored.
    // Smoke: empty-pattern guard still returns zero without throwing.
    const empty = await eng.query('?s ?p', 5);
    if (empty.matches.length !== 0) {
        throw new Error('malformed pattern unexpectedly matched');
    }

    console.log(
        `PASS: dog matches=${q.matches.length} entities=${lookup.entities.length} ` +
        `blocks=${eng.vfs.blockCount} volume=${volumeRel}`,
    );
    console.log(
        `  quoted-hash BIDX=${JSON.stringify(qLegacyWouldMiss)} ` +
        `stripped-hash BIDX=${JSON.stringify(qStripped)} (legacy single-hash would miss)`,
    );
} finally {
    server.close();
}
