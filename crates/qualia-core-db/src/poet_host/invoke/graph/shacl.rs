//! SHACL + Qualia extensions over the live graph (SPARQL-star reifiers included).

use super::super::args;
use crate::lexicon::generate_60bit_token;
use crate::poet_host::{hash_val, PoetSnapshot};
use crate::query::shacl_compiler::{validate_shacl_property, ShaclConstraint, ShaclDatatype};
use vibe::{Diagnostic, Span, Value};

const RDF_REIFIES: &[u8] = b"http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Names apps may pass as `kind`. Keep in lockstep with `ShaclConstraint`.
pub const EXTENSION_KINDS: &[&str] = &[
    "minCount",
    "maxCount",
    "minLength",
    "maxLength",
    "datatype",
    "in",
    "deonticObligate",
    "deonticPermit",
    "deonticForbid",
    "deonticNotExpired",
    "epistemicKnowledge",
    "epistemicBelief",
    "commonKnowledge",
    "reifier",
];

pub fn validate(snap: &PoetSnapshot, args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(any(
        not(target_arch = "wasm32"),
        feature = "wasm-ontology",
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-full"
    ))]
    {
        let subject = args::rec(args_v, "subject")
            .and_then(hash_val)
            .or_else(|| hash_val(args_v))
            .ok_or_else(|| args::bad(span, "SHACL.validate needs subject"))?;
        let path = args::rec(args_v, "path")
            .and_then(hash_val)
            .unwrap_or_else(|| generate_60bit_token(RDF_REIFIES));
        let mut buf = [ShaclConstraint::MinCount(1); 8];
        let n = parse_constraints(args_v, &mut buf, span)?;
        let ok =
            snap.with_live_quins(|quins| validate_shacl_property(quins, subject, path, &buf[..n]));
        return Ok(Value::Bool(ok));
    }
    #[cfg(not(any(
        not(target_arch = "wasm32"),
        feature = "wasm-ontology",
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-full"
    )))]
    {
        let _ = (snap, args_v);
        Err(vibe::Diagnostic::new(
            vibe::DiagCode::E300,
            span,
            "SHACL is not in this wasm profile",
        ))
    }
}

pub fn extensions(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    Ok(Value::List(
        EXTENSION_KINDS
            .iter()
            .map(|k| Value::String((*k).into()))
            .collect(),
    ))
}

fn parse_constraints(
    args_v: &Value,
    out: &mut [ShaclConstraint],
    span: Span,
) -> Result<usize, Diagnostic> {
    if let Some(xs) = args::rec(args_v, "constraints").and_then(args::list) {
        let mut n = 0;
        for item in xs {
            if n >= out.len() {
                break;
            }
            out[n] = one_constraint(item, span)?;
            n += 1;
        }
        if n == 0 {
            out[0] = ShaclConstraint::MinCount(1);
            return Ok(1);
        }
        return Ok(n);
    }
    if let Some(kind) = args::rec_str(args_v, "kind").or_else(|| args::as_str(args_v)) {
        if kind == "reifier" || kind.eq_ignore_ascii_case("EmergencyAlertShape") {
            out[0] = ShaclConstraint::MinCount(1);
            return Ok(1);
        }
        out[0] = named(kind, args_v, span)?;
        return Ok(1);
    }
    out[0] = ShaclConstraint::MinCount(1);
    Ok(1)
}

fn one_constraint(item: &Value, span: Span) -> Result<ShaclConstraint, Diagnostic> {
    if let Some(kind) = args::as_str(item) {
        return named(kind, item, span);
    }
    let kind = args::rec_str(item, "kind").ok_or_else(|| args::bad(span, "constraint.kind"))?;
    named(kind, item, span)
}

fn named(kind: &str, item: &Value, span: Span) -> Result<ShaclConstraint, Diagnostic> {
    Ok(match kind {
        "minCount" => ShaclConstraint::MinCount(args::rec_u64(item, "value").unwrap_or(1) as u32),
        "maxCount" => ShaclConstraint::MaxCount(args::rec_u64(item, "value").unwrap_or(1) as u32),
        "minLength" => ShaclConstraint::MinLength(args::rec_u64(item, "value").unwrap_or(0) as u32),
        "maxLength" => ShaclConstraint::MaxLength(
            args::rec_u64(item, "value").unwrap_or(u32::MAX as u64) as u32,
        ),
        "reifier" => ShaclConstraint::MinCount(1),
        "in" => {
            let values_list = args::rec(item, "values")
                .and_then(args::list)
                .ok_or_else(|| args::bad(span, "in constraint needs values list"))?;
            let mut hashes = [0u64; 8];
            let mut count = 0u8;
            for v in values_list {
                if count >= 8 {
                    return Err(args::bad(span, "in constraint supports at most 8 values"));
                }
                let h = hash_val(v)
                    .or_else(|| args::as_str(v).map(|s| crate::q_hash(s)))
                    .ok_or_else(|| args::bad(span, "in values must be strings or IRIs"))?;
                hashes[count as usize] = h;
                count += 1;
            }
            if count == 0 {
                return Err(args::bad(span, "in constraint needs at least one value"));
            }
            ShaclConstraint::In {
                count,
                values: hashes,
            }
        }
        "datatype" => {
            let dt = args::rec_str(item, "value").unwrap_or("xsd:integer");
            let tag = match dt {
                "xsd:string" | "string" => ShaclDatatype::String,
                "xsd:decimal" | "decimal" => ShaclDatatype::Decimal,
                "xsd:boolean" | "boolean" => ShaclDatatype::Boolean,
                "xsd:dateTime" | "dateTime" => ShaclDatatype::DateTime,
                _ => ShaclDatatype::Integer,
            };
            ShaclConstraint::Datatype(tag)
        }
        "deonticObligate" => ShaclConstraint::DeonticObligate,
        "deonticPermit" => ShaclConstraint::DeonticPermit,
        "deonticForbid" => ShaclConstraint::DeonticForbid,
        "deonticNotExpired" => ShaclConstraint::DeonticNotExpired {
            now_unix: args::rec_u64(item, "now").unwrap_or(0) as u32,
        },
        "epistemicKnowledge" => ShaclConstraint::EpistemicKnowledge {
            min_certainty: args::rec_u64(item, "min").unwrap_or(0) as u8,
        },
        "epistemicBelief" => ShaclConstraint::EpistemicBelief {
            min_certainty: args::rec_u64(item, "min").unwrap_or(0) as u8,
        },
        "commonKnowledge" => ShaclConstraint::CommonKnowledge,
        other => {
            return Err(args::bad(
                span,
                format!("unknown SHACL kind {other}; see SHACL.extensions"),
            ))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_list_includes_reifier() {
        match extensions(&Value::Null, Span { start: 0, end: 0 }).unwrap() {
            Value::List(xs) => {
                assert!(xs
                    .iter()
                    .any(|v| matches!(v, Value::String(s) if s == "reifier")))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn extensions_list_includes_in_and_length() {
        match extensions(&Value::Null, Span { start: 0, end: 0 }).unwrap() {
            Value::List(xs) => {
                assert!(xs
                    .iter()
                    .any(|v| matches!(v, Value::String(s) if s == "in")));
                assert!(xs
                    .iter()
                    .any(|v| matches!(v, Value::String(s) if s == "minLength")));
                assert!(xs
                    .iter()
                    .any(|v| matches!(v, Value::String(s) if s == "maxLength")));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_in_constraint() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("kind".into(), Value::String("in".into()));
        m.insert(
            "values".into(),
            Value::List(vec![
                Value::String("apple".into()),
                Value::String("banana".into()),
            ]),
        );
        let mut buf = [ShaclConstraint::MinCount(1); 8];
        let n = parse_constraints(&Value::Record(m), &mut buf, Span { start: 0, end: 0 }).unwrap();
        assert_eq!(n, 1);
        match &buf[0] {
            ShaclConstraint::In { count, values } => {
                assert_eq!(*count, 2);
                assert_eq!(values[0], crate::q_hash("apple"));
                assert_eq!(values[1], crate::q_hash("banana"));
            }
            other => panic!("expected In, got {other:?}"),
        }
    }

    #[test]
    fn parse_minlength_constraint() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("kind".into(), Value::String("minLength".into()));
        m.insert("value".into(), Value::U64(3));
        let mut buf = [ShaclConstraint::MinCount(1); 8];
        let n = parse_constraints(&Value::Record(m), &mut buf, Span { start: 0, end: 0 }).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], ShaclConstraint::MinLength(3));
    }
}
