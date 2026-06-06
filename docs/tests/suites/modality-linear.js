// Linear Logic modality tests.
// Mirrors crates/qualia-core-db/src/modalities/linear.rs exactly.
// Resource consumption via metadata bit 59 (CONSUMED_BIT).

import { TestRunner } from '../test-runner.js';
import { makeQuin } from './primitives.js';

const CONSUMED_BIT = 1n << 59n;

function consumeQuin(q) {
    return { ...q, metadata: q.metadata | CONSUMED_BIT };
}

function isConsumed(q) {
    return (q.metadata & CONSUMED_BIT) !== 0n;
}

export function register(runner) {
    runner.describe('Modality: Linear Logic', () => {

        runner.it('fresh quin is not consumed', () => {
            runner.expect(isConsumed(makeQuin())).toBeFalsy();
        });

        runner.it('consumed quin has bit 59 set', () => {
            const q = consumeQuin(makeQuin());
            runner.expect(isConsumed(q)).toBeTruthy();
        });

        runner.it('consume is idempotent', () => {
            const q  = consumeQuin(makeQuin());
            const q2 = consumeQuin(q);
            runner.expect(isConsumed(q2)).toBeTruthy();
            runner.expect(q2.metadata).toBe(q.metadata);
        });

        runner.it('consume does not alter other metadata bits', () => {
            const q  = makeQuin(0n, 0n, 0n, 0n, 0b01n << 61n); // routing lane bits
            const c  = consumeQuin(q);
            runner.expect(c.metadata & (0b11n << 61n)).toBe(0b01n << 61n);
        });

        runner.it('CONSUMED_BIT = 1 << 59', () => {
            runner.expect(CONSUMED_BIT).toBe(576460752303423488n); // 2^59
        });

        runner.it('isConsumed with explicit metadata value', () => {
            const q = makeQuin(0n, 0n, 0n, 0n, CONSUMED_BIT | 42n);
            runner.expect(isConsumed(q)).toBeTruthy();
        });

        runner.it('non-consumed quin has bit 59 clear', () => {
            const q = makeQuin(0n, 0n, 0n, 0n, 42n);
            runner.expect(isConsumed(q)).toBeFalsy();
        });
    });
}

export default register;
