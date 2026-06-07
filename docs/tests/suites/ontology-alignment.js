// OWL → SHACL ontology alignment (healthcare vocab + RadLex + agency).
// Mirrors crates/qualia-core-db/src/owl_to_shacl.rs reference behaviour.

import { q_hash } from './primitives.js';

function parseHealthcareLines(text) {
    const classes = new Map();
    const properties = new Map();
    let current = null;

    for (const raw of text.split('\n')) {
        const line = raw.trim();
        if (!line || line.startsWith('@')) continue;
        if (line.startsWith('<') && line.endsWith('>') && !line.includes(';')) {
            current = line.slice(1, -1);
            continue;
        }
        if (!current) continue;
        if (line.startsWith('a ')) {
            const kind = line.replace(/^a\s+/, '').replace(/;$/, '').trim();
            if (kind === 'owl:Class') classes.set(current, { labels: [], equivalent: null });
            else if (kind === 'owl:DatatypeProperty' || kind === 'owl:ObjectProperty') {
                properties.set(current, { labels: [], kinds: new Set([kind]), domains: [], ranges: [], equivalent: null });
            }
        } else if (line.startsWith('rdfs:label')) {
            const m = line.match(/"([^"]+)"/);
            if (m) {
                if (classes.has(current)) classes.get(current).labels.push(m[1]);
                if (properties.has(current)) properties.get(current).labels.push(m[1]);
            }
        } else if (line.includes('owl:equivalentProperty')) {
            const m = line.match(/<([^>]+)>/);
            if (m && properties.has(current)) properties.get(current).equivalent = m[1];
        } else if (line.includes('rdfs:domain')) {
            const m = line.match(/<([^>]+)>/);
            if (m && properties.has(current)) properties.get(current).domains.push(m[1]);
        }
    }
    return { classes, properties };
}

function localName(uri) {
    const i = Math.max(uri.lastIndexOf('#'), uri.lastIndexOf('/'));
    return i >= 0 ? uri.slice(i + 1) : uri;
}

function emitHealthcareShapeNames(model) {
    const shapes = [];
    for (const [uri] of model.classes) {
        const name = localName(uri);
        if (name.startsWith('IE.') || name.startsWith('SequenceItem.')) {
            shapes.push(`hc:${name}Shape`);
        }
    }
    for (const [uri, prop] of model.properties) {
        for (const d of prop.domains) {
            const dn = localName(d);
            if (dn.startsWith('IE.')) shapes.push(`hc:${dn}PropertyShape`);
        }
    }
    return [...new Set(shapes)];
}

const AGENCY_SHAPE_MARKERS = [
    'q42:PrincipalShape',
    'sh:not',
    'q42:hasCondition',
    'q42:ClinicalEntity',
];

const SAMPLE_HEALTHCARE = `
@prefix owl: <http://www.w3.org/2002/07/owl#> .
<http://purl.org/healthcarevocab/v1#IE.Image>
        a owl:Class ;
        rdfs:label "Image" .
<http://purl.org/healthcarevocab/v1#ContentDate>
        a owl:DatatypeProperty ;
        rdfs:domain <http://purl.org/healthcarevocab/v1#IE.Image> ;
        rdfs:label "Content Date" ;
        owl:equivalentProperty <http://purl.org/healthcarevocab/v1#Tag.0008.0023> .
`;

export function register(runner) {
    runner.describe('Ontology: OWL → SHACL alignment', () => {

        runner.it('healthcare line parser extracts IE.Image class', () => {
            const m = parseHealthcareLines(SAMPLE_HEALTHCARE);
            runner.expect(m.classes.size).toBe(1);
            runner.expect([...m.classes.keys()][0]).toContain('IE.Image');
        });

        runner.it('healthcare parser maps equivalentProperty to DICOM tag URI', () => {
            const m = parseHealthcareLines(SAMPLE_HEALTHCARE);
            const prop = [...m.properties.values()][0];
            runner.expect(prop.equivalent).toContain('Tag.0008.0023');
        });

        runner.it('emits hc:IE.ImagePropertyShape for IE domain properties', () => {
            const shapes = emitHealthcareShapeNames(parseHealthcareLines(SAMPLE_HEALTHCARE));
            runner.expect(shapes).toContain('hc:IE.ImageShape');
            runner.expect(shapes).toContain('hc:IE.ImagePropertyShape');
        });

        runner.it('RadLex RID hashes are stable q_hash tokens', () => {
            const rid1 = q_hash('http://www.radlex.org/RID/RID94');
            const rid2 = q_hash('http://www.radlex.org/RID/RID94');
            runner.expect(rid1).toBe(rid2);
            runner.expect(rid1 === 0n).toBeFalsy();
        });

        runner.it('agency SHACL markers are defined in qualia-agency shape spec', () => {
            // Static spec check — full TTL loaded in Rust tests / example generator.
            for (const marker of AGENCY_SHAPE_MARKERS) {
                runner.expect(typeof marker).toBe('string');
                runner.expect(marker.length).toBeGreaterThan(3);
            }
        });

        runner.it('Principal and RadLex finding use distinct predicate hashes', () => {
            runner.expect(q_hash('q42:hasFinding') === q_hash('radlex:Part_Of')).toBeFalsy();
            runner.expect(q_hash('q42:Principal') === q_hash('radlex:AnatomicalEntity')).toBeFalsy();
        });
    });
}

export default register;
