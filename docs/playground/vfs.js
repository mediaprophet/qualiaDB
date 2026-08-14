/**
 * Qualia-DB Virtual File System (VFS)
 *
 * Unified block I/O abstraction for the browser-side Webizen VM.
 * Implements demand-paging: JS fetches only the bytes needed before calling
 * the WASM VM, avoiding SharedArrayBuffer / COOP+COEP headers that GitHub
 * Pages cannot serve.
 *
 * Q42 v3 unified volume (preferred):
 *   [0..256)       Q42VolumeHeader (magic "Q42\\0", version 3)
 *   [lex_offset]   Q42LEX blob (structural vocabulary)
 *   [bidx_offset]  BIDX blob (object-range index)
 *   [reserved FIDX/PIDX]  optional field-range and postings (flags 0x0008 / 0x0010)
 *   [block_dir]    BlockDirectoryEntry × block_count (16 bytes each)
 *   [data_offset]  LZ4-compressed SuperBlock payloads
 *
 * Boot: `Range: bytes=0-8191` fetches the preamble (header + lex + bidx +
 * block directory).  Subsequent `readBlock(i)` issues targeted Range
 * requests for individual compressed SuperBlocks.
 *
 * All pre-release datasets use v3. Non-v3 files are rejected at mount time.
 *
 * Priority order for readBlock():
 *   1. OPFS local vault  — user-provided or previously cached data
 *   2. HTTP Range request — LZ4-compressed SuperBlock from block directory
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
/** Resolve manifest dataset paths from docs/ root (data/…) or playground-local files. */
export function resolveManifestDatasetUrl(path) {
    if (!path || path.startsWith('http://') || path.startsWith('https://')) return path;
    if (path.startsWith('data/')) return new URL(`../${path}`, import.meta.url).href;
    return new URL(path, import.meta.url).href;
}

const OPFS_DATASETS_DIR = 'qualia-datasets';
const OPFS_VOLUME_FILE  = 'volume.q42';
const OPFS_CACHE_META   = 'cache.json';

function sanitizeCacheKey(key) {
    return String(key || 'default').replace(/[^a-zA-Z0-9._-]/g, '_');
}

function cacheKeyFromUrl(url) {
    try {
        const u = new URL(url);
        const slug = u.pathname.replace(/^\/+/, '').replace(/\//g, '_');
        return sanitizeCacheKey(slug || 'default');
    } catch (_) {
        return 'default';
    }
}

/** Human-readable OPFS cache line for demo UI badges. */
export function formatOpfsCacheLabel(cache) {
    if (!cache) return '';
    if (cache.complete) {
        const mb = (cache.bytesCached / 1024 / 1024).toFixed(1);
        return ` · ${mb} MB in browser storage`;
    }
    if (cache.prefetching) {
        const pct = cache.totalBytes
            ? Math.min(99, Math.round((cache.bytesCached / cache.totalBytes) * 100))
            : 0;
        return pct > 0
            ? ` · caching ${pct}% to browser storage…`
            : ' · caching to browser storage…';
    }
    if (cache.bytesCached > 0) {
        const mb = (cache.bytesCached / 1024 / 1024).toFixed(1);
        return ` · ${mb} MB cached locally`;
    }
    return '';
}

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

        const candidatePaths = [entry.url, ...(entry.fallbackUrls ?? [])];
        const seen = new Set();
        const urls = [];
        for (const path of candidatePaths) {
            const url = resolveManifestDatasetUrl(path);
            if (!seen.has(url)) {
                seen.add(url);
                urls.push(url);
            }
        }

        let lastErr = null;
        for (const url of urls) {
            const vfs = new VFS(
                url,
                entry.lexUrl ?? null,
                entry.compressed ?? false,
                entry.bidxUrl ?? null,
            );
            try {
                await vfs.init({
                    loadLex: opts.loadLex ?? true,
                    cacheKey: datasetId,
                    prefetchToOpfs: opts.prefetchToOpfs ?? true,
                });
                this._mounted.set(datasetId, vfs);
                console.log(`[VFSProvider] Mounted "${datasetId}" → ${url}`);
                return vfs;
            } catch (e) {
                lastErr = e;
                console.warn(`[VFSProvider] Mount failed for "${datasetId}" at ${url}:`, e.message);
            }
        }

        throw lastErr ?? new Error(`VFSProvider: could not mount "${datasetId}"`);
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

const Q42_MAGIC       = 0x51; // 'Q'
const Q42_MAGIC2      = 0x34; // '4'
const Q42_MAGIC3      = 0x32; // '2'
const Q42_MAGIC4      = 0x00;
const Q42_VERSION_V3  = 3;
const HEADER_SIZE     = 256;
const DIR_ENTRY_SIZE  = 16;
const FLAG_BLOCKS_LZ4 = 0x0001;
const FLAG_OBJECT_SORTED = 0x0002;
const FLAG_VOLUME_ROOT = 0x0004;
const FLAG_FIELD_RANGES = 0x0008;
const FLAG_FIELD_POSTINGS = 0x0010;
const FLAG_PERMISSIVE_COMMONS = 0x0020;
const FLAG_SANCTUARY = 0x0040;
/** Initial Range fetch — covers lex+bidx+block_dir for typical ontology volumes. */
const PREAMBLE_PROBE  = 8191;

// ---------------------------------------------------------------------------
// Q42 v3 volume header + block directory (embedded lex/bidx preamble)
// ---------------------------------------------------------------------------

/**
 * Parse the 256-byte Q42 v3 volume header.
 * @param {DataView} dv — view over at least 256 bytes at offset 0
 * @returns {object|null}
 */
function hasQ42V3Magic(bytes) {
    if (!bytes || bytes.length < HEADER_SIZE) return false;
    const dv = new DataView(bytes.buffer, bytes.byteOffset, HEADER_SIZE);
    return parseQ42Header(dv) !== null;
}

export function parseQ42Header(dv) {
    if (dv.byteLength < HEADER_SIZE) return null;
    if (dv.getUint8(0) !== Q42_MAGIC || dv.getUint8(1) !== Q42_MAGIC2 ||
        dv.getUint8(2) !== Q42_MAGIC3 || dv.getUint8(3) !== Q42_MAGIC4) {
        return null;
    }
    const version = dv.getUint16(4, true);
    if (version !== Q42_VERSION_V3) return null;

    const flags = dv.getUint16(6, true);
    // Named v3 fields occupy 0..176; FIDX/PIDX live in _reserved[16..48] (file 192..224).
    const reserved = 176;
    return {
        version,
        flags,
        flagNames: decodeFlagNames(flags),
        lexOffset:        Number(dv.getBigUint64(8,  true)),
        lexLength:        Number(dv.getBigUint64(16, true)),
        bidxOffset:       Number(dv.getBigUint64(24, true)),
        bidxLength:       Number(dv.getBigUint64(32, true)),
        blockDirOffset:   Number(dv.getBigUint64(40, true)),
        blockDirLength:   Number(dv.getBigUint64(48, true)),
        dataOffset:       Number(dv.getBigUint64(56, true)),
        dataLength:       Number(dv.getBigUint64(64, true)),
        blockCount:       Number(dv.getBigUint64(72, true)),
        blockSize:        dv.getUint32(80, true),
        quinsPerBlock:    dv.getUint32(84, true),
        merkleRoot:       Array.from(new Uint8Array(dv.buffer, dv.byteOffset + 104, 32)),
        fidxOffset:       flags & FLAG_FIELD_RANGES ? Number(dv.getBigUint64(reserved + 16, true)) : 0,
        fidxLength:       flags & FLAG_FIELD_RANGES ? Number(dv.getBigUint64(reserved + 24, true)) : 0,
        pidxOffset:       flags & FLAG_FIELD_POSTINGS ? Number(dv.getBigUint64(reserved + 32, true)) : 0,
        pidxLength:       flags & FLAG_FIELD_POSTINGS ? Number(dv.getBigUint64(reserved + 40, true)) : 0,
        hasFieldRanges:   (flags & FLAG_FIELD_RANGES) !== 0,
        hasFieldPostings: (flags & FLAG_FIELD_POSTINGS) !== 0,
        isVolumeRoot:     (flags & FLAG_VOLUME_ROOT) !== 0,
        isCommons:        (flags & FLAG_PERMISSIVE_COMMONS) !== 0,
        isSanctuary:      (flags & FLAG_SANCTUARY) !== 0,
    };
}

function decodeFlagNames(flags) {
    const names = [];
    if (flags & FLAG_BLOCKS_LZ4) names.push('lz4');
    if (flags & FLAG_OBJECT_SORTED) names.push('object-sorted');
    if (flags & FLAG_VOLUME_ROOT) names.push('volume-root');
    if (flags & FLAG_FIELD_RANGES) names.push('field-ranges');
    if (flags & FLAG_FIELD_POSTINGS) names.push('field-postings');
    if (flags & FLAG_PERMISSIVE_COMMONS) names.push('permissive-commons');
    if (flags & FLAG_SANCTUARY) names.push('sanctuary');
    return names;
}

/**
 * Parse block-directory entries from a preamble buffer.
 * @param {Uint8Array} buf
 * @param {object} header
 * @returns {Array<{relOffset:number, compLen:number, uncompLen:number}>}
 */
function parseBlockDirectory(buf, header) {
    const entries = [];
    const base = header.blockDirOffset;
    for (let i = 0; i < header.blockCount; i++) {
        const off = base + i * DIR_ENTRY_SIZE;
        const dv = new DataView(buf.buffer, buf.byteOffset + off, DIR_ENTRY_SIZE);
        entries.push({
            relOffset:  Number(dv.getBigUint64(0, true)),
            compLen:    dv.getUint32(8,  true),
            uncompLen:  dv.getUint32(12, true),
        });
    }
    return entries;
}

/**
 * Maps block indices to LZ4-compressed byte ranges in a v3 unified volume.
 * The preamble (header + lex + bidx + block directory) is fetched once via
 * HTTP Range; subsequent reads target only the compressed SuperBlock payloads.
 */
export class V3BlockOffsetMap {
    /**
     * @param {object} header — parsed Q42VolumeHeader
     * @param {Array} dirEntries — BlockDirectoryEntry list
     * @param {number} totalBytes — full file size
     */
    constructor(header, dirEntries, totalBytes) {
        this._header     = header;
        this._entries    = dirEntries;
        this._total      = totalBytes;
        this._blockSize  = header.blockSize || BLOCK_SIZE;
    }

    get count() { return this._entries.length; }
    get totalBytes() { return this._total; }
    get dataOffset() { return this._header.dataOffset; }
    get blockSize() { return this._blockSize; }

    /**
     * Compressed byte range for HTTP Range fetch of block `index`.
     * @param {number} index
     * @returns {{ start: number, end: number, size: number, uncompLen: number }}
     */
    compressedRange(index) {
        const e = this._entries[index];
        const start = this._header.dataOffset + e.relOffset;
        return {
            start,
            end:       start + e.compLen - 1,
            size:      e.compLen,
            uncompLen: e.uncompLen,
        };
    }
}

// ---------------------------------------------------------------------------
// BlockOffsetMap — index-free, fixed-stride offset registry (legacy v1)
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
 * Decompress a single lz4_flex `compress_prepend_size` payload.
 * First 4 bytes = uncompressed length (u32 LE), remainder = raw LZ4 block.
 *
 * @param {Uint8Array} raw
 * @param {number} [expectedUncompLen]
 * @returns {Uint8Array}
 */
function _decompressLz4PrependSize(raw, expectedUncompLen) {
    const uncompLen = expectedUncompLen ?? _readU32LE(raw, 0);
    return _decodeLz4Block(raw.subarray(4), uncompLen);
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
     * @param {string}       remoteUrl    — URL of the hosted unified v3 .q42
     * @param {string}       [lexUrl]     — URL of the .q42-lex side-car
     * @param {boolean}      [compressed] — true when remoteUrl is an LZ4 block stream
     * @param {string|null}  [bidxUrl]    — URL of the .q42.bidx block-index side-car
     */
    constructor(remoteUrl, lexUrl = null, compressed = false, bidxUrl = null) {
        this._remoteUrl      = remoteUrl;
        /** Side-car URLs — null means rely on embedded v3 preamble or skip. */
        this._lexUrl         = lexUrl;
        this._bidxUrl        = bidxUrl;
        this._compressed     = compressed;
        /** @type {Map<bigint, string>} */
        this._lexMap         = new Map();
        this._lexLoaded      = false;
        this._opfsRoot       = null;
        /** @type {FileSystemDirectoryHandle|null} */
        this._opfsVault      = null;
        /** @type {FileSystemDirectoryHandle|null} */
        this._opfsBlocks     = null;
        this._cacheKey       = cacheKeyFromUrl(remoteUrl);
        this._prefetching    = false;
        this._opfsCacheStatus = {
            complete: false,
            bytesCached: 0,
            totalBytes: 0,
            prefetching: false,
            source: 'none',
        };
        this._totalBytes     = 0;
        /** @type {BlockOffsetMap|V3BlockOffsetMap|null} */
        this._blockOffsetMap = null;
        /** @type {BigUint64Array|null}  — interleaved [min0,max0, min1,max1, …] */
        this._blockRanges    = null;
        this._bidxLoaded     = false;
        this._rangeWarned    = false;
        /** True when lex+bidx were parsed from the embedded v3 preamble. */
        this._embeddedPreamble = false;
        this._volumeV3       = false;
        this._volumeHeader   = null;
        this._telemetry      = { netRequests: 0, opfsHits: 0, lastFaultMs: 0, totalFaultMs: 0 };
    }

    // -------------------------------------------------------------------------
    // Initialisation
    // -------------------------------------------------------------------------

    /**
     * Initialise the VFS.
     *
     * Boot sequence:
     *   1. Preamble Range fetch (bytes=0-N) — v3: embedded lex+bidx+block_dir;
     *      legacy: file-size probe only
     *   2. Side-car lex/bidx fetch — only when preamble did not embed them
     *
     * Telemetry is reset after boot so init probes are not counted as
     * query-level network fetches.
     *
     * @param {{ loadLex?: boolean, cacheKey?: string, prefetchToOpfs?: boolean }} [opts]
     */
    async init({ loadLex = true, cacheKey = null, prefetchToOpfs = true, onProgress = null } = {}) {
        if (cacheKey) this._cacheKey = sanitizeCacheKey(cacheKey);
        onProgress?.('Opening browser storage…', 2);
        try {
            this._opfsRoot = await navigator.storage.getDirectory();
            const datasets = await this._opfsRoot.getDirectoryHandle(OPFS_DATASETS_DIR, { create: true });
            this._opfsVault = await datasets.getDirectoryHandle(this._cacheKey, { create: true });
            this._opfsBlocks = await this._opfsVault.getDirectoryHandle('blocks', { create: true });
        } catch (_) {
            this._opfsRoot = null;
            this._opfsVault = null;
            this._opfsBlocks = null;
        }

        onProgress?.('Fetching dataset header…', 12);
        await this._bootFromHeader(onProgress);

        const sidecars = [];
        if (loadLex && !this._lexLoaded) sidecars.push(this._loadLexicon());
        if (!this._bidxLoaded) sidecars.push(this._loadBidx());
        if (sidecars.length) {
            onProgress?.('Loading vocabulary index…', 55);
            await Promise.all(sidecars);
        }

        onProgress?.('Dataset index ready', 68);
        await this._refreshOpfsCacheStatus();
        if (prefetchToOpfs && !this._opfsCacheStatus.complete) {
            this._prefetchVolumeToOpfs(onProgress);
        } else {
            onProgress?.('Dataset ready', 100);
        }

        this.resetTelemetry();
    }

    /** OPFS volume cache status for UI telemetry. */
    get opfsCache() { return { ...this._opfsCacheStatus }; }

    /** True when this volume uses the v3 unified format (embedded lex/bidx). */
    get embeddedPreamble() { return this._embeddedPreamble; }
    get volumeV3() { return this._volumeV3; }
    get volumeHeader() { return this._volumeHeader; }

    /** True once the BIDX has been loaded and `lookupBlocks()` is operative. */
    get bidxLoaded() { return this._bidxLoaded; }

    /**
     * Fetch the volume preamble and build routing tables.
     *
     * v3 unified volumes pack lex, bidx, and the block directory at the
     * front of the file.  A single Range request (`bytes=0-N`) gives the
     * client everything needed to route targeted SuperBlock fetches without
     * downloading the tensor payload.
     *
     * Legacy volumes (no Q42 magic) fall back to fixed-stride BlockOffsetMap
     * plus optional side-car lex/bidx URLs.
     */
    async _bootFromHeader(onProgress = null) {
        let preamble = null;

        // Probe: OPFS volume first (offline revisit), then HTTP Range.
        onProgress?.('Checking local cache…', 18);
        preamble = await this._readOpfsVolumeRange(0, PREAMBLE_PROBE + 1);
        if (preamble?.length && !hasQ42V3Magic(preamble)) {
            console.warn('[VFS] Stale OPFS volume — clearing and re-fetching from network');
            await this._invalidateOpfsVolume();
            preamble = null;
        }
        if (preamble?.length) {
            this._opfsCacheStatus.source = 'opfs';
            try {
                const fh = await this._opfsVault.getFileHandle(OPFS_VOLUME_FILE);
                this._totalBytes = (await fh.getFile()).size;
            } catch (_) { /* size unknown until header parse */ }
        }

        if (!preamble?.length) {
            onProgress?.('Downloading dataset header…', 28);
            preamble = await this._fetchVolumeBytes(0, PREAMBLE_PROBE + 1);
            if (preamble?.length) {
                if (preamble.length > PREAMBLE_PROBE + 1) {
                    await this._writeOpfsVolume(preamble);
                } else {
                    await this._writeOpfsVolumeRange(0, preamble);
                }
            }
        }

        if (!this._totalBytes) {
            try {
                const head = await fetch(this._remoteUrl, { method: 'HEAD', cache: 'no-store' });
                const cl = head.headers.get('Content-Length');
                if (cl) this._totalBytes = parseInt(cl, 10);
            } catch (_) { /* fully offline */ }
        }

        if (!preamble || preamble.length < HEADER_SIZE) {
            throw new Error(
                `[VFS] Could not read Q42 v3 header from ${this._remoteUrl} — ` +
                'the dataset may be missing on the server or cached as a stale 404. ' +
                'Hard-refresh (Ctrl+Shift+R) or clear site data for this origin, then retry.'
            );
        }

        const hdrDv = new DataView(preamble.buffer, preamble.byteOffset, HEADER_SIZE);
        const header = parseQ42Header(hdrDv);

        if (!header) {
            throw new Error(
                '[VFS] File is not a Q42 v3 unified volume — run qualia-cli ingest or q42 migrate meta'
            );
        }

        // v3: extend preamble if the probe window was too small.
        const preambleEnd = header.dataOffset;
        if (preamble.length < preambleEnd) {
            onProgress?.('Downloading dataset index…', 38);
            const cached = await this._readOpfsVolumeRange(0, preambleEnd);
            if (cached?.length >= preambleEnd) {
                preamble = cached;
            } else {
                try {
                    const extended = await this._fetchVolumeBytes(0, preambleEnd);
                    if (extended?.length >= preambleEnd) {
                        preamble = extended;
                        await this._writeOpfsVolumeRange(0, preamble);
                    }
                } catch (e) {
                    console.warn('[VFS] Extended preamble fetch failed:', e.message);
                }
            }
        }

        if (preamble.length < preambleEnd) {
            throw new Error(
                `[VFS] Preamble truncated (need ${preambleEnd} B, got ${preamble.length} B)`
            );
        }

        this._volumeV3 = true;
        this._embeddedPreamble = true;

        const dirEntries = parseBlockDirectory(preamble, header);
        this._blockOffsetMap = new V3BlockOffsetMap(
            header, dirEntries, this._totalBytes || preambleEnd + header.dataLength
        );

        onProgress?.('Parsing dataset index…', 48);
        this._parseLexiconBytes(
            preamble.subarray(header.lexOffset, header.lexOffset + header.lexLength)
        );
        this._parseBidxBytes(
            preamble.subarray(header.bidxOffset, header.bidxOffset + header.bidxLength),
            header.blockCount
        );

        onProgress?.('Dataset header ready', 62);
        this._opfsCacheStatus.totalBytes = this._totalBytes;
        this._volumeHeader = header;
        console.log(
            `[VFS] v3 preamble: lex=${header.lexLength} B, bidx=${header.bidxLength} B,` +
            ` fidx=${header.fidxLength} B, pidx=${header.pidxLength} B,` +
            ` ${header.blockCount} blocks, flags=${header.flagNames.join('|') || 'none'},` +
            ` data@${header.dataOffset}` +
            ` (${(this._totalBytes / 1024).toFixed(1)} KB total)` +
            (this._opfsCacheStatus.complete ? ' · OPFS cache hit' : '')
        );
    }

    /** Extract total file size from Content-Range or Content-Length. */
    _parseContentLength(resp) {
        const cr = resp.headers.get('Content-Range');
        if (cr) {
            const m = cr.match(/\/(\d+)$/);
            if (m) return parseInt(m[1], 10);
        }
        const cl = resp.headers.get('Content-Length');
        return cl ? parseInt(cl, 10) : 0;
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
        if (this._bidxLoaded || !this._bidxUrl) return;
        try {
            const resp = await fetch(this._bidxUrl);
            if (!resp.ok) return;
            this._parseBidxBytes(new Uint8Array(await resp.arrayBuffer()));
        } catch (e) {
            console.warn('[VFS] BIDX side-car load failed:', e.message);
        }
    }

    /**
     * Parse BIDX bytes (embedded preamble or side-car) into `_blockRanges`.
     * @param {Uint8Array} raw
     * @param {number} [expectedBlockCount]
     */
    _parseBidxBytes(raw, expectedBlockCount) {
        if (this._bidxLoaded || raw.length < 16) return;

        const magic = String.fromCharCode(raw[0], raw[1], raw[2], raw[3]);
        if (magic !== 'BIDX') {
            console.warn('[VFS] BIDX magic mismatch — skipping');
            return;
        }

        const dv         = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
        const blockCount = expectedBlockCount ?? dv.getUint32(8, true);

        if (this._blockOffsetMap && blockCount !== this._blockOffsetMap.count) {
            console.warn(
                `[VFS] BIDX block_count ${blockCount} ≠ file block_count ` +
                `${this._blockOffsetMap.count} — BIDX is stale, skipping`
            );
            return;
        }

        const rangesBytes = raw.byteLength - 16;
        if (rangesBytes !== blockCount * 16) {
            console.warn('[VFS] BIDX size mismatch — skipping');
            return;
        }
        this._blockRanges = new BigUint64Array(
            raw.buffer,
            raw.byteOffset + 16,
            blockCount * 2
        );
        this._bidxLoaded = true;
        console.log(`[VFS] BIDX loaded: ${blockCount} block ranges`);
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
        // 1. OPFS local vault (decoded superblock cache)
        if (this._opfsBlocks) {
            const cached = await this._readOpfsBlock(blockIndex);
            if (cached) return cached;
        }

        let bytes;

        if (!this._volumeV3 || !this._blockOffsetMap) {
            throw new Error('[VFS] readBlock called before v3 volume mount');
        }
        const { start, size, uncompLen } =
            this._blockOffsetMap.compressedRange(blockIndex);
        const compressed = await this._fetchRangeBlock(start, size);
        const decoded = _decompressLz4PrependSize(compressed, uncompLen);
        bytes = new Uint8Array(BLOCK_SIZE);
        bytes.set(decoded.subarray(0, Math.min(decoded.length, BLOCK_SIZE)));

        // 3. Persist to OPFS for future queries (fire-and-forget)
        if (this._opfsBlocks) {
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
        if (!this._opfsBlocks) return false;
        try {
            const fileName = `block_${blockId.toString().padStart(8, '0')}.qblk`;
            await this._opfsBlocks.getFileHandle(fileName);
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
        if (!this._opfsBlocks) throw new Error('OPFS unavailable');
        const fileName = `block_${blockIndex.toString().padStart(8, '0')}.qblk`;
        const fh = await this._opfsBlocks.getFileHandle(fileName, { create: true });
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
        const cached = await this._readOpfsVolumeRange(offset, size);
        if (cached?.length === size) {
            this._telemetry.opfsHits++;
            return cached;
        }

        const t0 = performance.now();
        const raw = await this._fetchVolumeBytes(offset, size);
        if (!raw || raw.length < size) {
            throw new Error(`VFS byte fetch failed at offset ${offset} (${this._remoteUrl})`);
        }
        const ms = performance.now() - t0;
        this._telemetry.netRequests++;
        this._telemetry.lastFaultMs = ms;
        this._telemetry.totalFaultMs += ms;

        await this._writeOpfsVolumeRange(offset, raw);
        return raw;
    }

    /**
     * Fetch a byte window from the remote volume.
     * Uses HTTP Range when possible; falls back to a cancellable stream read so
     * a cached 404 or hosts that mishandle Range still yield a valid preamble.
     */
    async _fetchVolumeBytes(offset, length) {
        if (!length) return null;
        const hi = offset + length - 1;
        try {
            const resp = await fetch(this._remoteUrl, {
                cache: 'no-store',
                headers: { Range: `bytes=${offset}-${hi}` },
            });
            if (resp.ok || resp.status === 206) {
                this._totalBytes = this._parseContentLength(resp) || this._totalBytes;
                const raw = new Uint8Array(await resp.arrayBuffer());
                if (raw.length >= length) {
                    if (raw.length >= offset + length) {
                        return raw.subarray(offset, offset + length);
                    }
                    if (!this._rangeWarned) {
                        this._rangeWarned = true;
                        console.warn(
                            '[VFS] Server returned HTTP 200 for a Range request — slicing client-side.'
                        );
                    }
                    return raw.subarray(0, length);
                }
            }
        } catch (e) {
            console.warn('[VFS] Range fetch failed, trying stream read:', e.message);
        }

        return this._fetchVolumeBytesStream(offset, length);
    }

    async _fetchVolumeBytesStream(offset, length) {
        const resp = await fetch(this._remoteUrl, { cache: 'no-store' });
        if (!resp.ok) return null;
        this._totalBytes = this._parseContentLength(resp) || this._totalBytes;

        const reader = resp.body?.getReader?.();
        if (!reader) {
            const all = new Uint8Array(await resp.arrayBuffer());
            return all.length >= offset + length ? all.subarray(offset, offset + length) : null;
        }

        const out = new Uint8Array(length);
        let filled = 0;
        let skipped = 0;
        try {
            while (filled < length) {
                const { done, value } = await reader.read();
                if (done) break;
                if (!value?.length) continue;

                if (skipped + value.length <= offset) {
                    skipped += value.length;
                    continue;
                }

                const startInChunk = Math.max(0, offset - skipped);
                const take = Math.min(value.length - startInChunk, length - filled);
                if (take > 0) {
                    out.set(value.subarray(startInChunk, startInChunk + take), filled);
                    filled += take;
                }
                skipped += value.length;
            }
        } finally {
            try { await reader.cancel(); } catch (_) { /* ignore */ }
        }

        return filled >= length ? out : null;
    }

    // -------------------------------------------------------------------------
    // Private helpers — OPFS
    // -------------------------------------------------------------------------

    async _readOpfsBlock(blockIndex) {
        if (!this._opfsBlocks) return null;
        try {
            const fileName = `block_${blockIndex.toString().padStart(8, '0')}.qblk`;
            const fh = await this._opfsBlocks.getFileHandle(fileName);
            const file = await fh.getFile();
            const buf  = await file.arrayBuffer();
            if (buf.byteLength === BLOCK_SIZE) {
                this._telemetry.opfsHits++;
                return new Uint8Array(buf);
            }
        } catch (_) { /* file doesn't exist */ }
        return null;
    }

    async _readOpfsVolumeRange(offset, size) {
        if (!this._opfsVault || !size) return null;
        try {
            const fh = await this._opfsVault.getFileHandle(OPFS_VOLUME_FILE);
            const file = await fh.getFile();
            if (file.size < offset + size) return null;
            return new Uint8Array(await file.slice(offset, offset + size).arrayBuffer());
        } catch (_) {
            return null;
        }
    }

    /** Drop a corrupt or outdated OPFS volume so the next boot re-fetches from HTTP. */
    async _invalidateOpfsVolume() {
        if (!this._opfsVault) return;
        for (const name of [OPFS_VOLUME_FILE, OPFS_CACHE_META]) {
            try {
                await this._opfsVault.removeEntry(name);
            } catch (_) { /* not present */ }
        }
        if (this._opfsBlocks) {
            try {
                // @ts-ignore — removeEntry on directory clears block sidecars
                for await (const [key] of this._opfsBlocks.entries()) {
                    await this._opfsBlocks.removeEntry(key);
                }
            } catch (_) { /* ignore */ }
        }
        this._opfsCacheStatus = {
            complete: false,
            bytesCached: 0,
            totalBytes: 0,
            prefetching: false,
            source: 'none',
        };
    }

    /** Public: clear browser OPFS cache for this dataset mount. */
    async clearOpfsCache() {
        await this._invalidateOpfsVolume();
    }

    async _writeOpfsVolume(bytes) {
        if (!this._opfsVault) return;
        const fh = await this._opfsVault.getFileHandle(OPFS_VOLUME_FILE, { create: true });
        const writable = await fh.createWritable();
        await writable.write(bytes);
        await writable.close();
        await this._writeCacheManifest({
            url: this._remoteUrl,
            size: bytes.byteLength,
            cachedAt: Date.now(),
            complete: this._totalBytes ? bytes.byteLength === this._totalBytes : true,
        });
        await this._refreshOpfsCacheStatus();
    }

    async _writeOpfsVolumeRange(offset, bytes) {
        if (!this._opfsVault) return;
        try {
            const fh = await this._opfsVault.getFileHandle(OPFS_VOLUME_FILE, { create: true });
            if (typeof fh.createSyncAccessHandle === 'function') {
                const access = await fh.createSyncAccessHandle({ mode: 'readwrite' });
                try {
                    access.write(bytes, { at: offset });
                    access.flush();
                } finally {
                    access.close();
                }
            } else {
                const existing = await this._readOpfsVolumeRange(0, Math.max(offset + bytes.length, this._totalBytes || 0));
                const out = existing?.length
                    ? existing
                    : new Uint8Array(Math.max(offset + bytes.length, this._totalBytes || offset + bytes.length));
                out.set(bytes, offset);
                await this._writeOpfsVolume(out);
                return;
            }
            await this._writeCacheManifest({
                url: this._remoteUrl,
                size: this._totalBytes || offset + bytes.length,
                cachedAt: Date.now(),
                complete: false,
            });
            await this._refreshOpfsCacheStatus();
        } catch (e) {
            console.warn('[VFS] OPFS range write failed:', e.message);
        }
    }

    async _writeCacheManifest(meta) {
        if (!this._opfsVault) return;
        const fh = await this._opfsVault.getFileHandle(OPFS_CACHE_META, { create: true });
        const writable = await fh.createWritable();
        await writable.write(JSON.stringify(meta));
        await writable.close();
    }

    async _refreshOpfsCacheStatus() {
        if (!this._opfsVault) return;
        try {
            const fh = await this._opfsVault.getFileHandle(OPFS_VOLUME_FILE);
            const file = await fh.getFile();
            const complete = this._totalBytes > 0 && file.size === this._totalBytes;
            this._opfsCacheStatus = {
                complete,
                bytesCached: file.size,
                totalBytes: this._totalBytes || file.size,
                prefetching: this._prefetching,
                source: complete ? 'opfs' : (file.size > 0 ? 'opfs-partial' : 'network'),
            };
        } catch (_) {
            this._opfsCacheStatus.bytesCached = 0;
            this._opfsCacheStatus.complete = false;
        }
    }

    async _prefetchVolumeToOpfs(onProgress = null) {
        if (!this._opfsVault || this._prefetching) return;
        if (await this._isVolumeFullyCached()) {
            await this._refreshOpfsCacheStatus();
            onProgress?.('Dataset ready (cached locally)', 100);
            return;
        }
        this._prefetching = true;
        this._opfsCacheStatus.prefetching = true;
        try {
            console.log(`[VFS] Caching ${this._cacheKey} to browser storage (OPFS)…`);
            onProgress?.('Caching dataset for offline use…', 72);
            const resp = await fetch(this._remoteUrl, { cache: 'no-store' });
            if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
            const total = this._parseContentLength(resp) || this._totalBytes || 0;
            this._opfsCacheStatus.totalBytes = total;

            const reader = resp.body?.getReader?.();
            if (!reader) {
                const buf = new Uint8Array(await resp.arrayBuffer());
                await this._writeOpfsVolume(buf);
                this._totalBytes = buf.byteLength;
                onProgress?.('Dataset cached locally', 100);
            } else {
                const chunks = [];
                let received = 0;
                while (true) {
                    const { done, value } = await reader.read();
                    if (done) break;
                    chunks.push(value);
                    received += value.length;
                    this._opfsCacheStatus.bytesCached = received;
                    if (total > 0) {
                        const pct = Math.min(99, Math.round((received / total) * 100));
                        onProgress?.(`Caching dataset… ${pct}%`, 72 + (pct / 100) * 28);
                    } else {
                        const mb = (received / 1024 / 1024).toFixed(1);
                        onProgress?.(`Caching dataset… ${mb} MB`, null);
                    }
                }
                const buf = new Uint8Array(received);
                let offset = 0;
                for (const chunk of chunks) {
                    buf.set(chunk, offset);
                    offset += chunk.length;
                }
                await this._writeOpfsVolume(buf);
                this._totalBytes = buf.byteLength;
                onProgress?.('Dataset cached locally', 100);
            }
            console.log(
                `[VFS] OPFS cache complete: ${(this._totalBytes / 1024 / 1024).toFixed(1)} MB` +
                ` (${this._cacheKey})`
            );
        } catch (e) {
            console.warn('[VFS] OPFS full-volume prefetch failed:', e.message);
        } finally {
            this._prefetching = false;
            this._opfsCacheStatus.prefetching = false;
            await this._refreshOpfsCacheStatus();
        }
    }

    async _isVolumeFullyCached() {
        if (!this._opfsVault || !this._totalBytes) return false;
        try {
            const fh = await this._opfsVault.getFileHandle(OPFS_VOLUME_FILE);
            const file = await fh.getFile();
            return file.size === this._totalBytes;
        } catch (_) {
            return false;
        }
    }

    // -------------------------------------------------------------------------
    // Private helpers — .q42-lex loader
    // -------------------------------------------------------------------------

    async _loadLexicon() {
        if (this._lexLoaded || !this._lexUrl) return;
        try {
            const resp = await fetch(this._lexUrl);
            if (!resp.ok) return;
            let raw = new Uint8Array(await resp.arrayBuffer());
            if (this._lexUrl.endsWith('.lz4')) {
                raw = decompressLz4Stream(raw);
            }
            this._parseLexiconBytes(raw);
        } catch (_) { /* lexicon optional */ }
    }

    /**
     * Parse Q42LEX bytes (embedded preamble or side-car) into `_lexMap`.
     * @param {Uint8Array} raw
     */
    _parseLexiconBytes(raw) {
        if (this._lexLoaded || raw.length < 32) return;

        const buf = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
        const magic = String.fromCharCode(
            buf.getUint8(0), buf.getUint8(1), buf.getUint8(2), buf.getUint8(3),
            buf.getUint8(4), buf.getUint8(5), buf.getUint8(6), buf.getUint8(7),
        );
        if (!magic.startsWith('Q42LEX')) {
            console.warn('[VFS] Q42LEX magic mismatch — skipping lexicon load');
            return;
        }

        const entryCount    = buf.getBigUint64(8,  true);
        const stringsOffset = buf.getBigUint64(16, true);
        const indexStart    = 32;

        const stringsBase = Number(stringsOffset);
        const strBlob = new Uint8Array(raw.buffer, raw.byteOffset + stringsBase, raw.length - stringsBase);
        const td = new TextDecoder('utf-8');

        for (let i = 0n; i < entryCount; i++) {
            const base = indexStart + Number(i) * 16;
            const hash   = buf.getBigUint64(base,     true);
            const strOff = buf.getBigUint64(base + 8, true);

            const off = Number(strOff);
            const tag = strBlob[off];
            let len, strStart;
            // v3 embedded lex uses a 1-byte type tag before u16 length.
            if (tag === 0x01 || tag === 0x03) {
                len = strBlob[off + 1] | (strBlob[off + 2] << 8);
                strStart = off + 3;
            } else if (tag === 0x02) {
                continue; // embedded triple — no string label
            } else {
                // Legacy side-car: u16 length at offset, no tag byte.
                len = strBlob[off] | (strBlob[off + 1] << 8);
                strStart = off + 2;
            }
            const str = td.decode(strBlob.subarray(strStart, strStart + len));
            this._lexMap.set(hash, str);
        }

        this._lexLoaded = true;
        console.log(`[VFS] Lexicon loaded: ${this._lexMap.size} entries`);
    }
}
