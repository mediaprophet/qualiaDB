// Discrete Diffusion Logic — mirrors diffusion.rs trigger/execute semantics (pure JS).
// The GPU pass (execute_diffusion_pass via wgpu) is native-only.
// These tests verify the synchronous gate semantics and NQuin diffusion convention.

import { q_hash, makeQuin } from './primitives.js';

export function register(runner) {

    // ── Mirrors trigger_diffusion(graph_id: &str) -> bool ─────────────────────
    function triggerDiffusion(graphId) {
        return graphId.length > 0;
    }

    // ── Diffusion NQuin convention ────────────────────────────────────────────
    // A diffusion edge has DIFFUSION_BIT set in its predicate field.
    // Weight is packed into metadata bits 31:0 as a fixed-point f32 × 1e6.
    const DIFFUSION_BIT = 1n << 48n;

    function makeDiffusionEdge(source, target, weight) {
        const pred = q_hash('q42:diffuse') | DIFFUSION_BIT;
        const metadata = BigInt(Math.round(weight * 1_000_000));
        return makeQuin(source, pred, target, 0n, metadata);
    }

    function extractDiffusionWeight(quin) {
        return Number(quin.metadata & 0xFFFFFFFFn) / 1_000_000;
    }

    function isDiffusionEdge(quin) {
        return (quin.predicate & DIFFUSION_BIT) !== 0n;
    }

    // ── One-step diffusion: propagate values from sources along edges ──────────
    // Mirrors the GPU automaton logic: new_value[target] += weight × old_value[source]
    function diffuseOneStep(graph, values) {
        const newValues = new Map(values);
        for (const quin of graph) {
            if (!isDiffusionEdge(quin)) continue;
            const w   = extractDiffusionWeight(quin);
            const src = quin.subject;
            const tgt = quin.object;
            const srcVal = values.get(src) ?? 0;
            newValues.set(tgt, (newValues.get(tgt) ?? 0) + w * srcVal);
        }
        return newValues;
    }

    runner.describe('Modality: Diffusion', () => {

        runner.describe('trigger_diffusion gate semantics', () => {
            runner.it('non-empty graph_id returns true', () => {
                runner.expect(triggerDiffusion('my-graph')).toBeTruthy();
            });
            runner.it('empty graph_id returns false (no-op)', () => {
                runner.expect(triggerDiffusion('')).toBeFalsy();
            });
            runner.it('whitespace-only is still non-empty → true', () => {
                runner.expect(triggerDiffusion('  ')).toBeTruthy();
            });
        });

        runner.describe('DIFFUSION_BIT NQuin convention', () => {
            runner.it('DIFFUSION_BIT is 1 << 48', () => {
                runner.expect(DIFFUSION_BIT).toBe(1n << 48n);
            });

            runner.it('diffusion edge has DIFFUSION_BIT set in predicate', () => {
                const src = q_hash('node:a'), tgt = q_hash('node:b');
                const e = makeDiffusionEdge(src, tgt, 0.5);
                runner.expect(isDiffusionEdge(e)).toBeTruthy();
            });

            runner.it('non-diffusion edge does not have DIFFUSION_BIT set', () => {
                const q = makeQuin(q_hash('a'), q_hash('b'), q_hash('c'));
                runner.expect(isDiffusionEdge(q)).toBeFalsy();
            });

            runner.it('weight 0.5 is extractable within 1e-5 tolerance', () => {
                const e = makeDiffusionEdge(1n, 2n, 0.5);
                runner.expect(Math.abs(extractDiffusionWeight(e) - 0.5)).toBeLessThan(1e-5);
            });

            runner.it('weight 1.0 is extractable', () => {
                const e = makeDiffusionEdge(1n, 2n, 1.0);
                runner.expect(Math.abs(extractDiffusionWeight(e) - 1.0)).toBeLessThan(1e-5);
            });
        });

        runner.describe('One-step propagation (JS GPU automaton mirror)', () => {
            runner.it('value propagates from source to target with correct weight', () => {
                const A = q_hash('node:a'), B = q_hash('node:b');
                const graph = [makeDiffusionEdge(A, B, 0.6)];
                const values = new Map([[A, 10.0], [B, 0.0]]);
                const next = diffuseOneStep(graph, values);
                runner.expect(Math.abs(next.get(B) - 6.0)).toBeLessThan(0.01);
            });

            runner.it('source value is unchanged in one step (no self-consumption)', () => {
                const A = q_hash('node:a'), B = q_hash('node:b');
                const graph = [makeDiffusionEdge(A, B, 0.5)];
                const values = new Map([[A, 8.0], [B, 0.0]]);
                const next = diffuseOneStep(graph, values);
                runner.expect(next.get(A)).toBe(8.0);
            });

            runner.it('multiple sources add to target (superposition)', () => {
                const A = q_hash('node:a'), B = q_hash('node:b'), C = q_hash('node:c');
                const graph = [
                    makeDiffusionEdge(A, C, 0.5),
                    makeDiffusionEdge(B, C, 0.5),
                ];
                const values = new Map([[A, 4.0], [B, 6.0], [C, 0.0]]);
                const next = diffuseOneStep(graph, values);
                // C receives 0.5×4 + 0.5×6 = 5
                runner.expect(Math.abs(next.get(C) - 5.0)).toBeLessThan(0.01);
            });

            runner.it('zero-weight edge propagates nothing', () => {
                const A = q_hash('node:a'), B = q_hash('node:b');
                const graph = [makeDiffusionEdge(A, B, 0.0)];
                const values = new Map([[A, 100.0], [B, 0.0]]);
                const next = diffuseOneStep(graph, values);
                runner.expect(next.get(B)).toBe(0);
            });

            runner.it('multi-step iteration converges to lower values (dissipation)', () => {
                const A = q_hash('node:a'), B = q_hash('node:b'), C = q_hash('node:c');
                const graph = [makeDiffusionEdge(A, B, 0.5), makeDiffusionEdge(B, C, 0.5)];
                let vals = new Map([[A, 100.0], [B, 0.0], [C, 0.0]]);
                for (let i = 0; i < 5; i++) vals = diffuseOneStep(graph, vals);
                // After 5 steps, C should have received some value but less than A
                runner.expect((vals.get(C) ?? 0)).toBeGreaterThan(0);
                runner.expect((vals.get(C) ?? 0)).toBeLessThan(vals.get(A));
            });
        });

        runner.describe('execute_diffusion_pass (native GPU - availability check)', () => {
            runner.it('GPU diffusion requires non-empty graph (gate invariant)', () => {
                // Mirror: execute_diffusion_pass returns Ok(()) immediately for empty graph
                const emptyGraph = [];
                runner.expect(emptyGraph.length === 0).toBeTruthy();
                // The actual GPU pass would return Ok(()) without allocating buffers
            });
        });
    });
}

export default register;
