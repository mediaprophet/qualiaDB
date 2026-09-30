/**
 * Q42LEX v4 — namespaced paged lexicon parsing in the browser VFS.
 *
 * Mirrors the Rust writer (`q42_lex_ns.rs`) to build a v4 byte layout, then
 * proves `VFS._parseLexiconBytes` reconstructs every term — including DID
 * (non-HTTP) namespaces and bare literals — and that plain v2 pages still
 * parse. Companion to the Rust tests `v4_round_trips_multi_ontology_namespaces`
 * and `v4_pages_decode_from_isolated_byte_ranges`.
 */
import { fnv1a64 } from '../playground/hash.js';
import { VFS } from '../playground/vfs.js';

const LEX_MAGIC = 'Q42LEX\0\0';
const PAGED_PAGE_HEADER_SIZE = 16;
const INDEX_ENTRY_SIZE = 16;

function nsSplit(term) {
    for (let i = term.length - 1; i >= 0; i--) {
        const c = term[i];
        if (c === '/' || c === '#' || c === ':') {
            return [term.slice(0, i + 1), term.slice(i + 1)];
        }
    }
    return null;
}

/**
 * Build paged Q42LEX bytes. version = 4 builds namespaced pages (per-page
 * namespace table + 0x04 entries for groups that strictly save bytes);
 * version = 2 builds plain verbatim pages.
 */
function buildPagedLexicon(terms, version) {
    const map = new Map(terms.map((t) => [fnv1a64(t), t]));
    const entries = [...map.entries()].sort((a, b) =>
        a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0
    );

    const nsIds = new Map();
    const nsTable = [];
    if (version === 4) {
        const members = new Map();
        for (const [, text] of entries) {
            const split = nsSplit(text);
            if (split) members.set(split[0], (members.get(split[0]) || 0) + 1);
        }
        for (const [ns, m] of members) {
            const n = ns.length;
            if (n >= 3 && m * (n - 2) > n + 2) {
                nsIds.set(ns, nsTable.length);
                nsTable.push(ns);
            }
        }
    }

    const nsTableBytes = nsTable.reduce((sum, ns) => sum + 2 + ns.length, 0);
    const blobOffset = PAGED_PAGE_HEADER_SIZE + entries.length * INDEX_ENTRY_SIZE;
    const entryBytes = entries.reduce((sum, [, text]) => {
        const split = nsSplit(text);
        if (split && nsIds.has(split[0])) {
            return sum + 1 + 2 + 2 + split[1].length;
        }
        return sum + 1 + 2 + text.length;
    }, 0);
    const pageLength = blobOffset + nsTableBytes + entryBytes;

    const total = 32 + 8 + 32 + pageLength;
    const buf = new ArrayBuffer(total);
    const dv = new DataView(buf);
    const u8 = new Uint8Array(buf);
    const enc = new TextEncoder();

    for (let i = 0; i < 8; i++) u8[i] = LEX_MAGIC.charCodeAt(i);
    dv.setBigUint64(8, BigInt(entries.length), true);
    dv.setBigUint64(16, 32n, true); // strings_offset
    dv.setBigUint64(24, BigInt(version), true);
    dv.setBigUint64(32, 1n, true); // page_count

    const dir = 40;
    dv.setBigUint64(dir, entries[0][0], true); // first_hash
    dv.setBigUint64(dir + 8, 72n, true); // page_offset (after header + dir)
    dv.setBigUint64(dir + 16, BigInt(pageLength), true);
    dv.setUint32(dir + 24, entries.length, true);

    const page = 72;
    dv.setUint32(page, entries.length, true);
    dv.setUint32(page + 4, nsTable.length, true);
    dv.setBigUint64(page + 8, BigInt(blobOffset), true);

    let cursor = page + blobOffset;
    for (const ns of nsTable) {
        const bytes = enc.encode(ns);
        dv.setUint16(cursor, bytes.length, true);
        u8.set(bytes, cursor + 2);
        cursor += 2 + bytes.length;
    }

    entries.forEach(([hash, text], i) => {
        const relative = cursor - page - blobOffset;
        const split = nsSplit(text);
        if (split && nsIds.has(split[0])) {
            const local = enc.encode(split[1]);
            u8[cursor] = 0x04;
            dv.setUint16(cursor + 1, nsIds.get(split[0]), true);
            dv.setUint16(cursor + 3, local.length, true);
            u8.set(local, cursor + 5);
            cursor += 5 + local.length;
        } else {
            const bytes = enc.encode(text);
            u8[cursor] = 0x01;
            dv.setUint16(cursor + 1, bytes.length, true);
            u8.set(bytes, cursor + 3);
            cursor += 3 + bytes.length;
        }
        const index = page + PAGED_PAGE_HEADER_SIZE + i * INDEX_ENTRY_SIZE;
        dv.setBigUint64(index, hash, true);
        dv.setBigUint64(index + 8, BigInt(relative), true);
    });

    return new Uint8Array(buf);
}

const TERMS = [
    'https://schema.org/Thing',
    'https://schema.org/Person',
    'https://schema.org/Organization',
    'https://schema.org/CreativeWork',
    'did:q42:z6MkpTHR8VNsA',
    'did:q42:z6MkpTHR8VNsB',
    'did:q42:z6MkpTHR8VNsC',
    'bare-literal-no-separator',
];

function assert(condition, message) {
    if (!condition) {
        console.error(`FAIL: ${message}`);
        process.exit(1);
    }
}

// v4: every term — HTTP IRI, DID namespace, and bare literal — round-trips.
const vfsV4 = new VFS('http://example.invalid/vol.q42');
vfsV4._parseLexiconBytes(buildPagedLexicon(TERMS, 4));
assert(vfsV4.lexLoaded, 'v4 lexicon must load');
for (const term of TERMS) {
    const got = vfsV4.lookup(fnv1a64(term));
    assert(
        got === term,
        `v4 term must round-trip: expected ${term}, got ${got}`
    );
}
console.log(`lexicon-v4: ${vfsV4._lexMap.size}/${TERMS.length} terms reconstructed`);

// v2: plain verbatim pages still parse (back-compat for pre-v4 volumes).
const vfsV2 = new VFS('http://example.invalid/vol.q42');
vfsV2._parseLexiconBytes(buildPagedLexicon(TERMS, 2));
for (const term of TERMS) {
    const got = vfsV2.lookup(fnv1a64(term));
    assert(got === term, `v2 term must round-trip: expected ${term}, got ${got}`);
}
console.log(`lexicon-v2 back-compat: ${vfsV2._lexMap.size}/${TERMS.length} terms reconstructed`);

console.log('lexicon-v4-namespaced.test.mjs: all assertions passed');
