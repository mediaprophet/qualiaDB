//! Constraint Validation API (`ValidityState`), form-level validation, cross-field relational
//! invariants, and Belnap 4-valued paraconsistent ($E_\tau$) conflict evaluation
//! (`validate.*` stdlib bindings for VibeScript v0.02 Zero-JS UI).

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::quantity::UnitId;
use crate::span::Span;
use crate::value::Value;

/// Dispatch a `validate.*` call in the VibeScript engine.
pub fn call_validation(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "validate.check_field" => eval_check_field(args, span).map(Some),
        "validate.check_form" => eval_check_form(args, span).map(Some),
        "validate.cross_field" => eval_cross_field(args, span).map(Some),
        "validate.paraconsistent" => eval_paraconsistent(args, span).map(Some),
        _ => Ok(None),
    }
}

/// `validate.check_field(value: Value, rules: Record)`
pub fn eval_check_field(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let value = args.first().unwrap_or(&Value::Null);
    let Some(Value::Record(rules)) = args.get(1) else {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "validate.check_field: argument #1 (rules) must be a Record",
        ));
    };
    Ok(Value::Record(check_field_against_rules(value, rules)))
}

pub(crate) fn check_field_against_rules(
    value: &Value,
    rules: &BTreeMap<String, Value>,
) -> BTreeMap<String, Value> {
    let required = matches!(rules.get("required"), Some(Value::Bool(true)));
    let is_empty = match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) | Value::Iri(s) => s.trim().is_empty(),
        Value::List(xs) => xs.is_empty(),
        _ => false,
    };

    let value_missing = required && is_empty;
    let (mut type_mismatch, mut pattern_mismatch, mut too_short, mut too_long) =
        (false, false, false, false);
    let (mut range_underflow, mut range_overflow, mut step_mismatch, mut unit_mismatch) =
        (false, false, false, false);
    let mut bad_input = matches!(rules.get("bad_input"), Some(Value::Bool(true)));
    let custom_err_msg = match rules.get("custom_error") {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Bool(true)) => Some("Custom validation error".to_string()),
        _ => None,
    };
    let custom_error = custom_err_msg.is_some();

    if !is_empty {
        if let Some(Value::String(t)) = rules
            .get("type")
            .or_else(|| rules.get("kind"))
            .or_else(|| rules.get("data_type"))
        {
            let s_opt = match value {
                Value::String(s) | Value::Iri(s) => Some(s.as_str()),
                _ => None,
            };
            match t.as_str() {
                "email" => type_mismatch = !s_opt.is_some_and(is_valid_email),
                "url" | "iri" => type_mismatch = !s_opt.is_some_and(is_valid_iri),
                "did" => type_mismatch = !s_opt.is_some_and(is_valid_did),
                "tel" => type_mismatch = !s_opt.is_some_and(is_valid_tel),
                "number" => {
                    let ok = matches!(value, Value::I64(_) | Value::U64(_) | Value::Quantity(_))
                        || matches!(value, Value::F64(n) if !n.is_nan())
                        || s_opt.is_some_and(|s| s.trim().parse::<f64>().is_ok());
                    if !ok {
                        bad_input = true;
                        type_mismatch = true;
                    }
                }
                _ => {}
            }
        }
        if let Value::String(s) | Value::Iri(s) = value {
            let n = s.chars().count() as i64;
            too_short = rules.get("min_length").and_then(Value::as_i64).is_some_and(|lo| n < lo);
            too_long = rules.get("max_length").and_then(Value::as_i64).is_some_and(|hi| n > hi);
            if let Some(Value::String(pat)) = rules.get("pattern") {
                pattern_mismatch = !pat.is_empty() && !matches_simple_pattern(s, pat);
            }
        }
        let num_val = value.as_f64().or_else(|| match value {
            Value::String(s) => s.trim().parse::<f64>().ok(),
            Value::Instant(i) => Some(i.secs as f64),
            _ => None,
        });
        if let Some(num) = num_val {
            let min_b = rules.get("min").and_then(Value::as_f64);
            let max_b = rules.get("max").and_then(Value::as_f64);
            range_underflow = min_b.is_some_and(|lo| num < lo);
            range_overflow = max_b.is_some_and(|hi| num > hi);
            if let Some(step) = rules.get("step").and_then(Value::as_f64).filter(|&s| s > 0.0) {
                let rem = ((num - min_b.unwrap_or(0.0)) / step).fract().abs();
                step_mismatch = rem > 1e-6 && (1.0 - rem).abs() > 1e-6;
            }
        }
        if let Some(Value::String(exp) | Value::Iri(exp)) = rules.get("expected_unit") {
            if !exp.is_empty() {
                let unit_ok = |u: &str| u == exp || UnitId::new(u).compatible(&UnitId::new(exp));
                unit_mismatch = match value {
                    Value::Quantity(q) => !unit_ok(&q.unit),
                    Value::Record(r) => !matches!(r.get("unit"), Some(Value::String(u) | Value::Iri(u)) if unit_ok(u)),
                    _ => true,
                };
            }
        }
    }

    let flags = [
        ("value_missing", value_missing, "Value is required"),
        ("bad_input", bad_input, "Bad numeric or structured input"),
        ("type_mismatch", type_mismatch, "Value does not match expected type"),
        ("pattern_mismatch", pattern_mismatch, "Value does not match required pattern"),
        ("too_short", too_short, "Value is shorter than minimum length"),
        ("too_long", too_long, "Value exceeds maximum length"),
        ("range_underflow", range_underflow, "Value is below minimum bound"),
        ("range_overflow", range_overflow, "Value exceeds maximum bound"),
        ("step_mismatch", step_mismatch, "Value does not align with step increment"),
        ("unit_mismatch", unit_mismatch, "Unit mismatch with expected physical dimension"),
        ("custom_error", custom_error, "Custom validation error"),
    ];
    let valid = flags.iter().all(|(_, failed, _)| !*failed);
    let msg = if valid {
        String::new()
    } else {
        custom_err_msg.unwrap_or_else(|| {
            flags.iter().find(|(_, f, _)| *f).map(|(_, _, m)| (*m).to_string()).unwrap_or_default()
        })
    };

    let mut rec = BTreeMap::new();
    for (k, v, _) in flags {
        rec.insert(k.to_string(), Value::Bool(v));
    }
    rec.insert("valid".to_string(), Value::Bool(valid));
    rec.insert("validation_message".to_string(), Value::String(msg));
    rec
}

/// `validate.check_form(values: Record, rules_by_field: Record)`
pub fn eval_check_form(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let (Some(Value::Record(values)), Some(Value::Record(rules_by_field))) = (args.first(), args.get(1)) else {
        return Err(Diagnostic::new(DiagCode::E100, span, "validate.check_form: expected (values, rules_by_field) Records"));
    };
    let (mut invalid_fields, mut reports) = (Vec::new(), BTreeMap::new());
    for (field_id, rule_val) in rules_by_field {
        let Value::Record(rules) = rule_val else { continue };
        let field_val = values
            .get(field_id)
            .or_else(|| match values.get("values") { Some(Value::Record(inner)) => inner.get(field_id), _ => None })
            .unwrap_or(&Value::Null);
        let report = check_field_against_rules(field_val, rules);
        if report.get("valid") != Some(&Value::Bool(true)) {
            invalid_fields.push(Value::String(field_id.clone()));
        }
        reports.insert(field_id.clone(), Value::Record(report));
    }
    let first_invalid = invalid_fields.first().cloned().unwrap_or(Value::Null);
    Ok(Value::Record(BTreeMap::from([
        ("valid".to_string(), Value::Bool(invalid_fields.is_empty())),
        ("invalid_fields".to_string(), Value::List(invalid_fields)),
        ("first_invalid_field".to_string(), first_invalid),
        ("reports".to_string(), Value::Record(reports)),
    ])))
}

/// `validate.cross_field(rule_name: String, condition: Bool, fields: List<Value>, message: String)`
pub fn eval_cross_field(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let args = if args.len() == 5 && matches!(args.first(), Some(Value::Record(_))) { &args[1..] } else { args };
    let (Some(Value::String(rule_name)), Some(Value::Bool(cond)), Some(Value::List(fields)), Some(Value::String(msg))) =
        (args.first(), args.get(1), args.get(2), args.get(3))
    else {
        return Err(Diagnostic::new(DiagCode::E100, span, "validate.cross_field: expected (rule_name, condition, fields, message)"));
    };
    Ok(Value::Record(BTreeMap::from([
        ("rule_name".to_string(), Value::String(rule_name.clone())),
        ("valid".to_string(), Value::Bool(*cond)),
        ("fields".to_string(), Value::List(fields.clone())),
        ("validation_message".to_string(), Value::String(if *cond { String::new() } else { msg.clone() })),
    ])))
}

/// `validate.paraconsistent(field_id: String, claims: List<Record>)`
pub fn eval_paraconsistent(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let (Some(Value::String(field_id) | Value::Iri(field_id)), Some(Value::List(claims))) = (args.first(), args.get(1)) else {
        return Err(Diagnostic::new(DiagCode::E100, span, "validate.paraconsistent: expected (field_id, claims)"));
    };
    let mut distinct_values: Vec<Value> = Vec::new();
    let (mut pooled_mu, mut pooled_lambda) = (0.0_f64, 0.0_f64);
    for item in claims {
        if let Value::Record(c) = item {
            if let Some(v) = c.get("value").filter(|v| !matches!(v, Value::Null)) {
                if !distinct_values.contains(v) {
                    distinct_values.push(v.clone());
                }
            }
            let mu = c.get("support_mu").or_else(|| c.get("mu")).and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0);
            let lam = c.get("refutation_lambda").or_else(|| c.get("lambda")).and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0);
            pooled_mu = pooled_mu.max(mu);
            pooled_lambda = pooled_lambda.max(lam);
        }
    }
    let distinct_count = distinct_values.len() as i64;
    let belnap = if distinct_count > 1 || (pooled_mu >= 0.5 && pooled_lambda >= 0.5) {
        "Both"
    } else if pooled_mu >= 0.5 {
        "True"
    } else if pooled_lambda >= 0.5 {
        "False"
    } else {
        "Neither"
    };
    Ok(Value::Record(BTreeMap::from([
        ("field_id".to_string(), Value::String(field_id.clone())),
        ("belnap_state".to_string(), Value::String(belnap.to_string())),
        ("mu".to_string(), Value::F64(pooled_mu)),
        ("lambda".to_string(), Value::F64(pooled_lambda)),
        ("quarantine_routed".to_string(), Value::Bool(belnap == "Both")),
        ("distinct_values_count".to_string(), Value::I64(distinct_count)),
    ])))
}

fn is_valid_email(s: &str) -> bool {
    let mut p = s.split('@');
    matches!((p.next(), p.next(), p.next()), (Some(l), Some(d), None) if !l.is_empty() && !d.is_empty() && !s.chars().any(char::is_whitespace) && d.contains('.') && !d.starts_with('.') && !d.ends_with('.'))
}

fn is_valid_iri(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(char::is_whitespace) && (s.contains("://") || s.starts_with("urn:") || s.starts_with("did:"))
}

fn is_valid_did(s: &str) -> bool {
    let segs: Vec<&str> = s.split(':').collect();
    segs.len() >= 3 && segs[0] == "did" && segs.iter().all(|p| !p.is_empty()) && !s.chars().any(char::is_whitespace)
}

fn is_valid_tel(s: &str) -> bool {
    s.chars().filter(char::is_ascii_digit).count() >= 3
        && s.chars().all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | ' ' | '(' | ')' | '.' | 'x'))
}

fn matches_simple_pattern(text: &str, pattern: &str) -> bool {
    let pat = pattern.strip_prefix('^').unwrap_or(pattern).strip_suffix('$').unwrap_or(pattern);
    if pat.contains('|') {
        return pat.split('|').any(|alt| matches_simple_pattern(text, alt));
    }
    let chars: Vec<char> = text.chars().collect();
    let mut positions = vec![0usize];
    for (atom, min_rep, max_rep) in parse_pattern_atoms(pat) {
        let mut next = Vec::new();
        for &pos in &positions {
            if min_rep == 0 && !next.contains(&pos) {
                next.push(pos);
            }
            let mut m = 0usize;
            while pos + m < chars.len() && m < max_rep && atom_matches(&atom, chars[pos + m]) {
                m += 1;
                if m >= min_rep && !next.contains(&(pos + m)) {
                    next.push(pos + m);
                }
            }
        }
        if next.is_empty() {
            return false;
        }
        positions = next;
    }
    positions.contains(&chars.len())
}

fn atom_matches(atom: &str, c: char) -> bool {
    match atom {
        "." => true,
        "\\d" => c.is_ascii_digit(),
        "\\w" => c.is_ascii_alphanumeric() || c == '_',
        "\\s" => c.is_whitespace(),
        s if s.starts_with('[') => {
            let b: Vec<char> = s[1..s.len() - 1].chars().collect();
            (0..b.len()).any(|i| {
                (i + 2 < b.len() && b[i + 1] == '-' && c >= b[i] && c <= b[i + 2])
                    || (b[i] != '-' && b[i] == c)
            })
        }
        s => s.starts_with(c),
    }
}

fn parse_pattern_atoms(pat: &str) -> Vec<(String, usize, usize)> {
    let (c, mut i, mut out): (Vec<char>, usize, Vec<_>) = (pat.chars().collect(), 0, Vec::new());
    while i < c.len() {
        let atom = if c[i] == '\\' && i + 1 < c.len() {
            i += 2;
            if matches!(c[i - 1], 'd' | 'w' | 's') { format!("\\{}", c[i - 1]) } else { c[i - 1].to_string() }
        } else if c[i] == '[' {
            let end = c[i..].iter().position(|&x| x == ']').map_or(c.len(), |p| i + p + 1);
            let s: String = c[i..end].iter().collect();
            i = end;
            s
        } else {
            i += 1;
            c[i - 1].to_string()
        };
        let (lo, hi) = match c.get(i) {
            Some('*') => { i += 1; (0, usize::MAX) }
            Some('+') => { i += 1; (1, usize::MAX) }
            Some('?') => { i += 1; (0, 1) }
            Some('{') => {
                let end = c[i..].iter().position(|&x| x == '}').map_or(c.len(), |p| i + p);
                let spec: String = c[i + 1..end].iter().collect();
                i = (end + 1).min(c.len());
                match spec.split_once(',') {
                    Some((a, b)) => {
                        let lo = a.trim().parse().unwrap_or(0);
                        (lo, if b.trim().is_empty() { usize::MAX } else { b.trim().parse().unwrap_or(lo) })
                    }
                    None => {
                        let n = spec.trim().parse().unwrap_or(1);
                        (n, n)
                    }
                }
            }
            _ => (1, 1),
        };
        out.push((atom, lo, hi));
    }
    out
}
