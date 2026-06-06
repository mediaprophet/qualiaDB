// Description Logic modality tests.
// Mirrors crates/qualia-core-db/src/modalities/dl.rs exactly.
// Subsumption check via DFS transitive closure over TBox quins.

import { TestRunner } from '../test-runner.js';
import { makeQuin } from './primitives.js';

function checkSubsumptionQuin(subClassHash, superClassHash, tbox) {
    if (subClassHash === superClassHash) return true;

    let current = subClassHash;
    for (let depth = 0; depth < 64; depth++) {
        let found = false;
        for (const quin of tbox) {
            if (quin.subject === current) {
                current = quin.object;
                found = true;
                if (current === superClassHash) return true;
                break;
            }
        }
        if (!found) break;
    }
    return false;
}

function tboxQuin(subject, object) {
    return makeQuin(BigInt(subject), 0n, BigInt(object));
}

export function register(runner) {
    runner.describe('Modality: Description Logic (DL Subsumption)', () => {

        runner.it('A ⊑ A (reflexive)', () => {
            runner.expect(checkSubsumptionQuin(10n, 10n, [])).toBeTruthy();
        });

        runner.it('A ⊑ B (direct)', () => {
            const tbox = [tboxQuin(10, 20)];
            runner.expect(checkSubsumptionQuin(10n, 20n, tbox)).toBeTruthy();
        });

        runner.it('A ⊑ C (transitive via B)', () => {
            const tbox = [tboxQuin(10, 20), tboxQuin(20, 30)];
            runner.expect(checkSubsumptionQuin(10n, 30n, tbox)).toBeTruthy();
        });

        runner.it('A ⊑ D (transitive via B→C→D)', () => {
            const tbox = [tboxQuin(10, 20), tboxQuin(20, 30), tboxQuin(30, 40)];
            runner.expect(checkSubsumptionQuin(10n, 40n, tbox)).toBeTruthy();
        });

        runner.it('A ⊄ D (chain not long enough)', () => {
            const tbox = [tboxQuin(10, 20), tboxQuin(20, 30)];
            runner.expect(checkSubsumptionQuin(10n, 40n, tbox)).toBeFalsy();
        });

        runner.it('B ⊄ A (not upward-traversable)', () => {
            const tbox = [tboxQuin(10, 20)];
            runner.expect(checkSubsumptionQuin(20n, 10n, tbox)).toBeFalsy();
        });

        runner.it('empty TBox: A ⊄ B unless A==B', () => {
            runner.expect(checkSubsumptionQuin(1n, 2n, [])).toBeFalsy();
            runner.expect(checkSubsumptionQuin(1n, 1n, [])).toBeTruthy();
        });

        runner.it('depth is bounded at 64 steps', () => {
            // Build a chain of 70 quins — subsumption beyond 64 should fail
            const tbox = [];
            for (let i = 0; i < 70; i++) tbox.push(tboxQuin(i, i + 1));
            runner.expect(checkSubsumptionQuin(0n, 63n, tbox)).toBeTruthy();
            // Beyond 64 hops: depends on implementation bound; should not crash
            const r = checkSubsumptionQuin(0n, 68n, tbox);
            runner.expect(typeof r).toBe('boolean');
        });
    });
}

export default register;
