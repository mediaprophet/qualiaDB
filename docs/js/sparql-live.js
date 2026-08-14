/**
 * Live SPARQL / triple-pattern runner for GH Pages.
 * Mounts a unified v3 .q42 via VFS, flattens live Quins, and calls
 * execute_ntriples_query (WASM). No mock result rows.
 */

import { VFSProvider, QUIN_SIZE } from '../playground/vfs.js?v=0.0.30-vfs-fullget1';

const SUPERBLOCK_HEADER = 160;
const DEFAULT_DATASET = 'schemaorg-30';

function getU64(view, off) {
    return BigInt(view.getUint32(off, true)) | (BigInt(view.getUint32(off + 4, true)) << 32n);
}

function extractLiveQuins(blockBytes) {
    if (!blockBytes || blockBytes.length < SUPERBLOCK_HEADER + QUIN_SIZE) {
        return new Uint8Array(0);
    }
    const view = new DataView(blockBytes.buffer, blockBytes.byteOffset);
    const live = Number(getU64(view, 16));
    const cap = Math.floor((blockBytes.length - SUPERBLOCK_HEADER) / QUIN_SIZE);
    const count = Math.max(0, Math.min(live > 0 ? live : cap, cap));
    const out = new Uint8Array(count * QUIN_SIZE);
    out.set(blockBytes.subarray(SUPERBLOCK_HEADER, SUPERBLOCK_HEADER + count * QUIN_SIZE));
    return out;
}

export class SparqlLiveSession {
    constructor() {
        this.ready = false;
        this.error = null;
        this.wasm = null;
        this.vfs = null;
        this.dbBytes = null;
        this.quinCount = 0;
        this.datasetId = DEFAULT_DATASET;
    }

    async init(datasetId = DEFAULT_DATASET) {
        this.datasetId = datasetId;
        const wasmMod = await import('../playground/qualia_core_db.js');
        await wasmMod.default();
        this.wasm = wasmMod;
        if (typeof wasmMod.execute_ntriples_query !== 'function') {
            throw new Error('WASM execute_ntriples_query is not in this build');
        }
        const provider = await VFSProvider.fromManifest(
            new URL('../playground/vfs-manifest.json', import.meta.url).href,
        );
        this.vfs = await provider.mount(datasetId, { loadLex: true, prefetchToOpfs: false });
        const chunks = [];
        let total = 0;
        for (let i = 0; i < this.vfs.blockCount; i++) {
            const block = await this.vfs.readBlock(i);
            const live = extractLiveQuins(block);
            if (live.length) {
                chunks.push(live);
                total += live.length;
            }
        }
        this.dbBytes = new Uint8Array(total);
        let offset = 0;
        for (const chunk of chunks) {
            this.dbBytes.set(chunk, offset);
            offset += chunk.length;
        }
        this.quinCount = total / QUIN_SIZE;
        this.ready = true;
        return this;
    }

    compile(query) {
        if (typeof this.wasm.compile_query_to_json === 'function') {
            try {
                return JSON.parse(this.wasm.compile_query_to_json(query));
            } catch (err) {
                return { error: String(err) };
            }
        }
        return null;
    }

    run(query, maxResults = 64) {
        if (!this.ready) throw new Error('SPARQL session is not ready');
        const t0 = performance.now();
        const raw = this.wasm.execute_ntriples_query(query, this.dbBytes, maxResults);
        const elapsed = performance.now() - t0;
        let parsed;
        try {
            parsed = JSON.parse(raw);
        } catch (_) {
            parsed = { raw };
        }
        return {
            elapsedMs: elapsed,
            quinCount: this.quinCount,
            datasetId: this.datasetId,
            compiled: this.compile(query),
            result: parsed,
        };
    }
}

let singleton = null;

export async function ensureSparqlLive(datasetId = DEFAULT_DATASET) {
    if (singleton?.ready && singleton.datasetId === datasetId) return singleton;
    singleton = new SparqlLiveSession();
    await singleton.init(datasetId);
    return singleton;
}
