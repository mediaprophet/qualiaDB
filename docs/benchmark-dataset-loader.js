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
const Q42_MAGIC = [0x51, 0x34, 0x32, 0x00];
const Q42_VOLUME_HEADER_SIZE = 256;
const Q42_LEX_MAGIC = 'Q42LEX\0\0';
const INDEX_ENTRY_SIZE = 16;

function readBigUint64Safe(view, offset) {
    if (offset + 8 > view.byteLength) return 0n;
    return view.getBigUint64(offset, true);
}

function tokenLabel(token) {
    if (token.startsWith('<') && token.endsWith('>')) return token.slice(1, -1);
    if (token.startsWith('"')) {
        const bytes = new TextEncoder().encode(token);
        let i = 1;
        while (i < bytes.length) {
            if (bytes[i] === 0x5c) { i += 2; continue; }
            if (bytes[i] === 0x22) break;
            i += 1;
        }
        return token.slice(1, i);
    }
    return token;
}

function formatHash(value) {
    return `0x${value.toString(16).padStart(16, '0')}`;
}

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
        const o = (i * 13 + 3) % n;
        lines.push(`<http://q.test/s/${i}> <http://q.test/p/${p}> <http://q.test/s/${o}> .`);
    }
    return lines.join('\n');
}

function encodeQuin(subject, predicate, object) {
    const buf = new Uint8Array(48);
    const dv = new DataView(buf.buffer);
    dv.setBigUint64(0, subject, true);
    dv.setBigUint64(8, predicate, true);
    dv.setBigUint64(16, object, true);
    dv.setBigUint64(24, 0n, true);
    dv.setBigUint64(32, 0n, true);
    dv.setBigUint64(40, subject ^ predicate ^ object, true);
    return buf;
}

export function parseNTToFlatDb(text) {
    const quins = [];
    const index = new Map();
    const triples = [];
    const labelMap = new Map();
    for (const line of text.split('\n')) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;
        const parts = trimmed.replace(/\s+\.\s*$/, '').split(/\s+/);
        if (parts.length < 3) continue;
        const s = hashToken(parts[0]);
        const p = hashToken(parts[1]);
        const o = hashToken(parts[2]) & OBJECT_HASH_MASK;
        const subject = tokenLabel(parts[0]);
        const predicate = tokenLabel(parts[1]);
        const object = tokenLabel(parts[2]);
        quins.push(encodeQuin(s, p, o));
        triples.push({
            subject,
            predicate,
            object,
            subjectHash: s,
            predicateHash: p,
            objectHash: o,
        });
        if (!labelMap.has(s)) labelMap.set(s, subject);
        if (!labelMap.has(p)) labelMap.set(p, predicate);
        if (!labelMap.has(o)) labelMap.set(o, object);
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
    return { db, index, quinCount: quins.length, triples, labelMap };
}

function quinCount(quins) {
    return quins.length;
}

function quinCountToBytes(count) {
    return count * 48;
}

function flattenSuperblockBytes(blocks) {
    const totalQuins = blocks.reduce((sum, block) => sum + block.count, 0);
    const db = new Uint8Array(totalQuins * QUIN_SIZE);
    let outOffset = 0;
    for (const block of blocks) {
        for (let i = 0; i < block.count; i++) {
            const start = i * QUIN_SIZE;
            db.set(block.ledger.subarray(start, start + QUIN_SIZE), outOffset);
            outOffset += QUIN_SIZE;
        }
    }
    return { db, quinCount: totalQuins };
}

function parseLexiconBytes(bytes) {
    if (!bytes || bytes.byteLength < 32) return null;
    const magic = String.fromCharCode(...bytes.subarray(0, 8));
    if (magic !== Q42_LEX_MAGIC) return null;
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const entryCount = Number(readBigUint64Safe(view, 8));
    const stringsOffset = Number(readBigUint64Safe(view, 16));
    if (stringsOffset > bytes.byteLength) return null;
    const labels = new Map();
    for (let i = 0; i < entryCount; i++) {
        const off = 32 + i * INDEX_ENTRY_SIZE;
        if (off + INDEX_ENTRY_SIZE > bytes.byteLength) break;
        const hash = view.getBigUint64(off, true);
        const rel = Number(view.getBigUint64(off + 8, true));
        const start = stringsOffset + rel;
        if (start + 2 > bytes.byteLength) continue;
        const tag = bytes[start];
        const tagged = tag === 0x01 && start + 3 <= bytes.byteLength;
        const lenLe = tagged
            ? (bytes[start + 1] | (bytes[start + 2] << 8))
            : (bytes[start] | (bytes[start + 1] << 8));
        const lenBe = tagged
            ? ((bytes[start + 1] << 8) | bytes[start + 2])
            : ((bytes[start] << 8) | bytes[start + 1]);
        const textStart = tagged ? start + 3 : start + 2;
        const useBigEndian =
            ((lenLe === 0 || lenLe > 2048 || textStart + lenLe > bytes.byteLength) && lenBe > 0 && lenBe <= 2048 && textStart + lenBe <= bytes.byteLength);
        const len = useBigEndian ? lenBe : lenLe;
        const textEnd = textStart + len;
        if (textEnd > bytes.byteLength) continue;
        const text = new TextDecoder().decode(bytes.subarray(textStart, textEnd));
        labels.set(hash, text);
        labels.set(hash & OBJECT_HASH_MASK, text);
    }
    return labels;
}

async function fetchOptionalLexicon(manifest, lexUrl) {
    if (!lexUrl) return null;
    try {
        const assetUrl = new URL(lexUrl, manifest._manifestUrl || window.location.href).toString();
        const res = await fetch(assetUrl);
        if (!res.ok) return null;
        const bytes = new Uint8Array(await res.arrayBuffer());
        return parseLexiconBytes(bytes);
    } catch {
        return null;
    }
}

function isUnifiedQ42Volume(buffer) {
    const bytes = new Uint8Array(buffer, 0, Math.min(4, buffer.byteLength));
    return bytes.length === 4 && Q42_MAGIC.every((b, i) => bytes[i] === b);
}

function parseUnifiedQ42Volume(buffer) {
    const bytes = new Uint8Array(buffer);
    const view = new DataView(buffer);
    if (bytes.byteLength < Q42_VOLUME_HEADER_SIZE) {
        throw new Error('Q42 volume too small for header');
    }

    const blockDirOffset = Number(readBigUint64Safe(view, 40));
    const dataOffset = Number(readBigUint64Safe(view, 56));
    const blockCount = Number(readBigUint64Safe(view, 72));
    const lexOffset = Number(readBigUint64Safe(view, 8));
    const lexLength = Number(readBigUint64Safe(view, 16));
    const blocks = [];

    for (let i = 0; i < blockCount; i++) {
        const dirOffset = blockDirOffset + i * 16;
        if (dirOffset + 16 > bytes.byteLength) break;
        const relOffset = Number(readBigUint64Safe(view, dirOffset));
        const compLen = view.getUint32(dirOffset + 8, true);
        const payloadStart = dataOffset + relOffset;
        const payloadEnd = payloadStart + compLen;
        if (compLen === 0 || payloadEnd > bytes.byteLength) continue;
        const payload = bytes.subarray(payloadStart, payloadEnd);
        const superblock = decompressLz4FlexBlock(payload);
        if (superblock.byteLength < BLOCK_SIZE) continue;
        const blockView = new DataView(superblock.buffer, superblock.byteOffset, superblock.byteLength);
        const active = Number(blockView.getBigUint64(16, true));
        const count = Math.min(active, QUINS_PER_BLOCK);
        blocks.push({
            count,
            ledger: superblock.subarray(HEADER_SIZE, HEADER_SIZE + QUINS_PER_BLOCK * QUIN_SIZE),
        });
    }

    const lexBytes = (lexOffset > 0 && lexLength > 0 && lexOffset + lexLength <= bytes.byteLength)
        ? bytes.subarray(lexOffset, lexOffset + lexLength)
        : null;
    const labelMap = parseLexiconBytes(lexBytes);
    return {
        ...flattenSuperblockBytes(blocks),
        labelMap,
    };
}

export function parseSuperblockQ42(buffer) {
    const view = new DataView(buffer);
    const blocks = [];
    let offset = 0;
    while (offset + BLOCK_SIZE <= buffer.byteLength) {
        const active = Number(view.getBigUint64(offset + 16, true));
        offset += HEADER_SIZE;
        const count = Math.min(active, QUINS_PER_BLOCK);
        blocks.push({
            count,
            ledger: new Uint8Array(buffer, offset, QUINS_PER_BLOCK * QUIN_SIZE),
        });
        offset += QUINS_PER_BLOCK * QUIN_SIZE;
    }
    return {
        ...flattenSuperblockBytes(blocks),
        labelMap: null,
    };
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
        ? (isUnifiedQ42Volume(buffer) ? parseUnifiedQ42Volume(buffer) : parseSuperblockQ42(buffer))
        : parseCq42(buffer);
    const sidecarLexPath = manifest.paths?.q42_lex || (storageFormat === 'q42' ? `${url}.lex` : null);
    const labelMap = parsed.labelMap?.size ? parsed.labelMap : await fetchOptionalLexicon(manifest, sidecarLexPath);
    const index = buildSubjectIndex(parsed.db);
    return {
        db: parsed.db,
        index,
        quinCount: parsed.quinCount,
        triples: null,
        labelMap: labelMap || null,
        format: storageFormat === 'q42' ? 'q42-superblock' : 'cq42-lz4',
        loadMs: performance.now() - started,
        label: storageFormat === 'q42'
            ? `.q42 SuperBlocks (${parsed.quinCount.toLocaleString()} quins)`
            : `.c.q42 LZ4 (${parsed.quinCount.toLocaleString()} quins)`,
    };
}

export function decodeFlatDb(db, labelMap = null, maxQuins = Infinity) {
    const triples = [];
    const view = new DataView(db.buffer, db.byteOffset, db.byteLength);
    const total = Math.min(Math.floor(db.byteLength / QUIN_SIZE), maxQuins);
    for (let i = 0; i < total; i++) {
        const off = i * QUIN_SIZE;
        const subjectHash = view.getBigUint64(off, true);
        const predicateHash = view.getBigUint64(off + 8, true);
        const objectHash = view.getBigUint64(off + 16, true);
        const objectKey = objectHash & OBJECT_HASH_MASK;
        triples.push({
            subjectHash,
            predicateHash,
            objectHash,
            subject: labelMap?.get(subjectHash) ?? formatHash(subjectHash),
            predicate: labelMap?.get(predicateHash) ?? formatHash(predicateHash),
            object: labelMap?.get(objectHash) ?? labelMap?.get(objectKey) ?? formatHash(objectHash),
        });
    }
    return triples;
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
        twohopSecond = `http://q.test/s/${(i * 13 + 3) % n}`;
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
