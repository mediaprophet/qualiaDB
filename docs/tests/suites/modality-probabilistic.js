// Probabilistic Logic + Diffusion modality tests.
// Mirrors crates/qualia-core-db/src/modalities/probabilistic.rs
// and crates/qualia-core-db/src/modalities/diffusion.rs.

import { TestRunner } from '../test-runner.js';

// ─── Probabilistic: evaluate_threshold ────────────────────────────────────────

function evaluateThreshold(weight, threshold) {
    return weight >= threshold;
}

// ─── Diffusion: trigger_diffusion ─────────────────────────────────────────────

function triggerDiffusion(_graphId) {
    // MVP: always returns true (matches Rust impl)
    return true;
}

export function register(runner) {
    runner.describe('Modality: Probabilistic Logic', () => {

        runner.it('weight > threshold → true', () => {
            runner.expect(evaluateThreshold(0.9, 0.5)).toBeTruthy();
        });

        runner.it('weight == threshold → true', () => {
            runner.expect(evaluateThreshold(0.5, 0.5)).toBeTruthy();
        });

        runner.it('weight < threshold → false', () => {
            runner.expect(evaluateThreshold(0.1, 0.5)).toBeFalsy();
        });

        runner.it('weight = 1.0, threshold = 1.0 → true', () => {
            runner.expect(evaluateThreshold(1.0, 1.0)).toBeTruthy();
        });

        runner.it('weight = 0.0, threshold = 0.0 → true', () => {
            runner.expect(evaluateThreshold(0.0, 0.0)).toBeTruthy();
        });

        runner.it('weight = 0.0, threshold = 0.001 → false', () => {
            runner.expect(evaluateThreshold(0.0, 0.001)).toBeFalsy();
        });
    });

    runner.describe('Modality: Diffusion Logic', () => {

        runner.it('trigger_diffusion returns true (MVP mock)', () => {
            runner.expect(triggerDiffusion('graph_001')).toBeTruthy();
        });

        runner.it('trigger_diffusion accepts any string graph ID', () => {
            runner.expect(triggerDiffusion('did:wellfare:user123/graph/A')).toBeTruthy();
        });

        runner.it('trigger_diffusion result is boolean', () => {
            runner.expect(typeof triggerDiffusion('')).toBe('boolean');
        });
    });
}

export default register;
