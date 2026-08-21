//! `Render.animation_*` invoke wrappers for zero-heap mathematical curves and presets.

use std::collections::BTreeMap;

use super::super::args;
use poet_vibe::animation::{
    evaluate_preset, AnimationFamily, CubicBezier, EasingCurve, Motor, SpringConfig, SpringState1D,
};
use poet_vibe::{Diagnostic, Span, Value};

/// `Render.animation_eval_curve` — Evaluate an easing curve at normalized progress `t`.
///
/// Args:
/// - `curve`: curve name (e.g. "cubic-in-out", "ease-out-bounce", "elastic-out")
/// - `t`: normalized time progress (0.0 to 1.0)
///
/// Returns: `f64` (curve output value)
pub fn animation_eval_curve(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let curve_name = args::rec_str(args_v, "curve").ok_or_else(|| {
        args::bad(
            span,
            "animation_eval_curve needs { curve: string, t: float }",
        )
    })?;
    let t = args::rec_f64(args_v, "t").unwrap_or(0.0);

    if let Some(curve) = EasingCurve::from_name(curve_name) {
        Ok(Value::F64(curve.eval(t)))
    } else if curve_name.starts_with("cubic-bezier") || curve_name == "ease" {
        let bez = CubicBezier::ease_in_out();
        Ok(Value::F64(bez.eval(t)))
    } else {
        Err(args::bad(
            span,
            format!("unknown animation curve '{curve_name}'"),
        ))
    }
}

/// `Render.animation_spring_step` — Step an analytical spring-damper system.
///
/// Args:
/// - `current`: current position (f64)
/// - `target`: target equilibrium position (f64)
/// - `velocity`: current velocity (f64, optional default 0.0)
/// - `stiffness`: spring constant k (f64, default 280.0)
/// - `damping`: damping coefficient c (f64, default 30.0)
/// - `dt`: delta time in seconds (f64, default 1/60)
///
/// Returns: `{ position: f64, velocity: f64, settled: bool }`
pub fn animation_spring_step(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let current = args::rec_f64(args_v, "current").ok_or_else(|| {
        args::bad(
            span,
            "animation_spring_step needs { current: float, target: float, ... }",
        )
    })?;
    let target = args::rec_f64(args_v, "target").unwrap_or(0.0);
    let velocity = args::rec_f64(args_v, "velocity").unwrap_or(0.0);
    let stiffness = args::rec_f64(args_v, "stiffness").unwrap_or(280.0);
    let damping = args::rec_f64(args_v, "damping").unwrap_or(30.0);
    let dt = args::rec_f64(args_v, "dt").unwrap_or(1.0 / 60.0);

    let config = SpringConfig::new(stiffness, damping);
    let state = SpringState1D::new(current, velocity, target);
    let (next_pos, next_vel, settled) = state.evaluate_at(&config, dt);

    let mut map = BTreeMap::new();
    map.insert("position".to_string(), Value::F64(next_pos));
    map.insert("velocity".to_string(), Value::F64(next_vel));
    map.insert("settled".to_string(), Value::Bool(settled));

    Ok(Value::Record(map))
}

/// `Render.animation_sclerp` — Screw Linear Interpolation between two 3D PGA Motors.
///
/// Args:
/// - `m0`: starting motor [r_w, r_x, r_y, r_z, d_w, d_x, d_y, d_z] or `{ rot: [w,x,y,z], trans: [x,y,z] }`
/// - `m1`: ending motor
/// - `t`: interpolation progress [0.0, 1.0]
///
/// Returns: `{ motor: [f64; 8], rot: [f64; 4], trans: [f64; 3] }`
pub fn animation_sclerp(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let t = args::rec_f64(args_v, "t").unwrap_or(0.0);

    let m0 = extract_motor(args_v, "m0").unwrap_or_else(Motor::identity);
    let m1 = extract_motor(args_v, "m1").unwrap_or_else(Motor::identity);

    let interpolated = Motor::sclerp(&m0, &m1, t);
    let (rot, trans) = interpolated.to_rotation_translation();

    let mut map = BTreeMap::new();
    let motor_list = vec![
        Value::F64(interpolated.r_w),
        Value::F64(interpolated.r_x),
        Value::F64(interpolated.r_y),
        Value::F64(interpolated.r_z),
        Value::F64(interpolated.d_w),
        Value::F64(interpolated.d_x),
        Value::F64(interpolated.d_y),
        Value::F64(interpolated.d_z),
    ];
    let rot_list = vec![
        Value::F64(rot[0]),
        Value::F64(rot[1]),
        Value::F64(rot[2]),
        Value::F64(rot[3]),
    ];
    let trans_list = vec![
        Value::F64(trans[0]),
        Value::F64(trans[1]),
        Value::F64(trans[2]),
    ];

    map.insert("motor".to_string(), Value::List(motor_list));
    map.insert("rot".to_string(), Value::List(rot_list));
    map.insert("trans".to_string(), Value::List(trans_list));

    Ok(Value::Record(map))
}

/// `Render.animation_eval_preset` — Evaluate a standardized preset from the 10 animation families.
///
/// Args:
/// - `family`: family name (e.g. "hud-glass-ui", "spatial-kinematics", "dynamics")
/// - `preset`: preset name (e.g. "glass_reveal", "orbit_spin", "spring_settle")
/// - `t`: time in seconds
///
/// Returns: `{ scalar: f64, vector: [f64; 3], secondary: f64, settled: bool }`
pub fn animation_eval_preset(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let family_str = args::rec_str(args_v, "family").ok_or_else(|| {
        args::bad(
            span,
            "animation_eval_preset needs { family: string, preset: string, t: float }",
        )
    })?;
    let preset = args::rec_str(args_v, "preset").ok_or_else(|| {
        args::bad(
            span,
            "animation_eval_preset needs preset name (e.g. 'glass_reveal')",
        )
    })?;
    let t = args::rec_f64(args_v, "t").unwrap_or(0.0);

    let family = AnimationFamily::from_name(family_str)
        .ok_or_else(|| args::bad(span, format!("unknown animation family '{family_str}'")))?;

    let sample = evaluate_preset(family, preset, t);
    let mut map = BTreeMap::new();
    map.insert("scalar".to_string(), Value::F64(sample.scalar));
    map.insert(
        "vector".to_string(),
        Value::List(vec![
            Value::F64(sample.vector[0]),
            Value::F64(sample.vector[1]),
            Value::F64(sample.vector[2]),
        ]),
    );
    map.insert("secondary".to_string(), Value::F64(sample.secondary));
    map.insert("settled".to_string(), Value::Bool(sample.settled));

    Ok(Value::Record(map))
}

fn extract_motor(args_v: &Value, key: &str) -> Option<Motor> {
    let v = args::rec(args_v, key)?;
    if let Value::List(elems) = v {
        if elems.len() >= 8 {
            return Some(Motor {
                r_w: num_val(&elems[0]),
                r_x: num_val(&elems[1]),
                r_y: num_val(&elems[2]),
                r_z: num_val(&elems[3]),
                d_w: num_val(&elems[4]),
                d_x: num_val(&elems[5]),
                d_y: num_val(&elems[6]),
                d_z: num_val(&elems[7]),
            });
        }
    }
    None
}

fn num_val(v: &Value) -> f64 {
    match v {
        Value::F64(f) => *f,
        Value::I64(i) => *i as f64,
        Value::U64(u) => *u as f64,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_animation_eval_curve() {
        let mut map = BTreeMap::new();
        map.insert(
            "curve".to_string(),
            Value::String("cubic-in-out".to_string()),
        );
        map.insert("t".to_string(), Value::F64(0.5));
        let res = animation_eval_curve(&Value::Record(map), Span::new(0, 0)).expect("eval_curve");
        match res {
            Value::F64(v) => assert!((v - 0.5).abs() < 1e-4),
            other => panic!("expected f64, got {other:?}"),
        }
    }

    #[test]
    fn invoke_animation_spring_step() {
        let mut map = BTreeMap::new();
        map.insert("current".to_string(), Value::F64(0.0));
        map.insert("target".to_string(), Value::F64(10.0));
        map.insert("velocity".to_string(), Value::F64(0.0));
        map.insert("dt".to_string(), Value::F64(1.0 / 60.0));
        let res = animation_spring_step(&Value::Record(map), Span::new(0, 0)).expect("spring_step");
        match res {
            Value::Record(fields) => {
                let pos = match fields.get("position") {
                    Some(Value::F64(f)) => *f,
                    _ => panic!("missing position"),
                };
                assert!(pos > 0.0, "spring should move towards target: {pos}");
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn invoke_animation_eval_preset() {
        let mut map = BTreeMap::new();
        map.insert(
            "family".to_string(),
            Value::String("hud-glass-ui".to_string()),
        );
        map.insert(
            "preset".to_string(),
            Value::String("glass_reveal".to_string()),
        );
        map.insert("t".to_string(), Value::F64(0.2));
        let res = animation_eval_preset(&Value::Record(map), Span::new(0, 0)).expect("eval_preset");
        match res {
            Value::Record(fields) => {
                assert!(fields.contains_key("scalar"));
                assert!(fields.contains_key("vector"));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }
}
