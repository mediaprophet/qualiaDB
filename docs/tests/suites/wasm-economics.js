// WASM Economics tests.
// Covers Monte Carlo VaR simulation (run_semantic_simulation) and
// quantum DFT receptor binding prediction (predict_receptor_binding_wasm).

import { TestRunner } from '../test-runner.js';
import { loadWasm } from '../wasm-loader.js';

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Economics', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        runner.describe('Monte Carlo VaR (run_semantic_simulation)', () => {

            runner.it('returns mean and value_at_risk fields', () => {
                if (!mod.run_semantic_simulation) return;
                const r = mod.run_semantic_simulation({
                    initial_price: 100.0, drift: 0.05, volatility: 0.2,
                    time_horizon: 1, simulation_steps: 252,
                });
                runner.expect(r).toHaveProperty('mean');
                runner.expect(r).toHaveProperty('value_at_risk');
            });

            runner.it('mean is positive for positive drift', () => {
                if (!mod.run_semantic_simulation) return;
                const r = mod.run_semantic_simulation({
                    initial_price: 100.0, drift: 0.10, volatility: 0.1,
                    time_horizon: 1, simulation_steps: 252,
                });
                runner.expect(r.mean).toBeGreaterThan(0);
            });

            runner.it('VaR is less than initial price (loss quantile)', () => {
                if (!mod.run_semantic_simulation) return;
                const r = mod.run_semantic_simulation({
                    initial_price: 100.0, drift: 0.00, volatility: 0.3,
                    time_horizon: 1, simulation_steps: 252,
                });
                runner.expect(r.value_at_risk).toBeLessThan(100.0);
            });

            runner.it('higher volatility increases VaR loss', () => {
                if (!mod.run_semantic_simulation) return;
                const low = mod.run_semantic_simulation({
                    initial_price: 100.0, drift: 0.0, volatility: 0.05,
                    time_horizon: 1, simulation_steps: 252,
                });
                const high = mod.run_semantic_simulation({
                    initial_price: 100.0, drift: 0.0, volatility: 0.50,
                    time_horizon: 1, simulation_steps: 252,
                });
                runner.expect(high.value_at_risk).toBeLessThan(low.value_at_risk);
            });
        });

        runner.describe('Quantum DFT Receptor Binding (predict_receptor_binding_wasm)', () => {

            runner.it('returns a finite number', () => {
                if (!mod.predict_receptor_binding_wasm) return;
                const r = mod.predict_receptor_binding_wasm();
                runner.expect(typeof r).toBe('number');
                runner.expect(isFinite(r)).toBeTruthy();
            });

            runner.it('binding affinity is negative (attractive = kcal/mol convention)', () => {
                if (!mod.predict_receptor_binding_wasm) return;
                const r = mod.predict_receptor_binding_wasm();
                // Negative = stronger binding (kcal/mol convention)
                runner.expect(r).toBeLessThan(0);
            });
        });
    });
}

export default register;
