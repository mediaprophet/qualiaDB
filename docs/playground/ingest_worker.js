/**
 * Browser-side N-Triples Ingest Worker
 *
 * Accepts a File or Blob of N-Triples data, parses it line-by-line,
 * packs 48-byte QualiaQuin records into 40,960-byte SuperBlocks, and
 * writes them to the Origin Private File System (OPFS) vault.
 *
 * A .q42.lex reverse-lexicon is also produced and stored as "lex.json"
 * in OPFS (JSON Map for simplicity; the CLI produces the efficient binary form).
 *
 * Message protocol (postMessage):
 *   → { type: 'ingest', file: File }
 *   ← { type: 'progress', triples: number, blocks: number }
 *   ← { type: 'done', triples: number, blocks: number, lexEntries: number }
 *   ← { type: 'error', message: string }
 */

// Inline FNV-1a 64-bit — must match hash.js exactly (no import in workers
// that may be loaded as classic scripts; duplicate is intentional).
const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME  = 0x100000001b3n;
const MSB        = 1n << 63n;
const _enc       = new TextEncoder();

function fnv1a64(str) {
    const bytes = _enc.encode(str);
    let h = FNV_OFFSET;
    for (const b of bytes) {
        h = BigInt.asUintN(64, (h ^ BigInt(b)) * FNV_PRIME);
    }
    return h;
}

function hashToken(token) {
    let inner = token;
    if (token.startsWith('<') && token.endsWith('>')) {
        inner = token.slice(1, -1);
    } else if (token.startsWith('"')) {
        const rest = token.slice(1);
        let i = 0;
        while (i < rest.length) {
            if (rest[i] === '\\') { i += 2; continue; }
            if (rest[i] === '"')  { inner = rest.slice(0, i); break; }
            i++;
        }
    }
    // did:q42 routing
    if (inner.startsWith('did:q42:')) {
        const payload = inner.slice('did:q42:'.length);
        if (payload) return fnv1a64(payload) | MSB;
    }
    return fnv1a64(inner);
}

// ---------------------------------------------------------------------------
// Layout constants (must match Rust exactly)
// ---------------------------------------------------------------------------

const QUIN_SIZE      = 48;
const QUINS_PER_BLOCK = 850;
const BLOCK_SIZE     = 40_960; // QualiaSuperBlock
const HEADER_SIZE    = 160;    // 8+8+8+4+4+128

// ---------------------------------------------------------------------------
// WASM bootstrap
//
// Dynamic import() works in classic workers on Chrome 80+, Firefox 114+,
// Safari 15.4+. The worker stays a classic script so the calling HTML needs
// no change. If the import fails (older browser, file:// origin without COOP
// headers, or WASM not yet rebuilt), we fall back to the pure-JS path below.
// ---------------------------------------------------------------------------

let wasmPackQuins = null; // set to pack_quins_into_superblock once WASM is ready

async function loadWasm() {
    try {
        const mod = await import('./qualia_core_db.js');
        await mod.default(); // initialise the WASM binary
        if (typeof mod.pack_quins_into_superblock === 'function') {
            wasmPackQuins = mod.pack_quins_into_superblock;
        }
    } catch {
        // WASM unavailable; JS fallback is used instead.
    }
}

// ---------------------------------------------------------------------------
// JS fallback: pack a QualiaQuin into 48 bytes (all fields little-endian u64)
//
// ECC parity = subject XOR predicate XOR object XOR context XOR metadata.
// This matches NQuin::calculate_parity() in Rust (lib.rs:273).
// ---------------------------------------------------------------------------

function setU64LE(view, byteOffset, value) {
    const lo = Number(value & 0xffffffffn);
    const hi = Number((value >> 32n) & 0xffffffffn);
    view.setUint32(byteOffset,     lo, true);
    view.setUint32(byteOffset + 4, hi, true);
}

function packQuin(view, offset, subject, predicate, object_) {
    const context  = 0n;
    const metadata = 0n;
    const parity   = subject ^ predicate ^ object_ ^ context ^ metadata;
    setU64LE(view, offset,      subject);
    setU64LE(view, offset +  8, predicate);
    setU64LE(view, offset + 16, object_);
    setU64LE(view, offset + 24, context);
    setU64LE(view, offset + 32, metadata);
    setU64LE(view, offset + 40, parity);
}

// ---------------------------------------------------------------------------
// Build one 40,960-byte SuperBlock buffer (pure-JS fallback path)
// ---------------------------------------------------------------------------

function buildSuperBlock(seqId, quins) {
    const buf  = new ArrayBuffer(BLOCK_SIZE);
    const view = new DataView(buf);
    const u8   = new Uint8Array(buf);

    // Header
    let pos = 0;
    setU64LE(view, pos, BigInt(seqId)); pos += 8; // block_sequence_id
    setU64LE(view, pos, 0n);            pos += 8; // storage_owner_did
    setU64LE(view, pos, BigInt(quins.length)); pos += 8; // active_quin_count
    view.setUint32(pos, 0, true);       pos += 4; // validation_checksum
    view.setUint32(pos, 0, true);       pos += 4; // hardware_profile_flags
    // layout_padding: 128 zero bytes (already zeroed)
    pos += 128;

    // Quin ledger (parity computed by packQuin)
    for (let i = 0; i < QUINS_PER_BLOCK; i++) {
        if (i < quins.length) {
            const { s, p, o } = quins[i];
            packQuin(view, pos, s, p, o);
        }
        // Zero quins (padding) are already zeroed by ArrayBuffer
        pos += QUIN_SIZE;
    }

    return u8;
}

// ---------------------------------------------------------------------------
// Worker message handler
// ---------------------------------------------------------------------------

self.onmessage = async (evt) => {
    const { type, file } = evt.data;
    if (type !== 'ingest') return;

    try {
        await doIngest(file);
    } catch (err) {
        self.postMessage({ type: 'error', message: String(err) });
    }
};

async function doIngest(file) {
    // Load WASM first so pack_quins_into_superblock is available for writeBlock.
    await loadWasm();

    // Open OPFS vault
    const opfsRoot = await navigator.storage.getDirectory();

    const text = await file.text();
    const lines = text.split('\n');

    // Collect reverse-lexicon: hash (as string) → canonical string
    const lexMap = new Map(); // Map<string, string> (BigInt keys serialised for JSON)

    let pending  = [];   // current block's Quins
    let blockSeq = 0;
    let triples  = 0;

    const PROGRESS_EVERY = 5000;

    for (const raw of lines) {
        const line = raw.trim();
        if (!line || line.startsWith('#')) continue;

        const tokens = line.split(/\s+/);
        if (tokens.length < 3) continue;
        const [sToken, pToken, oToken] = tokens;

        const sHash = hashToken(sToken);
        const pHash = hashToken(pToken);
        const oHash = hashToken(oToken);

        lexMap.set(sHash.toString(), stripToken(sToken));
        lexMap.set(pHash.toString(), stripToken(pToken));
        lexMap.set(oHash.toString(), stripToken(oToken));

        pending.push({ s: sHash, p: pHash, o: oHash });
        triples++;

        if (pending.length === QUINS_PER_BLOCK) {
            await writeBlock(opfsRoot, blockSeq, pending);
            blockSeq++;
            pending = [];
        }

        if (triples % PROGRESS_EVERY === 0) {
            self.postMessage({ type: 'progress', triples, blocks: blockSeq });
        }
    }

    // Flush final partial block
    if (pending.length > 0) {
        await writeBlock(opfsRoot, blockSeq, pending);
        blockSeq++;
    }

    // Write lexicon as JSON (BigInt keys serialised as decimal strings)
    const lexJson = JSON.stringify(Object.fromEntries(lexMap));
    const lexHandle = await opfsRoot.getFileHandle('lex.json', { create: true });
    const lexWritable = await lexHandle.createWritable();
    await lexWritable.write(new TextEncoder().encode(lexJson));
    await lexWritable.close();

    // Write a manifest so the VFS knows this dataset is available
    const manifest = {
        name: file.name,
        blockCount: blockSeq,
        triples,
        source: 'opfs',
        ingestedAt: new Date().toISOString(),
    };
    const mHandle = await opfsRoot.getFileHandle('manifest.json', { create: true });
    const mWritable = await mHandle.createWritable();
    await mWritable.write(new TextEncoder().encode(JSON.stringify(manifest)));
    await mWritable.close();

    self.postMessage({
        type: 'done',
        triples,
        blocks: blockSeq,
        lexEntries: lexMap.size,
    });
}

async function writeBlock(opfsRoot, blockIndex, quins) {
    let bytes;

    if (wasmPackQuins) {
        // WASM path: Rust computes the correct XOR parity for every quin.
        // Build a flat N×48 byte buffer; parity slot (bytes 40-47) can be zeros —
        // pack_quins_into_superblock() overwrites it with the correct value.
        const raw  = new Uint8Array(quins.length * 48);
        const view = new DataView(raw.buffer);
        for (let i = 0; i < quins.length; i++) {
            const off = i * 48;
            setU64LE(view, off,      quins[i].s);
            setU64LE(view, off + 8,  quins[i].p);
            setU64LE(view, off + 16, quins[i].o);
            // bytes 24-47: context, metadata, parity — all zero; Rust fills parity.
        }
        // Returned Uint8Array is a view into WASM memory; copy before awaiting.
        const wasmResult = wasmPackQuins(BigInt(blockIndex), 0n, raw);
        bytes = new Uint8Array(wasmResult);
    } else {
        // JS fallback path (used when WASM is unavailable or not yet compiled).
        bytes = buildSuperBlock(blockIndex, quins);
    }

    const fileName = `block_${blockIndex.toString().padStart(8, '0')}.qblk`;
    const fh       = await opfsRoot.getFileHandle(fileName, { create: true });
    const writable = await fh.createWritable();
    await writable.write(bytes);
    await writable.close();
}

function stripToken(token) {
    if (token.startsWith('<') && token.endsWith('>')) return token.slice(1, -1);
    if (token.startsWith('"')) {
        const rest = token.slice(1);
        let i = 0;
        while (i < rest.length) {
            if (rest[i] === '\\') { i += 2; continue; }
            if (rest[i] === '"')  return rest.slice(0, i);
            i++;
        }
    }
    return token;
}
