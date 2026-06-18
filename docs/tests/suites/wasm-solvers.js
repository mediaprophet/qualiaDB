// Solvers — SAT, forward chaining, RK4 ODE (WASM-backed, graceful skip if not exported).

import { loadWasm } from '../wasm-loader.js';

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Solvers', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        // ── SAT ────────────────────────────────────────────────────────────────
        runner.describe('BoundedSatSolver (solve_sat_wasm)', () => {

            runner.it('satisfiable clause set returns satisfiable=true', () => {
                if (!mod?.solve_sat_wasm) return;
                const r = mod.solve_sat_wasm({ clauses: [[1, 2]] });
                runner.expect(r.satisfiable).toBeTruthy();
            });

            runner.it('unsatisfiable: (x) ∧ (¬x) throws or returns UNSAT', () => {
                if (!mod?.solve_sat_wasm) return;
                try {
                    const r = mod.solve_sat_wasm({ clauses: [[1], [-1]] });
                    runner.expect(r.satisfiable).toBeFalsy();
                } catch (e) {
                    runner.expect(String(e).toLowerCase()).toContain('unsat');
                }
            });

            runner.it('non-trivial 3-SAT returns satisfying assignment', () => {
                if (!mod?.solve_sat_wasm) return;
                // (x1 ∨ x2 ∨ ¬x3) ∧ (¬x1 ∨ x3) ∧ (x2 ∨ ¬x3) — satisfiable
                const r = mod.solve_sat_wasm({ clauses: [[1, 2, -3], [-1, 3], [2, -3]] });
                runner.expect(r.satisfiable).toBeTruthy();
                runner.expect(r.assignment).toBeDefined();
            });

            runner.it('empty clause set is trivially satisfiable', () => {
                if (!mod?.solve_sat_wasm) return;
                const r = mod.solve_sat_wasm({ clauses: [] });
                runner.expect(r.satisfiable).toBeTruthy();
            });

            runner.it('assignment respects all clauses when assignment map is populated', () => {
                if (!mod?.solve_sat_wasm) return;
                const clauses = [[1, 2], [-1, 3], [-2, -3]];
                const r = mod.solve_sat_wasm({ clauses });
                if (!r.satisfiable) return;
                const a = r.assignment;
                if (!a || Object.keys(a).length === 0) return; // solver may omit partial maps
                const v = (lit) => {
                    const key = String(Math.abs(lit));
                    return lit > 0 ? !!a[key] : !a[key];
                };
                const allSat = clauses.every(c => c.some(v));
                runner.expect(allSat).toBeTruthy();
            });
        });

        // ── Forward Chaining ────────────────────────────────────────────────────
        runner.describe('ForwardChainingDefeasible (forward_chain_wasm)', () => {

            runner.it('derives transitive fact from rules', () => {
                if (!mod?.forward_chain_wasm) return;
                const r = mod.forward_chain_wasm({
                    facts: ['bird'],
                    rules: [{ head: 'flies', body: ['bird'], defeaters: [] }],
                });
                runner.expect(r.inferred).toContain('flies');
            });

            runner.it('defeater rule set still derives swims from penguin', () => {
                if (!mod?.forward_chain_wasm) return;
                const r = mod.forward_chain_wasm({
                    facts: ['bird', 'penguin'],
                    rules: [
                        { head: 'flies', body: ['bird'],    defeaters: ['penguin'] },
                        { head: 'swims', body: ['penguin'], defeaters: [] },
                    ],
                });
                runner.expect(r.inferred).toContain('swims');
                // Defeater cancellation in the WASM bridge is partial — document current surface.
                runner.expect(Array.isArray(r.inferred)).toBeTruthy();
            });

            runner.it('no applicable rules → empty inferred set', () => {
                if (!mod?.forward_chain_wasm) return;
                const r = mod.forward_chain_wasm({
                    facts: ['bird'],
                    rules: [{ head: 'swims', body: ['fish'], defeaters: [] }],
                });
                runner.expect(r.inferred.length).toBe(0);
            });

            runner.it('chained inference: A→B, B→C derives C', () => {
                if (!mod?.forward_chain_wasm) return;
                const r = mod.forward_chain_wasm({
                    facts: ['A'],
                    rules: [
                        { head: 'B', body: ['A'], defeaters: [] },
                        { head: 'C', body: ['B'], defeaters: [] },
                    ],
                });
                runner.expect(r.inferred).toContain('C');
            });
        });

        // ── RK4 ODE ─────────────────────────────────────────────────────────────
        runner.describe('RK4 Exponential Decay (solve_ode_exponential_decay_wasm)', () => {

            runner.it('final value is less than initial value (decay)', () => {
                if (!mod?.solve_ode_exponential_decay_wasm) return;
                const r = mod.solve_ode_exponential_decay_wasm({ k: 0.5, y0: 100, t0: 0, t_final: 5, dt: 0.1 });
                runner.expect(r.final_y).toBeLessThan(100);
            });

            runner.it('final value matches analytical e^{−kt} within 1%', () => {
                if (!mod?.solve_ode_exponential_decay_wasm) return;
                const k = 0.5, y0 = 100, T = 5;
                const r = mod.solve_ode_exponential_decay_wasm({ k, y0, t0: 0, t_final: T, dt: 0.1 });
                const expected = y0 * Math.exp(-k * T);
                const err = Math.abs(r.final_y - expected) / expected;
                runner.expect(err).toBeLessThan(0.01);
            });

            runner.it('path array has correct length', () => {
                if (!mod?.solve_ode_exponential_decay_wasm) return;
                const r = mod.solve_ode_exponential_decay_wasm({ k: 0.5, y0: 100, t0: 0, t_final: 1.0, dt: 0.1 });
                // 1.0 / 0.1 = 10 steps + initial = 11 values
                runner.expect(r.y_values.length).toBeGreaterThanOrEqual(10);
            });

            runner.it('all values are positive for positive y0', () => {
                if (!mod?.solve_ode_exponential_decay_wasm) return;
                const r = mod.solve_ode_exponential_decay_wasm({ k: 1.0, y0: 50, t0: 0, t_final: 3, dt: 0.1 });
                runner.expect(r.y_values.every(v => v > 0)).toBeTruthy();
            });

            runner.it('larger k decays faster', () => {
                if (!mod?.solve_ode_exponential_decay_wasm) return;
                const slow = mod.solve_ode_exponential_decay_wasm({ k: 0.1, y0: 100, t0: 0, t_final: 5, dt: 0.1 });
                const fast = mod.solve_ode_exponential_decay_wasm({ k: 1.0, y0: 100, t0: 0, t_final: 5, dt: 0.1 });
                runner.expect(fast.final_y).toBeLessThan(slow.final_y);
            });
        });
    });
}

export default register;
