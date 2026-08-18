//! EMF `capability.invoke` wrappers — interference, attenuation, Doppler, field grid.
//!
//! These marshal Vibe `Value` args into the `PhysicsSimulationLibrary` EMF
//! solvers and shape results back into `Value::Record`.
//! All functions are **classical simulations** — no QPU required.

use super::super::args;
use crate::modalities::manifold::ManifoldCoordinate10D;
use crate::specialized_libs::physics_simulation::{EmfSource, PhysicsSimulationLibrary};
use poet_vibe::{Diagnostic, Span, Value};

/// Convert a `ManifoldCoordinate10D` to a Vibe `Record`.
fn manifold_to_value(m: &ManifoldCoordinate10D) -> Value {
    args::record([
        ("scale", Value::F64(m.scale as f64)),
        ("attention_depth", Value::F64(m.attention_depth as f64)),
        ("epistemic_weight", Value::F64(m.epistemic_weight as f64)),
        ("topological_spin", Value::F64(m.topological_spin as f64)),
        ("temporal_decay", Value::F64(m.temporal_decay as f64)),
        ("entropy_bias", Value::F64(m.entropy_bias as f64)),
        ("spatial_phase", Value::F64(m.spatial_phase as f64)),
        ("recurrence_frequency", Value::F64(m.recurrence_frequency as f64)),
        ("density_threshold", Value::F64(m.density_threshold as f64)),
        ("manifold_curvature", Value::F64(m.manifold_curvature as f64)),
    ])
}

/// Parse sources from a Vibe `Value` (flat list of [x,y,z,A,f,φ, ...]).
fn parse_sources(v: &Value, span: Span) -> Result<Vec<EmfSource>, Diagnostic> {
    let flat = args::rec_f64_list(v, "sources").ok_or_else(|| {
        args::bad(span, "needs { sources: [x,y,z,A,f,φ, ...], ... }")
    })?;
    EmfSource::parse_flat(&flat)
        .map_err(|e| args::bad(span, format!("sources: {e}")))
}

/// `Physics.emf_interference` — superposition of N EMF sources at a 3D point.
pub fn emf_interference(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sources = parse_sources(args_v, span)?;
    let x = args::rec_f64(args_v, "x").unwrap_or(0.0);
    let y = args::rec_f64(args_v, "y").unwrap_or(0.0);
    let z = args::rec_f64(args_v, "z").unwrap_or(0.0);
    let t = args::rec_f64(args_v, "t").unwrap_or(0.0);
    let c = args::rec_f64(args_v, "c").unwrap_or(0.0);
    let lib = PhysicsSimulationLibrary::new();
    let r = lib
        .run_emf_interference(&sources, x, y, z, t, c)
        .map_err(|e| args::bad(span, format!("emf_interference: {e:?}")))?;
    Ok(args::record([
        ("instantaneous_value", Value::F64(r.instantaneous_value)),
        ("amplitude", Value::F64(r.amplitude)),
        ("phase", Value::F64(r.phase)),
        ("frequency_effective", Value::F64(r.frequency_effective)),
        ("num_sources", Value::U64(r.num_sources as u64)),
    ]))
}

/// `Physics.emf_attenuation` — inverse-square + atmospheric absorption.
pub fn emf_attenuation(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let source_power = args::rec_f64(args_v, "source_power").ok_or_else(|| {
        args::bad(span, "emf_attenuation needs { source_power, frequency, distance, absorption_coeff? }")
    })?;
    let frequency = args::rec_f64(args_v, "frequency").unwrap_or(1e9);
    let distance = args::rec_f64(args_v, "distance").unwrap_or(1.0);
    let absorption_coeff = args::rec_f64(args_v, "absorption_coeff").unwrap_or(0.0);
    let lib = PhysicsSimulationLibrary::new();
    let r = lib
        .run_emf_attenuation(source_power, frequency, distance, absorption_coeff)
        .map_err(|e| args::bad(span, format!("emf_attenuation: {e:?}")))?;
    Ok(args::record([
        ("received_power", Value::F64(r.received_power)),
        ("attenuation_db", Value::F64(r.attenuation_db)),
        ("free_space_loss_db", Value::F64(r.free_space_loss_db)),
        ("absorption_loss_db", Value::F64(r.absorption_loss_db)),
        ("distance", Value::F64(r.distance)),
        ("frequency", Value::F64(r.frequency)),
    ]))
}

/// `Physics.doppler_shift` — relativistic Doppler effect.
pub fn doppler_shift(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let source_frequency = args::rec_f64(args_v, "source_frequency").ok_or_else(|| {
        args::bad(span, "doppler_shift needs { source_frequency, relative_velocity, c? }")
    })?;
    let relative_velocity = args::rec_f64(args_v, "relative_velocity").unwrap_or(0.0);
    let c = args::rec_f64(args_v, "c").unwrap_or(0.0);
    let lib = PhysicsSimulationLibrary::new();
    let r = lib
        .run_doppler_shift(source_frequency, relative_velocity, c)
        .map_err(|e| args::bad(span, format!("doppler_shift: {e:?}")))?;
    Ok(args::record([
        ("observed_frequency", Value::F64(r.observed_frequency)),
        ("shift_ratio", Value::F64(r.shift_ratio)),
        ("relative_velocity", Value::F64(r.relative_velocity)),
        ("beta", Value::F64(r.beta)),
    ]))
}

/// `Physics.emf_field_grid_3d` — 4D physics grid (x×y×z×t) with 10D manifold tags.
pub fn emf_field_grid_3d(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sources = parse_sources(args_v, span)?;
    let bounds_flat = args::rec_f64_list(args_v, "bounds").ok_or_else(|| {
        args::bad(span, "emf_field_grid_3d needs { bounds: [x_min,x_max,y_min,y_max,z_min,z_max] }")
    })?;
    if bounds_flat.len() != 6 {
        return Err(args::bad(span, "bounds must have exactly 6 elements"));
    }
    let bounds = [
        bounds_flat[0], bounds_flat[1], bounds_flat[2],
        bounds_flat[3], bounds_flat[4], bounds_flat[5],
    ];
    let nx = args::rec_u64(args_v, "nx").unwrap_or(4) as usize;
    let ny = args::rec_u64(args_v, "ny").unwrap_or(4) as usize;
    let nz = args::rec_u64(args_v, "nz").unwrap_or(4) as usize;
    let nt = args::rec_u64(args_v, "nt").unwrap_or(2) as usize;
    let t_start = args::rec_f64(args_v, "t_start").unwrap_or(0.0);
    let t_end = args::rec_f64(args_v, "t_end").unwrap_or(1.0);
    let c = args::rec_f64(args_v, "c").unwrap_or(0.0);
    let lib = PhysicsSimulationLibrary::new();
    let r = lib
        .run_emf_field_grid_3d(&sources, bounds, nx, ny, nz, nt, t_start, t_end, c)
        .map_err(|e| args::bad(span, format!("emf_field_grid_3d: {e:?}")))?;
    let manifold_values: Vec<Value> = r.manifold_coords.iter().map(manifold_to_value).collect();
    Ok(args::record([
        ("nx", Value::U64(r.nx as u64)),
        ("ny", Value::U64(r.ny as u64)),
        ("nz", Value::U64(r.nz as u64)),
        ("nt", Value::U64(r.nt as u64)),
        ("bounds", args::f64_list_value(r.bounds.iter().copied())),
        ("times", args::f64_list_value(r.times.iter().copied())),
        ("amplitudes", args::f64_list_value(r.amplitudes.iter().copied())),
        ("phases", args::f64_list_value(r.phases.iter().copied())),
        ("frequencies", args::f64_list_value(r.frequencies.iter().copied())),
        ("manifold_coords", Value::List(manifold_values)),
        ("num_sources", Value::U64(r.num_sources as u64)),
    ]))
}

/// `Physics.emf_sample_at_depth` — depth-aware sampling for render integration.
pub fn emf_sample_at_depth(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sources = parse_sources(args_v, span)?;
    let camera = args::rec_f64_list(args_v, "camera").ok_or_else(|| {
        args::bad(span, "emf_sample_at_depth needs { camera: [cx,cy,cz], direction: [dx,dy,dz], depths: [d1,...] }")
    })?;
    let direction = args::rec_f64_list(args_v, "direction").unwrap_or_else(|| vec![0.0, 0.0, 1.0]);
    let depths = args::rec_f64_list(args_v, "depths").unwrap_or_else(|| vec![1.0]);
    let t = args::rec_f64(args_v, "t").unwrap_or(0.0);
    let c = args::rec_f64(args_v, "c").unwrap_or(0.0);
    if camera.len() != 3 {
        return Err(args::bad(span, "camera must have exactly 3 elements"));
    }
    if direction.len() != 3 {
        return Err(args::bad(span, "direction must have exactly 3 elements"));
    }
    let lib = PhysicsSimulationLibrary::new();
    let r = lib
        .run_emf_sample_at_depth(
            &sources,
            [camera[0], camera[1], camera[2]],
            [direction[0], direction[1], direction[2]],
            &depths,
            t,
            c,
        )
        .map_err(|e| args::bad(span, format!("emf_sample_at_depth: {e:?}")))?;
    let sample_values: Vec<Value> = r
        .samples
        .iter()
        .map(|s| {
            args::record([
                ("depth", Value::F64(s.depth)),
                ("amplitude", Value::F64(s.amplitude)),
                ("phase", Value::F64(s.phase)),
                ("frequency", Value::F64(s.frequency)),
                ("perspective_scale", Value::F64(s.perspective_scale)),
                ("display_attenuation", Value::F64(s.display_attenuation)),
                ("lod_level", Value::U64(s.lod_level as u64)),
                ("manifold_coord", manifold_to_value(&s.manifold_coord)),
            ])
        })
        .collect();
    Ok(args::record([
        ("samples", Value::List(sample_values)),
        ("num_depths", Value::U64(r.num_depths as u64)),
        ("time", Value::F64(r.time)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    fn f64_list(xs: &[f64]) -> Value {
        Value::List(xs.iter().map(|x| Value::F64(*x)).collect())
    }

    #[test]
    fn invoke_emf_interference_constructive() {
        let sources = f64_list(&[0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
        let args = rec(&[
            ("sources", sources),
            ("x", Value::F64(1.0)),
            ("y", Value::F64(0.0)),
            ("z", Value::F64(0.0)),
            ("t", Value::F64(0.0)),
            ("c", Value::F64(1.0)),
        ]);
        let r = emf_interference(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let amp = m.get("amplitude").and_then(args::as_f64).unwrap();
                assert!(amp > 1.5, "constructive: amplitude should be ~2, got {amp}");
                let n = m.get("num_sources").and_then(args::as_u64).unwrap();
                assert_eq!(n, 2);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn invoke_emf_attenuation_inverse_square() {
        let args = rec(&[
            ("source_power", Value::F64(100.0)),
            ("frequency", Value::F64(1e9)),
            ("distance", Value::F64(2.0)),
        ]);
        let r = emf_attenuation(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let p = m.get("received_power").and_then(args::as_f64).unwrap();
                assert!(p > 0.0, "received power should be positive");
                let attdb = m.get("attenuation_db").and_then(args::as_f64).unwrap();
                assert!(attdb > 0.0, "attenuation should be positive");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn invoke_doppler_shift_approaching() {
        let args = rec(&[
            ("source_frequency", Value::F64(1e9)),
            ("relative_velocity", Value::F64(0.1 * 299_792_458.0)),
        ]);
        let r = doppler_shift(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let f_obs = m.get("observed_frequency").and_then(args::as_f64).unwrap();
                assert!(f_obs > 1e9, "approaching should increase frequency: {f_obs}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn invoke_emf_field_grid_returns_grid() {
        let sources = f64_list(&[0.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
        let bounds = f64_list(&[-5.0, 5.0, -5.0, 5.0, -5.0, 5.0]);
        let args = rec(&[
            ("sources", sources),
            ("bounds", bounds),
            ("nx", Value::U64(3)),
            ("ny", Value::U64(3)),
            ("nz", Value::U64(3)),
            ("nt", Value::U64(2)),
            ("c", Value::F64(1.0)),
        ]);
        let r = emf_field_grid_3d(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let amps = m.get("amplitudes");
                assert!(amps.is_some());
                if let Some(Value::List(xs)) = amps {
                    assert_eq!(xs.len(), 3 * 3 * 3 * 2);
                    assert!(xs.iter().all(|v| args::as_f64(v).map(|f| f.is_finite()).unwrap_or(false)));
                }
                let mcs = m.get("manifold_coords");
                assert!(mcs.is_some());
                if let Some(Value::List(xs)) = mcs {
                    assert_eq!(xs.len(), 3 * 3 * 3 * 2);
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn invoke_emf_sample_at_depth_returns_samples() {
        let sources = f64_list(&[0.0, 0.0, 10.0, 1.0, 1.0, 0.0]);
        let camera = f64_list(&[0.0, 0.0, 0.0]);
        let direction = f64_list(&[0.0, 0.0, 1.0]);
        let depths = f64_list(&[1.0, 10.0, 100.0]);
        let args = rec(&[
            ("sources", sources),
            ("camera", camera),
            ("direction", direction),
            ("depths", depths),
            ("c", Value::F64(1.0)),
        ]);
        let r = emf_sample_at_depth(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                if let Some(Value::List(xs)) = m.get("samples") {
                    assert_eq!(xs.len(), 3);
                }
                let n = m.get("num_depths").and_then(args::as_u64).unwrap();
                assert_eq!(n, 3);
            }
            other => panic!("{other:?}"),
        }
    }
}
