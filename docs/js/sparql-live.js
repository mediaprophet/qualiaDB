/**
 * Live SPARQL / triple-pattern runner for GH Pages.
 * GETs the Schema.org v3 .q42 in one shot, flattens live Quins, and calls
 * execute_ntriples_query (WASM). No mock rows and no HTTP Range paging.
 */

import { flattenUnifiedVolumeQuins, QUIN_SIZE } from '../playground/vfs.js?v=0.0.30-vfs-fullget2';

const DEFAULT_DATASET = 'schemaorg-30';
const SCHEMAORG_Q42 = new URL(
    '../data/schemaorg/30.0/schemaorg-current-https.q42',
    import.meta.url,
).href;

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
        const volumeUrl = datasetId === DEFAULT_DATASET
            ? SCHEMAORG_Q42
            : new URL('../data/schemaorg/30.0/schemaorg-current-https.q42', import.meta.url).href;
        const resp = await fetch(volumeUrl, { cache: 'no-store' });
        if (!resp.ok) {
            throw new Error(`Schema.org Q42 HTTP ${resp.status} (${volumeUrl})`);
        }
        const bytes = new Uint8Array(await resp.arrayBuffer());
        this.dbBytes = flattenUnifiedVolumeQuins(bytes);
        this.quinCount = this.dbBytes.length / QUIN_SIZE;
        if (!this.quinCount) {
            throw new Error('Schema.org Q42 decoded to zero Quins');
        }
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
