// Agency alignment: Principal (human being) may HAVE Things but IS NOT a Thing.
// Mirrors crates/qualia-core-db/shapes/qualia-agency.shacl.ttl + ingest invariants.

import { q_hash, makeQuin } from './primitives.js';

const P_PRINCIPAL       = q_hash('q42:Principal');
const P_THING           = q_hash('q42:Thing');
const P_CLINICAL        = q_hash('q42:ClinicalEntity');
const P_IMAGING         = q_hash('q42:ImagingEntity');
const P_HAS_CONDITION   = q_hash('q42:hasCondition');
const P_HAS_FINDING     = q_hash('q42:hasFinding');
const P_HAS_STUDY       = q_hash('q42:hasImagingStudy');
const P_HAS_SERIES      = q_hash('q42:hasDicomSeries');
const P_RDF_TYPE        = q_hash('rdf:type');

const OWL_THING = q_hash('owl:Thing');

function classifyNode(quins, nodeHash) {
    const types = new Set();
    for (const q of quins) {
        if (q.subject === nodeHash && q.predicate === P_RDF_TYPE) {
            types.add(q.object);
        }
    }
    return types;
}

/** Validate possession graph: Principals must not be typed as Things. */
function validateAgencyGraph(quins) {
    const errors = [];
    const principals = new Set();

    for (const q of quins) {
        if (q.predicate === P_RDF_TYPE && q.object === P_PRINCIPAL) {
            principals.add(q.subject);
        }
        if (q.predicate === P_HAS_CONDITION || q.predicate === P_HAS_FINDING) {
            if (q.subject === q.object) {
                errors.push('Principal cannot be identical to possessed clinical entity');
            }
        }
    }

    for (const principal of principals) {
        const types = classifyNode(quins, principal);
        if (types.has(P_THING) || types.has(OWL_THING)) {
            errors.push(`Principal ${principal.toString(16)} typed as Thing`);
        }
        for (const q of quins) {
            if (q.subject !== principal) continue;
            if (q.predicate === P_HAS_CONDITION || q.predicate === P_HAS_FINDING) {
                if (q.object === principal) {
                    errors.push('hasCondition/hasFinding object must not be the Principal');
                }
            }
        }
    }

    return errors;
}

function possessionQuins(principalDid, conditionHash) {
    return [
        makeQuin(principalDid, P_RDF_TYPE, P_PRINCIPAL, 0n),
        makeQuin(principalDid, P_HAS_CONDITION, conditionHash, 0n),
        makeQuin(conditionHash, P_RDF_TYPE, P_CLINICAL, 0n),
    ];
}

export function register(runner) {
    runner.describe('Agency: Principal ≠ Thing', () => {

        runner.it('q42:Principal and q42:Thing are distinct hashes', () => {
            runner.expect(q_hash('q42:Principal') === q_hash('q42:Thing')).toBeFalsy();
        });

        runner.it('valid possession graph: Principal hasCondition ClinicalEntity', () => {
            const patient = q_hash('did:patient:alice');
            const diabetes = q_hash('snomed:44054006');
            const errs = validateAgencyGraph(possessionQuins(patient, diabetes));
            runner.expect(errs.length).toBe(0);
        });

        runner.it('rejects Principal typed as q42:Thing', () => {
            const patient = q_hash('did:patient:bob');
            const quins = [
                makeQuin(patient, P_RDF_TYPE, P_PRINCIPAL, 0n),
                makeQuin(patient, P_RDF_TYPE, P_THING, 0n),
            ];
            runner.expect(validateAgencyGraph(quins).length).toBeGreaterThan(0);
        });

        runner.it('rejects Principal identical to hasCondition object', () => {
            const patient = q_hash('did:patient:carol');
            const quins = [
                makeQuin(patient, P_RDF_TYPE, P_PRINCIPAL, 0n),
                makeQuin(patient, P_HAS_CONDITION, patient, 0n),
            ];
            runner.expect(validateAgencyGraph(quins).length).toBeGreaterThan(0);
        });

        runner.it('imaging possessions use hasDicomSeries not rdf:type on Principal', () => {
            const patient = q_hash('did:patient:dicom');
            const series = q_hash('1.2.840.…series');
            const quins = [
                makeQuin(patient, P_RDF_TYPE, P_PRINCIPAL, 0n),
                makeQuin(patient, P_HAS_SERIES, series, q_hash('q42:imagingStudy')),
                makeQuin(series, P_RDF_TYPE, P_IMAGING, q_hash('q42:imagingStudy')),
            ];
            runner.expect(validateAgencyGraph(quins).length).toBe(0);
            runner.expect(quins[1].predicate).toBe(P_HAS_SERIES);
        });

        runner.it('hasImagingStudy domain is Principal, range is ImagingEntity (hash distinctness)', () => {
            runner.expect(P_HAS_STUDY === P_HAS_CONDITION).toBeFalsy();
            runner.expect(P_IMAGING === P_CLINICAL).toBeFalsy();
        });
    });
}

export default register;
