// Spatio-Temporal modality tests — Allen's Interval Algebra (7 relations).
// Mirrors crates/qualia-core-db/src/modalities/spatio_temporal.rs exactly.

import { TestRunner } from '../test-runner.js';

function evaluateTemporal(op, t1s, t1e, t2s, t2e) {
    switch (op) {
        case 'Before':    return t1e < t2s;
        case 'Meets':     return t1e === t2s;
        case 'Overlaps':  return t1s < t2s && t1e > t2s && t1e < t2e;
        case 'Starts':    return t1s === t2s && t1e < t2e;
        case 'During':    return t1s > t2s && t1e < t2e;
        case 'Finishes':  return t1e === t2e && t1s > t2s;
        case 'Equals':    return t1s === t2s && t1e === t2e;
    }
}

export function register(runner) {
    runner.describe('Modality: Spatio-Temporal (Allen\'s Interval Algebra)', () => {

        runner.describe('Before (t1e < t2s)', () => {
            runner.it('[1,5] Before [10,20] → true',  () => runner.expect(evaluateTemporal('Before', 1,5,10,20)).toBeTruthy());
            runner.it('[1,10] Before [10,20] → false (meets, not before)', () => runner.expect(evaluateTemporal('Before', 1,10,10,20)).toBeFalsy());
            runner.it('[5,15] Before [10,20] → false (overlaps)', () => runner.expect(evaluateTemporal('Before', 5,15,10,20)).toBeFalsy());
        });

        runner.describe('Meets (t1e == t2s)', () => {
            runner.it('[1,10] Meets [10,20] → true',  () => runner.expect(evaluateTemporal('Meets', 1,10,10,20)).toBeTruthy());
            runner.it('[1,9]  Meets [10,20] → false',  () => runner.expect(evaluateTemporal('Meets', 1,9,10,20)).toBeFalsy());
            runner.it('[1,11] Meets [10,20] → false',  () => runner.expect(evaluateTemporal('Meets', 1,11,10,20)).toBeFalsy());
        });

        runner.describe('Overlaps (t1s<t2s and t1e>t2s and t1e<t2e)', () => {
            runner.it('[5,15] Overlaps [10,20] → true',  () => runner.expect(evaluateTemporal('Overlaps', 5,15,10,20)).toBeTruthy());
            runner.it('[5,25] Overlaps [10,20] → false (extends past t2e)', () => runner.expect(evaluateTemporal('Overlaps', 5,25,10,20)).toBeFalsy());
            runner.it('[1,5]  Overlaps [10,20] → false (before)', () => runner.expect(evaluateTemporal('Overlaps', 1,5,10,20)).toBeFalsy());
        });

        runner.describe('Starts (t1s==t2s and t1e<t2e)', () => {
            runner.it('[10,15] Starts [10,20] → true',  () => runner.expect(evaluateTemporal('Starts', 10,15,10,20)).toBeTruthy());
            runner.it('[10,20] Starts [10,20] → false (equal)', () => runner.expect(evaluateTemporal('Starts', 10,20,10,20)).toBeFalsy());
            runner.it('[9,15]  Starts [10,20] → false (different start)', () => runner.expect(evaluateTemporal('Starts', 9,15,10,20)).toBeFalsy());
        });

        runner.describe('During (t1s>t2s and t1e<t2e)', () => {
            runner.it('[12,18] During [10,20] → true',  () => runner.expect(evaluateTemporal('During', 12,18,10,20)).toBeTruthy());
            runner.it('[10,18] During [10,20] → false (same start)', () => runner.expect(evaluateTemporal('During', 10,18,10,20)).toBeFalsy());
            runner.it('[12,20] During [10,20] → false (same end)', () => runner.expect(evaluateTemporal('During', 12,20,10,20)).toBeFalsy());
        });

        runner.describe('Finishes (t1e==t2e and t1s>t2s)', () => {
            runner.it('[15,20] Finishes [10,20] → true',  () => runner.expect(evaluateTemporal('Finishes', 15,20,10,20)).toBeTruthy());
            runner.it('[10,20] Finishes [10,20] → false (equal)', () => runner.expect(evaluateTemporal('Finishes', 10,20,10,20)).toBeFalsy());
            runner.it('[15,19] Finishes [10,20] → false (diff end)', () => runner.expect(evaluateTemporal('Finishes', 15,19,10,20)).toBeFalsy());
        });

        runner.describe('Equals (t1s==t2s and t1e==t2e)', () => {
            runner.it('[10,20] Equals [10,20] → true',  () => runner.expect(evaluateTemporal('Equals', 10,20,10,20)).toBeTruthy());
            runner.it('[10,21] Equals [10,20] → false',  () => runner.expect(evaluateTemporal('Equals', 10,21,10,20)).toBeFalsy());
            runner.it('[9,20]  Equals [10,20] → false',  () => runner.expect(evaluateTemporal('Equals', 9,20,10,20)).toBeFalsy());
        });

        runner.it('mutual exclusion: Before and Meets are distinct', () => {
            // [1,10] meets [10,20] but is NOT before [10,20]
            runner.expect(evaluateTemporal('Before', 1,10,10,20)).toBeFalsy();
            runner.expect(evaluateTemporal('Meets',  1,10,10,20)).toBeTruthy();
        });
    });
}

export default register;
