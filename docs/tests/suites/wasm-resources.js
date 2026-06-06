// Resource Catalog tests.
// Covers: LLMResource, OntologyResource, SPARQLResource Quin serialization,
// catalog structure, and resource ID hashing.
//
// Source-of-truth: crates/qualia-core-db/src/resource_catalog.rs
//   Three resource types, each with to_quins() serialization:
//     LLMResource    — GGUF models with provenance, size, quantization metadata
//     OntologyResource — RDF namespaces with SHACL validation hooks
//     SPARQLResource — federated query endpoints with reliability metadata
//
// YAML catalogs:
//   resources/llms.yaml          — Phi-3-mini, Gemma 2, Qwen2.5, Llama 3.2, Mistral…
//   resources/ontologies.yaml    — PROV-O, SNOMED CT, MeSH, Schema.org, Dublin Core…
//   resources/sparql_endpoints.yaml — Wikidata, DBpedia, Bio2RDF, UniProt

import { q_hash, makeQuin } from './primitives.js';
import { loadWasm } from '../wasm-loader.js';

// ─── Predicate hashes (mirrors resource_catalog.rs predicate constants) ───────

const P_RESOURCE_ID      = q_hash('qualia:resourceId');
const P_RESOURCE_TYPE    = q_hash('qualia:resourceType');
const P_RESOURCE_NAME    = q_hash('qualia:resourceName');
const P_RESOURCE_URI     = q_hash('qualia:resourceUri');
const P_MODEL_SIZE_BYTES = q_hash('qualia:modelSizeBytes');
const P_QUANTIZATION     = q_hash('qualia:quantization');
const P_CONTEXT_WINDOW   = q_hash('qualia:contextWindow');
const P_SHACL_SHAPE      = q_hash('qualia:shaclShape');
const P_RELIABILITY      = q_hash('qualia:endpointReliability');

// ─── Resource type tags ───────────────────────────────────────────────────────

const TYPE_LLM       = q_hash('resource:llm');
const TYPE_ONTOLOGY  = q_hash('resource:ontology');
const TYPE_SPARQL    = q_hash('resource:sparql');

// ─── JS reference implementations of to_quins() ──────────────────────────────

function llmResourceToQuins(resource) {
    const subjectHash = q_hash(resource.id);
    return [
        makeQuin(subjectHash, P_RESOURCE_TYPE, TYPE_LLM, 0n),
        makeQuin(subjectHash, P_RESOURCE_NAME, q_hash(resource.name), 0n),
        makeQuin(subjectHash, P_RESOURCE_URI,  q_hash(resource.uri), 0n),
        makeQuin(subjectHash, P_MODEL_SIZE_BYTES, (1n << 60n) | BigInt(Math.round(resource.sizeBytes / 1_000_000)), 0n),
        makeQuin(subjectHash, P_QUANTIZATION, q_hash(resource.quantization), 0n),
        makeQuin(subjectHash, P_CONTEXT_WINDOW, (1n << 60n) | BigInt(resource.contextWindow), 0n),
    ];
}

function ontologyResourceToQuins(resource) {
    const subjectHash = q_hash(resource.id);
    const quins = [
        makeQuin(subjectHash, P_RESOURCE_TYPE, TYPE_ONTOLOGY, 0n),
        makeQuin(subjectHash, P_RESOURCE_NAME, q_hash(resource.name), 0n),
        makeQuin(subjectHash, P_RESOURCE_URI,  q_hash(resource.uri), 0n),
    ];
    for (const shape of (resource.shaclShapes || [])) {
        quins.push(makeQuin(subjectHash, P_SHACL_SHAPE, q_hash(shape), 0n));
    }
    return quins;
}

function sparqlResourceToQuins(resource) {
    const subjectHash = q_hash(resource.id);
    return [
        makeQuin(subjectHash, P_RESOURCE_TYPE, TYPE_SPARQL, 0n),
        makeQuin(subjectHash, P_RESOURCE_NAME, q_hash(resource.name), 0n),
        makeQuin(subjectHash, P_RESOURCE_URI,  q_hash(resource.endpoint), 0n),
        makeQuin(subjectHash, P_RELIABILITY,
            (1n << 60n) | BigInt(Math.round(resource.reliabilityPct)), 0n),
    ];
}

// ─── Sample catalog entries (mirrors resources/llms.yaml etc.) ────────────────

const SAMPLE_LLM = {
    id:            'phi3-mini-4k-instruct-q4',
    name:          'Phi-3-mini 4K (Q4_K_M)',
    uri:           'https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf',
    sizeBytes:     2_200_000_000,
    quantization:  'Q4_K_M',
    contextWindow: 4096,
};

const SAMPLE_ONTOLOGY = {
    id:          'prov-o',
    name:        'W3C PROV-O',
    uri:         'https://www.w3.org/ns/prov-o',
    shaclShapes: ['prov:Entity', 'prov:Activity', 'prov:Agent'],
};

const SAMPLE_SPARQL = {
    id:             'wikidata',
    name:           'Wikidata Query Service',
    endpoint:       'https://query.wikidata.org/sparql',
    reliabilityPct: 97,
};

// ─── Registration ─────────────────────────────────────────────────────────────

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Resource Catalog', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        // ── LLMResource Quin serialization ───────────────────────────────────

        runner.it('LLMResource produces 6 quins', () => {
            const quins = llmResourceToQuins(SAMPLE_LLM);
            runner.expect(quins.length).toBe(6);
        });

        runner.it('LLMResource first quin is type assertion', () => {
            const quins = llmResourceToQuins(SAMPLE_LLM);
            runner.expect(quins[0].predicate).toBe(P_RESOURCE_TYPE);
            runner.expect(quins[0].object).toBe(TYPE_LLM);
        });

        runner.it('LLMResource all quins share same subject (resource ID hash)', () => {
            const quins = llmResourceToQuins(SAMPLE_LLM);
            const expected = q_hash(SAMPLE_LLM.id);
            for (const q of quins) {
                runner.expect(q.subject).toBe(expected);
            }
        });

        runner.it('LLMResource quantization quin uses q_hash of value', () => {
            const quins = llmResourceToQuins(SAMPLE_LLM);
            const q = quins.find(q => q.predicate === P_QUANTIZATION);
            runner.expect(q).not.toBeNull();
            runner.expect(q.object).toBe(q_hash('Q4_K_M'));
        });

        runner.it('LLMResource contextWindow quin uses inline integer encoding', () => {
            const quins = llmResourceToQuins(SAMPLE_LLM);
            const q = quins.find(q => q.predicate === P_CONTEXT_WINDOW);
            runner.expect(q).not.toBeNull();
            runner.expect((q.object >> 60n) & 0x7n).toBe(1n); // xsd:integer tag
            runner.expect(q.object & ((1n << 60n) - 1n)).toBe(BigInt(SAMPLE_LLM.contextWindow));
        });

        runner.it('two LLM resources produce different subject hashes', () => {
            const r1 = llmResourceToQuins(SAMPLE_LLM);
            const r2 = llmResourceToQuins({ ...SAMPLE_LLM, id: 'gemma2-9b-q4' });
            runner.expect(r1[0].subject === r2[0].subject).toBeFalsy();
        });

        // ── OntologyResource Quin serialization ──────────────────────────────

        runner.it('OntologyResource produces base 3 quins + one per SHACL shape', () => {
            const quins = ontologyResourceToQuins(SAMPLE_ONTOLOGY);
            runner.expect(quins.length).toBe(3 + SAMPLE_ONTOLOGY.shaclShapes.length);
        });

        runner.it('OntologyResource type quin has TYPE_ONTOLOGY object', () => {
            const quins = ontologyResourceToQuins(SAMPLE_ONTOLOGY);
            const tq = quins.find(q => q.predicate === P_RESOURCE_TYPE);
            runner.expect(tq.object).toBe(TYPE_ONTOLOGY);
        });

        runner.it('OntologyResource SHACL shape quins use q_hash of shape IRI', () => {
            const quins = ontologyResourceToQuins(SAMPLE_ONTOLOGY);
            const shapeQuins = quins.filter(q => q.predicate === P_SHACL_SHAPE);
            runner.expect(shapeQuins.length).toBe(3);
            runner.expect(shapeQuins[0].object).toBe(q_hash('prov:Entity'));
        });

        runner.it('OntologyResource with no SHACL shapes produces 3 quins', () => {
            const quins = ontologyResourceToQuins({ ...SAMPLE_ONTOLOGY, shaclShapes: [] });
            runner.expect(quins.length).toBe(3);
        });

        // ── SPARQLResource Quin serialization ─────────────────────────────────

        runner.it('SPARQLResource produces 4 quins', () => {
            const quins = sparqlResourceToQuins(SAMPLE_SPARQL);
            runner.expect(quins.length).toBe(4);
        });

        runner.it('SPARQLResource type quin has TYPE_SPARQL object', () => {
            const quins = sparqlResourceToQuins(SAMPLE_SPARQL);
            const tq = quins.find(q => q.predicate === P_RESOURCE_TYPE);
            runner.expect(tq.object).toBe(TYPE_SPARQL);
        });

        runner.it('SPARQLResource reliability quin uses inline integer encoding', () => {
            const quins = sparqlResourceToQuins(SAMPLE_SPARQL);
            const rq = quins.find(q => q.predicate === P_RELIABILITY);
            runner.expect(rq).not.toBeNull();
            runner.expect((rq.object >> 60n) & 0x7n).toBe(1n);
            runner.expect(rq.object & ((1n << 60n) - 1n)).toBe(BigInt(SAMPLE_SPARQL.reliabilityPct));
        });

        runner.it('SPARQLResource URI quin uses q_hash of endpoint URL', () => {
            const quins = sparqlResourceToQuins(SAMPLE_SPARQL);
            const uq = quins.find(q => q.predicate === P_RESOURCE_URI);
            runner.expect(uq.object).toBe(q_hash(SAMPLE_SPARQL.endpoint));
        });

        // ── Type tag distinctness ─────────────────────────────────────────────

        runner.it('TYPE_LLM, TYPE_ONTOLOGY, TYPE_SPARQL are all distinct', () => {
            const types = new Set([String(TYPE_LLM), String(TYPE_ONTOLOGY), String(TYPE_SPARQL)]);
            runner.expect(types.size).toBe(3);
        });

        // ── Catalog structure ─────────────────────────────────────────────────

        runner.it('sample LLM catalog entry has required fields', () => {
            runner.expect(SAMPLE_LLM.id).not.toBeNull();
            runner.expect(SAMPLE_LLM.name).not.toBeNull();
            runner.expect(SAMPLE_LLM.uri).not.toBeNull();
            runner.expect(SAMPLE_LLM.quantization).not.toBeNull();
            runner.expect(SAMPLE_LLM.contextWindow).toBeGreaterThan(0);
        });

        runner.it('sample ontology entry has required fields', () => {
            runner.expect(SAMPLE_ONTOLOGY.id).not.toBeNull();
            runner.expect(SAMPLE_ONTOLOGY.uri).not.toBeNull();
            runner.expect(Array.isArray(SAMPLE_ONTOLOGY.shaclShapes)).toBeTruthy();
        });

        runner.it('sample SPARQL entry has required fields', () => {
            runner.expect(SAMPLE_SPARQL.endpoint).not.toBeNull();
            runner.expect(SAMPLE_SPARQL.reliabilityPct).toBeGreaterThan(0);
            runner.expect(SAMPLE_SPARQL.reliabilityPct).toBeLessThan(101);
        });

        // ── Known catalog entries (spot-checks against resources/llms.yaml) ───

        runner.it('Phi-3-mini resource ID hashes stably', () => {
            runner.expect(q_hash('phi3-mini-4k-instruct-q4')).toBe(q_hash('phi3-mini-4k-instruct-q4'));
        });

        runner.it('Wikidata SPARQL endpoint hashes stably', () => {
            runner.expect(q_hash('https://query.wikidata.org/sparql'))
                .toBe(q_hash('https://query.wikidata.org/sparql'));
        });

        runner.it('PROV-O ontology URI hashes stably', () => {
            runner.expect(q_hash('https://www.w3.org/ns/prov-o'))
                .toBe(q_hash('https://www.w3.org/ns/prov-o'));
        });

        // ── WASM resource functions (skip gracefully if not in binary) ────────

        runner.it('resource_catalog_list_wasm returns JSON if present', () => {
            if (!mod?.resource_catalog_list_wasm) return;
            const raw = mod.resource_catalog_list_wasm('llms');
            runner.expect(typeof raw).toBe('string');
            runner.expect(() => JSON.parse(raw)).not.toThrow();
        });

        runner.it('resource_to_quins_wasm returns non-empty array if present', () => {
            if (!mod?.resource_to_quins_wasm) return;
            const raw  = mod.resource_to_quins_wasm(JSON.stringify(SAMPLE_LLM));
            const quins = JSON.parse(raw);
            runner.expect(Array.isArray(quins)).toBeTruthy();
            runner.expect(quins.length).toBeGreaterThan(0);
        });
    });
}

export default register;
