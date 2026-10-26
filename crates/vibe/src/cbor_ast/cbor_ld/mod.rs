//! W3C CBOR-LD (`0xD9 0x0501` / `0xD9 0x0500`) Runtime Value Codec, Term Table,
//! and Provider/TLD-Scoped JSON Namespace Uplift.

pub mod json_uplift;
pub mod terms;
pub mod value_codec;

use std::collections::BTreeMap;

pub use json_uplift::{derive_provider_namespace, ingest_json_with_provider_ns};
pub use terms::*;
pub use value_codec::{decode_value_cbor_ld, encode_value_cbor_ld};

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Dispatches `cbor_ld.*` built-in calls for VibeScript (`vibe-0.1`).
pub fn call_cbor_ld(path: &str, args: &[Value], span: Span) -> Result<Option<Value>, Diagnostic> {
    match path {
        "cbor_ld.encode" => {
            let val = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "cbor_ld.encode requires a value")
            })?;
            let ctx = extract_opt_str(args.get(1)).unwrap_or(DEFAULT_CONTEXT_IRI);
            let bytes = encode_value_cbor_ld(val, ctx);
            let mut rec = BTreeMap::new();
            rec.insert("__cbor_ld".to_string(), Value::Bool(true));
            rec.insert("tag".to_string(), Value::I64(TAG_CBOR_LD_COMPRESSED as i64));
            rec.insert("context".to_string(), Value::String(ctx.to_string()));
            rec.insert("byte_len".to_string(), Value::I64(bytes.len() as i64));
            rec.insert("hex".to_string(), Value::String(hex_encode(&bytes)));
            rec.insert(
                "bytes".to_string(),
                Value::List(bytes.iter().map(|&b| Value::I64(b as i64)).collect()),
            );
            Ok(Some(Value::Record(rec)))
        }
        "cbor_ld.decode" => {
            let arg = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "cbor_ld.decode requires 1 argument")
            })?;
            let raw = extract_cbor_bytes(arg, span)?;
            let (_ctx, decoded) = decode_value_cbor_ld(&raw)
                .map_err(|e| Diagnostic::new(DiagCode::E600, span, format!("CBOR-LD decode error: {e}")))?;
            Ok(Some(decoded))
        }
        "cbor_ld.compress_iri" => {
            let iri = args
                .first()
                .and_then(|v| extract_opt_str(Some(v)))
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E600,
                        span,
                        "cbor_ld.compress_iri requires an IRI or string argument",
                    )
                })?;
            let ctx = extract_opt_str(args.get(1)).unwrap_or(DEFAULT_CONTEXT_IRI);
            Ok(Some(Value::U64(compress_iri(iri, ctx))))
        }
        "cbor_ld.expand_term" => {
            let term_id = match args.first() {
                Some(Value::U64(n)) => *n,
                Some(Value::I64(n)) if *n >= 0 => *n as u64,
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E600,
                        span,
                        "cbor_ld.expand_term requires a non-negative integer term ID",
                    ))
                }
            };
            let ctx = extract_opt_str(args.get(1)).unwrap_or(DEFAULT_CONTEXT_IRI);
            let expanded = expand_term(term_id, ctx).ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E600,
                    span,
                    format!("unknown CBOR-LD term ID {term_id}"),
                )
            })?;
            Ok(Some(Value::Iri(expanded)))
        }
        "cbor_ld.verify_deterministic" => {
            let arg = args.first().ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "cbor_ld.verify_deterministic requires 1 argument",
                )
            })?;
            let raw = match extract_cbor_bytes(arg, span) {
                Ok(b) => b,
                Err(_) => return Ok(Some(Value::Bool(false))),
            };
            let is_det = match decode_value_cbor_ld(&raw) {
                Ok((ctx, val)) => encode_value_cbor_ld(&val, &ctx) == raw,
                Err(_) => false,
            };
            Ok(Some(Value::Bool(is_det)))
        }
        "cbor_ld.ingest_json" => {
            let json_src = extract_opt_str(args.first()).ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "cbor_ld.ingest_json requires a JSON string as argument 0",
                )
            })?;
            let provider = extract_opt_str(args.get(1)).ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "cbor_ld.ingest_json requires a provider domain or TLD as argument 1",
                )
            })?;
            let rec = ingest_json_with_provider_ns(json_src, provider)
                .map_err(|e| Diagnostic::new(DiagCode::E600, span, e))?;
            Ok(Some(rec))
        }
        "cbor_ld.provider_namespace" => {
            let provider = extract_opt_str(args.first()).ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "cbor_ld.provider_namespace requires a provider domain or TLD",
                )
            })?;
            let (host, tld, ns_iri) = derive_provider_namespace(provider)
                .map_err(|e| Diagnostic::new(DiagCode::E600, span, e))?;
            let mut rec = BTreeMap::new();
            rec.insert("provider_host".to_string(), Value::String(host));
            rec.insert("provider_tld".to_string(), Value::String(tld));
            rec.insert("namespace".to_string(), Value::Iri(ns_iri));
            Ok(Some(Value::Record(rec)))
        }
        _ => Ok(None),
    }
}

fn extract_opt_str(v: Option<&Value>) -> Option<&str> {
    match v {
        Some(Value::String(s)) | Some(Value::Iri(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn extract_cbor_bytes(val: &Value, span: Span) -> Result<Vec<u8>, Diagnostic> {
    match val {
        Value::Record(map) => {
            if let Some(Value::String(hex)) = map.get("hex") {
                return hex_decode(hex, span);
            }
            if let Some(Value::List(list)) = map.get("bytes") {
                return extract_byte_list(list, span);
            }
            Err(Diagnostic::new(
                DiagCode::E600,
                span,
                "CBOR-LD record missing 'hex' or 'bytes' field",
            ))
        }
        Value::String(hex) => hex_decode(hex, span),
        Value::List(list) => extract_byte_list(list, span),
        _ => Err(Diagnostic::new(
            DiagCode::E600,
            span,
            "expected CBOR-LD record, hex string, or byte list",
        )),
    }
}

fn extract_byte_list(list: &[Value], span: Span) -> Result<Vec<u8>, Diagnostic> {
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let n = match item {
            Value::I64(v) if (0..=255).contains(v) => *v as u8,
            Value::U64(v) if *v <= 255 => *v as u8,
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "byte list elements must be integers in 0..=255",
                ))
            }
        };
        out.push(n);
    }
    Ok(out)
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

fn hex_decode(hex: &str, span: Span) -> Result<Vec<u8>, Diagnostic> {
    let clean = hex.trim().strip_prefix("0x").unwrap_or(hex.trim());
    if clean.len() % 2 != 0 {
        return Err(Diagnostic::new(
            DiagCode::E600,
            span,
            "hex string must have even length",
        ));
    }
    let mut out = Vec::with_capacity(clean.len() / 2);
    let bytes = clean.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = (bytes[i] as char).to_digit(16);
        let lo = (bytes[i + 1] as char).to_digit(16);
        match (hi, lo) {
            (Some(h), Some(l)) => out.push(((h << 4) | l) as u8),
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "invalid hex digit in CBOR-LD payload",
                ))
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Duration, Instant, Quantity, QuinRef};

    #[test]
    fn static_and_dynamic_term_compression_round_trip() {
        let static_id = compress_iri("qudt:KiloPascal", DEFAULT_CONTEXT_IRI);
        assert_eq!(static_id, 40);
        assert_eq!(
            expand_term(static_id, DEFAULT_CONTEXT_IRI).as_deref(),
            Some("qudt:KiloPascal")
        );

        let dyn_id = compress_iri("https://alice.org.au/ns#pressure", DEFAULT_CONTEXT_IRI);
        assert_ne!(dyn_id & DYNAMIC_TERM_BIT, 0);
        assert_eq!(dyn_id & !TOKEN_60BIT_MASK, 0);
        let expanded = expand_term(dyn_id, DEFAULT_CONTEXT_IRI).unwrap();
        assert!(expanded.starts_with("did:q42:term:"));
        assert_eq!(compress_iri(&expanded, DEFAULT_CONTEXT_IRI), dyn_id);
    }

    #[test]
    fn encode_decode_all_value_variants_deterministic() {
        let mut rec = BTreeMap::new();
        rec.insert("ui:target_id".to_string(), Value::String("btn_1".into()));
        rec.insert(
            "ui:value".to_string(),
            Value::Quantity(Quantity::new(101.325, "qudt:KiloPascal")),
        );
        rec.insert("instant".to_string(), Value::Instant(Instant::unix(1700000000, 42)));
        rec.insert("duration".to_string(), Value::Duration(Duration::from_nanos(5_000_000)));
        rec.insert(
            "quin".to_string(),
            Value::QuinRef(QuinRef::from_quin(11, 22, 33, 44)),
        );
        rec.insert(
            "denied".to_string(),
            Value::DisclosureDenied {
                capability: "clinical.read".into(),
                reason: "consent required".into(),
            },
        );
        rec.insert("iri".to_string(), Value::Iri("qudt:Meter".into()));
        rec.insert("u64_val".to_string(), Value::U64(999));
        let root = Value::Record(rec);

        let bytes = encode_value_cbor_ld(&root, DEFAULT_CONTEXT_IRI);
        // 0xD9 0x0501 = compressed CBOR-LD header
        assert_eq!(&bytes[0..3], &[0xD9, 0x05, 0x01]);
        let (ctx, decoded) = decode_value_cbor_ld(&bytes).unwrap();
        assert_eq!(ctx, DEFAULT_CONTEXT_IRI);
        assert_eq!(decoded, root);

        let span = Span::new(0, 0);
        let enc_val = call_cbor_ld("cbor_ld.encode", &[root.clone()], span)
            .unwrap()
            .unwrap();
        let det = call_cbor_ld("cbor_ld.verify_deterministic", &[enc_val.clone()], span)
            .unwrap()
            .unwrap();
        assert_eq!(det, Value::Bool(true));
        let dec_val = call_cbor_ld("cbor_ld.decode", &[enc_val], span)
            .unwrap()
            .unwrap();
        assert_eq!(dec_val, root);
    }

    #[test]
    fn provider_tld_json_namespace_uplift_and_cbor_ld_encoding() {
        let json = r#"{"patient_id": "p-42", "systolic_kpa": 16.0, "active": true}"#;
        let res = ingest_json_with_provider_ns(json, "https://api.hospital.health/v1").unwrap();
        let Value::Record(map) = &res else {
            panic!("expected Record from ingest_json_with_provider_ns");
        };
        assert_eq!(
            map.get("provider_host"),
            Some(&Value::String("api.hospital.health".into()))
        );
        assert_eq!(
            map.get("provider_tld"),
            Some(&Value::String("health".into()))
        );
        assert_eq!(
            map.get("context"),
            Some(&Value::String("https://api.hospital.health/ns#".into()))
        );

        let span = Span::new(0, 0);
        let decoded = call_cbor_ld("cbor_ld.decode", &[res], span).unwrap().unwrap();
        let Value::Record(uplifted) = decoded else {
            panic!("expected decoded Record");
        };
        assert_eq!(
            uplifted.get("https://api.hospital.health/ns#patient_id"),
            Some(&Value::String("p-42".into()))
        );
        assert_eq!(
            uplifted.get("@provider_tld"),
            Some(&Value::String("health".into()))
        );

        // Also test bare TLD input like ".gov.au"
        let (host, tld, ns) = derive_provider_namespace(".gov.au").unwrap();
        assert_eq!(host, "tld.gov.au");
        assert_eq!(tld, "au");
        assert_eq!(ns, "https://tld.gov.au/ns#");
    }
}
