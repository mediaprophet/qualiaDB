//! Compile-time/runtime structural reflection, schema generation, validation,
//! NaN-safe structural equality, and canonical FNV-1a hashing (`vibe-0.2`).

use std::collections::BTreeMap;

use crate::cbor_ast::{encode_value_cbor_ld, DEFAULT_CONTEXT_IRI};
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Dispatches `reflect.*` standard library calls.
pub fn call_reflection(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "reflect.type_of" => {
            let val = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "reflect.type_of requires 1 argument")
            })?;
            Ok(Some(type_of_value(val)))
        }
        "reflect.schema_of" => {
            let val = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "reflect.schema_of requires 1 argument")
            })?;
            Ok(Some(schema_of_value(val)))
        }
        "reflect.validate_schema" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "reflect.validate_schema requires (value, schema_record)",
                ));
            }
            Ok(Some(validate_schema_value(&args[0], &args[1], span)?))
        }
        "reflect.deep_equals" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "reflect.deep_equals requires 2 arguments",
                ));
            }
            Ok(Some(Value::Bool(deep_equals(&args[0], &args[1]))))
        }
        "reflect.canonical_hash" => {
            let val = args.first().ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "reflect.canonical_hash requires 1 argument",
                )
            })?;
            Ok(Some(canonical_hash_value(val)))
        }
        _ => Ok(None),
    }
}

/// Returns a structured descriptor for any runtime [`Value`].
/// Crucially, `Value::Null` reports `kind: "Option::None"` (never JS `"object"`).
pub fn type_of_value(val: &Value) -> Value {
    let (kind, type_iri, is_null, is_nan, is_numeric, is_collection) = classify_value(val);
    let mut rec = BTreeMap::new();
    rec.insert("kind".to_string(), Value::String(kind));
    rec.insert("type_iri".to_string(), Value::Iri(type_iri.to_string()));
    rec.insert("is_null".to_string(), Value::Bool(is_null));
    rec.insert("is_nan".to_string(), Value::Bool(is_nan));
    rec.insert("is_numeric".to_string(), Value::Bool(is_numeric));
    rec.insert("is_collection".to_string(), Value::Bool(is_collection));
    Value::Record(rec)
}

pub(crate) fn classify_value(val: &Value) -> (String, &'static str, bool, bool, bool, bool) {
    match val {
        Value::Null => (
            "Option::None".to_string(),
            "https://qualia.world/context/vibe/v1#OptionNone",
            true,
            false,
            false,
            false,
        ),
        Value::Bool(_) => ("Bool".to_string(), "xsd:boolean", false, false, false, false),
        Value::I64(_) => ("I64".to_string(), "xsd:integer", false, false, true, false),
        Value::U64(_) => (
            "U64".to_string(),
            "xsd:unsignedLong",
            false,
            false,
            true,
            false,
        ),
        Value::F64(f) => (
            "F64".to_string(),
            "xsd:double",
            false,
            f.is_nan(),
            true,
            false,
        ),
        Value::String(_) => ("String".to_string(), "xsd:string", false, false, false, false),
        Value::Iri(_) | Value::Prefixed(_, _) => {
            ("Iri".to_string(), "xsd:anyURI", false, false, false, false)
        }
        Value::Blank(_) => (
            "Blank".to_string(),
            "rdf:BlankNode",
            false,
            false,
            false,
            false,
        ),
        Value::Var(_) => (
            "Var".to_string(),
            "https://qualia.world/context/vibe/v1#Var",
            false,
            false,
            false,
            false,
        ),
        Value::List(_) => ("List".to_string(), "rdf:List", false, false, false, true),
        Value::Record(_) => (
            "Record".to_string(),
            "sh:NodeShape",
            false,
            false,
            false,
            true,
        ),
        Value::Quantity(q) => (
            "Quantity".to_string(),
            "qudt:QuantityValue",
            false,
            q.value.is_nan(),
            true,
            false,
        ),
        Value::Instant(_) => (
            "Instant".to_string(),
            "xsd:dateTime",
            false,
            false,
            false,
            false,
        ),
        Value::Duration(_) => (
            "Duration".to_string(),
            "xsd:duration",
            false,
            false,
            false,
            false,
        ),
        Value::QuinRef(_) => (
            "QuinRef".to_string(),
            "https://qualia.world/context/vibe/v1#QuinRef",
            false,
            false,
            false,
            false,
        ),
        Value::DisclosureDenied { .. } => (
            "DisclosureDenied".to_string(),
            "https://qualia.world/context/vibe/v1#DisclosureDenied",
            false,
            false,
            false,
            false,
        ),
        Value::Ok(_) => (
            "Result::Ok".to_string(),
            "https://qualia.world/context/vibe/v1#ResultOk",
            false,
            false,
            false,
            false,
        ),
        Value::Err(_) => (
            "Result::Err".to_string(),
            "https://qualia.world/context/vibe/v1#ResultErr",
            false,
            false,
            false,
            false,
        ),
        Value::Enum(e) => (
            format!("Enum::{}::{}", e.enum_name, e.variant_name),
            "https://qualia.world/context/vibe/v1#Enum",
            false,
            false,
            false,
            false,
        ),
        _ => (
            "OpaqueHandle".to_string(),
            "https://qualia.world/context/vibe/v1#Handle",
            false,
            false,
            false,
            false,
        ),
    }
}

/// Generates a deterministic SHACL / JSON-Schema / CBOR-LD descriptor for `val`.
pub fn schema_of_value(val: &Value) -> Value {
    let (kind, _iri, _, _, _, _) = classify_value(val);
    let mut properties = BTreeMap::new();
    let mut required = Vec::new();

    if let Value::Record(map) = val {
        for (k, v) in map {
            let (f_kind, f_iri, is_null, _, _, _) = classify_value(v);
            let mut prop_desc = BTreeMap::new();
            prop_desc.insert("kind".to_string(), Value::String(f_kind));
            prop_desc.insert("type_iri".to_string(), Value::Iri(f_iri.to_string()));
            properties.insert(k.clone(), Value::Record(prop_desc));
            if !is_null {
                required.push(Value::String(k.clone()));
            }
        }
    }

    let mut schema = BTreeMap::new();
    schema.insert(
        "@context".to_string(),
        Value::Iri(DEFAULT_CONTEXT_IRI.to_string()),
    );
    schema.insert("kind".to_string(), Value::String(kind));
    schema.insert("properties".to_string(), Value::Record(properties));
    schema.insert("required".to_string(), Value::List(required));

    let prelim = Value::Record(schema.clone());
    let cbor_bytes = encode_value_cbor_ld(&prelim, DEFAULT_CONTEXT_IRI);
    schema.insert("cbor_ld_hex".to_string(), Value::String(hex_encode(&cbor_bytes)));
    Value::Record(schema)
}

/// Validates `val` against a schema record (`required`, `properties`).
pub fn validate_schema_value(
    val: &Value,
    schema_val: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::Record(schema) = schema_val else {
        return Err(Diagnostic::new(
            DiagCode::E600,
            span,
            "reflect.validate_schema expects a Record as the second argument",
        ));
    };
    let mut errors: Vec<Value> = Vec::new();

    let Value::Record(rec) = val else {
        errors.push(Value::String("expected value to be a Record".to_string()));
        return Ok(build_validation_result(false, errors, val.clone()));
    };

    if let Some(Value::List(req_list)) = schema.get("required") {
        for req_item in req_list {
            if let Value::String(field_name) = req_item {
                match rec.get(field_name) {
                    None | Some(Value::Null) => {
                        errors.push(Value::String(format!(
                            "missing required field '{field_name}'"
                        )));
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(Value::Record(props)) = schema.get("properties") {
        for (field_name, rule) in props {
            if let Some(field_val) = rec.get(field_name) {
                if matches!(field_val, Value::Null) {
                    continue;
                }
                validate_field_rule(field_name, field_val, rule, &mut errors);
            }
        }
    }

    Ok(build_validation_result(
        errors.is_empty(),
        errors,
        val.clone(),
    ))
}

fn validate_field_rule(
    field_name: &str,
    field_val: &Value,
    rule: &Value,
    errors: &mut Vec<Value>,
) {
    let (actual_kind, _, _, is_nan, is_num, _) = classify_value(field_val);
    match rule {
        Value::String(expected_kind) => {
            if !kind_matches(&actual_kind, is_num, expected_kind) {
                errors.push(Value::String(format!(
                    "field '{field_name}': expected {expected_kind}, got {actual_kind}"
                )));
            } else if is_nan {
                errors.push(Value::String(format!(
                    "field '{field_name}': numeric value must not be NaN"
                )));
            }
        }
        Value::Record(rule_map) => {
            if let Some(Value::String(expected_kind)) = rule_map.get("kind") {
                if !kind_matches(&actual_kind, is_num, expected_kind) {
                    errors.push(Value::String(format!(
                        "field '{field_name}': expected {expected_kind}, got {actual_kind}"
                    )));
                    return;
                }
            }
            if is_nan {
                errors.push(Value::String(format!(
                    "field '{field_name}': numeric value must not be NaN"
                )));
                return;
            }
            if let Some(num) = field_val.as_f64() {
                if let Some(min_v) = rule_map.get("min").and_then(|v| v.as_f64()) {
                    if num < min_v {
                        errors.push(Value::String(format!(
                            "field '{field_name}': value {num} < min {min_v}"
                        )));
                    }
                }
                if let Some(max_v) = rule_map.get("max").and_then(|v| v.as_f64()) {
                    if num > max_v {
                        errors.push(Value::String(format!(
                            "field '{field_name}': value {num} > max {max_v}"
                        )));
                    }
                }
            }
            if let Value::String(s) = field_val {
                if let Some(min_len) = rule_map.get("min_length").and_then(|v| v.as_i64()) {
                    if (s.chars().count() as i64) < min_len {
                        errors.push(Value::String(format!(
                            "field '{field_name}': length < min_length {min_len}"
                        )));
                    }
                }
            }
        }
        _ => {}
    }
}

fn kind_matches(actual_kind: &str, is_num: bool, expected: &str) -> bool {
    if actual_kind.eq_ignore_ascii_case(expected) {
        return true;
    }
    match expected {
        "Number" | "Numeric" => is_num,
        "Int" | "Integer" => actual_kind == "I64" || actual_kind == "U64",
        "Float" => actual_kind == "F64",
        _ => false,
    }
}

fn build_validation_result(valid: bool, errors: Vec<Value>, value: Value) -> Value {
    let mut out = BTreeMap::new();
    out.insert("valid".to_string(), Value::Bool(valid));
    out.insert("errors".to_string(), Value::List(errors));
    out.insert("value".to_string(), value);
    Value::Record(out)
}

/// Structural value equality where `NaN == NaN` holds deterministically.
pub fn deep_equals(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::F64(x), Value::F64(y)) => (x.is_nan() && y.is_nan()) || x == y,
        (Value::Quantity(qx), Value::Quantity(qy)) => {
            qx.unit == qy.unit
                && ((qx.value.is_nan() && qy.value.is_nan()) || qx.value == qy.value)
        }
        (Value::I64(x), Value::U64(y)) => *x >= 0 && (*x as u64) == *y,
        (Value::U64(x), Value::I64(y)) => *y >= 0 && *x == (*y as u64),
        (Value::List(xs), Value::List(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys.iter()).all(|(x, y)| deep_equals(x, y))
        }
        (Value::Record(mx), Value::Record(my)) => {
            mx.len() == my.len()
                && mx
                    .iter()
                    .all(|(k, vx)| my.get(k).is_some_and(|vy| deep_equals(vx, vy)))
        }
        (Value::Ok(x), Value::Ok(y)) | (Value::Err(x), Value::Err(y)) => deep_equals(x, y),
        (Value::Enum(ex), Value::Enum(ey)) => {
            ex.enum_name == ey.enum_name
                && ex.variant_name == ey.variant_name
                && ex.payload.len() == ey.payload.len()
                && ex
                    .payload
                    .iter()
                    .zip(ey.payload.iter())
                    .all(|(x, y)| deep_equals(x, y))
        }
        _ => a == b,
    }
}

/// Computes a deterministic 64-bit FNV-1a hash over the canonical CBOR-LD encoding of `val`.
pub fn canonical_hash_value(val: &Value) -> Value {
    let normalized = normalize_nan(val);
    let cbor = encode_value_cbor_ld(&normalized, DEFAULT_CONTEXT_IRI);
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in &cbor {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let mut rec = BTreeMap::new();
    rec.insert("hash_u64".to_string(), Value::U64(hash));
    rec.insert("hash_hex".to_string(), Value::String(format!("{hash:016x}")));
    Value::Record(rec)
}

fn normalize_nan(val: &Value) -> Value {
    match val {
        Value::F64(f) if f.is_nan() => Value::F64(f64::from_bits(0x7ff8_0000_0000_0000)),
        Value::List(xs) => Value::List(xs.iter().map(normalize_nan).collect()),
        Value::Record(m) => Value::Record(
            m.iter()
                .map(|(k, v)| (k.clone(), normalize_nan(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}
