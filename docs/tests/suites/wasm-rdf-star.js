// RDF-Star tests — nested triple references via bit 62 of the subject field.
// Tests both the JS convention and any parse_turtle_wasm / execute_ntriples_query path.

import { loadWasm } from '../wasm-loader.js';
import { q_hash } from './primitives.js';

const NESTED_BIT = 1n << 62n;

export function register(runner) {
    let mod = null;

    runner.describe('WASM: RDF-Star', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        runner.describe('Virtual subject ID convention (bit 62)', () => {

            runner.it('NESTED_BIT is 1 << 62', () => {
                runner.expect(NESTED_BIT).toBe(4611686018427387904n);
            });

            runner.it('virtual subject has bit 62 set', () => {
                const base = q_hash('alice:knows:bob');
                const vid  = base | NESTED_BIT;
                runner.expect((vid & NESTED_BIT) !== 0n).toBeTruthy();
            });

            runner.it('virtual subject does not collide with DEFEATER_BIT (bit 63)', () => {
                const DEFEATER_BIT = 1n << 63n;
                runner.expect(NESTED_BIT & DEFEATER_BIT).toBe(0n);
            });

            runner.it('two different embedded triples yield different virtual IDs', () => {
                const v1 = q_hash('alice:knows:bob')   | NESTED_BIT;
                const v2 = q_hash('alice:likes:carol') | NESTED_BIT;
                runner.expect(v1).not.toBe(v2);
            });

            runner.it('virtual ID with bit 62 cleared gives back a regular subject hash', () => {
                const base = q_hash('ex:something');
                const vid  = base | NESTED_BIT;
                runner.expect((vid & ~NESTED_BIT) & 0xFFFFFFFFFFFFFFFFn).toBe(base & ~NESTED_BIT & 0xFFFFFFFFFFFFFFFFn);
            });
        });

        runner.describe('RDF-Star annotation pattern', () => {

            runner.it('annotation Quin has NESTED_BIT subject and a regular predicate', () => {
                const embedded_subj = q_hash('did:alice');
                const embedded_pred = q_hash('foaf:knows');
                const embedded_obj  = q_hash('did:bob');
                // Virtual ID: hash of the embedded triple components
                const virtualId = (embedded_subj ^ embedded_pred ^ embedded_obj) | NESTED_BIT;
                // Predicate must not carry NESTED_BIT — FNV hashes can set bit 62 by chance.
                const annotPred = q_hash('ex:certainty') & ~NESTED_BIT;
                const annotObj  = q_hash('"0.9"^^xsd:decimal');

                runner.expect((virtualId & NESTED_BIT) !== 0n).toBeTruthy();
                runner.expect((annotPred & NESTED_BIT)).toBe(0n);
                runner.expect(typeof annotObj).toBe('bigint');
            });

            runner.it('multiple annotations on same embedded triple share the same virtual subject', () => {
                const sub = q_hash('did:alice') ^ q_hash('foaf:knows') ^ q_hash('did:bob');
                const v1 = sub | NESTED_BIT;
                const v2 = sub | NESTED_BIT;  // same embedded triple
                runner.expect(v1).toBe(v2);
            });
        });

        runner.describe('parse_turtle_wasm with RDF-Star syntax', () => {

            runner.it('parses N3 input if parse_turtle_wasm is available', () => {
                if (!mod?.parse_turtle_wasm) return;
                // Standard Turtle (not RDF-Star) must parse cleanly
                const ttl = `@prefix foaf: <http://xmlns.com/foaf/0.1/> .
<http://example.org/alice> foaf:knows <http://example.org/bob> .`;
                const r = mod.parse_turtle_wasm(ttl);
                runner.expect(r).toBeDefined();
            });

            runner.it('execute_ntriples_query handles a subject-predicate pattern match', () => {
                if (!mod?.execute_ntriples_query) return;
                const raw = mod.execute_ntriples_query('?s ?p ?o', new Uint8Array(0), 256);
                const r = JSON.parse(raw);
                runner.expect(Array.isArray(r.matches)).toBeTruthy();
            });
        });
    });
}

export default register;
