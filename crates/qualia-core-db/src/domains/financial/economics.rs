use rand_distr::{Distribution, StandardNormal};

#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
use rayon::prelude::*;

use crate::ode_solver::{rk4_step, PhysicalState};

/// Simulates a single path of Geometric Brownian Motion (GBM)
/// dS = mu * S * dt + sigma * S * dW
pub fn simulate_gbm_path(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
) -> f64 {
    let dt = time_horizon / steps as f64;
    let mut current_price = initial_price;
    let mut rng = rand::rng();

    for _ in 0..steps {
        let z: f64 = StandardNormal.sample(&mut rng);
        // Discrete GBM approximation
        current_price *=
            f64::exp((drift - 0.5 * volatility.powi(2)) * dt + volatility * f64::sqrt(dt) * z);
    }

    current_price
}

/// Runs a Monte Carlo simulation to calculate the expected end value
/// and the Value at Risk (VaR) at a 95% confidence interval.
/// Automatically utilizes Rayon parallel execution on Desktop/Server builds,
/// while degrading gracefully to a single thread on Android to preserve battery.
pub fn run_monte_carlo_var(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
    paths: usize,
) -> (f64, f64) {
    // Abstract execution iterator based on OS target
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    let mut final_prices: Vec<f64> = (0..paths)
        .into_par_iter()
        .map(|_| simulate_gbm_path(initial_price, drift, volatility, time_horizon, steps))
        .collect();

    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    let mut final_prices: Vec<f64> = (0..paths)
        .into_iter()
        .map(|_| simulate_gbm_path(initial_price, drift, volatility, time_horizon, steps))
        .collect();

    // Sort to find percentiles
    final_prices.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Calculate Expected Value (Mean)
    let mean: f64 = final_prices.iter().sum::<f64>() / paths as f64;

    // Calculate 95% VaR (Value at 5th percentile)
    let var_index = (paths as f64 * 0.05).floor() as usize;
    let var_95 = initial_price - final_prices[var_index]; // Potential loss

    (mean, var_95)
}

/// Evaluates a simple macroeconomic System Dynamics flow.
/// Models the Equation of Exchange (M*V = P*Q).
/// - state[0] = Money Supply (M)
/// - state[1] = Price Level (P)
pub fn simulate_macroeconomic_flow(
    initial_m: f64,
    initial_p: f64,
    velocity: f64,
    real_gdp: f64, // Q
    time_horizon: f64,
    steps: usize,
) -> PhysicalState {
    let dt = time_horizon / steps as f64;
    let mut state = PhysicalState {
        time: 0.0,
        values: vec![initial_m, initial_p],
    };

    // dy/dt
    let macro_derivative = |_t: f64, y: &[f64]| -> Vec<f64> {
        let current_m = y[0];
        let current_p = y[1];

        // M increases slowly due to inflation/printing (e.g. 2% growth)
        let dm_dt = current_m * 0.02;

        // P adapts based on P = (M * V) / Q
        // dP/dt nudges P towards the target price level
        let target_p = (current_m * velocity) / real_gdp;
        let dp_dt = 0.5 * (target_p - current_p); // Adjustment rate

        vec![dm_dt, dp_dt]
    };

    for _ in 0..steps {
        rk4_step(&mut state, dt, &macro_derivative);
    }

    state
}

/// Context regarding the physical state of the node
pub struct SystemContext {
    pub current_battery_level: f32,
    pub cpu_temperature: f32,
    pub network_congestion_index: f64,
}

/// Get mock system context
pub fn get_current_system_context() -> SystemContext {
    SystemContext {
        current_battery_level: 0.8,
        cpu_temperature: 45.0,
        network_congestion_index: 0.2,
    }
}

/// Calculates bandwidth liability in USD based on gb routed and context.
pub fn calculate_bandwidth_liability(bytes: usize, context: &SystemContext) -> f64 {
    let gb_routed = bytes as f64 / 1_073_741_824.0;
    let mut base_rate = 0.05; // .05 per GB

    // Dynamically adjust based on system context
    base_rate += context.network_congestion_index * 0.05;

    // Low battery -> demands higher compensation
    if context.current_battery_level < 0.2 {
        base_rate += 0.05;
    }

    // High temperature -> throttling penalty
    if context.cpu_temperature > 70.0 {
        base_rate += 0.02;
    }

    gb_routed * base_rate
}

// ─── Resilience economics: supply-shock propagation + survival-first pricing ─────
//
// These two models implement the resilience-economics scope: map the *local* fallout
// of a macro disruption onto dependent sectors (food/fuel security), and price an
// autonomous basecamp's internal resources to guarantee survival before any external
// trade. Both are bounded, **zero-heap** (fixed `MAX_SECTORS` stack buffers) — the
// rest of this module's Monte-Carlo / RK4 code is cold-path heap, but these are not.
//
// Honest scope: `propagate_supply_shock` does NOT predict geopolitics. It is standard
// Leontief input-output analysis — GIVEN an inter-sector coupling matrix and a shock
// vector, it computes the propagated total impact. The geopolitical judgement (what
// the shock IS) stays with the human; the engine carries the propagation arithmetic.

/// Maximum sectors in a bounded input-output (Leontief) model.
pub const MAX_SECTORS: usize = 32;

/// Propagate a supply/geopolitical shock through an inter-sector input-output
/// (Leontief) coupling matrix to its total downstream impact.
///
/// `coupling[i*n + j]` is the technical coefficient `a_ij` — the units of sector `i`'s
/// output consumed as direct input per unit of sector `j`'s output. `shock[j]` is the
/// initial direct disruption to sector `j` (e.g. a shipping-strait closure cutting
/// imported fuel). The total impact is the Neumann series of the Leontief inverse
/// `(I − A)⁻¹·shock = shock + A·shock + A²·shock + …`, iterated until the round's
/// contribution falls below `tolerance` (L1) or `max_rounds` is hit. Converges when
/// the matrix is productive (column sums < 1). Returns the number of rounds performed.
/// To read "fuel/food security" impact, inspect the corresponding entries of `impact_out`.
pub fn propagate_supply_shock(
    coupling: &[f64],
    shock: &[f64],
    n: usize,
    max_rounds: u32,
    tolerance: f64,
    impact_out: &mut [f64],
) -> usize {
    if n == 0
        || n > MAX_SECTORS
        || coupling.len() < n * n
        || shock.len() < n
        || impact_out.len() < n
    {
        return 0;
    }
    let mut term = [0.0f64; MAX_SECTORS];
    let mut next = [0.0f64; MAX_SECTORS];
    // k = 0 term: the direct shock itself.
    for i in 0..n {
        term[i] = shock[i];
        impact_out[i] = shock[i];
    }
    let mut rounds = 0usize;
    for _ in 0..max_rounds {
        rounds += 1;
        // next = A · term
        let mut l1 = 0.0f64;
        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..n {
                acc += coupling[i * n + j] * term[j];
            }
            next[i] = acc;
            l1 += acc.abs();
        }
        for i in 0..n {
            impact_out[i] += next[i];
            term[i] = next[i];
        }
        if l1 < tolerance {
            break;
        }
    }
    rounds
}

/// Survival-first internal resource pricing for an autonomous resilience basecamp.
///
/// For each resource: if the `stock` does not cover the `survival_demand` (the amount
/// that MUST be met for the community to survive), the internal shadow price rises
/// toward `survival_premium × production_cost` as coverage falls to zero — a scarcity
/// signal that reserves the resource for survival rather than trade. Once survival
/// demand is met, the survival portion is reserved and only the genuine surplus
/// (`stock − survival_demand`) is exposed as tradeable, priced at marginal
/// `production_cost`. Writes prices into `price_out` and tradeable surplus into
/// `tradeable_surplus_out`; returns the count. Zero-heap.
pub fn resilience_resource_pricing(
    stock: &[f64],
    survival_demand: &[f64],
    production_cost: &[f64],
    survival_premium: f64,
    n: usize,
    price_out: &mut [f64],
    tradeable_surplus_out: &mut [f64],
) -> usize {
    let count = n
        .min(stock.len())
        .min(survival_demand.len())
        .min(production_cost.len())
        .min(price_out.len())
        .min(tradeable_surplus_out.len());
    for i in 0..count {
        let demand = survival_demand[i];
        let cost = production_cost[i];
        if demand <= 0.0 {
            // No survival requirement: everything is tradeable at marginal cost.
            price_out[i] = cost;
            tradeable_surplus_out[i] = stock[i].max(0.0);
            continue;
        }
        let coverage = stock[i] / demand;
        if coverage < 1.0 {
            // Deficit: linear premium from cost (coverage=1) up to survival_premium×cost (coverage=0).
            let c = coverage.clamp(0.0, 1.0);
            price_out[i] = cost * (1.0 + (survival_premium - 1.0).max(0.0) * (1.0 - c));
            tradeable_surplus_out[i] = 0.0;
        } else {
            // Survival met: reserve the survival demand, expose the surplus at marginal cost.
            price_out[i] = cost;
            tradeable_surplus_out[i] = stock[i] - demand;
        }
    }
    count
}

#[cfg(test)]
mod resilience_tests {
    use super::*;

    #[test]
    fn supply_shock_propagates_to_dependent_sectors() {
        // 2-sector economy, each consuming 50% of the other's output.
        // A = [[0, 0.5], [0.5, 0]]; shock the first sector (e.g. imported fuel cut).
        // Leontief inverse (I−A)⁻¹ = 1/0.75 · [[1,0.5],[0.5,1]] → impact = [1.333…, 0.667…].
        let a = [0.0, 0.5, 0.5, 0.0];
        let shock = [1.0, 0.0];
        let mut impact = [0.0f64; 2];
        let rounds = propagate_supply_shock(&a, &shock, 2, 100, 1e-9, &mut impact);
        assert!(rounds > 1);
        assert!(
            (impact[0] - 4.0 / 3.0).abs() < 1e-6,
            "sector 0 total impact, got {}",
            impact[0]
        );
        // Sector 1 was NOT directly shocked but is impacted via the supply coupling.
        assert!(
            impact[1] > 0.6 && impact[1] < 0.7,
            "propagated impact on sector 1, got {}",
            impact[1]
        );
    }

    #[test]
    fn supply_shock_rejects_bad_dimensions() {
        let mut out = [0.0f64; 2];
        assert_eq!(
            propagate_supply_shock(&[0.0], &[1.0], 0, 10, 1e-9, &mut out),
            0
        );
        // n exceeds the matrix size
        assert_eq!(
            propagate_supply_shock(&[0.0; 4], &[1.0, 0.0], 3, 10, 1e-9, &mut out),
            0
        );
    }

    #[test]
    fn resilience_pricing_prioritizes_survival() {
        // r0: deficit (stock 5 vs survival 10 → coverage 0.5), cost 2, premium 3.
        //     price = 2·(1 + (3−1)·(1−0.5)) = 2·2 = 4; no tradeable surplus.
        // r1: surplus (stock 20 vs survival 10), cost 2 → price = cost = 2; surplus = 10.
        let stock = [5.0, 20.0];
        let demand = [10.0, 10.0];
        let cost = [2.0, 2.0];
        let mut price = [0.0f64; 2];
        let mut surplus = [0.0f64; 2];
        let n =
            resilience_resource_pricing(&stock, &demand, &cost, 3.0, 2, &mut price, &mut surplus);
        assert_eq!(n, 2);
        assert!(
            (price[0] - 4.0).abs() < 1e-9,
            "deficit survival-premium price, got {}",
            price[0]
        );
        assert_eq!(
            surplus[0], 0.0,
            "a deficit resource exposes no tradeable surplus"
        );
        assert!(
            (price[1] - 2.0).abs() < 1e-9,
            "surplus priced at marginal cost, got {}",
            price[1]
        );
        assert!(
            (surplus[1] - 10.0).abs() < 1e-9,
            "tradeable surplus = stock − survival demand"
        );
    }
}
