//! Projectile via `PhysicsSimulationLibrary` (real ODE path).

use super::super::args;
use crate::specialized_libs::physics_simulation::PhysicsSimulationLibrary;
use poet_vibe::{Diagnostic, Span, Value};

pub fn projectile(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let v0 = args::rec_f64(args_v, "v0").unwrap_or(10.0);
    let angle = args::rec_f64(args_v, "angle_rad").unwrap_or(std::f64::consts::FRAC_PI_4);
    let g = args::rec_f64(args_v, "g").unwrap_or(9.81);
    let drag = args::rec_f64(args_v, "drag").unwrap_or(0.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(64) as usize;
    let max_time = args::rec_f64(args_v, "max_time").unwrap_or(10.0);
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_projectile_motion(v0, angle, g, drag, samples, max_time)
        .map_err(|e| args::bad(span, format!("projectile: {e:?}")))?;
    Ok(args::record([
        ("range", Value::F64(r.range)),
        ("max_height", Value::F64(r.max_height)),
        ("time_of_flight", Value::F64(r.time_of_flight)),
        ("landed", Value::Bool(r.landed)),
    ]))
}
