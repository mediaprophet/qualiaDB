//! In-process catalog kernels for hosts that do not attach Poet.
//!
//! These are real Vibe kernels (animation, HID-shaped records, byte-backed
//! local SPARQL over admitted 48-byte `NQuin` slices). Unknown ids still fail
//! closed. Poet overrides `Host::capability_invoke` to reach the engine.

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
    if id.starts_with("Cosmic.") {
        return super::cosmic::invoke(id, args, span);
    }
    if let Some(result) = super::stdlib::try_invoke(id, args, span) {
        return result;
    }
    if let Some(result) = super::logic::try_invoke(id, args, span) {
        return result;
    }
    if let Some(result) = super::specialized::try_invoke(id, args, span) {
        return result;
    }
    if id == "GraphDatabase.volume_open" {
        if let Some(res) = try_local_volume_open(args, span) {
            return res;
        }
    }
    if id == "GraphDatabase.sparql" {
        return eval_local_sparql(args, span);
    }
    if id.starts_with("HID.") {
        return Ok(hid_record(id, args));
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

/// Converts a sample to a value.
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

/// Payload from args.
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

#[derive(Clone, Copy, Debug)]
struct LocalQuin {
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
    metadata: u64,
    parity: u64,
}

impl LocalQuin {
    fn from_le_bytes(chunk: &[u8]) -> Option<Self> {
        if chunk.len() != 48 {
            return None;
        }
        let subject = u64::from_le_bytes(chunk[0..8].try_into().ok()?);
        let predicate = u64::from_le_bytes(chunk[8..16].try_into().ok()?);
        let object = u64::from_le_bytes(chunk[16..24].try_into().ok()?);
        let context = u64::from_le_bytes(chunk[24..32].try_into().ok()?);
        let metadata = u64::from_le_bytes(chunk[32..40].try_into().ok()?);
        let parity = u64::from_le_bytes(chunk[40..48].try_into().ok()?);
        Some(Self {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        })
    }

    fn to_record(self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("subject".into(), Value::U64(self.subject));
        rec.insert("predicate".into(), Value::U64(self.predicate));
        rec.insert("object".into(), Value::U64(self.object));
        rec.insert("context".into(), Value::U64(self.context));
        rec.insert("metadata".into(), Value::U64(self.metadata));
        rec.insert("parity".into(), Value::U64(self.parity));
        Value::Record(rec)
    }
}

std::thread_local! {
    static LOCAL_ADMITTED_QUINS: std::cell::RefCell<Vec<LocalQuin>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn extract_u8_slice(args: &Value, key: &str, span: Span) -> Result<Option<Vec<u8>>, Diagnostic> {
    let Value::Record(map) = args else {
        return Ok(None);
    };
    let Some(val) = map.get(key) else {
        return Ok(None);
    };
    let Value::List(items) = val else {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("`{key}` must be a list of u8 bytes"),
        ));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::U64(n) if *n <= 255 => out.push(*n as u8),
            Value::I64(n) if *n >= 0 && *n <= 255 => out.push(*n as u8),
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("`{key}` byte element out of range [0..255]"),
                ))
            }
        }
    }
    Ok(Some(out))
}

fn parse_quins_from_bytes(bytes: &[u8], span: Span) -> Result<Vec<LocalQuin>, Diagnostic> {
    if bytes.is_empty() || bytes.len() % 48 != 0 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "local Q42 slice must be a non-empty multiple of 48-byte NQuin records",
        ));
    }
    let mut quins = Vec::with_capacity(bytes.len() / 48);
    for chunk in bytes.chunks_exact(48) {
        if let Some(q) = LocalQuin::from_le_bytes(chunk) {
            quins.push(q);
        }
    }
    Ok(quins)
}

fn try_local_volume_open(args: &Value, span: Span) -> Option<Result<Value, Diagnostic>> {
    let bytes = match extract_u8_slice(args, "bytes", span) {
        Ok(Some(b)) => b,
        Ok(None) => return None,
        Err(e) => return Some(Err(e)),
    };
    let quins = match parse_quins_from_bytes(&bytes, span) {
        Ok(q) => q,
        Err(e) => return Some(Err(e)),
    };
    let count = quins.len() as u64;
    let load = match args {
        Value::Record(m) => match m.get("load") {
            Some(Value::Bool(b)) => *b,
            _ => true,
        },
        _ => true,
    };
    if load {
        LOCAL_ADMITTED_QUINS.with(|cell| {
            *cell.borrow_mut() = quins;
        });
    }
    let key = string_field(args, "key")
        .or_else(|| string_field(args, "path"))
        .unwrap_or("entry.q42");
    let mut rec = BTreeMap::new();
    rec.insert("path".into(), Value::String(key.into()));
    rec.insert("loaded".into(), Value::Bool(load));
    rec.insert("created".into(), Value::Bool(false));
    rec.insert("quin_count".into(), Value::U64(count));
    rec.insert("block_count".into(), Value::U64(0));
    rec.insert("version".into(), Value::U64(0));
    Some(Ok(Value::Record(rec)))
}

fn u64_field(args: &Value, key: &str) -> Option<u64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::U64(n)) => Some(*n),
            Some(Value::I64(n)) if *n >= 0 => Some(*n as u64),
            _ => None,
        },
        _ => None,
    }
}

fn eval_local_sparql(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let inline = extract_u8_slice(args, "bytes", span)?;
    let quins = if let Some(raw) = inline {
        parse_quins_from_bytes(&raw, span)?
    } else {
        LOCAL_ADMITTED_QUINS.with(|cell| cell.borrow().clone())
    };
    if quins.is_empty() {
        return Ok(Value::List(Vec::new()));
    }
    let subj = u64_field(args, "subject");
    let pred = u64_field(args, "predicate");
    let obj = u64_field(args, "object");
    let ctx = u64_field(args, "context");
    let mut rows = Vec::new();
    for q in quins {
        if subj.is_some_and(|s| s != q.subject)
            || pred.is_some_and(|p| p != q.predicate)
            || obj.is_some_and(|o| o != q.object)
            || ctx.is_some_and(|c| c != q.context)
        {
            continue;
        }
        rows.push(q.to_record());
    }
    Ok(Value::List(rows))
}
