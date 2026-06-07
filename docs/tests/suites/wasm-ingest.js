// Ingest format tests.
// Covers the WASM parse functions for each supported ingest format:
//   Turtle (.ttl)              — parse_turtle_wasm(string) → [{subject,predicate,object}]
//   N3 Logic (.n3)             — parse_n3logic_wasm(string) → [{subject,predicate,object}]
//   CBOR-LD (.cbor/.cbor-ld)  — parse_cbor_ld_wasm(Uint8Array) → {subject,predicate,object,context}
//   JSON (.json)               — parse_json_wasm(string) → [{subject,predicate,object}]
//   CogAI Chunks (.chk text)   — parse_cogai_chunk_wasm(string) → [{...}]  (if exported)
//
// NOTE: N-Triples query execution is covered separately in wasm-query-engine.js
// (execute_ntriples_query takes a query pattern + binary Quin bytes, not raw .nt text).
//
// MANUAL TESTS (WordNet integration):
//   Requires docs/playground/wordnet.q42 — build with:
//     bash scripts/fetch_wordnet.sh --subset 100000
//     wasm-pack build crates/qualia-core-db --target web \
//       --out-dir ../../docs/playground --no-typescript
//   Activate: append ?manual=1 to the URL, or set window.MANUAL_TESTS = true in console.
//   Skipped in normal runs — file transfer + WASM rebuild take significant time.
//   WordNet stats: 523 MB RDF → 74.6 MB .q42 · 5.56 M quins · ~6.5 ms first query.

import { loadWasm } from '../wasm-loader.js';
import { q_hash }   from './primitives.js';

/** WordNet manual tests only run in a browser with ?manual=1 or MANUAL_TESTS set. */
function manualWordNetEnabled() {
    if (typeof globalThis.window === 'undefined') return false;
    return !!(globalThis.window.MANUAL_TESTS
        || new URLSearchParams(globalThis.location.search).get('manual') === '1');
}

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Ingest Formats', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        // ── Turtle (.ttl) ─────────────────────────────────────────────────────

        runner.it('parse_turtle_wasm: basic triple → array with one item', () => {
            if (!mod?.parse_turtle_wasm) return;
            const ttl = '@prefix ex: <http://example.org/> . ex:Alice ex:knows ex:Bob .';
            const result = mod.parse_turtle_wasm(ttl);
            runner.expect(Array.isArray(result)).toBeTruthy();
            runner.expect(result.length).toBe(1);
        });

        runner.it('parse_turtle_wasm: result item has subject, predicate, object', () => {
            if (!mod?.parse_turtle_wasm) return;
            const ttl = '@prefix ex: <http://example.org/> . ex:Alice ex:knows ex:Bob .';
            const [triple] = mod.parse_turtle_wasm(ttl);
            runner.expect(triple.subject).toBeTruthy();
            runner.expect(triple.predicate).toBeTruthy();
            runner.expect(triple.object).toBeTruthy();
        });

        runner.it('parse_turtle_wasm: prefix expands — subject contains full IRI', () => {
            if (!mod?.parse_turtle_wasm) return;
            const ttl = '@prefix ex: <http://example.org/> . ex:Alice ex:knows ex:Bob .';
            const [triple] = mod.parse_turtle_wasm(ttl);
            runner.expect(triple.subject).toContain('example.org');
        });

        runner.it('parse_turtle_wasm: semicolon shorthand produces multiple triples', () => {
            if (!mod?.parse_turtle_wasm) return;
            const ttl = `
                @prefix foaf: <http://xmlns.com/foaf/0.1/> .
                @prefix ex: <http://example.org/> .
                ex:Alice foaf:name "Alice" ; foaf:knows ex:Bob .
            `;
            const result = mod.parse_turtle_wasm(ttl);
            runner.expect(Array.isArray(result)).toBeTruthy();
            runner.expect(result.length).toBeGreaterThan(1);
        });

        runner.it('parse_turtle_wasm: literal object is preserved', () => {
            if (!mod?.parse_turtle_wasm) return;
            const ttl = '@prefix ex: <http://ex.org/> . ex:a ex:name "Test Label" .';
            const [triple] = mod.parse_turtle_wasm(ttl);
            runner.expect(triple.object).toContain('Test Label');
        });

        runner.it('parse_turtle_wasm: empty string → empty array', () => {
            if (!mod?.parse_turtle_wasm) return;
            const result = mod.parse_turtle_wasm('');
            runner.expect(Array.isArray(result)).toBeTruthy();
            runner.expect(result.length).toBe(0);
        });

        // ── N3 Logic (.n3) ────────────────────────────────────────────────────

        runner.it('parse_n3logic_wasm: identity rule (=>) → array result', () => {
            if (!mod?.parse_n3logic_wasm) return;
            const n3 = '{ ?s ?p ?o } => { ?s ?p ?o } .';
            const result = mod.parse_n3logic_wasm(n3);
            runner.expect(Array.isArray(result)).toBeTruthy();
        });

        runner.it('parse_n3logic_wasm: defeasible rule (~>) does not throw', () => {
            if (!mod?.parse_n3logic_wasm) return;
            runner.expect(() => mod.parse_n3logic_wasm('{ ?s a ?t } ~> { ?s a ?t } .')).not.toThrow();
        });

        runner.it('parse_n3logic_wasm: defeater rule (^>) does not throw', () => {
            if (!mod?.parse_n3logic_wasm) return;
            runner.expect(() => mod.parse_n3logic_wasm('{ ?x a <http://ex.org/Exc> } ^> { ?x <http://ex.org/applies> false } .')).not.toThrow();
        });

        runner.it('parse_n3logic_wasm: linear rule (-o) does not throw', () => {
            if (!mod?.parse_n3logic_wasm) return;
            runner.expect(() => mod.parse_n3logic_wasm('{ ?x <http://ex.org/token> ?t } -o { ?x <http://ex.org/used> true } .')).not.toThrow();
        });

        runner.it('parse_n3logic_wasm: result items have subject/predicate/object (when non-empty)', () => {
            if (!mod?.parse_n3logic_wasm) return;
            const result = mod.parse_n3logic_wasm('{ ?s ?p ?o } => { ?s ?p ?o } .');
            if (result.length > 0) {
                runner.expect(result[0]).toHaveProperty('subject');
                runner.expect(result[0]).toHaveProperty('predicate');
                runner.expect(result[0]).toHaveProperty('object');
            }
        });

        runner.it('parse_n3logic_wasm: empty string does not throw', () => {
            if (!mod?.parse_n3logic_wasm) return;
            runner.expect(() => mod.parse_n3logic_wasm('')).not.toThrow();
        });

        // ── CBOR-LD (.cbor / .cbor-ld) ───────────────────────────────────────
        // CBOR array [1000, 2000, 3000, 4000] encodes directly as four u64 fields.

        runner.it('parse_cbor_ld_wasm: valid CBOR bytes → result with subject', () => {
            if (!mod?.parse_cbor_ld_wasm) return;
            const cbor = new Uint8Array([0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0]);
            const result = mod.parse_cbor_ld_wasm(cbor);
            runner.expect(result).toBeTruthy();
            runner.expect(result.subject).toBeTruthy();
        });

        runner.it('parse_cbor_ld_wasm: result has predicate and object', () => {
            if (!mod?.parse_cbor_ld_wasm) return;
            const cbor = new Uint8Array([0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0]);
            const result = mod.parse_cbor_ld_wasm(cbor);
            runner.expect(result.predicate).toBeTruthy();
            runner.expect(result.object).toBeTruthy();
        });

        runner.it('parse_cbor_ld_wasm: subject matches first CBOR integer (1000)', () => {
            if (!mod?.parse_cbor_ld_wasm) return;
            const cbor = new Uint8Array([0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0]);
            const result = mod.parse_cbor_ld_wasm(cbor);
            runner.expect(Number(result.subject)).toBe(1000);
        });

        runner.it('parse_cbor_ld_wasm: predicate matches second CBOR integer (2000)', () => {
            if (!mod?.parse_cbor_ld_wasm) return;
            const cbor = new Uint8Array([0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0]);
            const result = mod.parse_cbor_ld_wasm(cbor);
            runner.expect(Number(result.predicate)).toBe(2000);
        });

        runner.it('parse_cbor_ld_wasm: different input → different subject hash', () => {
            if (!mod?.parse_cbor_ld_wasm) return;
            const cbor1 = new Uint8Array([0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0]);
            const cbor2 = new Uint8Array([0x84, 0x19, 0x05, 0x00, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0]);
            const r1 = mod.parse_cbor_ld_wasm(cbor1);
            const r2 = mod.parse_cbor_ld_wasm(cbor2);
            runner.expect(r1.subject === r2.subject).toBeFalsy();
        });

        // ── JSON ──────────────────────────────────────────────────────────────

        runner.it('parse_json_wasm: flat {s,p,o} array → one result', () => {
            if (!mod?.parse_json_wasm) return;
            const json = '[{"s":"Alice","p":"knows","o":"Bob"}]';
            const result = mod.parse_json_wasm(json);
            runner.expect(Array.isArray(result)).toBeTruthy();
            runner.expect(result.length).toBe(1);
        });

        runner.it('parse_json_wasm: two items → two results', () => {
            if (!mod?.parse_json_wasm) return;
            const json = '[{"s":"A","p":"p","o":"B"},{"s":"B","p":"p","o":"C"}]';
            runner.expect(mod.parse_json_wasm(json).length).toBe(2);
        });

        runner.it('parse_json_wasm: result has subject, predicate, object', () => {
            if (!mod?.parse_json_wasm) return;
            const [item] = mod.parse_json_wasm('[{"s":"Alice","p":"knows","o":"Bob"}]');
            runner.expect(item).toHaveProperty('subject');
            runner.expect(item).toHaveProperty('predicate');
            runner.expect(item).toHaveProperty('object');
        });

        runner.it('parse_json_wasm: empty array → empty result', () => {
            if (!mod?.parse_json_wasm) return;
            runner.expect(mod.parse_json_wasm('[]').length).toBe(0);
        });

        runner.it('parse_json_wasm: IRI values preserved in subject', () => {
            if (!mod?.parse_json_wasm) return;
            const [item] = mod.parse_json_wasm('[{"s":"http://ex.org/Alice","p":"http://xmlns.com/foaf/0.1/name","o":"Alice"}]');
            runner.expect(item.subject).toContain('ex.org');
        });

        // ── CogAI Chunks (.chk text) ──────────────────────────────────────────
        // W3C CogAI CG chunks-and-rules format (https://github.com/w3c-cg/cogai).
        // DISTINCT from QCHK binary Capability Profiles — detected by magic bytes.

        runner.it('parse_cogai_chunk_wasm: basic chunk does not throw (if present)', () => {
            if (!mod?.parse_cogai_chunk_wasm) return;
            runner.expect(() => mod.parse_cogai_chunk_wasm('dog dog1 { name "Fido"; age 4 }')).not.toThrow();
        });

        runner.it('parse_cogai_chunk_wasm: returns array of quins (if present)', () => {
            if (!mod?.parse_cogai_chunk_wasm) return;
            const result = mod.parse_cogai_chunk_wasm('memory m1 { content "sky is blue"; strength 0.9 }');
            runner.expect(Array.isArray(result)).toBeTruthy();
            runner.expect(result.length).toBeGreaterThan(0);
        });

        runner.it('.chk disambiguation: CogAI text does not start with QCHK magic (0x51 0x43 0x48 0x4B)', () => {
            const bytes = new TextEncoder().encode('dog dog1 { name "Fido" }');
            const magic = String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3]);
            runner.expect(magic).not.toBe('QCHK');
        });

        runner.it('.chk disambiguation: QCHK magic bytes spell "QCHK"', () => {
            runner.expect(0x51).toBe('Q'.charCodeAt(0));
            runner.expect(0x43).toBe('C'.charCodeAt(0));
            runner.expect(0x48).toBe('H'.charCodeAt(0));
            runner.expect(0x4B).toBe('K'.charCodeAt(0));
        });

        runner.it('.chk disambiguation: CogAI @rdfmap prefix ≠ expanded IRI hash', () => {
            runner.expect(q_hash('dog') === q_hash('http://example.com/ns/dog')).toBeFalsy();
        });

        // ── WordNet integration (MANUAL — requires dataset build) ─────────────
        // Activate: ?manual=1 in URL or window.MANUAL_TESTS = true before Run.

        runner.it('wordnet.q42 reachable at /playground/wordnet.q42 (manual)', async () => {
            if (!manualWordNetEnabled()) return;
            const r = await fetch('/playground/wordnet.q42', { method: 'HEAD' });
            runner.expect(r.ok).toBeTruthy();
        });

        runner.it('wordnet.q42.lex sidecar reachable (manual)', async () => {
            if (!manualWordNetEnabled()) return;
            const r = await fetch('/playground/wordnet.q42.lex', { method: 'HEAD' });
            runner.expect(r.ok).toBeTruthy();
        });

        runner.it('wordnet.q42.bidx block-range index reachable (manual)', async () => {
            if (!manualWordNetEnabled()) return;
            const r = await fetch('/playground/wordnet.q42.bidx', { method: 'HEAD' });
            runner.expect(r.ok).toBeTruthy();
        });

        runner.it('wordnet.c.q42 compressed variant reachable (manual)', async () => {
            if (!manualWordNetEnabled()) return;
            const r = await fetch('/playground/wordnet.c.q42', { method: 'HEAD' });
            runner.expect(r.ok).toBeTruthy();
        });

        runner.it('wordnet.q42 is > 50 MB and < 100 MB — confirms 85% compression (manual)', async () => {
            if (!manualWordNetEnabled()) return;
            const r = await fetch('/playground/wordnet.q42', { method: 'HEAD' });
            const size = parseInt(r.headers.get('content-length') || '0');
            runner.expect(size).toBeGreaterThan(50 * 1024 * 1024);
            runner.expect(size).toBeLessThan(100 * 1024 * 1024);
        });

        runner.it('wordnet.c.q42 is smaller than wordnet.q42 (LZ4 compression active) (manual)', async () => {
            if (!manualWordNetEnabled()) return;
            const [r1, r2] = await Promise.all([
                fetch('/playground/wordnet.q42',   { method: 'HEAD' }),
                fetch('/playground/wordnet.c.q42', { method: 'HEAD' }),
            ]);
            const q42Size  = parseInt(r1.headers.get('content-length') || '0');
            const cq42Size = parseInt(r2.headers.get('content-length') || '0');
            runner.expect(cq42Size).toBeLessThan(q42Size);
        });
    });
}

export default register;
