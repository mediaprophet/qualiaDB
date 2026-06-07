// Comorbidity compounding evaluator — mirrors comorbidity_eval.rs (JS reference).

import { q_hash, makeQuin } from './primitives.js';

const P_EXACERBATES    = q_hash('q42:exacerbates');
const P_HAS_CONDITION  = q_hash('q42:hasCondition');
const P_HAS_SEVERITY   = q_hash('q42:hasSeverity');
const NESTED_MASK      = 1n << 63n;
const INLINE_DECIMAL   = 0b010n << 60n;
const VALUE_MASK       = 0x0FFF_FFFF_FFFF_FFFFn;

const HEART      = q_hash('Heart');
const DIABETES   = q_hash('Type 2 Diabetes Mellitus');
const HYPERTENSION = q_hash('Hypertension');

function nestedFingerprint(ante, pred, cons) {
    return ((ante ^ pred ^ cons) & VALUE_MASK) | NESTED_MASK;
}

function encodeSeverity(severity) {
    const scaled = BigInt(Math.round(Math.max(0, Math.min(1, severity)) * 1_000_000));
    return (scaled & VALUE_MASK) | INLINE_DECIMAL;
}

function decodeSeverityMilli(object) {
    if ((object & (0b111n << 60n)) !== INLINE_DECIMAL) return 500;
    const scaled = Number(object & VALUE_MASK);
    return Math.min(1000, Math.round((scaled * 1000) / 1_000_000));
}

function conditionIntersectsOrgan(condition, organ) {
    if (organ === 0n) return true;
    if (condition === organ) return true;
    if (organ === HEART) {
        return condition === DIABETES || condition === HYPERTENSION
            || condition === HEART || condition === q_hash('Diabetic Neuropathy');
    }
    return condition === organ;
}

function evalComorbidity(patientHash, targetOrgan, quins) {
    const conditions = [];
    const severities = [];

    for (const q of quins) {
        if (q.predicate === P_HAS_CONDITION && q.subject === patientHash) {
            conditions.push(q.object);
        }
        if ((q.subject & NESTED_MASK) !== 0n && q.predicate === P_HAS_SEVERITY
            && q.context === patientHash) {
            severities.push([q.subject, decodeSeverityMilli(q.object)]);
        }
    }

    const verdicts = [];
    for (const condition of conditions) {
        if (!conditionIntersectsOrgan(condition, targetOrgan)) continue;
        let riskMilli = 400;
        for (const q of quins) {
            if (q.predicate !== P_EXACERBATES || q.context !== patientHash) continue;
            if (q.subject !== condition && q.object !== condition) continue;
            const fp = nestedFingerprint(q.subject, q.predicate, q.object);
            const sev = severities.find(([s]) => s === fp);
            const factor = sev ? sev[1] : 500;
            riskMilli = Math.min(1000, Math.round((riskMilli * factor) / 1000));
        }
        verdicts.push({ conditionHash: condition, compoundedRiskMilli: riskMilli });
    }
    return verdicts;
}

export function register(runner) {
    runner.describe('Comorbidity: compounded risk', () => {

        runner.it('empty graph → no verdicts', () => {
            const p = q_hash('did:patient:test');
            runner.expect(evalComorbidity(p, HEART, []).length).toBe(0);
        });

        runner.it('single condition on heart → baseline risk 400 milli', () => {
            const p = q_hash('did:patient:heart');
            const quins = [makeQuin(p, P_HAS_CONDITION, HEART, p)];
            const v = evalComorbidity(p, HEART, quins);
            runner.expect(v.length).toBe(1);
            runner.expect(v[0].compoundedRiskMilli).toBe(400);
        });

        runner.it('diabetes + hypertension exacerbation compounds risk on heart target', () => {
            const p = q_hash('did:patient:compound');
            const ante = DIABETES;
            const cons = HYPERTENSION;
            const fp = nestedFingerprint(ante, P_EXACERBATES, cons);
            const quins = [
                makeQuin(p, P_HAS_CONDITION, DIABETES, p),
                makeQuin(p, P_HAS_CONDITION, HYPERTENSION, p),
                makeQuin(ante, P_EXACERBATES, cons, p),
                makeQuin(fp, P_HAS_SEVERITY, encodeSeverity(0.8), p),
            ];
            const v = evalComorbidity(p, HEART, quins);
            const diabetes = v.find(x => x.conditionHash === DIABETES);
            runner.expect(diabetes).toBeTruthy();
            runner.expect(diabetes.compoundedRiskMilli).toBeLessThan(400);
        });

        runner.it('organ filter skips unrelated conditions', () => {
            const p = q_hash('did:patient:filter');
            const liver = q_hash('Liver');
            const quins = [makeQuin(p, P_HAS_CONDITION, liver, p)];
            runner.expect(evalComorbidity(p, HEART, quins).length).toBe(0);
        });

        runner.it('severity encoding uses inline decimal tag (resolver convention)', () => {
            const obj = encodeSeverity(0.5);
            runner.expect((obj >> 60n) & 0b111n).toBe(0b010n);
        });
    });
}

export default register;
