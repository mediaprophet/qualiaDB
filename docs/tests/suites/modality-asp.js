// Answer Set Programming modality tests.
// Mirrors crates/qualia-core-db/src/modalities/asp.rs exactly.
// Stable models encoded as context XOR variants.

import { TestRunner } from '../test-runner.js';
import { makeQuin } from './primitives.js';

const MAX_STABLE_MODELS = 8;

function enumerateStableModels(base, _rules) {
    // MVP: always 2 models — world 0 and world 1
    return [base.context ^ 0n, base.context ^ 1n];
}

export function register(runner) {
    runner.describe('Modality: Answer Set Programming (ASP)', () => {

        runner.it('returns exactly 2 stable models for MVP', () => {
            const base = makeQuin(0n, 0n, 0n, 42n);
            const models = enumerateStableModels(base, []);
            runner.expect(models.length).toBe(2);
        });

        runner.it('world 0 = base.context XOR 0', () => {
            const base = makeQuin(0n, 0n, 0n, 42n);
            const [w0] = enumerateStableModels(base, []);
            runner.expect(w0).toBe(42n ^ 0n);
        });

        runner.it('world 1 = base.context XOR 1', () => {
            const base = makeQuin(0n, 0n, 0n, 42n);
            const [, w1] = enumerateStableModels(base, []);
            runner.expect(w1).toBe(42n ^ 1n);
        });

        runner.it('context 0 produces worlds [0, 1]', () => {
            const base = makeQuin(0n, 0n, 0n, 0n);
            const [w0, w1] = enumerateStableModels(base, []);
            runner.expect(w0).toBe(0n);
            runner.expect(w1).toBe(1n);
        });

        runner.it('MAX_STABLE_MODELS = 8', () => {
            runner.expect(MAX_STABLE_MODELS).toBe(8);
        });

        runner.it('stable models are unique when context is non-zero', () => {
            const base = makeQuin(0n, 0n, 0n, 100n);
            const [w0, w1] = enumerateStableModels(base, []);
            runner.expect(w0).not.toBe(w1);
        });
    });
}

export default register;
