//! Immutable RFC 3986 URI Value Codec (`uri.*`) and Machine-Serializable
//! Structured Error Values (`error.*`) (`vibe-0.2`).

use std::collections::BTreeMap;

use super::reflection::hex_encode;
use crate::cbor_ast::{call_cbor_ld, encode_value_cbor_ld, DEFAULT_CONTEXT_IRI};
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Dispatches `uri.*` and `error.*` standard library calls.
pub fn call_uri_and_error(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "uri.parse" => {
            let s = extract_str(args.first(), "uri.parse", span)?;
            Ok(Some(parse_uri_value(s)))
        }
        "uri.format" => {
            let Some(Value::Record(rec)) = args.first() else {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "uri.format expects a URI Record argument",
                ));
            };
            Ok(Some(Value::String(format_uri_record(rec))))
        }
        "uri.encode_component" => {
            let s = extract_str(args.first(), "uri.encode_component", span)?;
            Ok(Some(Value::String(percent_encode(s))))
        }
        "uri.decode_component" => {
            let s = extract_str(args.first(), "uri.decode_component", span)?;
            match percent_decode(s) {
                Ok(decoded) => Ok(Some(Value::String(decoded))),
                Err(err) => {
                    let mut rec = BTreeMap::new();
                    rec.insert("ok".to_string(), Value::Bool(false));
                    rec.insert("value".to_string(), Value::String(String::new()));
                    rec.insert("error".to_string(), Value::String(err));
                    Ok(Some(Value::Record(rec)))
                }
            }
        }
        "uri.join" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "uri.join requires (base, segment)",
                ));
            }
            let base = extract_str(Some(&args[0]), "uri.join", span)?;
            let seg = extract_str(Some(&args[1]), "uri.join", span)?;
            Ok(Some(Value::String(join_uri(base, seg))))
        }
        "uri.with_query" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "uri.with_query requires (uri_str, params_record)",
                ));
            }
            let uri_str = extract_str(Some(&args[0]), "uri.with_query", span)?;
            let Value::Record(params) = &args[1] else {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "uri.with_query expects a Record as argument 1",
                ));
            };
            Ok(Some(Value::String(uri_with_query(uri_str, params))))
        }
        "error.create" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "error.create requires (code, message[, meta])",
                ));
            }
            let code = extract_str(Some(&args[0]), "error.create", span)?;
            let msg = extract_str(Some(&args[1]), "error.create", span)?;
            let meta = args.get(2).cloned().unwrap_or(Value::Null);
            Ok(Some(create_error_value(code, msg, meta)))
        }
        "error.is_error" => {
            let is_err = match args.first() {
                Some(Value::Err(_)) => true,
                Some(Value::Record(map)) => {
                    matches!(map.get("__error"), Some(Value::Bool(true)))
                }
                _ => false,
            };
            Ok(Some(Value::Bool(is_err)))
        }
        "error.to_cbor_ld" => {
            let val = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E600, span, "error.to_cbor_ld requires 1 argument")
            })?;
            call_cbor_ld("cbor_ld.encode", &[val.clone()], span)
        }
        _ => Ok(None),
    }
}

fn extract_str<'a>(val: Option<&'a Value>, op: &str, span: Span) -> Result<&'a str, Diagnostic> {
    match val {
        Some(Value::String(s)) | Some(Value::Iri(s)) => Ok(s.as_str()),
        _ => Err(Diagnostic::new(
            DiagCode::E600,
            span,
            format!("{op} expects a String or Iri argument"),
        )),
    }
}

/// Parses an RFC 3986 URI string into a total, non-throwing [`Value::Record`].
pub fn parse_uri_value(raw: &str) -> Value {
    let trimmed = raw.trim();
    let (before_frag, fragment) = match trimmed.split_once('#') {
        Some((b, f)) => (b, f.to_string()),
        None => (trimmed, String::new()),
    };
    let (before_query, query_str) = match before_frag.split_once('?') {
        Some((b, q)) => (b, q),
        None => (before_frag, ""),
    };

    let mut scheme = String::new();
    let mut host = String::new();
    let mut port: i64 = -1;
    let path: String;
    let mut valid = !trimmed.is_empty() && !trimmed.chars().any(|c| c.is_whitespace());

    if let Some((sch, rest)) = before_query.split_once("://") {
        scheme = sch.to_ascii_lowercase();
        let (authority, p) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, ""),
        };
        path = if p.is_empty() {
            "/".to_string()
        } else {
            p.to_string()
        };
        if let Some((h, port_s)) = authority.split_once(':') {
            host = h.to_ascii_lowercase();
            match port_s.parse::<i64>() {
                Ok(n) if (0..=65535).contains(&n) => port = n,
                _ => valid = false,
            }
        } else {
            host = authority.to_ascii_lowercase();
        }
        if scheme.is_empty() || host.is_empty() {
            valid = false;
        }
    } else if let Some((sch, rest)) = before_query.split_once(':') {
        scheme = sch.to_ascii_lowercase();
        path = rest.to_string();
    } else {
        path = before_query.to_string();
    }

    let tld = host
        .rsplit('.')
        .next()
        .filter(|s| !s.is_empty() && host.contains('.'))
        .unwrap_or("")
        .to_string();

    let segments: Vec<Value> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| Value::String(s.to_string()))
        .collect();

    let mut query_map = BTreeMap::new();
    if !query_str.is_empty() {
        for pair in query_str.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k_raw, v_raw) = pair.split_once('=').unwrap_or((pair, ""));
            let k = percent_decode(k_raw).unwrap_or_else(|_| k_raw.to_string());
            let v = percent_decode(v_raw).unwrap_or_else(|_| v_raw.to_string());
            query_map.insert(k, Value::String(v));
        }
    }

    let mut rec = BTreeMap::new();
    rec.insert("valid".to_string(), Value::Bool(valid));
    rec.insert("scheme".to_string(), Value::String(scheme));
    rec.insert("host".to_string(), Value::String(host));
    rec.insert("tld".to_string(), Value::String(tld));
    rec.insert("port".to_string(), Value::I64(port));
    rec.insert("path".to_string(), Value::String(path));
    rec.insert("segments".to_string(), Value::List(segments));
    rec.insert("query".to_string(), Value::Record(query_map));
    rec.insert("fragment".to_string(), Value::String(fragment));
    let normalized = format_uri_record(&rec);
    rec.insert("normalized".to_string(), Value::String(normalized));
    Value::Record(rec)
}

/// Formats a URI record back into a canonical RFC 3986 URI string.
pub fn format_uri_record(rec: &BTreeMap<String, Value>) -> String {
    let scheme = rec_str(rec, "scheme");
    let host = rec_str(rec, "host");
    let port = rec.get("port").and_then(|v| v.as_i64()).unwrap_or(-1);
    let path = rec_str(rec, "path");
    let fragment = rec_str(rec, "fragment");

    let mut out = String::new();
    if !scheme.is_empty() && !host.is_empty() {
        out.push_str(scheme);
        out.push_str("://");
        out.push_str(host);
        if (0..=65535).contains(&port) {
            out.push(':');
            out.push_str(&port.to_string());
        }
        if !path.is_empty() && !path.starts_with('/') {
            out.push('/');
        }
        out.push_str(path);
    } else if !scheme.is_empty() {
        out.push_str(scheme);
        out.push(':');
        out.push_str(path);
    } else {
        out.push_str(path);
    }

    if let Some(Value::Record(qmap)) = rec.get("query") {
        if !qmap.is_empty() {
            out.push('?');
            for (i, (k, v)) in qmap.iter().enumerate() {
                if i > 0 {
                    out.push('&');
                }
                out.push_str(&percent_encode(k));
                out.push('=');
                let val_s = match v {
                    Value::String(s) | Value::Iri(s) => s.clone(),
                    other => other.to_string(),
                };
                out.push_str(&percent_encode(&val_s));
            }
        }
    }
    if !fragment.is_empty() {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

fn rec_str<'a>(rec: &'a BTreeMap<String, Value>, key: &str) -> &'a str {
    match rec.get(key) {
        Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
        _ => "",
    }
}

/// RFC 3986 unreserved percent-encoding (`A-Z a-z 0-9 - _ . ~`).
pub fn percent_encode(input: &str) -> String {
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

/// Safe UTF-8 percent-decoding without panics or exceptions.
pub fn percent_decode(input: &str) -> Result<String, String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("incomplete percent-encoding sequence".into());
            }
            let hi = (bytes[i + 1] as char)
                .to_digit(16)
                .ok_or("invalid hex digit in percent-encoding")?;
            let lo = (bytes[i + 2] as char)
                .to_digit(16)
                .ok_or("invalid hex digit in percent-encoding")?;
            out.push(((hi << 4) | lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "decoded bytes are not valid UTF-8".into())
}

fn join_uri(base: &str, segment: &str) -> String {
    let b = base.trim_end_matches('/');
    let s = segment.trim_start_matches('/');
    if b.is_empty() {
        format!("/{s}")
    } else if s.is_empty() {
        b.to_string()
    } else {
        format!("{b}/{s}")
    }
}

fn uri_with_query(uri_str: &str, params: &BTreeMap<String, Value>) -> String {
    let Value::Record(mut parsed) = parse_uri_value(uri_str) else {
        return uri_str.to_string();
    };
    let mut query_map = match parsed.remove("query") {
        Some(Value::Record(m)) => m,
        _ => BTreeMap::new(),
    };
    for (k, v) in params {
        let s = match v {
            Value::String(st) | Value::Iri(st) => st.clone(),
            Value::I64(n) => n.to_string(),
            Value::U64(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            other => other.to_string(),
        };
        query_map.insert(k.clone(), Value::String(s));
    }
    parsed.insert("query".to_string(), Value::Record(query_map));
    format_uri_record(&parsed)
}

/// Creates a machine-serializable structured error record with precomputed CBOR-LD payload.
pub fn create_error_value(code: &str, message: &str, meta: Value) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("__error".to_string(), Value::Bool(true));
    rec.insert("code".to_string(), Value::String(code.to_string()));
    rec.insert("message".to_string(), Value::String(message.to_string()));
    rec.insert("meta".to_string(), meta);
    let prelim = Value::Record(rec.clone());
    let cbor = encode_value_cbor_ld(&prelim, DEFAULT_CONTEXT_IRI);
    rec.insert("cbor_ld_hex".to_string(), Value::String(hex_encode(&cbor)));
    Value::Record(rec)
}
