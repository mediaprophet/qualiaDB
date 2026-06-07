// Deontic logic — mirrors deontic_logic.rs (JS reference).

import { q_hash, makeQuin } from './primitives.js';

const OP_OBLIGATE = 0x10n;
const OP_PERMIT   = 0x11n;
const OP_FORBID   = 0x12n;
const DEFEATER_BIT = 1n << 63n;
const PATH_MASK    = 0x7FFF_FFFF_FFFF_FF00n;

function compileNorm(party, opcode, propertyPath, actionObject, contract, expiry, isDefeater) {
    // Mirror compile_norm_quin: mask DEFEATER_BIT from shifted path so only isDefeater sets bit 63.
    let predicate = ((BigInt(propertyPath) << 8n) & ~DEFEATER_BIT) | opcode;
    if (isDefeater) predicate |= DEFEATER_BIT;
    const q = makeQuin(party, predicate, actionObject, contract, BigInt(expiry));
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    return q;
}

function defeaterFingerprint(q) {
    const pathBits = q.predicate & PATH_MASK;
    return q.subject ^ q.context ^ pathBits;
}

function harvestDefeaters(quins) {
    const fps = new Set();
    for (const q of quins) {
        if ((q.predicate & DEFEATER_BIT) !== 0n) {
            fps.add(defeaterFingerprint(q));
        }
    }
    return fps;
}

function evaluateDeontic(quins, nowUnix) {
    const defeaters = harvestDefeaters(quins);
    const verdicts = [];
    for (const norm of quins) {
        if ((norm.predicate & DEFEATER_BIT) !== 0n) continue;
        const opcode = norm.predicate & 0xFFn;
        if (opcode !== OP_OBLIGATE && opcode !== OP_PERMIT && opcode !== OP_FORBID) continue;
        const expiry = Number(norm.metadata & 0xFFFF_FFFFn);
        let status = 'Active';
        if (expiry !== 0 && nowUnix > expiry) status = 'Expired';
        else if (defeaters.has(defeaterFingerprint(norm))) status = 'Defeated';
        verdicts.push({ norm, status, opcode: Number(opcode) });
    }
    return verdicts;
}

export function register(runner) {
    runner.describe('Deontic: O/P/F + unless defeaters', () => {

        runner.it('opcode constants are 0x10/0x11/0x12', () => {
            runner.expect(Number(OP_OBLIGATE)).toBe(0x10);
            runner.expect(Number(OP_PERMIT)).toBe(0x11);
            runner.expect(Number(OP_FORBID)).toBe(0x12);
        });

        runner.it('active obligation when no defeater and not expired', () => {
            const alice = q_hash('did:alice');
            const path  = q_hash('q42:disclose');
            const data  = q_hash('q42:data:confidential');
            const nda   = q_hash('contract:nda');
            const norm  = compileNorm(alice, OP_OBLIGATE, path, data, nda, 4_000_000_000, false);
            const v = evaluateDeontic([norm], 1_700_000_000);
            runner.expect(v.length).toBe(1);
            runner.expect(v[0].status).toBe('Active');
        });

        runner.it('defeater with DEFEATER_BIT overrides matching obligation', () => {
            const alice = q_hash('did:alice');
            const path  = q_hash('q42:disclose');
            const data  = q_hash('q42:data:confidential');
            const auditor = q_hash('q42:role:certified-auditor');
            const nda   = q_hash('contract:nda');
            const norm  = compileNorm(alice, OP_OBLIGATE, path, data, nda, 4_000_000_000, false);
            const unless = compileNorm(alice, OP_PERMIT, path, auditor, nda, 4_000_000_000, true);
            const v = evaluateDeontic([norm, unless], 1_700_000_000);
            runner.expect(v[0].status).toBe('Defeated');
        });

        runner.it('expired norm when now > metadata expiry', () => {
            const party = q_hash('did:bob');
            const path  = q_hash('q42:actInBestInterest');
            const norm  = compileNorm(party, OP_OBLIGATE, path, 1n, 1n, 1000, false);
            const v = evaluateDeontic([norm], 2000);
            runner.expect(v[0].status).toBe('Expired');
        });

        runner.it('smoker + pneumonia => high_risk UNLESS antibiotics (deontic pattern)', () => {
            // Comorbidity uses graph edges; deontic governs whether rule fires.
            const smoker = q_hash('condition:smoking');
            const pneumonia = q_hash('condition:pneumonia');
            const antibiotics = q_hash('treatment:antibiotics');
            const patient = q_hash('did:patient:rule');
            const obligPath = q_hash('q42:obligateHighRisk');
            const risk = q_hash('risk:high');
            const oblig = compileNorm(smoker, OP_OBLIGATE, obligPath, risk, patient, 0, false);
            const unless = compileNorm(smoker, OP_PERMIT, obligPath, antibiotics, patient, 0, true);
            const withoutTreatment = evaluateDeontic([oblig], 0);
            const withTreatment = evaluateDeontic([oblig, unless], 0);
            runner.expect(withoutTreatment[0].status).toBe('Active');
            runner.expect(withTreatment[0].status).toBe('Defeated');
        });
    });
}

export default register;
