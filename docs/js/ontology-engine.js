/**
 * Ontology browser engine — WASM + VFS over .q42 volumes listed in vfs-manifest.json.
 */

import { parseBigDecimal, hashToken, toHex16, hasMsb } from '../playground/hash.js';
import { VFS, QUIN_SIZE } from '../playground/vfs.js?v=0.0.30-vfs-fullget1';
import { fetchWasmBinary } from './wasm-fetch.js';

const DOCS_ROOT = new URL('../', import.meta.url);
const PLAYGROUND = new URL('../playground/', import.meta.url);
const HEADER_BYTES = 160;
const CONCURRENCY = 32;

const REL_PROFILES = {
    wordnet: [
        ['hypernyms', /hypernym/i],
        ['hyponyms', /hyponym/i],
        ['synonyms', /synonym|synset_ref|equivalent/i],
        ['similar', /similar/i],
        ['lemmas', /lemma/i],
        ['glosses', /gloss|definition/i],
    ],
    schemaorg: [
        ['superClass', /subclassof|subClassOf/i],
        ['subClasses', /subclassof/i],
        ['domains', /domainincludes|domain/i],
        ['ranges', /rangeincludes|range/i],
        ['inverse', /inverseof/i],
        ['labels', /label|name/i],
        ['comments', /comment|description/i],
        ['superseded', /supersededby/i],
    ],
    w3c: [
        ['superClass', /subclassof|subClassOf/i],
        ['subClasses', /subclassof/i],
        ['domains', /domain/i],
        ['ranges', /range/i],
        ['inverse', /inverseof/i],
        ['labels', /label|preflabel|altlabel/i],
        ['comments', /comment|definition|description/i],
        ['definitions', /definition|skos:definition/i],
    ],
};

const CATEGORY_SAMPLES = {
    wordnet: {
        noun: ['dog', 'cat', 'water', 'computer', 'vehicle', 'food', 'animal', 'city', 'person'],
        verb: ['run', 'walk', 'think', 'speak', 'read', 'move', 'learn', 'write', 'play'],
        adjective: ['happy', 'beautiful', 'good', 'big', 'small', 'angry', 'sad', 'fast', 'old'],
        adverb: ['quickly', 'slowly', 'happily', 'carefully', 'loudly', 'quietly', 'well', 'often'],
    },
    schemaorg: {
        classes: ['Person', 'Organization', 'Event', 'Place', 'Product', 'CreativeWork', 'Thing'],
        properties: ['name', 'description', 'url', 'image', 'author', 'datePublished', 'location'],
        types: ['Intangible', 'StructuredValue', 'Action', 'MedicalEntity', 'Offer', 'Review'],
    },
    w3c: {
        classes: ['Class', 'Resource', 'Property', 'Ontology'],
        properties: ['label', 'comment', 'subClassOf', 'domain', 'range'],
        terms: ['Concept', 'Shape', 'Activity', 'Dataset', 'Sensor'],
    },
};

function getU64(view, off) {
    return BigInt(view.getUint32(off, true)) | (BigInt(view.getUint32(off + 4, true)) << 32n);
}

function esc(s) {
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function classifyPredicate(label, profile) {
    const rules = REL_PROFILES[profile] ?? REL_PROFILES.wordnet;
    const lower = label.toLowerCase();
    for (const [kind, re] of rules) {
        if (re.test(lower)) return kind;
    }
    return 'other';
}

function guessPos(iri, lemmas = []) {
    const text = `${iri} ${lemmas.join(' ')}`.toLowerCase();
    if (/-[nv]|_v_| verb/.test(text)) return 'verb';
    if (/-[na]| adj|adjective/.test(text)) return 'adjective';
    if (/-[nr]| adv|adverb/.test(text)) return 'adverb';
    if (/-n| noun/.test(text)) return 'noun';
    return 'type';
}

export function parseSparqlBgp(sparql) {
    const m = sparql.match(/WHERE\s*\{([^}]+)\}/is);
    if (!m) return null;
    const body = m[1]
        .replace(/OPTIONAL\s*\{[^}]*\}/gis, '')
        .replace(/FILTER\s*\([^)]*\)/gi, '')
        .replace(/GRAPH\s+\S+\s*\{/gi, '')
        .trim();
    const triple = body.split(/\s*\.\s*/).map(t => t.trim()).filter(Boolean)[0];
    return triple || null;
}

export class OntologyEngine {
    constructor() {
        this.vfs = null;
        this.execQuery = null;
        this.wasmReady = false;
        this.datasets = [];
        this.activeDataset = null;
        this.datasetLabel = 'Ontology';
        this._dbBytes = null;
        this.bootRequestedId = null;
        this.bootFallbackFrom = null;
    }

    playgroundUrl(path) {
        return new URL(path, PLAYGROUND).href;
    }

    docsUrl(path) {
        return new URL(path, DOCS_ROOT).href;
    }

    resolveManifestUrl(path) {
        if (!path) return null;
        if (path.startsWith('http://') || path.startsWith('https://')) return path;
        return this.docsUrl(path);
    }

    async initWasm(onProgress = null) {
        if (this.wasmReady) return;
        onProgress?.('Loading WASM module…', 5);
        const mod = await import(this.playgroundUrl('qualia_core_db.js'));
        onProgress?.('Fetching WASM binary…', 12);
        const wasmResp = await fetchWasmBinary(this.playgroundUrl('qualia_core_db_bg.wasm'));
        onProgress?.('Initialising WASM runtime…', 18);
        await mod.default(wasmResp);
        if (typeof mod.execute_ntriples_query !== 'function') {
            throw new Error('execute_ntriples_query missing from WASM build');
        }
        this.execQuery = mod.execute_ntriples_query;
        this.wasmReady = true;
    }

    async loadManifest() {
        const manifestResp = await fetch(this.playgroundUrl('vfs-manifest.json'));
        const manifest = manifestResp.ok ? await manifestResp.json() : { datasets: [] };
        this.datasets = manifest.datasets ?? [];
        return this.datasets;
    }

    datasetFromUrl() {
        const params = new URLSearchParams(location.search);
        return params.get('dataset') || null;
    }

    /**
     * Pages-hosted volumes we can boot without a 100+ MB download.
     * WordNet is listed in the manifest but is not published on GitHub Pages.
     */
    pickBootDataset(datasets, requested) {
        const list = Array.isArray(datasets) ? datasets : [];
        if (requested) {
            const hit = list.find(d => d.id === requested);
            if (hit) return hit.id;
        }
        const preferred = ['schemaorg-30', 'w3c-skos', 'w3c-rdfs', 'w3c-owl'];
        for (const id of preferred) {
            if (list.some(d => d.id === id && d.hosted !== false)) return id;
        }
        const hosted = list.find(d => d.hosted !== false && d.id !== 'wordnet');
        return hosted?.id ?? list.find(d => d.hosted !== false)?.id ?? list[0]?.id ?? null;
    }

    async init(datasetId = null, { onProgress } = {}) {
        await this.initWasm(onProgress);
        onProgress?.('Loading dataset manifest…', 22);
        await this.loadManifest();
        const requested = datasetId ?? this.datasetFromUrl();
        const id = this.pickBootDataset(this.datasets, requested);
        if (!id) throw new Error('No datasets in vfs-manifest.json');
        this.bootRequestedId = requested;
        this.bootFallbackFrom = null;
        try {
            return await this.mountDataset(id, { onProgress });
        } catch (err) {
            const fallback = this.pickBootDataset(
                this.datasets.filter(d => d.id !== id),
                null,
            );
            if (!fallback) throw err;
            this.bootFallbackFrom = id;
            onProgress?.(
                `${id} is not available here — mounting ${fallback} instead…`,
                26,
            );
            return this.mountDataset(fallback, { onProgress });
        }
    }

    _datasetVolumeUrls(entry) {
        const urls = [entry.url, ...(entry.fallbackUrls ?? [])]
            .map(u => this.resolveManifestUrl(u))
            .filter(Boolean);
        return [...new Set(urls)];
    }

    async mountDataset(datasetId, { onProgress } = {}) {
        await this.initWasm(onProgress);
        if (!this.datasets.length) await this.loadManifest();

        const entry = this.datasets.find(d => d.id === datasetId);
        if (!entry) throw new Error(`Unknown dataset ${datasetId}`);

        this.activeDataset = entry;
        this.datasetLabel = entry.label ?? datasetId;
        this._dbBytes = null;

        const volumeUrls = this._datasetVolumeUrls(entry);
        let lastErr = null;
        for (const url of volumeUrls) {
            try {
                onProgress?.(`Mounting ${entry.label ?? datasetId}…`, 28);
                this.vfs = new VFS(
                    url,
                    entry.lexUrl ? this.resolveManifestUrl(entry.lexUrl) : null,
                    entry.compressed ?? false,
                    entry.bidxUrl ? this.resolveManifestUrl(entry.bidxUrl) : null,
                );
                await this.vfs.init({
                    loadLex: true,
                    cacheKey: datasetId,
                    prefetchToOpfs: true,
                    onProgress: (msg, pct) => {
                        if (pct == null) onProgress?.(msg, null);
                        else onProgress?.(msg, 28 + (pct / 100) * 42);
                    },
                });
                onProgress?.('Dataset mounted', 70);
                return this.getStats();
            } catch (err) {
                lastErr = err;
                try { await this.vfs?.clearOpfsCache?.(); } catch (_) { /* ignore */ }
                this.vfs = null;
                console.warn(`[Ontology] Mount failed for ${url}:`, err.message);
            }
        }
        throw lastErr ?? new Error(`Failed to mount dataset ${datasetId}`);
    }

    get profile() {
        return this.activeDataset?.profile ?? 'wordnet';
    }

    labelFor(hashish) {
        if (!this.vfs) return toHex16(parseBigDecimal(String(hashish)));
        const h = parseBigDecimal(String(hashish));
        return this.vfs.lookup(h) || toHex16(h);
    }

    formatToken(value) {
        const v = String(value).trim();
        if (v.startsWith('<') && v.endsWith('>')) return v;
        if (v.startsWith('http://') || v.startsWith('https://')) return `<${v}>`;
        if (/^\d+$/.test(v)) return v;
        return `"${v.replace(/"/g, '\\"')}"`;
    }

    normalizeIri(term) {
        const t = term.trim();
        if (t.startsWith('<') && t.endsWith('>')) return t.slice(1, -1);
        if (t.startsWith('http://') || t.startsWith('https://')) return t;
        if (this.profile === 'schemaorg') {
            const local = t.replace(/^schema:/i, '');
            return `https://schema.org/${local}`;
        }
        if (this.profile === 'w3c') {
            const ns = this.activeDataset?.namespace;
            const px = this.activeDataset?.prefix;
            if (ns && px) {
                const local = t.replace(new RegExp(`^${px}:`, 'i'), '').replace(/^<|>$/g, '');
                if (!local.includes('/') && !local.startsWith('http')) {
                    return `${ns}${local}`;
                }
            }
        }
        return t;
    }

    async query(pattern, maxResults = 200, onProgress = null) {
        if (!this.vfs) throw new Error('Dataset not mounted');
        const normalized = pattern.trim();
        if (!normalized) {
            return { matches: [], vm_cycles: 0, direct_jump_ops: 0, lexicon_lookup_ops: 0 };
        }
        if (this._dbBytes?.length) return this._queryBuffer(normalized, this._dbBytes, maxResults);
        return this._streamingQuery(normalized, maxResults, onProgress);
    }

    async querySparql(sparql, maxResults = 200) {
        const bgp = parseSparqlBgp(sparql);
        if (!bgp) throw new Error('Could not extract a triple pattern from WHERE { }. Browser WASM scans one pattern; full SPARQL runs on qualia-cli daemon :4242/query.');
        const result = await this.query(bgp, maxResults);
        return { ...result, bgp };
    }

    async lookupEntity(term, onProgress = null) {
        const raw = term.trim();
        if (!raw) throw new Error('Empty search term');

        if (this.profile === 'schemaorg' || this.profile === 'w3c') {
            return this._lookupVocabulary(raw, onProgress);
        }
        return this._lookupWordNet(raw, onProgress);
    }

    async _lookupWordNet(word, onProgress = null) {
        const lemma = word.toLowerCase();
        const hits = await this.query(`?s ?p "${lemma}"`, 64, onProgress);
        if (!hits.matches.length) {
            return { term: lemma, found: false, entities: [], profile: 'wordnet' };
        }

        const entities = [];
        const seen = new Set();
        for (const hit of hits.matches) {
            if (seen.has(hit.s)) continue;
            seen.add(hit.s);
            entities.push(await this._expandEntity(hit.s, 'wordnet'));
        }
        return { term: lemma, found: true, entities, profile: 'wordnet' };
    }

    async _lookupVocabulary(term, onProgress = null) {
        const profile = this.profile;
        const iri = this.normalizeIri(term);
        let edges = await this.query(`<${iri}> ?p ?o`, 256, onProgress);

        if (!edges.matches.length) {
            const short = iri.split(/[/#]/).pop() ?? term;
            edges = await this.query(`?s ?p "${short}"`, 64, onProgress);
            if (!edges.matches.length) {
                edges = await this.query(`?s <http://www.w3.org/2000/01/rdf-schema#label> "${short}"`, 64, onProgress);
            }
            if (!edges.matches.length) {
                return { term, found: false, entities: [], profile };
            }
            const entities = [];
            const seen = new Set();
            for (const hit of edges.matches) {
                if (seen.has(hit.s)) continue;
                seen.add(hit.s);
                entities.push(await this._expandEntity(hit.s, profile));
            }
            return { term, found: true, entities, profile };
        }

        const entity = await this._expandFromEdges(iri, edges.matches, profile);
        return { term, found: true, entities: [entity], profile };
    }

    async _expandEntity(subjectHash, profile) {
        const subjectIri = this.labelFor(subjectHash);
        const token = subjectIri.startsWith('http') ? `<${subjectIri}>` : this.formatToken(subjectHash);
        const edges = await this.query(`${token} ?p ?o`, 256);
        return this._expandFromEdges(subjectIri, edges.matches, profile);
    }

    _expandFromEdges(iri, matches, profile) {
        const relations = { other: [] };
        const rules = REL_PROFILES[profile] ?? REL_PROFILES.wordnet;
        for (const [, kind] of rules) relations[kind] = [];

        for (const edge of matches) {
            const pred = this.labelFor(edge.p);
            const obj = this.labelFor(edge.o);
            const kind = classifyPredicate(pred, profile);
            const bucket = relations[kind] ?? relations.other;
            if (!bucket.includes(obj)) bucket.push(obj);
        }

        const summary = relations.glosses?.[0]
            ?? relations.comments?.[0]
            ?? relations.definitions?.[0]
            ?? relations.labels?.[0]
            ?? relations.other.find(t => t.length > 8 && !t.startsWith('http'))
            ?? '';

        const posTag = profile === 'wordnet'
            ? guessPos(iri, relations.lemmas ?? [])
            : (profile === 'w3c' ? 'w3c:term' : 'schema:type');

        return {
            iri,
            pos: posTag,
            gloss: summary,
            relations,
            edgeCount: matches.length,
        };
    }

    async hierarchyDepth(term, maxDepth = 8) {
        const lookup = await this.lookupEntity(term);
        if (!lookup.found || !lookup.entities.length) return 0;

        const relKey = (this.profile === 'schemaorg' || this.profile === 'w3c') ? 'superClass' : 'hypernyms';
        let depth = 0;
        let frontier = lookup.entities[0].relations[relKey]?.slice(0, 4) ?? [];
        const visited = new Set();

        while (frontier.length && depth < maxDepth) {
            depth++;
            const next = [];
            for (const iri of frontier) {
                if (visited.has(iri)) continue;
                visited.add(iri);
                const token = iri.startsWith('http') ? `<${iri}>` : `"${iri.replace(/"/g, '\\"')}"`;
                const edges = await this.query(`${token} ?p ?o`, 48);
                const predRe = (this.profile === 'schemaorg' || this.profile === 'w3c') ? /subclassof/i : /hypernym/i;
                for (const edge of edges.matches) {
                    if (!predRe.test(this.labelFor(edge.p))) continue;
                    const obj = this.labelFor(edge.o);
                    if (!visited.has(obj)) next.push(obj);
                }
            }
            frontier = next;
        }
        return depth;
    }

    getStats() {
        if (!this.vfs) {
            return { terms: 0, entities: 0, relations: 0, depth: 0, triples: 0, blocks: 0 };
        }
        const header = this.vfs.volumeHeader;
        const blocks = header?.blockCount ?? this.vfs.blockCount ?? 0;
        const triples = header
            ? header.blockCount * (header.quinsPerBlock || 850)
            : blocks * 850;
        const terms = this.vfs._lexMap?.size ?? 0;
        const relCount = (REL_PROFILES[this.profile] ?? REL_PROFILES.wordnet).length;
        const entities = Math.max(1, Math.round(triples / (this.profile === 'schemaorg' ? 3 : 6)));
        const stats = {
            terms,
            entities,
            relations: relCount,
            depth: '—',
            triples,
            blocks,
            label: this.datasetLabel,
            profile: this.profile,
            wasmReady: this.wasmReady,
            datasetId: this.activeDataset?.id ?? '',
        };
        if (this.profile === 'wordnet') {
            stats.words = terms;
            stats.synsets = entities;
        }
        return stats;
    }

    getCategorySamples(category) {
        const ds = this.activeDataset;
        if (ds?.categorySamples?.[category]) return ds.categorySamples[category];
        if (category === 'terms' && ds?.quickExamples?.length) return ds.quickExamples;
        const profile = this.profile;
        return CATEGORY_SAMPLES[profile]?.[category] ?? [];
    }

    getSampleQueries() {
        return this.activeDataset?.sampleQueries ?? [];
    }

    _queryBuffer(pattern, bytes, maxResults) {
        if (this.wasmReady && this.execQuery) {
            return JSON.parse(this.execQuery(pattern, bytes, maxResults));
        }
        return this._jsFallbackQuery(pattern, bytes, maxResults);
    }

    async _streamingQuery(pattern, maxResults, onProgress = null) {
        const vfs = this.vfs;
        const tokens = pattern.trim().split(/\s+/).filter(t => t !== '.');
        if (tokens.length < 3) {
            return { matches: [], vm_cycles: 0, direct_jump_ops: 0, lexicon_lookup_ops: 0 };
        }
        const [sT, pT, oT] = tokens;
        const sH = sT.startsWith('?') ? null : hashToken(sT);
        const pH = pT.startsWith('?') ? null : hashToken(pT);
        const oH = oT.startsWith('?') ? null : hashToken(oT);

        let candidateBlocks = null;
        if (oH !== null) candidateBlocks = vfs.lookupBlocks(oH);
        const blockList = candidateBlocks ?? Array.from({ length: vfs.blockCount }, (_, i) => i);

        const matches = [];
        let cycles = 0, dj = 0, lx = 0;
        const totalBlocks = blockList.length;
        if (onProgress && totalBlocks > 1) {
            onProgress('Searching graph…', 0);
        }

        for (let base = 0; base < blockList.length && matches.length < maxResults; base += CONCURRENCY) {
            if (onProgress && totalBlocks > 1) {
                onProgress('Searching graph…', Math.min(99, Math.round((base / totalBlocks) * 100)));
            }
            const slice = blockList.slice(base, base + CONCURRENCY);
            const blocks = await Promise.all(slice.map(bi => vfs.readBlock(bi).catch(() => null)));

            for (const blockBytes of blocks) {
                if (!blockBytes || matches.length >= maxResults) break;
                const view = new DataView(blockBytes.buffer, blockBytes.byteOffset);
                const live = Number(getU64(view, 16));
                const quinSlots = Math.min(
                    live > 0 ? live : Math.floor((blockBytes.length - HEADER_BYTES) / QUIN_SIZE),
                    Math.floor((blockBytes.length - HEADER_BYTES) / QUIN_SIZE),
                );

                for (let qi = 0; qi < quinSlots && matches.length < maxResults; qi++) {
                    const b = HEADER_BYTES + qi * QUIN_SIZE;
                    const s = getU64(view, b);
                    const p = getU64(view, b + 8);
                    const o = getU64(view, b + 16);
                    if (s === 0n && p === 0n && o === 0n) continue;

                    let ok = true;
                    if (sH !== null) { cycles++; hasMsb(sH) ? dj++ : lx++; if (s !== sH) ok = false; }
                    if (ok && pH !== null) { cycles++; hasMsb(pH) ? dj++ : lx++; if (p !== pH) ok = false; }
                    if (ok && oH !== null) { cycles++; hasMsb(oH) ? dj++ : lx++; if (o !== oH) ok = false; }
                    if (ok) {
                        matches.push({
                            s: String(s), p: String(p), o: String(o),
                            c: String(getU64(view, b + 24)),
                            m: String(getU64(view, b + 32)),
                        });
                    }
                }
            }
        }

        if (onProgress && totalBlocks > 1) {
            onProgress('Search complete', 100);
        }

        return { matches, vm_cycles: cycles, direct_jump_ops: dj, lexicon_lookup_ops: lx };
    }

    _jsFallbackQuery(pattern, bytes, maxResults) {
        const tokens = pattern.trim().split(/\s+/).filter(t => t !== '.');
        if (tokens.length < 3) {
            return { matches: [], vm_cycles: 0, direct_jump_ops: 0, lexicon_lookup_ops: 0 };
        }
        const [sT, pT, oT] = tokens;
        const sH = sT.startsWith('?') ? null : hashToken(sT);
        const pH = pT.startsWith('?') ? null : hashToken(pT);
        const oH = oT.startsWith('?') ? null : hashToken(oT);

        const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        const quins = Math.floor(bytes.length / QUIN_SIZE);
        const matches = [];
        let cycles = 0, dj = 0, lx = 0;

        for (let i = 0; i < quins && matches.length < maxResults; i++) {
            const b = i * QUIN_SIZE;
            const s = getU64(view, b);
            const p = getU64(view, b + 8);
            const o = getU64(view, b + 16);
            let ok = true;
            if (sH !== null) { cycles++; hasMsb(sH) ? dj++ : lx++; if (s !== sH) ok = false; }
            if (ok && pH !== null) { cycles++; hasMsb(pH) ? dj++ : lx++; if (p !== pH) ok = false; }
            if (ok && oH !== null) { cycles++; hasMsb(oH) ? dj++ : lx++; if (o !== oH) ok = false; }
            if (ok) {
                matches.push({ s: String(s), p: String(p), o: String(o), c: '0', m: '0' });
            }
        }
        return { matches, vm_cycles: cycles, direct_jump_ops: dj, lexicon_lookup_ops: lx };
    }
}

/** @deprecated Use OntologyEngine */
export class WordNetEngine extends OntologyEngine {
    async init(datasetId = 'wordnet') {
        return super.init(datasetId);
    }
    async lookupWord(word) {
        const r = await this.lookupEntity(word);
        return { word: r.term, found: r.found, synsets: r.entities };
    }
    async hypernymDepth(word, maxDepth = 8) {
        return this.hierarchyDepth(word, maxDepth);
    }
}

export { esc, CATEGORY_SAMPLES };