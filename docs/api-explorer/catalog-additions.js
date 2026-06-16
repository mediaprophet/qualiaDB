    },

    {
        id: 'wasm.parse_csv_wasm',
        category: 'WASM API',
        name: 'parse_csv_wasm()',
        summary: 'Parses CSV data and converts it to QualiaQuins with configurable field mappings. Supports header-based field mapping and datatype specification.',
        params: [
            { name: 'csv_data', type: 'string', desc: 'CSV string with optional header row' },
            { name: 'base_class_hash', type: 'u64', desc: 'Hash of the base class URI for all rows' },
            { name: 'field_mappings', type: 'array', desc: 'Array of { source_key, predicate_hash, datatype }' },
        ],
        returns: '{ quin_count, quins }',
        snippets: [
            js(`
import init, { parse_csv_wasm, q_hash } from './playground/qualia_core_db.js';
await init();

const csv = 'name,age,city\\nAlice,30,NYC\\nBob,25,LA';
const params = {
  csv_data: csv,
  base_class_hash: q_hash('http://example.org/Person'),
  field_mappings: [
    { source_key: 'name', predicate_hash: q_hash('http://example.org/name'), datatype: 'string' },
    { source_key: 'age', predicate_hash: q_hash('http://example.org/age'), datatype: 'integer' },
    { source_key: 'city', predicate_hash: q_hash('http://example.org/city'), datatype: 'string' }
  ]
};
const result = parse_csv_wasm(JSON.stringify(params));
console.log(result.quin_count);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.parse_csv_wasm) return { error: 'WASM not loaded or feature missing' };
            try {
                const params = JSON.parse(inputs.payload || '{"csv_data":"name,age\\nAlice,30","base_class_hash":12345,"field_mappings":[]}' );
                return wasm.parse_csv_wasm(JSON.stringify(params));
            } catch (e) {
                return { error: e.toString() };
            }
        },
        liveInputs: [{ name: 'payload', label: 'JSON Parameters', default: '{"csv_data":"name,age\\nAlice,30","base_class_hash":12345,"field_mappings":[]}' }],
    },

    {
        id: 'wasm.serialize_csv_wasm',
        category: 'WASM API',
        name: 'serialize_csv_wasm()',
        summary: 'Serializes QualiaQuins back to CSV format with configurable headers and predicate mappings.',
        params: [
            { name: 'quins', type: 'array', desc: 'Array of QualiaQuin objects' },
            { name: 'headers', type: 'array', desc: 'CSV header row' },
            { name: 'predicate_hashes', type: 'array', desc: 'Predicate hashes for each column' },
            { name: 'datatypes', type: 'array', desc: 'Datatypes for each column (string, integer, decimal)' },
        ],
        returns: '{ csv_data }',
        snippets: [
            js(`
import init, { serialize_csv_wasm, q_hash } from './playground/qualia_core_db.js';
await init();

const quins = [
  [q_hash('Alice'), q_hash('name'), q_hash('Alice'), 0, 0, 0]
];
const params = {
  quins: quins,
  headers: ['name'],
  predicate_hashes: [q_hash('http://example.org/name')],
  datatypes: ['string']
};
const result = serialize_csv_wasm(JSON.stringify(params));
console.log(result.csv_data);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.serialize_csv_wasm) return { error: 'WASM not loaded or feature missing' };
            try {
                const params = JSON.parse(inputs.payload || '{"quins":[],"headers":["name"],"predicate_hashes":[12345],"datatypes":["string"]}' );
                return wasm.serialize_csv_wasm(JSON.stringify(params));
            } catch (e) {
                return { error: e.toString() };
            }
        },
        liveInputs: [{ name: 'payload', label: 'JSON Parameters', default: '{"quins":[],"headers":["name"],"predicate_hashes":[12345],"datatypes":["string"]}' }],
    },

    {
        id: 'wasm.serialize_rdf_wasm',
        category: 'WASM API',
        name: 'serialize_rdf_wasm()',
        summary: 'Serializes QualiaQuins to multiple RDF formats: N-Triples, Turtle, N-Quads, TriG, N3, or JSON-LD.',
        params: [
            { name: 'quins', type: 'array', desc: 'Array of QualiaQuin objects' },
            { name: 'format', type: 'string', desc: 'Format: nt, turtle, nquads, trig, n3, or jsonld' },
        ],
        returns: '{ rdf_data }',
        snippets: [
            js(`
import init, { serialize_rdf_wasm, q_hash } from './playground/qualia_core_db.js';
await init();

const quins = [
  [q_hash('Alice'), q_hash('knows'), q_hash('Bob'), 0, 0, 0]
];
const params = {
  quins: quins,
  format: 'turtle'
};
const result = serialize_rdf_wasm(JSON.stringify(params));
console.log(result.rdf_data);
`),
        ],
        live: async (wasm, _native, inputs) => {
            if (!wasm?.serialize_rdf_wasm) return { error: 'WASM not loaded or feature missing' };
            try {
                const params = JSON.parse(inputs.payload || '{"quins":[],"format":"turtle"}' );
                return wasm.serialize_rdf_wasm(JSON.stringify(params));
            } catch (e) {
                return { error: e.toString() };
            }
        },
        liveInputs: [{ name: 'payload', label: 'JSON Parameters', default: '{"quins":[],"format":"turtle"}' }],
    },

    {
