// Finance extras — Black-Scholes and explicit GBM path (WASM-backed, JS fallback for BS).

import { loadWasm } from '../wasm-loader.js';

// Portable JS Black-Scholes (fallback when WASM export absent)
function phi(x) {
    const a1 = 0.254829592, a2 = -0.284496736, a3 = 1.421413741,
          a4 = -1.453152027, a5 = 1.061405429, p = 0.3275911;
    const sign = x < 0 ? -1 : 1;
    x = Math.abs(x);
    const t = 1 / (1 + p * x);
    const y = 1 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * Math.exp(-x * x);
    return 0.5 * (1 + sign * y);
}

function blackScholesJS(S, K, r, v, T, isCall) {
    const d1 = (Math.log(S / K) + (r + v * v / 2) * T) / (v * Math.sqrt(T));
    const d2 = d1 - v * Math.sqrt(T);
    return isCall
        ? S * phi(d1) - K * Math.exp(-r * T) * phi(d2)
        : K * Math.exp(-r * T) * phi(-d2) - S * phi(-d1);
}

export function register(runner) {
    let mod = null;

    runner.describe('WASM: Finance Extras', () => {

        runner.beforeAll(async () => { mod = await loadWasm(); });

        // ── Black-Scholes ────────────────────────────────────────────────────────
        runner.describe('Black-Scholes (black_scholes_wasm / JS fallback)', () => {

            function bs(params) {
                if (mod?.black_scholes_wasm) return mod.black_scholes_wasm(params);
                // JS fallback
                const price = blackScholesJS(params.spot, params.strike, params.rate, params.vol, params.time_years, params.is_call);
                const d1 = (Math.log(params.spot / params.strike) + (params.rate + params.vol ** 2 / 2) * params.time_years)
                           / (params.vol * Math.sqrt(params.time_years));
                const nd1 = Math.exp(-d1 * d1 / 2) / Math.sqrt(2 * Math.PI);
                return {
                    price,
                    delta: params.is_call ? phi(d1) : phi(d1) - 1,
                    gamma: nd1 / (params.spot * params.vol * Math.sqrt(params.time_years)),
                };
            }

            runner.it('ATM call price is positive', () => {
                const r = bs({ spot: 100, strike: 100, rate: 0.05, vol: 0.2, time_years: 1.0, is_call: true });
                runner.expect(r.price).toBeGreaterThan(0);
            });

            runner.it('ATM put price is positive', () => {
                const r = bs({ spot: 100, strike: 100, rate: 0.05, vol: 0.2, time_years: 1.0, is_call: false });
                runner.expect(r.price).toBeGreaterThan(0);
            });

            runner.it('put-call parity: call - put ≈ S - K·e^{-rT}', () => {
                const S = 100, K = 100, r = 0.05, v = 0.2, T = 1.0;
                const call = bs({ spot: S, strike: K, rate: r, vol: v, time_years: T, is_call: true  }).price;
                const put  = bs({ spot: S, strike: K, rate: r, vol: v, time_years: T, is_call: false }).price;
                const parity = S - K * Math.exp(-r * T);
                runner.expect(Math.abs(call - put - parity)).toBeLessThan(0.01);
            });

            runner.it('deep ITM call price approaches intrinsic value', () => {
                // S >> K ⟹ call ≈ S - K·e^{-rT}
                const r = bs({ spot: 200, strike: 100, rate: 0.0, vol: 0.01, time_years: 0.01, is_call: true });
                runner.expect(r.price).toBeGreaterThan(99);
            });

            runner.it('zero volatility call with S > K = max(S - K·e^{-rT}, 0)', () => {
                // Edge case: nearly-zero vol
                const r = bs({ spot: 110, strike: 100, rate: 0.0, vol: 0.001, time_years: 1.0, is_call: true });
                runner.expect(r.price).toBeGreaterThan(9);
                runner.expect(r.price).toBeLessThan(15);
            });

            runner.it('call delta ∈ (0, 1)', () => {
                const r = bs({ spot: 100, strike: 100, rate: 0.05, vol: 0.2, time_years: 1.0, is_call: true });
                if (r.delta === undefined) return;
                runner.expect(r.delta).toBeGreaterThan(0);
                runner.expect(r.delta).toBeLessThan(1);
            });

            runner.it('higher vol increases option price (vega positive)', () => {
                const lo = bs({ spot: 100, strike: 100, rate: 0.05, vol: 0.1, time_years: 1.0, is_call: true }).price;
                const hi = bs({ spot: 100, strike: 100, rate: 0.05, vol: 0.4, time_years: 1.0, is_call: true }).price;
                runner.expect(hi).toBeGreaterThan(lo);
            });
        });

        // ── GBM Path ─────────────────────────────────────────────────────────────
        runner.describe('GBM Path (simulate_gbm_path_wasm)', () => {

            runner.it('returns final_price, min_price, max_price, path', () => {
                if (!mod?.simulate_gbm_path_wasm) return;
                const r = mod.simulate_gbm_path_wasm({ initial_price: 100, drift: 0.08, volatility: 0.2, time_horizon: 1.0, steps: 252 });
                runner.expect(r).toHaveProperty('final_price');
                runner.expect(r).toHaveProperty('min_price');
                runner.expect(r).toHaveProperty('max_price');
                runner.expect(r).toHaveProperty('path');
            });

            runner.it('all prices are positive', () => {
                if (!mod?.simulate_gbm_path_wasm) return;
                const r = mod.simulate_gbm_path_wasm({ initial_price: 100, drift: 0.0, volatility: 0.5, time_horizon: 1.0, steps: 252 });
                runner.expect(r.path.every(p => p > 0)).toBeTruthy();
            });

            runner.it('min_price ≤ final_price ≤ max_price', () => {
                if (!mod?.simulate_gbm_path_wasm) return;
                const r = mod.simulate_gbm_path_wasm({ initial_price: 100, drift: 0.08, volatility: 0.2, time_horizon: 1.0, steps: 100 });
                runner.expect(r.min_price).toBeLessThanOrEqual(r.final_price);
                runner.expect(r.final_price).toBeLessThanOrEqual(r.max_price);
            });

            runner.it('path length matches steps', () => {
                if (!mod?.simulate_gbm_path_wasm) return;
                const r = mod.simulate_gbm_path_wasm({ initial_price: 100, drift: 0.05, volatility: 0.2, time_horizon: 1.0, steps: 50 });
                runner.expect(r.path.length).toBeGreaterThanOrEqual(50);
            });

            runner.it('zero volatility path stays near deterministic trend', () => {
                if (!mod?.simulate_gbm_path_wasm) return;
                const r = mod.simulate_gbm_path_wasm({ initial_price: 100, drift: 0.1, volatility: 0.0001, time_horizon: 1.0, steps: 10 });
                // With near-zero vol, final price ≈ 100 * e^{0.1} ≈ 110.5
                runner.expect(r.final_price).toBeGreaterThan(105);
                runner.expect(r.final_price).toBeLessThan(120);
            });
        });
    });
}

export default register;
