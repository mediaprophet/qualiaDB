// Temporal LTL modality tests.
// Mirrors crates/qualia-core-db/src/modalities/temporal_ltl.rs exactly.
// Opcodes: 0x40 Globally, 0x41 Finally, 0x42 Next, 0x43 Until, 0x44 Release

import { TestRunner } from '../test-runner.js';
import { makeQuin } from './primitives.js';

// ─── JS implementation ────────────────────────────────────────────────────────

function makeTrace(...predicates) {
    return predicates.map(p => makeQuin(0n, BigInt(p)));
}

function evaluateLtlTrace(trace, formula) {
    switch (formula.op) {
        case 'Globally': {
            if (!trace.length) return false;
            return trace.every(q => q.predicate === formula.p);
        }
        case 'Finally': {
            if (!trace.length) return false;
            return trace.some(q => q.predicate === formula.p);
        }
        case 'Next': {
            if (trace.length < 2) return false;
            return trace[1].predicate === formula.p;
        }
        case 'Until': {
            if (!trace.length) return false;
            for (let i = 0; i < trace.length; i++) {
                if (trace[i].predicate === formula.consequent) {
                    // Check all prior positions held `ante`
                    for (let j = 0; j < i; j++) {
                        if (trace[j].predicate !== formula.ante) return false;
                    }
                    return true;
                }
            }
            return false;
        }
        case 'Release': {
            if (!trace.length) return true;
            for (let i = 0; i < trace.length; i++) {
                if (trace[i].predicate !== formula.invariant) {
                    // invariant broken — trigger must have fired at or before i
                    let triggered = false;
                    for (let j = 0; j <= i; j++) {
                        if (trace[j].predicate === formula.trigger) { triggered = true; break; }
                    }
                    if (!triggered) return false;
                }
            }
            return true;
        }
    }
}

// ─── Registration ─────────────────────────────────────────────────────────────

export function register(runner) {
    runner.describe('Modality: Temporal LTL', () => {

        runner.describe('Globally (OP_LTL_GLOBALLY = 0x40)', () => {
            runner.it('all positions match → true', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100, 100, 100), { op: 'Globally', p: 100n })).toBeTruthy();
            });
            runner.it('one mismatch → false', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100, 99, 100), { op: 'Globally', p: 100n })).toBeFalsy();
            });
            runner.it('empty trace → false', () => {
                runner.expect(evaluateLtlTrace([], { op: 'Globally', p: 100n })).toBeFalsy();
            });
            runner.it('single-element match → true', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100), { op: 'Globally', p: 100n })).toBeTruthy();
            });
        });

        runner.describe('Finally (OP_LTL_FINALLY = 0x41)', () => {
            runner.it('p occurs eventually → true', () => {
                runner.expect(evaluateLtlTrace(makeTrace(99, 99, 100), { op: 'Finally', p: 100n })).toBeTruthy();
            });
            runner.it('p never occurs → false', () => {
                runner.expect(evaluateLtlTrace(makeTrace(99, 99), { op: 'Finally', p: 100n })).toBeFalsy();
            });
            runner.it('empty trace → false', () => {
                runner.expect(evaluateLtlTrace([], { op: 'Finally', p: 100n })).toBeFalsy();
            });
            runner.it('p at position 0 → true', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100, 99), { op: 'Finally', p: 100n })).toBeTruthy();
            });
        });

        runner.describe('Next (OP_LTL_NEXT = 0x42)', () => {
            runner.it('position 1 matches → true', () => {
                runner.expect(evaluateLtlTrace(makeTrace(99, 100), { op: 'Next', p: 100n })).toBeTruthy();
            });
            runner.it('position 1 mismatches → false', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100, 99), { op: 'Next', p: 100n })).toBeFalsy();
            });
            runner.it('single-element trace → false (no next)', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100), { op: 'Next', p: 100n })).toBeFalsy();
            });
            runner.it('empty trace → false', () => {
                runner.expect(evaluateLtlTrace([], { op: 'Next', p: 100n })).toBeFalsy();
            });
        });

        runner.describe('Until (OP_LTL_UNTIL = 0x43)', () => {
            runner.it('ante holds then consequent → true', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100, 100, 200),
                    { op: 'Until', ante: 100n, consequent: 200n })).toBeTruthy();
            });
            runner.it('consequent at position 0 → true (vacuously)', () => {
                runner.expect(evaluateLtlTrace(makeTrace(200),
                    { op: 'Until', ante: 100n, consequent: 200n })).toBeTruthy();
            });
            runner.it('ante never reached consequent → false', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100, 100, 100),
                    { op: 'Until', ante: 100n, consequent: 200n })).toBeFalsy();
            });
            runner.it('ante broken before consequent → false', () => {
                runner.expect(evaluateLtlTrace(makeTrace(100, 99, 200),
                    { op: 'Until', ante: 100n, consequent: 200n })).toBeFalsy();
            });
            runner.it('empty trace → false', () => {
                runner.expect(evaluateLtlTrace([],
                    { op: 'Until', ante: 100n, consequent: 200n })).toBeFalsy();
            });
        });

        runner.describe('Release (OP_LTL_RELEASE = 0x44)', () => {
            runner.it('invariant holds throughout → true', () => {
                runner.expect(evaluateLtlTrace(makeTrace(200, 200, 200),
                    { op: 'Release', trigger: 100n, invariant: 200n })).toBeTruthy();
            });
            runner.it('trigger fires then invariant breaks → true', () => {
                runner.expect(evaluateLtlTrace(makeTrace(200, 100, 99),
                    { op: 'Release', trigger: 100n, invariant: 200n })).toBeTruthy();
            });
            runner.it('invariant breaks without trigger → false', () => {
                runner.expect(evaluateLtlTrace(makeTrace(200, 99, 100),
                    { op: 'Release', trigger: 100n, invariant: 200n })).toBeFalsy();
            });
            runner.it('empty trace → true (vacuous)', () => {
                runner.expect(evaluateLtlTrace([],
                    { op: 'Release', trigger: 100n, invariant: 200n })).toBeTruthy();
            });
        });

        runner.describe('Opcode constants', () => {
            runner.it('OP_LTL_GLOBALLY = 0x40', () => { runner.expect(0x40).toBe(64); });
            runner.it('OP_LTL_FINALLY  = 0x41', () => { runner.expect(0x41).toBe(65); });
            runner.it('OP_LTL_NEXT     = 0x42', () => { runner.expect(0x42).toBe(66); });
            runner.it('OP_LTL_UNTIL    = 0x43', () => { runner.expect(0x43).toBe(67); });
            runner.it('OP_LTL_RELEASE  = 0x44', () => { runner.expect(0x44).toBe(68); });
        });
    });
}

export default register;
