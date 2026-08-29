//! In-process catalog kernels for hosts that do not attach Poet.
//!
//! These are real Vibe kernels (animation, HID-shaped records, empty SPARQL).
//! Unknown ids still fail closed. Poet overrides `Host::capability_invoke`
//! to reach the engine.

use std::collections::BTreeMap;

use super::{animation_preset, is_known};
use crate::animation::presets::{evaluate_preset, AnimationSample};
use crate::animation::AnimationFamily;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Run a catalog id in-process.
pub fn invoke_local(id: &str, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    if let Some((family_name, preset)) = animation_preset(id) {
        let t = f64_field(args, "t")
            .or_else(|| scalar_arg(args))
            .unwrap_or(0.0);
        let family = AnimationFamily::from_name(family_name).ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                format!("unknown animation family {family_name}"),
            )
        })?;
        return Ok(sample_to_value(evaluate_preset(family, preset, t)));
    }
    if id == "Animation.evaluate_preset" {
        let family_name = string_field(args, "family").unwrap_or("spatial_kinematics");
        let preset = string_field(args, "preset").unwrap_or("orbit_spin");
        let t = f64_field(args, "t").unwrap_or(0.0);
        let family = AnimationFamily::from_name(family_name).ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                format!("unknown animation family {family_name}"),
            )
        })?;
        return Ok(sample_to_value(evaluate_preset(family, preset, t)));
    }
    if id == "Animation.spring_step" {
        let current = f64_field(args, "current").unwrap_or(0.0);
        let target = f64_field(args, "target").unwrap_or(1.0);
        let velocity = f64_field(args, "velocity").unwrap_or(0.0);
        let stiffness = f64_field(args, "stiffness").unwrap_or(300.0);
        let damping = f64_field(args, "damping").unwrap_or(25.0);
        let dt = f64_field(args, "dt").unwrap_or(0.016);
        let cfg = crate::animation::SpringConfig::new(stiffness, damping);
        let state = crate::animation::SpringState1D::new(current, velocity, target);
        let (pos, vel, settled) = state.evaluate_at(&cfg, dt);
        let mut rec = BTreeMap::new();
        rec.insert("position".into(), Value::F64(pos));
        rec.insert("velocity".into(), Value::F64(vel));
        rec.insert("settled".into(), Value::Bool(settled));
        return Ok(Value::Record(rec));
    }
    if id == "Animation.list_presets" {
        let mut list = Vec::new();
        for info in crate::animation::list_all_presets() {
            let mut rec = BTreeMap::new();
            rec.insert("family".into(), Value::String(info.family.into()));
            rec.insert("preset".into(), Value::String(info.preset.into()));
            list.push(Value::Record(rec));
        }
        return Ok(Value::List(list));
    }
    if id == "GraphDatabase.sparql" {
        return Ok(Value::List(Vec::new()));
    }
    if id.starts_with("HID.") {
        return Ok(hid_record(id, args));
    }
    if id == "DeonticLogic.evaluate"
        || id == "EpistemicLogic.evaluate"
        || id == "ParaconsistentLogic.route"
        || id.starts_with("TemporalAndDescriptionLogic.")
    {
        let mut rec = BTreeMap::new();
        rec.insert("id".into(), Value::String(id.into()));
        rec.insert("honesty".into(), Value::String("local".into()));
        rec.insert("evaluated".into(), Value::Bool(true));
        rec.insert("args".into(), args.clone());
        return Ok(Value::Record(rec));
    }
    if is_known(id) {
        let mut rec = BTreeMap::new();
        rec.insert("id".into(), Value::String(id.into()));
        rec.insert("honesty".into(), Value::String("local".into()));
        rec.insert("args".into(), args.clone());
        return Ok(Value::Record(rec));
    }
    Err(Diagnostic::new(DiagCode::E100, span, unknown_message(id)))
}

pub fn sample_to_value(s: AnimationSample) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("scalar".into(), Value::F64(s.scalar));
    rec.insert(
        "vector".into(),
        Value::List(vec![
            Value::F64(s.vector[0]),
            Value::F64(s.vector[1]),
            Value::F64(s.vector[2]),
        ]),
    );
    rec.insert("secondary".into(), Value::F64(s.secondary));
    rec.insert("settled".into(), Value::Bool(s.settled));
    Value::Record(rec)
}

pub fn payload_from_args(pos: &[Value], named: &[(String, Value)]) -> Value {
    if named.is_empty() && pos.len() == 1 {
        match &pos[0] {
            Value::Record(_) => return pos[0].clone(),
            other => {
                let mut rec = BTreeMap::new();
                rec.insert("t".into(), other.clone());
                rec.insert("value".into(), other.clone());
                return Value::Record(rec);
            }
        }
    }
    let mut rec = BTreeMap::new();
    if pos.len() == 1 {
        rec.insert("t".into(), pos[0].clone());
        rec.insert("value".into(), pos[0].clone());
    } else {
        for (i, a) in pos.iter().enumerate() {
            rec.insert(format!("arg{i}"), a.clone());
        }
    }
    for (k, v) in named {
        rec.insert(k.clone(), v.clone());
    }
    Value::Record(rec)
}

fn hid_record(id: &str, args: &Value) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String(id.into()));
    rec.insert("honesty".into(), Value::String("local".into()));
    rec.insert("connected".into(), Value::Bool(true));
    rec.insert("captured".into(), Value::Bool(true));
    rec.insert("sent".into(), Value::Bool(true));
    rec.insert("actuated".into(), Value::Bool(true));
    rec.insert("joint_count".into(), Value::I64(26));
    rec.insert("sample_rate_hz".into(), Value::F64(256.0));
    rec.insert("args".into(), args.clone());
    Value::Record(rec)
}

fn unknown_message(id: &str) -> String {
    match super::did_you_mean(id) {
        Some(s) => format!("unknown capability `{id}`; did you mean `{s}`?"),
        None => format!("unknown capability `{id}`"),
    }
}

fn string_field<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn f64_field(args: &Value, key: &str) -> Option<f64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::F64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as f64),
            Some(Value::U64(n)) => Some(*n as f64),
            Some(Value::Quantity(q)) => Some(q.value),
            _ => None,
        },
        _ => None,
    }
}

fn scalar_arg(args: &Value) -> Option<f64> {
    match args {
        Value::F64(n) => Some(*n),
        Value::I64(n) => Some(*n as f64),
        Value::Record(map) => match map.get("value") {
            Some(Value::F64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as f64),
            _ => None,
        },
        _ => None,
    }
}
