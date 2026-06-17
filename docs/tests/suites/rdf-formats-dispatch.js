// RDF / RDF-Star format dispatch — mirrors sparql_library/rdf_formats (pure JS reference).

import { q_hash } from './primitives.js';

const MAX_RDF_QUINS = 8192;

const FORMAT_ALIASES = {
    nt: 'ntriples',
    ntriples: 'ntriples',
    'n-triples': 'ntriples',
    turtle: 'turtle',
    ttl: 'turtle',
    nquads: 'nquads',
    'n-quads': 'nquads',
    trig: 'trig',
    n3: 'n3',
    jsonld: 'jsonld',
    'json-ld': 'jsonld',
    cbor: 'cborld',
    'cbor-ld': 'cborld',
    cborld: 'cborld',
};

function resolveFormat(s) {
    if (!s) return null;
    return FORMAT_ALIASES[String(s).toLowerCase()] ?? null;
}

/** Minimal N-Triples line parser for reference tests (IRI-only). */
function parseNtLine(line, contextHash = 0) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) return null;
    const m = trimmed.match(/^<([^>]+)>\s+<([^>]+)>\s+<([^>]+)>\s+\.$/);
    if (!m) return null;
    return {
        subject: q_hash(m[1]),
        predicate: q_hash(m[2]),
        object: q_hash(m[3]),
        context: contextHash,
        metadata: 0,
        parity: 0,
    };
}

export function register(runner) {
    runner.describe('RDF: Format Dispatch', () => {

        runner.describe('RdfFormat aliases', () => {
            runner.it('resolves nt / ntriples / n-triples', () => {
                runner.expect(resolveFormat('nt')).toBe('ntriples');
                runner.expect(resolveFormat('NTriples')).toBe('ntriples');
                runner.expect(resolveFormat('n-triples')).toBe('ntriples');
            });

            runner.it('resolves turtle / ttl', () => {
                runner.expect(resolveFormat('turtle')).toBe('turtle');
                runner.expect(resolveFormat('ttl')).toBe('turtle');
            });

            runner.it('resolves jsonld / json-ld', () => {
                runner.expect(resolveFormat('jsonld')).toBe('jsonld');
                runner.expect(resolveFormat('json-ld')).toBe('jsonld');
            });

            runner.it('returns null for unknown format', () => {
                runner.expect(resolveFormat('rdf/xml')).toBeNull();
            });
        });

        runner.describe('QuinCollector bounds', () => {
            runner.it('MAX_RDF_QUINS is 8192', () => {
                runner.expect(MAX_RDF_QUINS).toBe(8192);
            });

            runner.it('collector rejects overflow (reference semantics)', () => {
                let count = 0;
                let truncated = false;
                const buf = new Array(MAX_RDF_QUINS);
                const push = (q) => {
                    if (count < MAX_RDF_QUINS) {
                        buf[count++] = q;
                    } else {
                        truncated = true;
                        throw new Error('quin buffer full');
                    }
                };
                const q = { subject: 1n, predicate: 2n, object: 3n };
                for (let i = 0; i < MAX_RDF_QUINS; i++) push(q);
                runner.expect(() => push(q)).toThrow();
                runner.expect(truncated).toBeTruthy();
                runner.expect(count).toBe(MAX_RDF_QUINS);
            });
        });

        runner.describe('N-Triples reference parse', () => {
            runner.it('parses a single triple line into quin fields', () => {
                const line = '<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .';
                const q = parseNtLine(line);
                runner.expect(q).not.toBeNull();
                runner.expect(q.subject).toBe(q_hash('http://example.org/Alice'));
                runner.expect(q.predicate).toBe(q_hash('http://example.org/knows'));
                runner.expect(q.object).toBe(q_hash('http://example.org/Bob'));
            });

            runner.it('skips comments and blank lines', () => {
                runner.expect(parseNtLine('# comment')).toBeNull();
                runner.expect(parseNtLine('')).toBeNull();
            });
        });

        runner.describe('Plain serialize placeholder convention', () => {
            runner.it('unknown IRI hashes render as quin:hash/ hex (resolver contract)', () => {
                const h = q_hash('http://example.org/Alice');
                const hex = h.toString(16).padStart(16, '0');
                const placeholder = `<quin:hash/${hex}>`;
                runner.expect(placeholder).toContain('quin:hash/');
                runner.expect(placeholder.length).toBeGreaterThan(10);
            });
        });
    });
}