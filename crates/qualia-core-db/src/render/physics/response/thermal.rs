//! Driven thermal reservoirs using the canonical fixed-state ODE kernel.
use crate::solvers::calculus::{ODEFunction, RungeKutta4Static};
use crate::solvers::{SolverConfig, SolverResult, SolversError};
#[derive(Clone, Copy, Debug)]
pub struct ThermalTargets {
    pub temperatures: [f64; 2],
    pub rates: [f64; 2],
}
impl ODEFunction for ThermalTargets {
    fn derivatives(&self, _t: f64, y: &[f64; 4]) -> [f64; 4] {
        [
            (self.temperatures[0] - y[0]) * self.rates[0],
            (self.temperatures[1] - y[1]) * self.rates[1],
            0.0,
            0.0,
        ]
    }
}
/// Rates are inverse seconds. Selecting/calibrating heat-balance targets is a
/// caller/domain-model responsibility; integration itself is engine-owned.
pub fn step_thermal(
    temperatures: [f64; 2],
    targets: ThermalTargets,
    dt: f64,
) -> SolverResult<[f64; 2]> {
    if temperatures
        .iter()
        .chain(targets.temperatures.iter())
        .chain(targets.rates.iter())
        .any(|n| !n.is_finite())
        || !dt.is_finite()
        || !(0.0..=0.05).contains(&dt)
        || targets.rates.iter().any(|n| !(0.0..=10.0).contains(n))
    {
        return Err(SolversError::InvalidParameters);
    }
    if dt == 0.0 {
        return Ok(temperatures);
    }
    let mut solver = RungeKutta4Static::new(
        dt.min(1.0 / 240.0),
        SolverConfig {
            max_iterations: 16,
            ..SolverConfig::default()
        },
    );
    let state = solver.integrate(
        &targets,
        0.0,
        [temperatures[0], temperatures[1], 0.0, 0.0],
        dt,
    )?;
    Ok([state.y[0], state.y[1]])
}
