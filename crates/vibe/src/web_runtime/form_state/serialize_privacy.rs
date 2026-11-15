//! Form Serialization (Record, URL Params, RDF 1.2 Turtle, 48-byte `QuinRef`, Provider JSON Scoping),
//! Four-Boundary Disclosure Filtering, and Zero-Knowledge Field Proofs (`form.*`).

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::{QuinRef, Value};

use super::lifecycle::build_form_record;
use super::{
    extract_i64, extract_record, extract_schema_base, extract_str, fnv1a_64, get_sub_record,
};

/// Handle form serialization, provider namespace scoping, disclosure filtering, and ZK proofs.
pub(crate) fn call_serialize_privacy(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "form.to_record" => {
            let form = extract_record(args, 0, "form.to_record(form)", span)?;
            let values = if form.contains_key("values") {
                get_sub_record(&form, "values")
            } else {
                form
            };
            Ok(Some(Value::Record(values)))
        }
        "form.to_url_params" => {
            let form = extract_record(args, 0, "form.to_url_params(form)", span)?;
            let values = if form.contains_key("values") {
                get_sub_record(&form, "values")
            } else {
                form
            };
            let mut pairs = Vec::with_capacity(values.len());
            for (k, v) in &values {
                let raw_v = value_to_plain_str(v);
                pairs.push(format!("{}={}", percent_encode(k), percent_encode(&raw_v)));
            }
            Ok(Some(Value::String(pairs.join("&"))))
        }
        "form.to_rdf_turtle" => {
            let form = extract_record(args, 0, "form.to_rdf_turtle(form, subject_iri)", span)?;
            let subject_iri = extract_str(args, 1, "form.to_rdf_turtle(form, subject_iri)", span)?;
            let schema_base = extract_schema_base(&form);
            let values = if form.contains_key("values") {
                get_sub_record(&form, "values")
            } else {
                form
            };

            let mut out = String::new();
            for (k, v) in &values {
                let pred = resolve_predicate_iri(&schema_base, k);
                let obj = format_turtle_object(v);
                out.push_str(&format!("<{subject_iri}> <{pred}> {obj} .\n"));
            }
            Ok(Some(Value::String(out)))
        }
        "form.to_quins" => {
            let form = extract_record(
                args,
                0,
                "form.to_quins(form, subject_iri, context_iri)",
                span,
            )?;
            let subject_iri =
                extract_str(args, 1, "form.to_quins(form, subject_iri, context_iri)", span)?;
            let context_iri =
                extract_str(args, 2, "form.to_quins(form, subject_iri, context_iri)", span)?;
            let schema_base = extract_schema_base(&form);
            let values = if form.contains_key("values") {
                get_sub_record(&form, "values")
            } else {
                form
            };

            let s = fnv1a_64(subject_iri.as_bytes());
            let c = fnv1a_64(context_iri.as_bytes());
            let mut quins = Vec::with_capacity(values.len());
            for (k, v) in &values {
                let pred_iri = resolve_predicate_iri(&schema_base, k);
                let p = fnv1a_64(pred_iri.as_bytes());
                let o = hash_value_object(v);
                quins.push(Value::QuinRef(QuinRef::from_quin(s, p, o, c)));
            }
            Ok(Some(Value::List(quins)))
        }
        "form.from_provider_json" => {
            let form_id = extract_str(
                args,
                0,
                "form.from_provider_json(form_id, json_record, provider_tld)",
                span,
            )?;
            let json_record = extract_record(
                args,
                1,
                "form.from_provider_json(form_id, json_record, provider_tld)",
                span,
            )?;
            let raw_tld = extract_str(
                args,
                2,
                "form.from_provider_json(form_id, json_record, provider_tld)",
                span,
            )?;

            let clean_tld = raw_tld
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('#')
                .trim_end_matches("/ns")
                .trim_end_matches('/');
            let schema_iri = format!("https://{clean_tld}/ns#");

            let mut scoped_fields = BTreeMap::new();
            for (k, v) in json_record {
                let scoped_key = if k.starts_with("http://") || k.starts_with("https://") {
                    k
                } else {
                    format!("{schema_iri}{k}")
                };
                scoped_fields.insert(scoped_key, v);
            }

            let Value::Record(mut form) =
                build_form_record(form_id, schema_iri, scoped_fields)
            else {
                unreachable!()
            };
            form.insert(
                "provider_tld".to_string(),
                Value::String(clean_tld.to_string()),
            );
            Ok(Some(Value::Record(form)))
        }
        "form.filter_disclosure" => {
            let mut form = extract_record(
                args,
                0,
                "form.filter_disclosure(form, policies, requested_boundary)",
                span,
            )?;
            let (policies, boundary) = if args.len() >= 3 {
                let pol = extract_record(
                    args,
                    1,
                    "form.filter_disclosure(form, policies, requested_boundary)",
                    span,
                )?;
                let bnd = extract_str(
                    args,
                    2,
                    "form.filter_disclosure(form, policies, requested_boundary)",
                    span,
                )?;
                (pol, bnd)
            } else {
                let bnd = extract_str(
                    args,
                    1,
                    "form.filter_disclosure(form, requested_boundary)",
                    span,
                )?;
                (get_sub_record(&form, "policies"), bnd)
            };

            let has_values_wrapper = form.contains_key("values");
            let mut values = if has_values_wrapper {
                get_sub_record(&form, "values")
            } else {
                form.clone()
            };

            for (field_id, val) in &mut values {
                if !is_boundary_allowed(&policies, field_id, &boundary) {
                    *val = Value::DisclosureDenied {
                        capability: format!("disclosure.{boundary}.{field_id}"),
                        reason: "redacted by four-boundary disclosure policy".to_string(),
                    };
                }
            }

            if has_values_wrapper {
                form.insert("values".to_string(), Value::Record(values));
                Ok(Some(Value::Record(form)))
            } else {
                Ok(Some(Value::Record(values)))
            }
        }
        "form.zk_prove_field" => {
            let mut form = extract_record(
                args,
                0,
                "form.zk_prove_field(form, field_id, kind, lo, hi_opt)",
                span,
            )?;
            let field_id = extract_str(
                args,
                1,
                "form.zk_prove_field(form, field_id, kind, lo, hi_opt)",
                span,
            )?;
            let kind = extract_str(
                args,
                2,
                "form.zk_prove_field(form, field_id, kind, lo, hi_opt)",
                span,
            )?;
            let lo = extract_i64(
                args,
                3,
                "form.zk_prove_field(form, field_id, kind, lo, hi_opt)",
                span,
            )?;
            let hi = args.get(4).and_then(Value::as_i64).unwrap_or(i64::MAX);

            let has_values_wrapper = form.contains_key("values");
            let mut values = if has_values_wrapper {
                get_sub_record(&form, "values")
            } else {
                form.clone()
            };

            let raw_val = values
                .get(&field_id)
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("form.zk_prove_field: field '{field_id}' must be an integer"),
                    )
                })?;

            let verified = match kind.as_str() {
                "threshold" => raw_val >= lo,
                "range" => raw_val >= lo && raw_val <= hi,
                other => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("form.zk_prove_field: unsupported proof kind '{other}'"),
                    ))
                }
            };

            let commit_input = format!("zk:{kind}:{field_id}:{raw_val}:{lo}:{hi}");
            let commit_hash = format!("0x{:016x}", fnv1a_64(commit_input.as_bytes()));

            let mut proof = BTreeMap::new();
            proof.insert("__zk_proof".to_string(), Value::Bool(true));
            proof.insert("kind".to_string(), Value::String(kind));
            proof.insert("field_id".to_string(), Value::String(field_id.clone()));
            proof.insert("verified".to_string(), Value::Bool(verified));
            proof.insert("commitment_hash".to_string(), Value::String(commit_hash));

            values.insert(field_id, Value::Record(proof));
            if has_values_wrapper {
                form.insert("values".to_string(), Value::Record(values));
                Ok(Some(Value::Record(form)))
            } else {
                Ok(Some(Value::Record(values)))
            }
        }
        _ => Ok(None),
    }
}

fn resolve_predicate_iri(schema_base: &str, field_key: &str) -> String {
    if field_key.starts_with("http://") || field_key.starts_with("https://") {
        field_key.to_string()
    } else {
        format!("{schema_base}{field_key}")
    }
}

fn hash_value_object(v: &Value) -> u64 {
    match v {
        Value::U64(n) => *n,
        Value::I64(n) => *n as u64,
        Value::Bool(b) => u64::from(*b),
        Value::String(s) | Value::Iri(s) => fnv1a_64(s.as_bytes()),
        other => fnv1a_64(format!("{other}").as_bytes()),
    }
}

fn is_boundary_allowed(
    policies: &BTreeMap<String, Value>,
    field_id: &str,
    boundary: &str,
) -> bool {
    match policies.get(field_id) {
        Some(Value::DisclosureBoundary(b)) => match boundary {
            "publication" => b.publication,
            "replication" => b.replication,
            "agency" => b.agency,
            "exfiltration" => b.exfiltration,
            _ => false,
        },
        Some(Value::Record(rec)) => rec
            .get(boundary)
            .and_then(|v| match v {
                Value::Bool(allowed) => Some(*allowed),
                _ => None,
            })
            .unwrap_or(false),
        Some(Value::Bool(allowed)) => *allowed,
        None => true,
        _ => false,
    }
}

fn format_turtle_object(v: &Value) -> String {
    match v {
        Value::Iri(s) => format!("<{s}>"),
        Value::String(s) => {
            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{escaped}\"")
        }
        Value::Bool(b) => b.to_string(),
        Value::I64(n) => n.to_string(),
        Value::U64(n) => n.to_string(),
        Value::F64(n) => n.to_string(),
        other => format!("\"{other}\""),
    }
}

fn value_to_plain_str(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) | Value::Iri(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::I64(n) => n.to_string(),
        Value::U64(n) => n.to_string(),
        Value::F64(n) => n.to_string(),
        other => format!("{other}"),
    }
}

fn percent_encode(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0F) as usize] as char);
        }
    }
    out
}
