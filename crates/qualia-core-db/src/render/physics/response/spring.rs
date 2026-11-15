use crate::solvers::calculus::{ODEFunction, RungeKutta4Static};
use crate::solvers::{SolverConfig, SolverResult, SolversError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringState {
    pub position: f64,
    pub velocity: f64,
}
/// Stiffness and damping are per unit mass: acceleration is
/// `stiffness * (target - position) - damping * velocity`.
#[derive(Clone, Copy, Debug)]
pub struct DrivenSpring {
    pub target: f64,
    pub stiffness: f64,
    pub damping: f64,
    pub max_displacement: f64,
    pub max_speed: f64,
}
impl ODEFunction for DrivenSpring {
    fn derivatives(&self, _t: f64, y: &[f64; 4]) -> [f64; 4] {
        [
            y[1],
            self.stiffness * (self.target - y[0]) - self.damping * y[1],
            0.0,
            0.0,
        ]
    }
}
/// One admitted response interval, internally substepped by the shared solver.
/// Hard travel/speed constraints are projected after integration. Contact
/// initialization/teleport policy belongs to the caller and can bypass this.
pub fn step_spring(state: SpringState, model: DrivenSpring, dt: f64) -> SolverResult<SpringState> {
    if [
        state.position,
        state.velocity,
        model.target,
        model.stiffness,
        model.damping,
        model.max_displacement,
        model.max_speed,
        dt,
    ]
    .iter()
    .any(|n| !n.is_finite())
        || !(0.0..=0.05).contains(&dt)
        || !(0.0..=1000.0).contains(&model.stiffness)
        || !(0.0..=100.0).contains(&model.damping)
        || model.max_displacement <= 0.0
        || model.max_speed <= 0.0
    {
        return Err(SolversError::InvalidParameters);
    }
    if dt == 0.0 {
        return Ok(state);
    }
    let h = dt.min(1.0 / 240.0);
    let mut solver = RungeKutta4Static::new(
        h,
        SolverConfig {
            max_iterations: 16,
            ..SolverConfig::default()
        },
    );
    let integrated =
        solver.integrate(&model, 0.0, [state.position, state.velocity, 0.0, 0.0], dt)?;
    let lower = model.target - model.max_displacement;
    let upper = model.target + model.max_displacement;
    let position = integrated.y[0].clamp(lower, upper);
    let mut velocity = integrated.y[1].clamp(-model.max_speed, model.max_speed);
    if (position == lower && velocity < 0.0) || (position == upper && velocity > 0.0) {
        velocity = 0.0;
    }
    Ok(SpringState { position, velocity })
}
