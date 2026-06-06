// Dialectical Logic modality tests.
// Mirrors crates/qualia-core-db/src/modalities/dialectical.rs exactly.
// Hegelian synthesis: thesis ⊕ antithesis → synthesized quin with SYNTHESIZED_BIT.

import { TestRunner } from '../test-runner.js';
import { makeQuin } from './primitives.js';

const SYNTHESIZED_BIT = 1n << 58n;

function synthesizeDialectical(thesis, antithesis) {
    if (thesis.subject === antithesis.subject &&
        thesis.predicate === antithesis.predicate &&
        thesis.object !== antithesis.object) {
        const syn = { ...thesis };
        syn.context  = thesis.context ^ antithesis.context;
        syn.metadata = thesis.metadata | SYNTHESIZED_BIT;
        syn.object   = thesis.object ^ antithesis.object;
        syn.parity   = syn.subject ^ syn.predicate ^ syn.object ^ syn.context;
        return syn;
    }
    return null;
}

export function register(runner) {
    runner.describe('Modality: Dialectical Logic', () => {

        runner.it('contradicting thesis/antithesis produces synthesis', () => {
            const t = makeQuin(1n, 2n, 3n, 10n);
            const a = makeQuin(1n, 2n, 4n, 20n);
            const s = synthesizeDialectical(t, a);
            runner.expect(s).not.toBeNull();
        });

        runner.it('synthesized quin has SYNTHESIZED_BIT set', () => {
            const t = makeQuin(1n, 2n, 3n, 10n);
            const a = makeQuin(1n, 2n, 4n, 20n);
            const s = synthesizeDialectical(t, a);
            runner.expect((s.metadata & SYNTHESIZED_BIT) === SYNTHESIZED_BIT).toBeTruthy();
        });

        runner.it('synthesized context = thesis.context XOR antithesis.context', () => {
            const t = makeQuin(1n, 2n, 3n, 10n);
            const a = makeQuin(1n, 2n, 4n, 20n);
            const s = synthesizeDialectical(t, a);
            runner.expect(s.context).toBe(10n ^ 20n);
        });

        runner.it('synthesized object = thesis.object XOR antithesis.object', () => {
            const t = makeQuin(1n, 2n, 3n, 10n);
            const a = makeQuin(1n, 2n, 4n, 20n);
            const s = synthesizeDialectical(t, a);
            runner.expect(s.object).toBe(3n ^ 4n);
        });

        runner.it('parity = subject ^ predicate ^ object ^ context', () => {
            const t = makeQuin(1n, 2n, 3n, 10n);
            const a = makeQuin(1n, 2n, 4n, 20n);
            const s = synthesizeDialectical(t, a);
            runner.expect(s.parity).toBe(s.subject ^ s.predicate ^ s.object ^ s.context);
        });

        runner.it('same object → no synthesis (no contradiction)', () => {
            const t = makeQuin(1n, 2n, 3n, 10n);
            const a = makeQuin(1n, 2n, 3n, 20n); // same object
            runner.expect(synthesizeDialectical(t, a)).toBeNull();
        });

        runner.it('different predicate → no synthesis', () => {
            const t = makeQuin(1n, 2n, 3n, 10n);
            const a = makeQuin(1n, 9n, 4n, 20n);
            runner.expect(synthesizeDialectical(t, a)).toBeNull();
        });

        runner.it('different subject → no synthesis', () => {
            const t = makeQuin(1n, 2n, 3n, 10n);
            const a = makeQuin(9n, 2n, 4n, 20n);
            runner.expect(synthesizeDialectical(t, a)).toBeNull();
        });

        runner.it('SYNTHESIZED_BIT = 1 << 58', () => {
            runner.expect(SYNTHESIZED_BIT).toBe(288230376151711744n);
        });
    });
}

export default register;
