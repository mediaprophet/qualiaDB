//! Strict Non-Coercing Numeric Parsers (`num.parse_*`) and Exact Base-10 `Decimal`
//! Arithmetic (`decimal.*`) for VibeScript v0.02.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::{extract_i64, extract_str, opt_i64, opt_str};

/// Internal exact base-10 decimal (`value = mantissa * 10^-scale`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Dec {
    pub mantissa: i128,
    pub scale: u32,
}

/// Dispatch `num.parse_*` and `decimal.*` calls.
pub fn call_decimal(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "num.parse_i64" => {
            let Some(Value::String(s)) = args.first() else {
                return Ok(Some(parse_res(false, Value::Null, "non-string input")));
            };
            let radix = opt_i64(args, 1, 10);
            if !(2..=36).contains(&radix) {
                return Ok(Some(parse_res(false, Value::Null, "radix out of range 2..=36")));
            }
            Ok(Some(strict_parse_i64(s, radix as u32)))
        }
        "num.parse_u64" => {
            let Some(Value::String(s)) = args.first() else {
                return Ok(Some(parse_res(false, Value::Null, "non-string input")));
            };
            let radix = opt_i64(args, 1, 10);
            if !(2..=36).contains(&radix) {
                return Ok(Some(parse_res(false, Value::Null, "radix out of range 2..=36")));
            }
            Ok(Some(strict_parse_u64(s, radix as u32)))
        }
        "num.parse_f64" => {
            let Some(Value::String(s)) = args.first() else {
                return Ok(Some(parse_res(false, Value::Null, "non-string input")));
            };
            Ok(Some(strict_parse_f64(s)))
        }
        "decimal.from_str" => {
            let s = extract_str(args, 0, "decimal.from_str(text)", span)?;
            let d = parse_dec_str(&s, span)?;
            Ok(Some(dec_to_record(d)))
        }
        "decimal.from_parts" => {
            let mantissa = extract_i64(args, 0, "decimal.from_parts(mantissa, scale)", span)?;
            let scale = extract_i64(args, 1, "decimal.from_parts(mantissa, scale)", span)?;
            if !(0..=28).contains(&scale) {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "decimal.from_parts: scale must be in 0..=28",
                ));
            }
            Ok(Some(dec_to_record(Dec {
                mantissa: mantissa as i128,
                scale: scale as u32,
            })))
        }
        "decimal.add" => {
            let a = coerce_dec(args.first(), "decimal.add", span)?;
            let b = coerce_dec(args.get(1), "decimal.add", span)?;
            let (ma, mb, sc) = align_scales(a, b);
            Ok(Some(dec_to_record(strip_trailing_zeros(Dec {
                mantissa: ma.saturating_add(mb),
                scale: sc,
            }))))
        }
        "decimal.sub" => {
            let a = coerce_dec(args.first(), "decimal.sub", span)?;
            let b = coerce_dec(args.get(1), "decimal.sub", span)?;
            let (ma, mb, sc) = align_scales(a, b);
            Ok(Some(dec_to_record(strip_trailing_zeros(Dec {
                mantissa: ma.saturating_sub(mb),
                scale: sc,
            }))))
        }
        "decimal.mul" => {
            let a = coerce_dec(args.first(), "decimal.mul", span)?;
            let b = coerce_dec(args.get(1), "decimal.mul", span)?;
            Ok(Some(dec_to_record(strip_trailing_zeros(Dec {
                mantissa: a.mantissa.saturating_mul(b.mantissa),
                scale: a.scale + b.scale,
            }))))
        }
        "decimal.div" => {
            let a = coerce_dec(args.first(), "decimal.div", span)?;
            let b = coerce_dec(args.get(1), "decimal.div", span)?;
            if b.mantissa == 0 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "decimal.div: division by zero",
                ));
            }
            let target_scale = opt_i64(args, 2, 10).clamp(0, 24) as u32;
            let shift = target_scale + b.scale;
            let scaled_num = if shift >= a.scale {
                a.mantissa.saturating_mul(pow10(shift - a.scale))
            } else {
                a.mantissa / pow10(a.scale - shift)
            };
            let raw = Dec {
                mantissa: scaled_num.saturating_mul(10) / b.mantissa,
                scale: target_scale + 1,
            };
            let rounded = round_dec(raw, target_scale, "half_away_from_zero", span)?;
            Ok(Some(dec_to_record(strip_trailing_zeros(rounded))))
        }
        "decimal.round" => {
            let d = coerce_dec(args.first(), "decimal.round", span)?;
            let scale = extract_i64(args, 1, "decimal.round(d, scale, mode)", span)?.clamp(0, 28) as u32;
            let mode = opt_str(args, 2, "half_away_from_zero");
            let r = round_dec(d, scale, &mode, span)?;
            Ok(Some(dec_to_record(r)))
        }
        "decimal.to_fixed" => {
            let d = coerce_dec(args.first(), "decimal.to_fixed", span)?;
            let scale = extract_i64(args, 1, "decimal.to_fixed(d, scale)", span)?.clamp(0, 28) as u32;
            let mode = opt_str(args, 2, "half_away_from_zero");
            let r = round_dec(d, scale, &mode, span)?;
            Ok(Some(Value::String(format_dec(r.mantissa, r.scale))))
        }
        "decimal.eq" => {
            let a = coerce_dec(args.first(), "decimal.eq", span)?;
            let b = coerce_dec(args.get(1), "decimal.eq", span)?;
            let (ma, mb, _) = align_scales(a, b);
            Ok(Some(Value::Bool(ma == mb)))
        }
        "decimal.cmp" => {
            let a = coerce_dec(args.first(), "decimal.cmp", span)?;
            let b = coerce_dec(args.get(1), "decimal.cmp", span)?;
            let (ma, mb, _) = align_scales(a, b);
            let ord = if ma < mb {
                -1
            } else if ma > mb {
                1
            } else {
                0
            };
            Ok(Some(Value::I64(ord)))
        }
        _ => Ok(None),
    }
}

fn strict_parse_i64(s: &str, radix: u32) -> Value {
    if s.is_empty() {
        return parse_res(false, Value::Null, "empty string");
    }
    if s != s.trim() {
        return parse_res(false, Value::Null, "leading or trailing whitespace");
    }
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() {
        return parse_res(false, Value::Null, "missing digits");
    }
    let mut seen_digit = false;
    for ch in body.chars() {
        if ch.is_digit(radix) {
            seen_digit = true;
        } else if seen_digit {
            return parse_res(false, Value::Null, "trailing characters");
        } else {
            return parse_res(false, Value::Null, "invalid digit");
        }
    }
    match i64::from_str_radix(s, radix) {
        Ok(n) => parse_res(true, Value::I64(n), ""),
        Err(_) => parse_res(false, Value::Null, "integer overflow"),
    }
}

fn strict_parse_u64(s: &str, radix: u32) -> Value {
    if s.is_empty() {
        return parse_res(false, Value::Null, "empty string");
    }
    if s != s.trim() {
        return parse_res(false, Value::Null, "leading or trailing whitespace");
    }
    if s.starts_with('-') {
        return parse_res(false, Value::Null, "negative value for u64");
    }
    let body = s.strip_prefix('+').unwrap_or(s);
    if body.is_empty() {
        return parse_res(false, Value::Null, "missing digits");
    }
    let mut seen_digit = false;
    for ch in body.chars() {
        if ch.is_digit(radix) {
            seen_digit = true;
        } else if seen_digit {
            return parse_res(false, Value::Null, "trailing characters");
        } else {
            return parse_res(false, Value::Null, "invalid digit");
        }
    }
    match u64::from_str_radix(body, radix) {
        Ok(n) => parse_res(true, Value::U64(n), ""),
        Err(_) => parse_res(false, Value::Null, "integer overflow"),
    }
}

fn strict_parse_f64(s: &str) -> Value {
    if s.is_empty() {
        return parse_res(false, Value::Null, "empty string");
    }
    if s != s.trim() {
        return parse_res(false, Value::Null, "leading or trailing whitespace");
    }
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() {
        return parse_res(false, Value::Null, "missing digits");
    }
    let first = body.chars().next().unwrap_or(' ');
    if !first.is_ascii_digit() && first != '.' {
        return parse_res(false, Value::Null, "invalid float syntax");
    }
    for (idx, ch) in body.char_indices() {
        let valid = ch.is_ascii_digit()
            || ch == '.'
            || ch == 'e'
            || ch == 'E'
            || ((ch == '+' || ch == '-')
                && idx > 0
                && matches!(body.as_bytes()[idx - 1], b'e' | b'E'));
        if !valid {
            return parse_res(false, Value::Null, "trailing characters");
        }
    }
    match s.parse::<f64>() {
        Ok(v) if v.is_finite() => parse_res(true, Value::F64(v), ""),
        _ => parse_res(false, Value::Null, "invalid or non-finite float"),
    }
}

fn parse_res(ok: bool, value: Value, error: &str) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("ok".into(), Value::Bool(ok));
    rec.insert("value".into(), value);
    rec.insert("error".into(), Value::String(error.into()));
    Value::Record(rec)
}

pub(crate) fn coerce_dec(
    val: Option<&Value>,
    ctx: &str,
    span: Span,
) -> Result<Dec, Diagnostic> {
    match val {
        Some(Value::Record(m)) => {
            if let (Some(man), Some(sc)) = (
                m.get("mantissa").and_then(Value::as_i64),
                m.get("scale").and_then(Value::as_i64),
            ) {
                return Ok(Dec {
                    mantissa: man as i128,
                    scale: sc.max(0) as u32,
                });
            }
            if let Some(Value::String(s)) = m.get("text") {
                return parse_dec_str(s, span);
            }
            Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("{ctx}: invalid Decimal record"),
            ))
        }
        Some(Value::String(s)) => parse_dec_str(s, span),
        Some(Value::I64(n)) => Ok(Dec {
            mantissa: *n as i128,
            scale: 0,
        }),
        Some(Value::U64(n)) => Ok(Dec {
            mantissa: *n as i128,
            scale: 0,
        }),
        Some(Value::F64(f)) => parse_dec_str(&format!("{f}"), span),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: expected Decimal, String, or number"),
        )),
    }
}

pub(crate) fn parse_dec_str(s: &str, span: Span) -> Result<Dec, Diagnostic> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(DiagCode::E100, span, "empty decimal string"));
    }
    let (neg, rest) = if let Some(r) = trimmed.strip_prefix('-') {
        (true, r)
    } else if let Some(r) = trimmed.strip_prefix('+') {
        (false, r)
    } else {
        (false, trimmed)
    };
    let (int_part, frac_part) = match rest.split_once('.') {
        Some((i, f)) => (i, f),
        None => (rest, ""),
    };
    if (int_part.is_empty() && frac_part.is_empty())
        || !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
    {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("invalid decimal literal '{s}'"),
        ));
    }
    let scale = frac_part.len() as u32;
    let combined = format!("{int_part}{frac_part}");
    let mag: i128 = combined.parse().map_err(|_| {
        Diagnostic::new(DiagCode::E100, span, format!("decimal overflow '{s}'"))
    })?;
    Ok(Dec {
        mantissa: if neg { -mag } else { mag },
        scale,
    })
}

pub(crate) fn round_dec(
    d: Dec,
    target_scale: u32,
    mode: &str,
    span: Span,
) -> Result<Dec, Diagnostic> {
    if d.scale <= target_scale {
        let factor = pow10(target_scale - d.scale);
        return Ok(Dec {
            mantissa: d.mantissa.saturating_mul(factor),
            scale: target_scale,
        });
    }
    let diff = d.scale - target_scale;
    let factor = pow10(diff);
    let half = factor / 2;
    let q = d.mantissa / factor;
    let r = d.mantissa % factor;
    let abs_r = r.abs();
    let step = if d.mantissa >= 0 { 1 } else { -1 };

    let rounded = match mode {
        "half_away_from_zero" => {
            if abs_r >= half {
                q + step
            } else {
                q
            }
        }
        "half_even" | "bankers" => {
            if abs_r > half || (abs_r == half && q.abs() % 2 != 0) {
                q + step
            } else {
                q
            }
        }
        "floor" => {
            if r < 0 {
                q - 1
            } else {
                q
            }
        }
        "ceil" => {
            if r > 0 {
                q + 1
            } else {
                q
            }
        }
        "trunc" | "toward_zero" => q,
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("unsupported rounding mode '{mode}'"),
            ))
        }
    };
    Ok(Dec {
        mantissa: rounded,
        scale: target_scale,
    })
}

fn align_scales(a: Dec, b: Dec) -> (i128, i128, u32) {
    let sc = a.scale.max(b.scale);
    let ma = a.mantissa.saturating_mul(pow10(sc - a.scale));
    let mb = b.mantissa.saturating_mul(pow10(sc - b.scale));
    (ma, mb, sc)
}

fn strip_trailing_zeros(mut d: Dec) -> Dec {
    while d.scale > 0 && d.mantissa != 0 && d.mantissa % 10 == 0 {
        d.mantissa /= 10;
        d.scale -= 1;
    }
    if d.mantissa == 0 {
        d.scale = 0;
    }
    d
}

pub(crate) fn format_dec(mantissa: i128, scale: u32) -> String {
    if scale == 0 {
        return mantissa.to_string();
    }
    let neg = mantissa < 0;
    let abs = mantissa.unsigned_abs();
    let factor = pow10(scale) as u128;
    let int_part = abs / factor;
    let frac_part = abs % factor;
    let sign = if neg { "-" } else { "" };
    format!("{sign}{int_part}.{frac_part:0width$}", width = scale as usize)
}

fn dec_to_record(d: Dec) -> Value {
    let text = format_dec(d.mantissa, d.scale);
    let mut rec = BTreeMap::new();
    rec.insert("__decimal".into(), Value::Bool(true));
    rec.insert(
        "mantissa".into(),
        Value::I64(d.mantissa.clamp(i64::MIN as i128, i64::MAX as i128) as i64),
    );
    rec.insert("scale".into(), Value::I64(d.scale as i64));
    rec.insert("text".into(), Value::String(text));
    Value::Record(rec)
}

fn pow10(exp: u32) -> i128 {
    10_i128.saturating_pow(exp.min(36))
}
