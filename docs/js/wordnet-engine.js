/**
 * WordNet browser engine — WASM + VFS demand-paging over playground/wordnet.q42.
 * Shared by docs/wordnet.html (paths resolve from docs/js/ → docs/playground/).
 */

import { parseBigDecimal, hashToken, toHex16, hasMsb } from '../playground/hash.js';
import { VFS, QUIN_SIZE } from '../playground/vfs.js';

const PLAYGROUND = new URL('../playground/', import.meta.url);
const HEADER_BYTES = 160;
const CONCURRENCY = 32;

const REL_RULES = [
    ['hypernyms', /hypernym/i],
    ['hyponyms', /hyponym/i],
    ['synonyms', /synonym|synset_ref|equivalent/i],
    ['similar', /similar/i],
    ['lemmas', /lemma/i],
    ['glosses', /gloss|definition/i],
];

const CATEGORY_SAMPLES = {
    noun: ['dog', 'cat', 'water', 'computer', 'vehicle', 'food', 'animal', 'city', 'person'],
    verb: ['run', 'walk', 'think', 'speak', 'read', 'move', 'learn', 'write', 'play'],
    adjective: ['happy', 'beautiful', 'good', 'big', 'small', 'angry', 'sad', 'fast', 'old'],
    adverb: ['quickly', 'slowly', 'happily', 'carefully', 'loudly', 'quietly', 'well', 'often'],
};

function getU64(view, off) {
    return BigInt(view.getUint32(off, true)) | (BigInt(view.getUint32(off + 4, true)) << 32n);
}

function esc(s) {
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function classifyPredicate(label) {
    const lower = label.toLowerCase();
    for (const [kind, re] of REL_RULES) {
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
    return 'unknown';
}

/** Extract a single BGP triple pattern from a SPARQL query string. */
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

export class WordNetEngine {
    constructor() {
        this.vfs = null;
        this.execQuery = null;
        this.wasmReady = false;
        this.datasetLabel = 'Open English WordNet';
        this._dbBytes = null;
    }

    playgroundUrl(path) {
        return new URL(path, PLAYGROUND).href;
    }

    async init() {
        const wasmUrl = this.playgroundUrl('qualia_core_db.js');
        const mod = await import(wasmUrl);
        const wasmResp = await fetch(this.playgroundUrl('qualia_core_db_bg.wasm'));
        if (!wasmResp.ok) {
            throw new Error(`WASM fetch failed: ${wasmResp.status}`);
        }
        await mod.default(wasmResp);
        if (typeof mod.execute_ntriples_query !== 'function') {
            throw new Error('execute_ntriples_query missing from WASM build');
        }
        this.execQuery = mod.execute_ntriples_query;
        this.wasmReady = true;

        const manifestResp = await fetch(this.playgroundUrl('vfs-manifest.json'));
        const manifest = manifestResp.ok ? await manifestResp.json() : { datasets: [] };
        const entry = manifest.datasets?.find(d => d.id === 'wordnet') ?? manifest.datasets?.[0];
        if (!entry) {
            throw new Error('wordnet dataset missing from vfs-manifest.json');
        }
        this.datasetLabel = entry.label ?? 'WordNet';

        this.vfs = new VFS(
            this.playgroundUrl(entry.url),
            entry.lexUrl ? this.playgroundUrl(entry.lexUrl) : null,
            entry.compressed ?? false,
            entry.bidxUrl ? this.playgroundUrl(entry.bidxUrl) : null,
        );
        await this.vfs.init({ loadLex: true });
        return this.getStats();
    }

    labelFor(hashish) {
        if (!this.vfs) return toHex16(parseBigDecimal(String(hashish)));
        const h = parseBigDecimal(String(hashish));
        const s = this.vfs.lookup(h);
        return s || toHex16(h);
    }

    formatToken(hashish) {
        const label = this.labelFor(hashish);
        if (label.startsWith('http://') || label.startsWith('https://')) {
            return `<${label}>`;
        }
        if (/^\d+$/.test(label)) return label;
        return `"${label.replace(/"/g, '\\"')}"`;
    }

    async query(pattern, maxResults = 200) {
        if (!this.vfs) throw new Error('Dataset not mounted');
        const normalized = pattern.trim();
        if (!normalized) return { matches: [], vm_cycles: 0, direct_jump_ops: 0, lexicon_lookup_ops: 0 };

        if (this._dbBytes?.length) {
            return this._queryBuffer(normalized, this._dbBytes, maxResults);
        }

        return this._streamingQuery(normalized, maxResults);
    }

    async querySparql(sparql, maxResults = 200) {
        const bgp = parseSparqlBgp(sparql);
        if (!bgp) throw new Error('Could not parse a single BGP triple from WHERE { }');
        const result = await this.query(bgp, maxResults);
        return { ...result, bgp };
    }

    async lookupWord(word) {
        const lemma = word.toLowerCase().trim();
        if (!lemma) throw new Error('Empty search term');

        const hits = await this.query(`?s ?p "${lemma}"`, 64);
        if (!hits.matches.length) {
            return { word: lemma, found: false, synsets: [] };
        }

        const seen = new Set();
        const synsets = [];

        for (const hit of hits.matches) {
            const subjectKey = hit.s;
            if (seen.has(subjectKey)) continue;
            seen.add(subjectKey);

            const subjectIri = this.labelFor(hit.s);
            const subjectToken = subjectIri.startsWith('http') ? `<${subjectIri}>` : this.formatToken(hit.s);
            const edges = await this.query(`${subjectToken} ?p ?o`, 256);

            const relations = {
                hypernyms: [], hyponyms: [], synonyms: [], similar: [],
                lemmas: [], glosses: [], other: [],
            };

            for (const edge of edges.matches) {
                const pred = this.labelFor(edge.p);
                const obj = this.labelFor(edge.o);
                const kind = classifyPredicate(pred);
                const bucket = relations[kind] ?? relations.other;
                if (!bucket.includes(obj)) bucket.push(obj);
            }

            const gloss = relations.glosses[0]
                ?? relations.other.find(t => t.length > 12 && !t.startsWith('http'))
                ?? '';

            synsets.push({
                iri: subjectIri,
                pos: guessPos(subjectIri, relations.lemmas),
                gloss,
                relations,
                edgeCount: edges.matches.length,
            });
        }

        return { word: lemma, found: true, synsets };
    }

    async hypernymDepth(word, maxDepth = 8) {
        const lookup = await this.lookupWord(word);
        if (!lookup.found || !lookup.synsets.length) return 0;

        let depth = 0;
        let frontier = lookup.synsets[0].relations.hypernyms.slice(0, 4);
        const visited = new Set();

        while (frontier.length && depth < maxDepth) {
            depth++;
            const next = [];
            for (const iri of frontier) {
                if (visited.has(iri)) continue;
                visited.add(iri);
                const token = iri.startsWith('http') ? `<${iri}>` : `"${iri.replace(/"/g, '\\"')}"`;
                const edges = await this.query(`${token} ?p ?o`, 48);
                for (const edge of edges.matches) {
                    if (!/hypernym/i.test(this.labelFor(edge.p))) continue;
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
            return { words: 0, synsets: 0, relations: 0, depth: 0, triples: 0, blocks: 0 };
        }
        const blocks = this.vfs.blockCount ?? 0;
        const triples = blocks * 850;
        const words = this.vfs._lexMap?.size ?? 0;
        return {
            words,
            synsets: Math.max(1, Math.round(triples / 6)),
            relations: REL_RULES.length,
            depth: '—',
            triples,
            blocks,
            label: this.datasetLabel,
            wasmReady: this.wasmReady,
        };
    }

    getCategorySamples(category) {
        return CATEGORY_SAMPLES[category] ?? [];
    }

    _queryBuffer(pattern, bytes, maxResults) {
        if (this.wasmReady && this.execQuery) {
            return JSON.parse(this.execQuery(pattern, bytes, maxResults));
        }
        return this._jsFallbackQuery(pattern, bytes, maxResults);
    }

    async _streamingQuery(pattern, maxResults) {
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

        for (let base = 0; base < blockList.length && matches.length < maxResults; base += CONCURRENCY) {
            const slice = blockList.slice(base, base + CONCURRENCY);
            const blocks = await Promise.all(slice.map(bi => vfs.readBlock(bi).catch(() => null)));

            for (const blockBytes of blocks) {
                if (!blockBytes || matches.length >= maxResults) break;
                const view = new DataView(blockBytes.buffer, blockBytes.byteOffset);
                const quinSlots = Math.floor((blockBytes.length - HEADER_BYTES) / QUIN_SIZE);

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
                matches.push({
                    s: String(s), p: String(p), o: String(o),
                    c: '0', m: '0',
                });
            }
        }
        return { matches, vm_cycles: cycles, direct_jump_ops: dj, lexicon_lookup_ops: lx };
    }
}

export { esc, CATEGORY_SAMPLES };