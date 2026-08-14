// Data format parser/serializer tests for WASM
// Tests the new CSV, JSON, and RDF parsing/serialization capabilities
// added in version 0.0.28

import { loadWasm } from '../wasm-loader.js';
import { q_hash } from './primitives.js';

/** Pack six u64 fields for serde_wasm_bindgen (accepts number or BigInt). */
function packQuin(subject, predicate, object, context = 0, metadata = 0, parity = 0) {
    const u = (v) => (typeof v === 'bigint' ? v : BigInt(v));
    return [u(subject), u(predicate), u(object), u(context), u(metadata), u(parity)];
}

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Data Format Parsers', () => {
        runner.beforeAll(async () => { mod = await loadWasm(); });

        // ── CSV Parser ─────────────────────────────────────────────────────

        runner.it('parse_csv_wasm: basic CSV with header → quins', () => {
            if (!mod?.parse_csv_wasm) return;
            const result = mod.parse_csv_wasm({
                csv_data: 'name,age,city\nAlice,30,NYC\nBob,25,LA',
                base_class_hash: q_hash('http://example.org/Person'),
                field_mappings: [
                    { source_key: 'name', predicate_hash: q_hash('http://example.org/name'), datatype: 'string' },
                    { source_key: 'age', predicate_hash: q_hash('http://example.org/age'), datatype: 'integer' },
                    { source_key: 'city', predicate_hash: q_hash('http://example.org/city'), datatype: 'string' },
                ],
            });
            runner.expect(result).toBeDefined();
            runner.expect(result.quin_count).toBeGreaterThan(0);
            runner.expect(result.quins.length).toBe(result.quin_count);
            runner.expect(typeof result.quins[0][0]).toBe('string');
        });

        runner.it('parse_csv_wasm: handles empty CSV', () => {
            if (!mod?.parse_csv_wasm) return;
            const result = mod.parse_csv_wasm({
                csv_data: '',
                base_class_hash: q_hash('http://example.org/Person'),
                field_mappings: [],
            });
            runner.expect(result).toBeDefined();
            runner.expect(result.quin_count).toBe(0);
        });

        // ── JSON Parser (mapping profile) ──────────────────────────────────

        runner.it('parse_json_wasm: basic JSON object → quins', () => {
            if (!mod?.parse_json_mapping_wasm) return;
            const result = mod.parse_json_mapping_wasm({
                json_data: '{"name": "Alice", "age": 30, "city": "NYC"}',
                base_class_hash: q_hash('http://example.org/Person'),
                field_mappings: [
                    { source_key: 'name', predicate_hash: q_hash('http://example.org/name'), datatype: 'string' },
                    { source_key: 'age', predicate_hash: q_hash('http://example.org/age'), datatype: 'integer' },
                    { source_key: 'city', predicate_hash: q_hash('http://example.org/city'), datatype: 'string' },
                ],
            });
            runner.expect(result).toBeDefined();
            runner.expect(result.quin_count).toBeGreaterThan(0);
            runner.expect(result.quins.length).toBe(result.quin_count);
            runner.expect(typeof result.quins[0][0]).toBe('string');
        });

        runner.it('parse_json_wasm: handles JSON array', () => {
            if (!mod?.parse_json_mapping_wasm) return;
            const result = mod.parse_json_mapping_wasm({
                json_data: '[{"name": "Alice"}, {"name": "Bob"}]',
                base_class_hash: q_hash('http://example.org/Person'),
                field_mappings: [
                    { source_key: 'name', predicate_hash: q_hash('http://example.org/name'), datatype: 'string' },
                ],
            });
            runner.expect(result).toBeDefined();
            runner.expect(result.quin_count).toBe(2);
        });

        // ── CSV Serializer ─────────────────────────────────────────────────

        runner.it('serialize_csv_wasm: quins → CSV string', () => {
            if (!mod?.serialize_csv_wasm) return;
            const result = mod.serialize_csv_wasm({
                quins: [
                    packQuin(q_hash('http://example.org/Alice'), q_hash('http://example.org/name'), q_hash('Alice')),
                    // Tagged integer 30: type tag in bit 60, value in the low bits.
                    // (Must stay within u64::MAX — 30n << 60n overflows 64 bits.)
                    packQuin(q_hash('http://example.org/Alice'), q_hash('http://example.org/age'), (1n << 60n) | 30n),
                ],
                field_names: ['name', 'age'],
                predicate_hashes: [q_hash('http://example.org/name'), q_hash('http://example.org/age')],
                datatypes: ['string', 'integer'],
            });
            runner.expect(result).toBeDefined();
            runner.expect(result.csv_data).toContain('name,age');
        });

        // ── JSON Serializer ────────────────────────────────────────────────

        runner.it('serialize_json_wasm: quins → JSON string', () => {
            if (!mod?.serialize_json_wasm) return;
            const result = mod.serialize_json_wasm({
                quins: [
                    packQuin(q_hash('http://example.org/Alice'), q_hash('http://example.org/name'), q_hash('Alice')),
                ],
                field_names: ['name'],
                predicate_hashes: [q_hash('http://example.org/name')],
                datatypes: ['string'],
            });
            runner.expect(result).toBeDefined();
            runner.expect(result.json_data).toContain('name');
        });

        // ── RDF Serializer ───────────────────────────────────────────────

        runner.it('serialize_rdf_wasm: quins → N-Triples', () => {
            if (!mod?.serialize_rdf_wasm) return;
            const result = mod.serialize_rdf_wasm({
                quins: [
                    packQuin(q_hash('http://example.org/Alice'), q_hash('http://example.org/knows'), q_hash('http://example.org/Bob')),
                ],
                format: 'nt',
            });
            runner.expect(result).toBeDefined();
            runner.expect(typeof result.rdf_data).toBe('string');
            runner.expect(result.rdf_data.length).toBeGreaterThan(0);
        });

        runner.it('serialize_rdf_wasm: quins → Turtle', () => {
            if (!mod?.serialize_rdf_wasm) return;
            const result = mod.serialize_rdf_wasm({
                quins: [
                    packQuin(q_hash('http://example.org/Alice'), q_hash('http://example.org/knows'), q_hash('http://example.org/Bob')),
                ],
                format: 'turtle',
            });
            runner.expect(result).toBeDefined();
            runner.expect(typeof result.rdf_data).toBe('string');
            runner.expect(result.rdf_data.length).toBeGreaterThan(0);
        });

        runner.it('serialize_rdf_wasm: quins → JSON-LD', () => {
            if (!mod?.serialize_rdf_wasm) return;
            const result = mod.serialize_rdf_wasm({
                quins: [
                    packQuin(q_hash('http://example.org/Alice'), q_hash('http://example.org/knows'), q_hash('http://example.org/Bob')),
                ],
                format: 'jsonld',
            });
            runner.expect(result).toBeDefined();
            runner.expect(typeof result.rdf_data).toBe('string');
            runner.expect(result.rdf_data.length).toBeGreaterThan(0);
        });
    });
}

export default register;
