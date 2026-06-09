/**
 * Browser benchmark dataset loader — Schema.org NT / .q42 / .c.q42 → flat QualiaQuin bytes.
 * Mirrors benchmarks/qualia_wasm/bench.mjs and qualia-cli compress / q42_comparative_bench.
 */

const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const MASK64 = 0xffffffffffffffffn;
const OBJECT_HASH_MASK = 0x0fffffffffffffffn;

const BLOCK_SIZE = 40960;
const HEADER_SIZE = 160;
const QUINS_PER_BLOCK = 850;
const QUIN_SIZE = 48;

export function qHash(s) {
    let hash = FNV_OFFSET;
    for (const b of new TextEncoder().encode(s)) {
        hash ^= BigInt(b);
        hash = (hash * FNV_PRIME) & MASK64;
    }
    return hash;
}

export function hashToken(token) {
    if (token.startsWith('<') && token.endsWith('>')) {
        return qHash(token.slice(1, -1));
    }
    if (token.startsWith('"')) {
        const bytes = new TextEncoder().encode(token);
        let i = 1;
        while (i < bytes.length) {
            if (bytes[i] === 0x5c) { i += 2; continue; }
            if (bytes[i] === 0x22) break;
            i += 1;
        }
        return qHash(token.slice(1, i));
    }
    return qHash(token);
}

export function generateSyntheticNT(n) {
    const lines = [];
    for (let i = 0; i < n; i++) {
        const p = i % 5;
        const o = (i * 13) % n;
        lines.push(`<http://q.test/s/${i}> <http://q.test/p/${p}> <http://q.test/o/${o}> .`);
    }
    return lines.join('\n');
}

function encodeQuin(subject, predicate, object) {
    const buf = new Uint8Array(48);
    const dv = new DataView(buf.buffer);
    dv.setBigUint64(0, subject, true);
    dv.setBigUint64(8, predicate, true);
    dv.setBigUint64(16, object, true);
    return buf;
}

export function parseNTToFlatDb(text) {
    const quins = [];
    const index = new Map();
    for (const line of text.split('\n')) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;
        const parts = trimmed.replace(/\s+\.\s*$/, '').split(/\s+/);
        if (parts.length < 3) continue;
        const s = hashToken(parts[0]);
        const p = hashToken(parts[1]);
        const o = hashToken(parts[2]) & OBJECT_HASH_MASK;
        quins.push(encodeQuin(s, p, o));
        let bucket = index.get(s);
        if (!bucket) {
            bucket = [];
            index.set(s, bucket);
        }
        bucket.push(o);
    }
    const db = new Uint8Array(quinCountToBytes(quinCount(quins)));
    let off = 0;
    for (const q of quins) {
        db.set(q, off);
        off += 48;
    }
    return { db, index, quinCount: quins.length };
}

function quinCount(quins) {
    return quins.length;
}

function quinCountToBytes(count) {
    return count * 48;
}

export function parseSuperblockQ42(buffer) {
    const view = new DataView(buffer);
    const chunks = [];
    let offset = 0;
    while (offset + BLOCK_SIZE <= buffer.byteLength) {
        const active = Number(view.getBigUint64(offset + 16, true));
        offset += HEADER_SIZE;
        const count = Math.min(active, QUINS_PER_BLOCK);
        for (let i = 0; i < count; i++) {
            chunks.push(new Uint8Array(buffer, offset + i * QUIN_SIZE, QUIN_SIZE));
        }
        offset += QUINS_PER_BLOCK * QUIN_SIZE;
    }
    const db = new Uint8Array(chunks.length * QUIN_SIZE);
    let off = 0;
    for (const c of chunks) {
        db.set(c, off);
        off += QUIN_SIZE;
    }
    return { db, quinCount: chunks.length };
}

/** lz4_flex block: u32 LE uncompressed size + LZ4 block bytes */
function decompressLz4FlexBlock(payload) {
    const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
    const expectedLen = view.getUint32(0, true);
    const compressed = payload.subarray(4);
    const out = decompressLz4Block(compressed, expectedLen);
    return out;
}

function decompressLz4Block(src, maxOutputSize) {
    const dst = new Uint8Array(maxOutputSize);
    let s = 0;
    let d = 0;
    while (s < src.length) {
        const token = src[s++];
        let literalLen = token >> 4;
        if (literalLen === 15) {
            let len;
            do {
                len = src[s++];
                literalLen += len;
            } while (len === 255);
        }
        for (let i = 0; i < literalLen; i++) dst[d++] = src[s++];
        if (s >= src.length) break;
        const offset = src[s++] | (src[s++] << 8);
        let matchLen = (token & 0x0f) + 4;
        if ((token & 0x0f) === 15) {
            let len;
            do {
                len = src[s++];
                matchLen += len;
            } while (len === 255);
        }
        let m = d - offset;
        for (let i = 0; i < matchLen; i++) {
            dst[d++] = dst[m++];
        }
    }
    return dst.subarray(0, d);
}

export function parseCq42(buffer) {
    const view = new DataView(buffer);
    const chunks = [];
    let offset = 0;
    while (offset + 16 <= buffer.byteLength) {
        const compLen = view.getUint32(offset + 8, true);
        offset += 16;
        if (compLen === 0 || offset + compLen > buffer.byteLength) break;
        const payload = new Uint8Array(buffer, offset, compLen);
        offset += compLen;
        const decompressed = decompressLz4FlexBlock(payload);
        chunks.push(decompressed);
    }
    const total = chunks.reduce((n, c) => n + c.length, 0);
    const db = new Uint8Array(total);
    let off = 0;
    for (const c of chunks) {
        db.set(c, off);
        off += c.length;
    }
    if (db.length % QUIN_SIZE !== 0) {
        throw new Error(`decompressed .c.q42 length ${db.length} is not a multiple of 48`);
    }
    return { db, quinCount: db.length / QUIN_SIZE };
}

export function buildSubjectIndex(db) {
    const index = new Map();
    const view = new DataView(db.buffer, db.byteOffset, db.byteLength);
    for (let off = 0; off + QUIN_SIZE <= db.byteLength; off += QUIN_SIZE) {
        const s = view.getBigUint64(off, true);
        const o = view.getBigUint64(off + 16, true);
        let bucket = index.get(s);
        if (!bucket) {
            bucket = [];
            index.set(s, bucket);
        }
        bucket.push(o);
    }
    return index;
}

export async function fetchManifest(profileId) {
    const res = await fetch(`./benchmark-datasets/${profileId}.json`);
    if (!res.ok) throw new Error(`Dataset manifest not found: ${profileId}`);
    const manifest = await res.json();
    manifest._manifestUrl = res.url;
    return manifest;
}

export async function loadDataset(manifest, storageFormat) {
    const started = performance.now();
    if (manifest.generate_synthetic || storageFormat === 'synthetic') {
        const n = manifest.synthetic_n || 10000;
        const parsed = parseNTToFlatDb(generateSyntheticNT(n));
        return {
            ...parsed,
            format: 'synthetic-nt',
            loadMs: performance.now() - started,
            label: `Synthetic ${n.toLocaleString()} triples`,
        };
    }

    const pathKey = storageFormat === 'nt' ? 'nt' : storageFormat === 'q42' ? 'q42' : 'cq42';
    const url = manifest.paths?.[pathKey];
    if (!url) {
        if (manifest.generate_synthetic) {
            return loadDataset(manifest, 'synthetic');
        }
        throw new Error(`No path for storage format ${storageFormat} — run scripts/prepare_schemaorg_benchmark.ps1`);
    }

    const assetUrl = new URL(url, manifest._manifestUrl || window.location.href).toString();
    const res = await fetch(assetUrl);
    if (!res.ok) {
        throw new Error(
            `Failed to fetch ${url} (${res.status}). Run scripts/prepare_schemaorg_benchmark.ps1 first.`
        );
    }

    if (storageFormat === 'nt') {
        const text = await res.text();
        const parsed = parseNTToFlatDb(text);
        return {
            ...parsed,
            format: 'ntriples',
            loadMs: performance.now() - started,
            label: `N-Triples (${parsed.quinCount.toLocaleString()} quins)`,
        };
    }

    const buffer = await res.arrayBuffer();
    const parsed = storageFormat === 'q42'
        ? parseSuperblockQ42(buffer)
        : parseCq42(buffer);
    const index = buildSubjectIndex(parsed.db);
    return {
        db: parsed.db,
        index,
        quinCount: parsed.quinCount,
        format: storageFormat === 'q42' ? 'q42-superblock' : 'cq42-lz4',
        loadMs: performance.now() - started,
        label: storageFormat === 'q42'
            ? `.q42 SuperBlocks (${parsed.quinCount.toLocaleString()} quins)`
            : `.c.q42 LZ4 (${parsed.quinCount.toLocaleString()} quins)`,
    };
}

export function queriesForManifest(manifest, suite) {
    const q = manifest.queries || {};
    const pointSubject = q.point_subject || 'http://q.test/s/0';
    const filterPredicate = q.filter_predicate || 'http://q.test/p/0';
    const twohopStart = q.twohop_start || pointSubject;
    let twohopSecond = q.twohop_second || null;
    if (!twohopSecond && manifest.generate_synthetic) {
        const m = String(twohopStart).match(/\/s\/(\d+)$/);
        const i = m ? Number(m[1]) : 0;
        const n = manifest.synthetic_n || manifest.n_triples || 10000;
        twohopSecond = `http://q.test/o/${(i * 13) % n}`;
    }
    return {
        point: `<${pointSubject}> ?p ?o .`,
        twohop: null,
        twohop1: `<${twohopStart}> ?p ?o .`,
        twohop2: twohopSecond ? `<${twohopSecond}> ?p ?o .` : null,
        filter: `?s <${filterPredicate}> ?o .`,
        ingest: '?s ?p ?o .',
        twohopStart,
        twohopSecond,
        pointSubject,
        filterPredicate,
    };
}
