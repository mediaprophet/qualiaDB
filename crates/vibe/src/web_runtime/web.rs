//! HTTP response, range-slice, relationship mail routing, and P2P permit primitives (`web.*`).

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Strict Content-Security-Policy header enforced by `web.response` to eradicate JavaScript execution.
pub const ZERO_JS_CSP_HEADER: &str =
    "default-src 'self'; script-src 'none'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";

/// Dispatch a `web.*` call in the VibeScript engine.
pub fn call_web(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "web.response" => {
            let status = extract_i64(args, 0, "web.response(status, content_type, body)", span)?;
            let content_type =
                extract_str(args, 1, "web.response(status, content_type, body)", span)?;
            let body_val = args.get(2).cloned().unwrap_or(Value::String(String::new()));
            let body_str = match &body_val {
                Value::String(s) | Value::Iri(s) => s.clone(),
                Value::Record(r) => r
                    .get("hex")
                    .and_then(|v| match v {
                        Value::String(h) => Some(h.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| format!("{body_val:?}")),
                _ => format!("{body_val:?}"),
            };

            let mut headers = BTreeMap::new();
            headers.insert(
                "Content-Security-Policy".to_string(),
                Value::String(ZERO_JS_CSP_HEADER.to_string()),
            );
            headers.insert(
                "X-Webizen-Runtime".to_string(),
                Value::String("vibescript/vbc1; zero-js=1; cbor-ld=1".to_string()),
            );

            let mut rec = BTreeMap::new();
            rec.insert("status".to_string(), Value::I64(status));
            rec.insert("content_type".to_string(), Value::String(content_type));
            rec.insert("body".to_string(), Value::String(body_str));
            rec.insert("payload".to_string(), body_val);
            rec.insert("zero_js".to_string(), Value::Bool(true));
            rec.insert("headers".to_string(), Value::Record(headers));
            Ok(Some(Value::Record(rec)))
        }
        "web.range_response" => {
            let total = extract_i64(
                args,
                0,
                "web.range_response(total_bytes, start_byte, end_byte, crc32c)",
                span,
            )?;
            let start = extract_i64(
                args,
                1,
                "web.range_response(total_bytes, start_byte, end_byte, crc32c)",
                span,
            )?;
            let end = extract_i64(
                args,
                2,
                "web.range_response(total_bytes, start_byte, end_byte, crc32c)",
                span,
            )?;
            let crc = extract_str(
                args,
                3,
                "web.range_response(total_bytes, start_byte, end_byte, crc32c)",
                span,
            )?;

            if start < 0 || end < start || total <= end {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "web.range_response: invalid byte range bounds",
                ));
            }
            let slice_len = (end - start) + 1;
            let content_range = format!("bytes {start}-{end}/{total}");

            let mut rec = BTreeMap::new();
            rec.insert("status".to_string(), Value::I64(206));
            rec.insert(
                "accept_ranges".to_string(),
                Value::String("bytes".to_string()),
            );
            rec.insert("content_range".to_string(), Value::String(content_range));
            rec.insert("content_length".to_string(), Value::I64(slice_len));
            rec.insert("crc32c".to_string(), Value::String(crc));
            rec.insert("zero_js".to_string(), Value::Bool(true));
            Ok(Some(Value::Record(rec)))
        }
        "web.route_mail" => {
            let alias = extract_str(
                args,
                0,
                "web.route_mail(alias, sender_email, allowed_sender)",
                span,
            )?;
            let sender = extract_str(
                args,
                1,
                "web.route_mail(alias, sender_email, allowed_sender)",
                span,
            )?;
            let allowed = extract_str(
                args,
                2,
                "web.route_mail(alias, sender_email, allowed_sender)",
                span,
            )?;

            let local_part = alias.split('@').next().unwrap_or("unknown").to_string();
            let mut rec = BTreeMap::new();
            rec.insert("alias".to_string(), Value::String(alias));
            rec.insert(
                "relationship".to_string(),
                Value::String(local_part.clone()),
            );

            if allowed.starts_with("hook:") {
                rec.insert(
                    "verdict".to_string(),
                    Value::String("ExecuteProgrammableHook".to_string()),
                );
                rec.insert("hook".to_string(), Value::String(allowed));
                rec.insert("admitted".to_string(), Value::Bool(true));
            } else if sender.eq_ignore_ascii_case(&allowed) {
                rec.insert(
                    "verdict".to_string(),
                    Value::String("DeliverToRelationshipThread".to_string()),
                );
                rec.insert("admitted".to_string(), Value::Bool(true));
            } else {
                rec.insert(
                    "verdict".to_string(),
                    Value::String("RejectedUnauthorizedSender".to_string()),
                );
                rec.insert(
                    "pinpointed_leak_source".to_string(),
                    Value::String(local_part),
                );
                rec.insert("admitted".to_string(), Value::Bool(false));
            }
            Ok(Some(Value::Record(rec)))
        }
        "web.check_permit" => {
            let tier = extract_i64(args, 0, "web.check_permit", span)?;
            let util_pct = extract_i64(args, 1, "web.check_permit", span)?;
            let req_units = extract_i64(args, 2, "web.check_permit", span)?;
            let peer_quota = extract_i64(args, 3, "web.check_permit", span)?;

            let tier_ceiling = match tier {
                0 => 100,
                1 => 95,
                2 => 80,
                _ => 50,
            };

            let admitted = req_units <= peer_quota && util_pct <= tier_ceiling;
            let status = if req_units > peer_quota {
                "DeniedPeerQuotaExhausted"
            } else if util_pct > tier_ceiling {
                "ShedByHostBudgetPriorityRule"
            } else {
                "Admitted"
            };

            let mut rec = BTreeMap::new();
            rec.insert("admitted".to_string(), Value::Bool(admitted));
            rec.insert("status".to_string(), Value::String(status.to_string()));
            rec.insert("tier_ceiling_pct".to_string(), Value::I64(tier_ceiling));
            rec.insert(
                "remaining_peer_units".to_string(),
                Value::I64((peer_quota - req_units).max(0)),
            );
            Ok(Some(Value::Record(rec)))
        }
        _ => Ok(None),
    }
}

fn extract_str(args: &[Value], idx: usize, ctx: &str, span: Span) -> Result<String, Diagnostic> {
    match args.get(idx) {
        Some(Value::String(s)) | Some(Value::Iri(s)) => Ok(s.clone()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a string"),
        )),
    }
}

fn extract_i64(args: &[Value], idx: usize, ctx: &str, span: Span) -> Result<i64, Diagnostic> {
    match args.get(idx) {
        Some(Value::I64(n)) => Ok(*n),
        Some(Value::U64(n)) => Ok(*n as i64),
        Some(Value::F64(n)) => Ok(*n as i64),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a number"),
        )),
    }
}
