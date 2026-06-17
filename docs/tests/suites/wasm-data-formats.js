// Data format parser/serializer tests for WASM
// Tests the new CSV, JSON, and RDF parsing/serialization capabilities
// added in version 0.0.17

import { loadWasm } from '../wasm-loader.js';
import { q_hash } from './primitives.js';

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Data Format Parsers', () => {
        runner.beforeAll(async () => { mod = await loadWasm(); });

        // ── CSV Parser ─────────────────────────────────────────────────────

        runner.it('parse_csv_wasm: basic CSV with header → quins', () => {
            if (!mod?.parse_csv_wasm) return;
            const csv = 'name,age,city\nAlice,30,NYC\nBob,25,LA';
            const params = {
                csv_data: csv,
                base_class_hash: q_hash('http://example.org/Person'),
                field_mappings: [
                    { source_key: 'name', predicate_hash: q_hash('http://example.org/name'), datatype: 'string' },
                    { source_key: 'age', predicate_hash: q_hash('http://example.org/age'), datatype: 'integer' },
                    { source_key: 'city', predicate_hash: q_hash('http://example.org/city'), datatype: 'string' }
                ]
            };
            const result = mod.parse_csv_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.quin_count).toBeGreaterThan(0);
        });

        runner.it('parse_csv_wasm: handles empty CSV', () => {
            if (!mod?.parse_csv_wasm) return;
            const csv = '';
            const params = {
                csv_data: csv,
                base_class_hash: q_hash('http://example.org/Person'),
                field_mappings: []
            };
            const result = mod.parse_csv_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.quin_count).toBe(0);
        });

        // ── JSON Parser ────────────────────────────────────────────────────

        runner.it('parse_json_wasm: basic JSON object → quins', () => {
            if (!mod?.parse_json_wasm) return;
            const json = '{"name": "Alice", "age": 30, "city": "NYC"}';
            const params = {
                json_data: json,
                base_class_hash: q_hash('http://example.org/Person'),
                field_mappings: [
                    { json_key: 'name', predicate_hash: q_hash('http://example.org/name'), datatype: 'string' },
                    { json_key: 'age', predicate_hash: q_hash('http://example.org/age'), datatype: 'integer' },
                    { json_key: 'city', predicate_hash: q_hash('http://example.org/city'), datatype: 'string' }
                ]
            };
            const result = mod.parse_json_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.quin_count).toBeGreaterThan(0);
        });

        runner.it('parse_json_wasm: handles JSON array', () => {
            if (!mod?.parse_json_wasm) return;
            const json = '[{"name": "Alice"}, {"name": "Bob"}]';
            const params = {
                json_data: json,
                base_class_hash: q_hash('http://example.org/Person'),
                field_mappings: [
                    { json_key: 'name', predicate_hash: q_hash('http://example.org/name'), datatype: 'string' }
                ]
            };
            const result = mod.parse_json_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.quin_count).toBeGreaterThan(0);
        });

        // ── CSV Serializer ───────────────────────────────────────────────────

        runner.it('serialize_csv_wasm: quins → CSV string', () => {
            if (!mod?.serialize_csv_wasm) return;
            const quins = [
                [q_hash('http://example.org/Alice'), q_hash('http://example.org/name'), q_hash('Alice'), 0, 0, 0],
                [q_hash('http://example.org/Alice'), q_hash('http://example.org/age'), 30 << 3, 0, 0, 0]
            ];
            const params = {
                quins: quins,
                headers: ['name', 'age'],
                predicate_hashes: [q_hash('http://example.org/name'), q_hash('http://example.org/age')],
                datatypes: ['string', 'integer']
            };
            const result = mod.serialize_csv_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.csv_data).toContain('name,age');
        });

        // ── JSON Serializer ───────────────────────────────────────────────────

        runner.it('serialize_json_wasm: quins → JSON string', () => {
            if (!mod?.serialize_json_wasm) return;
            const quins = [
                [q_hash('http://example.org/Alice'), q_hash('http://example.org/name'), q_hash('Alice'), 0, 0, 0]
            ];
            const params = {
                quins: quins,
                field_names: ['name'],
                predicate_hashes: [q_hash('http://example.org/name')],
                datatypes: ['string']
            };
            const result = mod.serialize_json_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.json_data).toContain('name');
        });

        // ── RDF Serializer ────────────────────────────────────────────────────

        runner.it('serialize_rdf_wasm: quins → N-Triples', () => {
            if (!mod?.serialize_rdf_wasm) return;
            const quins = [
                [q_hash('http://example.org/Alice'), q_hash('http://example.org/knows'), q_hash('http://example.org/Bob'), 0, 0, 0]
            ];
            const params = {
                quins: quins,
                format: 'nt'
            };
            const result = mod.serialize_rdf_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.rdf_data).toContain('<http://example.org/Alice>');
        });

        runner.it('serialize_rdf_wasm: quins → Turtle', () => {
            if (!mod?.serialize_rdf_wasm) return;
            const quins = [
                [q_hash('http://example.org/Alice'), q_hash('http://example.org/knows'), q_hash('http://example.org/Bob'), 0, 0, 0]
            ];
            const params = {
                quins: quins,
                format: 'turtle'
            };
            const result = mod.serialize_rdf_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.rdf_data).toContain('@prefix');
        });

        runner.it('serialize_rdf_wasm: quins → JSON-LD', () => {
            if (!mod?.serialize_rdf_wasm) return;
            const quins = [
                [q_hash('http://example.org/Alice'), q_hash('http://example.org/knows'), q_hash('http://example.org/Bob'), 0, 0, 0]
            ];
            const params = {
                quins: quins,
                format: 'jsonld'
            };
            const result = mod.serialize_rdf_wasm(JSON.stringify(params));
            runner.expect(result).toBeDefined();
            runner.expect(result.rdf_data).toContain('@context');
        });
    });
}
