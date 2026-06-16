/**
 * Qualia-DB Virtual File System (VFS)
 *
 * Unified block I/O abstraction for the browser-side Webizen VM.
 * Implements the "Pre-fetch Synchronous" model: JS fetches raw SuperBlock bytes
 * before calling the WASM VM, avoiding SharedArrayBuffer / COOP+COEP headers
 * that GitHub Pages cannot serve.
 *
 * Priority order for readBlock():
 *   1. OPFS local vault  — user-provided or previously cached data
 *   2. HTTP Range request — hosted .q42 file on GitHub Pages / CDN
 *
 * .q42-lex side-car format (for reverse hash → string lookup):
 *   Header  (32 bytes): magic[8] | entry_count:u64LE | strings_offset:u64LE | version:u64LE
 *   Index   (entry_count × 16 bytes, sorted by hash): hash:u64LE | str_off:u64LE
 *   Strings (variable): for each entry — length:u16LE + UTF-8 bytes
 */

import { parseBigDecimal } from './hash.js';

// ---------------------------------------------------------------------------
// VFSProvider — manifest-driven multi-dataset orchestrator
// ---------------------------------------------------------------------------

/**
 * Loads the `vfs-manifest.json` registry and provides a `mount(id)` factory
 * that returns a ready-to-query `VFS` instance for any listed dataset.
 *
 * Usage:
 *   const provider = await VFSProvider.fromManifest();
 *   const vfs = await provider.mount('wordnet');
 *   const bytes = await vfs.readAll();
 */
export class VFSProvider {
    constructor(manifest) {
        /** @type {{ version: number, datasets: Array }} */
        this._manifest = manifest;
        /** @type {Map<string, VFS>} */
        this._mounted  = new Map();
    }

    /**
     * Load the manifest and return a VFSProvider.
     *
     * @param {string} [manifestUrl]
     * @returns {Promise<VFSProvider>}
     */
    static async fromManifest(manifestUrl = './vfs-manifest.json') {
        let manifest;
        try {
            const resp = await fetch(manifestUrl);
            if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
            manifest = await resp.json();
        } catch (e) {
            console.warn('[VFSProvider] Could not load manifest:', e.message);
            manifest = { version: 1, datasets: [] };
        }
        return new VFSProvider(manifest);
    }

    /** All dataset descriptors from the manifest. */
    get available() { return this._manifest.datasets ?? []; }

    /**
     * Mount a dataset by its manifest `id`.  Returns the initialised VFS.
     * Subsequent calls with the same id return the cached VFS.
     *
     * @param {string} datasetId
     * @param {{ loadLex?: boolean }} [opts]
     * @returns {Promise<VFS>}
     */
    async mount(datasetId, opts = {}) {
        if (this._mounted.has(datasetId)) return this._mounted.get(datasetId);

        const entry = this.available.find(d => d.id === datasetId);
        if (!entry) throw new Error(`VFSProvider: unknown dataset "${datasetId}"`);

        const vfs = new VFS(entry.url, entry.lexUrl, entry.compressed ?? false, entry.bidxUrl ?? null);
        await vfs.init({ loadLex: opts.loadLex ?? true });

        this._mounted.set(datasetId, vfs);
        console.log(`[VFSProvider] Mounted "${datasetId}" → ${entry.url}`);
        return vfs;
    }

    /**
     * Unmount a previously mounted dataset (releases cached VFS).
     *
     * @param {string} datasetId
     */
    unmount(datasetId) { this._mounted.delete(datasetId); }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

export const BLOCK_SIZE      = 40_960;  // bytes per QualiaSuperBlock
export const QUIN_SIZE       = 48;      // bytes per QualiaQuin
export const QUINS_PER_BLOCK = 850;

// ---------------------------------------------------------------------------
// BlockOffsetMap — index-free, fixed-stride offset registry
// ---------------------------------------------------------------------------

/**
 * Maps block indices to exact byte ranges within a fixed-stride file.
 *
 * For the uncompressed `.q42` format every SuperBlock is exactly BLOCK_SIZE
 * bytes, so `range(i)` is trivially `[i*BS, (i+1)*BS - 1]`.  This class
 * provides a stable API that can be swapped for a real variable-stride index
 * (e.g. parsed from a future file-level header) without changing callers.
 *
 * Created during VFS.init() from the Content-Range header of the
 * header-first boot fetch — no separate HEAD request needed.
 */
export class BlockOffsetMap {
    /**
     * @param {number} totalBytes — full file size in bytes (from Content-Range)
     * @param {number} [blockSize]
     */
    constructor(totalBytes, blockSize = BLOCK_SIZE) {
        this._total     = totalBytes;
        this._blockSize = blockSize;
        this._count     = Math.ceil(totalBytes / blockSize);
    }

    /** Number of blocks in the file. */
    get count() { return this._count; }

    /** Raw file size in bytes. */
    get totalBytes() { return this._total; }

    /**
     * Byte range for block `index`.  The last block may be smaller than
     * `blockSize` if the file is not an exact multiple.
     *
     * @param {number} index
     * @returns {{ start: number, end: number, size: number }}
     */
    range(index) {
        const start = index * this._blockSize;
        const end   = Math.min(start + this._blockSize, this._total) - 1;
        return { start, end, size: end - start + 1 };
    }
}

const LEX_MAGIC = 'Q42LEX\0\0';

// ---------------------------------------------------------------------------
// LZ4 block-stream decoder
// ---------------------------------------------------------------------------
// Handles the format written by qualia-cli compress:
//   Per block: [block_id:u64][comp_len:u32][uncomp_len:u32][lz4_flex payload]
//   lz4_flex payload: [uncomp_len:u32 LE][LZ4 raw block data]

function _readU32LE(buf, off) {
    return (buf[off] | buf[off+1]<<8 | buf[off+2]<<16 | buf[off+3]<<24) >>> 0;
}

/**
 * Decode a single LZ4 raw block into `dst` (Uint8Array of `uncompLen` bytes).
 * `src` must be the raw LZ4 block data (no prepend-size header).
 */
function _decodeLz4Block(src, uncompLen) {
    const dst = new Uint8Array(uncompLen);
    let si = 0, di = 0;
    while (si < src.length) {
        const token = src[si++];

        // Literals
        let litLen = token >>> 4;
        if (litLen === 15) { let b; do { b = src[si++]; litLen += b; } while (b === 255); }
        dst.set(src.subarray(si, si + litLen), di);
        si += litLen; di += litLen;

        if (si >= src.length) break; // last sequence has no match

        // Match offset (little-endian u16)
        const offset = src[si] | (src[si+1] << 8); si += 2;

        // Match length (minimum match = 4)
        let matchLen = (token & 0xf) + 4;
        if ((token & 0xf) === 15) { let b; do { b = src[si++]; matchLen += b; } while (b === 255); }

        // Copy match (may overlap — must copy byte-by-byte)
        let mPos = di - offset;
        for (let i = 0; i < matchLen; i++) dst[di++] = dst[mPos++];
    }
    return dst;
}

/**
 * Decompress a full LZ4 block-stream (as produced by qualia-cli compress).
 * Returns a flat Uint8Array of all decompressed bytes concatenated.
 *
 * @param {Uint8Array} raw  — the complete compressed file bytes
 * @returns {Uint8Array}
 */
export function decompressLz4Stream(raw) {
    const chunks = [];
    let totalSize = 0;
    let offset = 0;

    while (offset + 16 <= raw.length) {
        // block_id (8 bytes, ignored), comp_len (4), uncomp_len (4)
        const compLen   = _readU32LE(raw, offset + 8);
        const uncompLen = _readU32LE(raw, offset + 12);
        offset += 16;

        if (compLen === 0 || offset + compLen > raw.length) break;

        // lz4_flex prepends a 4-byte uncompressed size — skip it
        const lz4Data = raw.subarray(offset + 4, offset + compLen);
        const decoded = _decodeLz4Block(lz4Data, uncompLen);

        chunks.push(decoded);
        totalSize += decoded.length;
        offset += compLen;
    }

    const result = new Uint8Array(totalSize);
    let pos = 0;
    for (const chunk of chunks) { result.set(chunk, pos); pos += chunk.length; }
    return result;
}

// ---------------------------------------------------------------------------
// VFS class
// ---------------------------------------------------------------------------

export class VFS {
    /**
     * @param {string}       remoteUrl    — URL of the hosted .q42 (or .c.q42) file
     * @param {string}       [lexUrl]     — URL of the .q42-lex side-car
     * @param {boolean}      [compressed] — true when remoteUrl is an LZ4 block stream
     * @param {string|null}  [bidxUrl]    — URL of the .q42.bidx block-index side-car
     */
    constructor(remoteUrl, lexUrl, compressed = false, bidxUrl = null) {
        this._remoteUrl      = remoteUrl;
        this._lexUrl         = lexUrl ?? (remoteUrl + '.lex');
        this._bidxUrl        = bidxUrl ?? (remoteUrl + '.bidx');
        this._compressed     = compressed;
        /** @type {Map<bigint, string>} */
        this._lexMap         = new Map();
        this._lexLoaded      = false;
        this._opfsRoot       = null;
        this._totalBytes     = 0;
        /** @type {BlockOffsetMap|null} */
        this._blockOffsetMap = null;
        /** @type {BigUint64Array|null}  — interleaved [min0,max0, min1,max1, …] */
        this._blockRanges    = null;
        this._bidxLoaded     = false;
        this._rangeWarned    = false;
        this._telemetry      = { netRequests: 0, opfsHits: 0, lastFaultMs: 0, totalFaultMs: 0 };
    }

    // -------------------------------------------------------------------------
    // Initialisation
    // -------------------------------------------------------------------------

    /**
     * Initialise the VFS.
     *
     * Boot sequence (all run concurrently after OPFS init):
     *   1. Header-first Range: bytes=0-1023 → `BlockOffsetMap` (file size)
     *   2. Lexicon side-car fetch → hash → string map
     *   3. BIDX side-car fetch   → block range index for O(log N) lookup
     *
     * Telemetry is reset after boot so init probes are not counted as
     * query-level network fetches.
     *
     * @param {{ loadLex?: boolean }} [opts]
     */
    async init({ loadLex = true } = {}) {
        try {
            this._opfsRoot = await navigator.storage.getDirectory();
        } catch (_) {
            this._opfsRoot = null;
        }

        await Promise.all([
            this._bootFromHeader(),
            loadLex ? this._loadLexicon() : Promise.resolve(),
            this._loadBidx(),
        ]);

        this.resetTelemetry();
    }

    /** True once the BIDX has been loaded and `lookupBlocks()` is operative. */
    get bidxLoaded() { return this._bidxLoaded; }

    /**
     * Fetch bytes 0-1023 of the remote file.
     *
     * The `Content-Range` response header has the form
     * `bytes 0-1023/<total>`, giving us the total file size without a
     * separate HEAD request.  A HEAD fallback is used when the server does
     * not return `Content-Range` (e.g. a static server that answers 200
     * instead of 206 for range requests).
     */
    async _bootFromHeader() {
        try {
            const ctrl = new AbortController();
            const resp = await fetch(this._remoteUrl, {
                headers: { Range: 'bytes=0-1023' },
                signal:  ctrl.signal,
            });
            if (resp.ok || resp.status === 206) {
                const cr = resp.headers.get('Content-Range');
                if (cr) {
                    // "bytes 0-1023/268361728"
                    const m = cr.match(/\/(\d+)$/);
                    if (m) this._totalBytes = parseInt(m[1], 10);
                }
                if (!this._totalBytes) {
                    // Server returned 200 (no partial-content support) —
                    // fall back to Content-Length of the full response.
                    const cl = resp.headers.get('Content-Length');
                    if (cl) this._totalBytes = parseInt(cl, 10);
                }
            }
            // Abort the body — we only needed the headers.
            ctrl.abort();
        } catch (_) { /* offline or abort — try HEAD fallback below */ }

        // HEAD fallback for servers that reject Range requests entirely.
        if (!this._totalBytes) {
            try {
                const head = await fetch(this._remoteUrl, { method: 'HEAD' });
                const cl = head.headers.get('Content-Length');
                if (cl) this._totalBytes = parseInt(cl, 10);
            } catch (_) { /* fully offline */ }
        }

        if (this._totalBytes > 0) {
            this._blockOffsetMap = new BlockOffsetMap(this._totalBytes, BLOCK_SIZE);
            console.log(
                `[VFS] BlockOffsetMap: ${this._blockOffsetMap.count} blocks` +
                ` × ${BLOCK_SIZE} B = ${(this._totalBytes / 1024 / 1024).toFixed(1)} MB`
            );
        }
    }

    /** Number of SuperBlocks in the file (derived from BlockOffsetMap). */
    get blockCount() {
        return this._blockOffsetMap ? this._blockOffsetMap.count : 0;
    }

    /** The BlockOffsetMap built during header-first boot. */
    get blockOffsetMap() { return this._blockOffsetMap; }

    /** Snapshot of demand-paging counters (copied so callers can't mutate). */
    get telemetry() { return { ...this._telemetry }; }

    /** Reset all counters — call before each query to get per-query stats. */
    resetTelemetry() {
        this._telemetry = { netRequests: 0, opfsHits: 0, lastFaultMs: 0, totalFaultMs: 0 };
    }

    /**
     * Fetch the first 1024 bytes of the remote file.
     *
     * After `init()` these bytes are already encoded in the `BlockOffsetMap`;
     * this method is provided for callers that want to inspect the raw
     * SuperBlock header bytes (e.g. to read `active_quin_count` for block 0).
     *
     * @returns {Promise<Uint8Array>}
     */
    async readHeader() {
        return this._fetchRangeBlock(0, 1024);
    }

    // -------------------------------------------------------------------------
    // Block-level index (BIDX)
    // -------------------------------------------------------------------------

    /**
     * Load the `.q42.bidx` side-car produced by `qualia-cli ingest`.
     *
     * Binary format (16-byte header + block_count × 16 bytes):
     * ```
     * [0..4]   magic       b"BIDX"
     * [4..8]   version     u32 LE = 1
     * [8..12]  block_count u32 LE
     * [12..16] reserved    u32 LE = 0
     * [16..]   [min_obj_hash: u64 LE, max_obj_hash: u64 LE] × block_count
     * ```
     * Stored as a flat `BigUint64Array` — `_blockRanges[i*2]` = min for block i,
     * `_blockRanges[i*2+1]` = max for block i.  Binary-searchable because ranges
     * are non-overlapping and sorted ascending (ingestor sorts by object hash).
     */
    async _loadBidx() {
        try {
            const resp = await fetch(this._bidxUrl);
            if (!resp.ok) return; // BIDX is optional
            const raw = new Uint8Array(await resp.arrayBuffer());
            if (raw.length < 16) return;

            // Verify magic
            const magic = String.fromCharCode(raw[0], raw[1], raw[2], raw[3]);
            if (magic !== 'BIDX') {
                console.warn('[VFS] .bidx magic mismatch — skipping BIDX load');
                return;
            }

            const dv         = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
            const blockCount = dv.getUint32(8, true); // LE

            // Sanity-check against BlockOffsetMap
            if (this._blockOffsetMap && blockCount !== this._blockOffsetMap.count) {
                console.warn(
                    `[VFS] BIDX block_count ${blockCount} ≠ file block_count ` +
                    `${this._blockOffsetMap.count} — BIDX is stale, skipping`
                );
                return;
            }

            // Slice the range data directly from the buffer — zero-copy
            const rangesBytes = raw.byteLength - 16;
            if (rangesBytes !== blockCount * 16) {
                console.warn('[VFS] BIDX size mismatch — skipping BIDX load');
                return;
            }
            this._blockRanges = new BigUint64Array(
                raw.buffer,
                raw.byteOffset + 16,
                blockCount * 2
            );
            this._bidxLoaded = true;
            console.log(`[VFS] BIDX loaded: ${blockCount} block ranges`);
        } catch (e) {
            console.warn('[VFS] BIDX load failed:', e.message);
        }
    }

    /**
     * Binary-search the BIDX for blocks whose object-hash range brackets
     * `targetHash`.
     *
     * Returns an array of block indices (typically 1-2) whose `[min, max]`
     * range includes the hash, or `null` when the BIDX is not loaded.
     * Callers should fall back to a full linear scan when `null` is returned.
     *
     * O(log N) search — N = block count (≤ 6 540 for WordNet).
     *
     * @param {bigint} targetHash
     * @returns {number[]|null}
     */
    lookupBlocks(targetHash) {
        if (!this._bidxLoaded || !this._blockRanges) return null;

        const ranges = this._blockRanges;
        const n      = ranges.length >> 1; // block count
        const hash   = BigInt.asUintN(64, targetHash);

        // Binary search: find leftmost block where max_hash >= hash
        let lo = 0, hi = n;
        while (lo < hi) {
            const mid = (lo + hi) >> 1;
            if (ranges[mid * 2 + 1] < hash) lo = mid + 1;
            else hi = mid;
        }

        // Collect consecutive blocks where min_hash <= hash (usually 1-2)
        const candidates = [];
        for (let i = lo; i < n; i++) {
            if (ranges[i * 2] > hash) break;
            candidates.push(i);
        }

        return candidates.length > 0 ? candidates : null;
    }

    // -------------------------------------------------------------------------
    // Block I/O
    // -------------------------------------------------------------------------

    /**
     * Read one SuperBlock by index.
     *
     * Priority order:
     *   1. OPFS local vault  — zero-latency after first fetch
     *   2. HTTP Range request — fetches exact bytes from `BlockOffsetMap`
     *
     * After a successful Range fetch the block is asynchronously written to
     * OPFS (fire-and-forget) so subsequent queries are disk-only.
     *
     * @param {number} blockIndex — zero-based superblock index
     * @returns {Promise<Uint8Array>} raw block bytes (BLOCK_SIZE bytes)
     */
    async readBlock(blockIndex) {
        // 1. OPFS local vault
        if (this._opfsRoot) {
            const cached = await this._readOpfsBlock(blockIndex);
            if (cached) return cached;
        }

        // 2. HTTP Range request — exact byte range from BlockOffsetMap
        const { start, size } = this._blockOffsetMap
            ? this._blockOffsetMap.range(blockIndex)
            : { start: blockIndex * BLOCK_SIZE, size: BLOCK_SIZE };

        const bytes = await this._fetchRangeBlock(start, size);

        // 3. Persist to OPFS for future queries (fire-and-forget)
        if (this._opfsRoot) {
            this.writeBlock(blockIndex, bytes).catch(() => {});
        }
        return bytes;
    }

    /**
     * Check whether block `blockId` is cached in OPFS without fetching it.
     * Returns `false` when OPFS is unavailable.
     *
     * @param {number} blockId
     * @returns {Promise<boolean>}
     */
    async is_cached(blockId) {
        if (!this._opfsRoot) return false;
        try {
            const fileName = `block_${blockId.toString().padStart(8, '0')}.qblk`;
            await this._opfsRoot.getFileHandle(fileName);
            return true;
        } catch (_) {
            return false;
        }
    }

    /**
     * Read ALL blocks from the remote file in one fetch (for small datasets).
     * Suitable when the entire .q42 file fits comfortably in browser memory.
     *
     * @returns {Promise<Uint8Array>}
     */
    async readAll() {
        const resp = await fetch(this._remoteUrl);
        if (!resp.ok) throw new Error(`VFS readAll: HTTP ${resp.status}`);
        const buf = await resp.arrayBuffer();
        return new Uint8Array(buf);
    }

    /**
     * Fetch the full LZ4 block-stream .c.q42, decompress it, and return a flat
     * Uint8Array of raw 48-byte Quin records (no SuperBlock headers).
     * Only valid when this._compressed === true.
     *
     * @returns {Promise<Uint8Array>}
     */
    async readAllDecompressed() {
        const resp = await fetch(this._remoteUrl);
        if (!resp.ok) throw new Error(`VFS readAllDecompressed: HTTP ${resp.status}`);
        const raw = new Uint8Array(await resp.arrayBuffer());
        return decompressLz4Stream(raw);
    }

    /** Whether this VFS was opened against an LZ4-compressed data file. */
    get compressed() { return this._compressed; }

    /**
     * Write a block to the OPFS local vault (used by the ingest worker).
     *
     * @param {number} blockIndex
     * @param {Uint8Array} bytes  — must be exactly BLOCK_SIZE bytes
     */
    async writeBlock(blockIndex, bytes) {
        if (!this._opfsRoot) throw new Error('OPFS unavailable');
        const fileName = `block_${blockIndex.toString().padStart(8, '0')}.qblk`;
        const fh = await this._opfsRoot.getFileHandle(fileName, { create: true });
        const writable = await fh.createWritable();
        await writable.write(bytes);
        await writable.close();
    }

    // -------------------------------------------------------------------------
    // Lexicon reverse lookup
    // -------------------------------------------------------------------------

    /**
     * Look up a 64-bit hash (as BigInt) in the lexicon.
     * Returns the canonical string (stripped IRI / literal), or a hex fallback.
     *
     * @param {bigint} hash
     * @returns {string}
     */
    lookup(hash) {
        return this._lexMap.get(hash) ?? `0x${hash.toString(16).padStart(16, '0')}`;
    }

    /** True once the lexicon has been loaded. */
    get lexLoaded() { return this._lexLoaded; }

    // -------------------------------------------------------------------------
    // Private helpers — HTTP
    // -------------------------------------------------------------------------

    async _fetchRangeBlock(offset, size) {
        const t0 = performance.now();
        const hi = offset + size - 1;
        const resp = await fetch(this._remoteUrl, {
            headers: { Range: `bytes=${offset}-${hi}` }
        });
        if (!resp.ok && resp.status !== 206) {
            throw new Error(`VFS Range fetch failed: HTTP ${resp.status}`);
        }
        const buf = await resp.arrayBuffer();
        const ms = performance.now() - t0;
        this._telemetry.netRequests++;
        this._telemetry.lastFaultMs = ms;
        this._telemetry.totalFaultMs += ms;

        const raw = new Uint8Array(buf);
        // HTTP 206 Partial Content → server honored the Range; return as-is.
        // HTTP 200 → server ignored Range (e.g. Python SimpleHTTPServer).
        // In that case, slice the correct window so the caller gets the right
        // 40 KB block.  This is less efficient (full file downloaded) but keeps
        // the API contract intact.  A warning is emitted on the first occurrence.
        if (resp.status === 200 && raw.byteLength !== size) {
            if (!this._rangeWarned) {
                this._rangeWarned = true;
                console.warn(
                    '[VFS] Server returned HTTP 200 for a Range request — ' +
                    'Range header was ignored.  Demand-paging disabled; ' +
                    'use a server that supports HTTP 206 (nginx, GitHub Pages, ' +
                    'express/serve-static) for true block streaming.'
                );
            }
            return raw.subarray(offset, offset + size);
        }
        return raw;
    }

    // -------------------------------------------------------------------------
    // Private helpers — OPFS
    // -------------------------------------------------------------------------

    async _readOpfsBlock(blockIndex) {
        try {
            const fileName = `block_${blockIndex.toString().padStart(8, '0')}.qblk`;
            const fh = await this._opfsRoot.getFileHandle(fileName);
            const file = await fh.getFile();
            const buf  = await file.arrayBuffer();
            if (buf.byteLength === BLOCK_SIZE) {
                this._telemetry.opfsHits++;
                return new Uint8Array(buf);
            }
        } catch (_) { /* file doesn't exist */ }
        return null;
    }

    // -------------------------------------------------------------------------
    // Private helpers — .q42-lex loader
    // -------------------------------------------------------------------------

    async _loadLexicon() {
        let buf;
        try {
            const resp = await fetch(this._lexUrl);
            if (!resp.ok) return; // lexicon optional
            const raw = new Uint8Array(await resp.arrayBuffer());
            // Decompress if the lex URL ends in .lz4
            const decompressed = this._lexUrl.endsWith('.lz4')
                ? decompressLz4Stream(raw)
                : raw;
            buf = new DataView(decompressed.buffer, decompressed.byteOffset, decompressed.byteLength);
        } catch (_) { return; }

        // Verify magic
        const magic = String.fromCharCode(
            ...new Uint8Array(buf.buffer, 0, 8)
        );
        if (!magic.startsWith('Q42LEX')) {
            console.warn('[VFS] .q42-lex magic mismatch — skipping lexicon load');
            return;
        }

        const entryCount     = buf.getBigUint64(8,  true); // LE
        const stringsOffset  = buf.getBigUint64(16, true); // LE
        const indexStart     = 32;

        const stringsBase = Number(stringsOffset);
        const strBlob = new Uint8Array(buf.buffer, stringsBase);
        const td = new TextDecoder('utf-8');

        for (let i = 0n; i < entryCount; i++) {
            const base = indexStart + Number(i) * 16;
            const hash   = buf.getBigUint64(base,     true);
            const strOff = buf.getBigUint64(base + 8, true);

            const off = Number(strOff);
            const len = strBlob[off] | (strBlob[off + 1] << 8); // u16 LE
            const str = td.decode(strBlob.subarray(off + 2, off + 2 + len));

            this._lexMap.set(hash, str);
        }

        this._lexLoaded = true;
        console.log(`[VFS] Lexicon loaded: ${this._lexMap.size} entries`);
    }
}
