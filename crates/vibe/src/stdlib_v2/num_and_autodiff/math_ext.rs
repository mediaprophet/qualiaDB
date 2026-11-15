//! Euclidean Division/Modulo, GCD/LCM, Neumaier Compensated Sum, Safe Iterable Extrema,
//! Checked/Saturating Arithmetic, and Seeded SplitMix64 PRNG (`math.*`) for VibeScript v0.02.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::decimal::{coerce_dec, format_dec, round_dec};
use super::{extract_f64, extract_i64, extract_list, opt_i64, opt_str};

/// Dispatch a `math.*` call from `stdlib_v2`.
pub fn call_math_ext(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "math.idiv" => {
            let a = extract_i64(args, 0, "math.idiv(a, b)", span)?;
            let b = extract_i64(args, 1, "math.idiv(a, b)", span)?;
            if b == 0 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "math.idiv: division by zero",
                ));
            }
            Ok(Some(Value::I64(floor_div(a, b))))
        }
        "math.imod" => {
            let a = extract_i64(args, 0, "math.imod(a, b)", span)?;
            let b = extract_i64(args, 1, "math.imod(a, b)", span)?;
            if b == 0 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "math.imod: modulo by zero",
                ));
            }
            Ok(Some(Value::I64(a.rem_euclid(b))))
        }
        "math.divmod" => {
            let a = extract_i64(args, 0, "math.divmod(a, b)", span)?;
            let b = extract_i64(args, 1, "math.divmod(a, b)", span)?;
            if b == 0 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "math.divmod: division by zero",
                ));
            }
            let q = floor_div(a, b);
            let r = a.rem_euclid(b);
            let mut rec = BTreeMap::new();
            rec.insert("quotient".into(), Value::I64(q));
            rec.insert("remainder".into(), Value::I64(r));
            Ok(Some(Value::Record(rec)))
        }
        "math.gcd" => {
            let a = extract_i64(args, 0, "math.gcd(a, b)", span)?;
            let b = extract_i64(args, 1, "math.gcd(a, b)", span)?;
            Ok(Some(Value::I64(gcd_i64(a, b))))
        }
        "math.lcm" => {
            let a = extract_i64(args, 0, "math.lcm(a, b)", span)?;
            let b = extract_i64(args, 1, "math.lcm(a, b)", span)?;
            if a == 0 || b == 0 {
                return Ok(Some(Value::I64(0)));
            }
            let g = gcd_i64(a, b) as i128;
            let l = ((a as i128).abs() / g).saturating_mul((b as i128).abs());
            Ok(Some(Value::I64(l.min(i64::MAX as i128) as i64)))
        }
        "math.lerp" => {
            let a = extract_f64(args, 0, "math.lerp(a, b, t)", span)?;
            let b = extract_f64(args, 1, "math.lerp(a, b, t)", span)?;
            let t = extract_f64(args, 2, "math.lerp(a, b, t)", span)?;
            Ok(Some(Value::F64(a + (b - a) * t)))
        }
        "math.sum_precise" => {
            let list = extract_list(args, 0, "math.sum_precise(list)", span)?;
            let mut sum = 0.0_f64;
            let mut c = 0.0_f64;
            for (idx, item) in list.iter().enumerate() {
                let x = item.as_f64().ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("math.sum_precise: element #{idx} must be a number"),
                    )
                })?;
                let t = sum + x;
                if sum.abs() >= x.abs() {
                    c += (sum - t) + x;
                } else {
                    c += (x - t) + sum;
                }
                sum = t;
            }
            Ok(Some(Value::F64(sum + c)))
        }
        "math.min_list" => eval_list_extrema(args, span, true),
        "math.max_list" => eval_list_extrema(args, span, false),
        "math.add_checked" => {
            let a = extract_i64(args, 0, "math.add_checked(a, b)", span)?;
            let b = extract_i64(args, 1, "math.add_checked(a, b)", span)?;
            Ok(Some(checked_record(a.checked_add(b))))
        }
        "math.sub_checked" => {
            let a = extract_i64(args, 0, "math.sub_checked(a, b)", span)?;
            let b = extract_i64(args, 1, "math.sub_checked(a, b)", span)?;
            Ok(Some(checked_record(a.checked_sub(b))))
        }
        "math.mul_checked" => {
            let a = extract_i64(args, 0, "math.mul_checked(a, b)", span)?;
            let b = extract_i64(args, 1, "math.mul_checked(a, b)", span)?;
            Ok(Some(checked_record(a.checked_mul(b))))
        }
        "math.add_saturating" => {
            let a = extract_i64(args, 0, "math.add_saturating(a, b)", span)?;
            let b = extract_i64(args, 1, "math.add_saturating(a, b)", span)?;
            Ok(Some(Value::I64(a.saturating_add(b))))
        }
        "math.sub_saturating" => {
            let a = extract_i64(args, 0, "math.sub_saturating(a, b)", span)?;
            let b = extract_i64(args, 1, "math.sub_saturating(a, b)", span)?;
            Ok(Some(Value::I64(a.saturating_sub(b))))
        }
        "math.mul_saturating" => {
            let a = extract_i64(args, 0, "math.mul_saturating(a, b)", span)?;
            let b = extract_i64(args, 1, "math.mul_saturating(a, b)", span)?;
            Ok(Some(Value::I64(a.saturating_mul(b))))
        }
        "math.round_mode" => {
            let d = coerce_dec(args.first(), "math.round_mode(x, decimals, mode)", span)?;
            let decimals = opt_i64(args, 1, 0).clamp(0, 20) as u32;
            let mode = opt_str(args, 2, "half_away_from_zero");
            let r = round_dec(d, decimals, &mode, span)?;
            let s = format_dec(r.mantissa, r.scale);
            let f = s.parse::<f64>().unwrap_or(0.0);
            Ok(Some(Value::F64(f)))
        }
        "math.prng_seed" => {
            let seed = extract_i64(args, 0, "math.prng_seed(seed)", span)?;
            Ok(Some(make_prng_rec(seed as u64)))
        }
        "math.prng_next_u64" => {
            let st = extract_prng_state(args.first(), "math.prng_next_u64", span)?;
            let (val, next_st) = splitmix64_next(st);
            let mut rec = BTreeMap::new();
            rec.insert("value".into(), Value::U64(val));
            rec.insert("state".into(), make_prng_rec(next_st));
            Ok(Some(Value::Record(rec)))
        }
        "math.prng_next_f64" => {
            let st = extract_prng_state(args.first(), "math.prng_next_f64", span)?;
            let (val, next_st) = splitmix64_next(st);
            let unit_f64 = (val >> 11) as f64 * (1.0 / ((1u64 << 53) as f64));
            let mut rec = BTreeMap::new();
            rec.insert("value".into(), Value::F64(unit_f64));
            rec.insert("state".into(), make_prng_rec(next_st));
            Ok(Some(Value::Record(rec)))
        }
        "math.prng_range" => {
            let st = extract_prng_state(args.first(), "math.prng_range", span)?;
            let lo = extract_i64(args, 1, "math.prng_range(prng, lo, hi)", span)?;
            let hi = extract_i64(args, 2, "math.prng_range(prng, lo, hi)", span)?;
            if lo > hi {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "math.prng_range: lo must be <= hi",
                ));
            }
            let (val, next_st) = splitmix64_next(st);
            let span_u = (hi as i128 - lo as i128 + 1) as u64;
            let sampled = if span_u == 0 {
                val as i64
            } else {
                lo + (val % span_u) as i64
            };
            let mut rec = BTreeMap::new();
            rec.insert("value".into(), Value::I64(sampled));
            rec.insert("state".into(), make_prng_rec(next_st));
            Ok(Some(Value::Record(rec)))
        }
        _ => Ok(None),
    }
}

fn floor_div(a: i64, b: i64) -> i64 {
    let d = a / b;
    let r = a % b;
    if (r > 0 && b < 0) || (r < 0 && b > 0) {
        d - 1
    } else {
        d
    }
}

fn gcd_i64(a: i64, b: i64) -> i64 {
    let mut x = (a as i128).abs();
    let mut y = (b as i128).abs();
    while y != 0 {
        let t = y;
        y = x % y;
        x = t;
    }
    x.min(i64::MAX as i128) as i64
}

fn eval_list_extrema(
    args: &[Value],
    span: Span,
    is_min: bool,
) -> Result<Option<Value>, Diagnostic> {
    let list = extract_list(args, 0, "math.min_list/max_list(list)", span)?;
    if list.is_empty() {
        return Ok(Some(Value::Null));
    }
    let mut all_int = true;
    let mut best_f = if is_min {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    let mut best_i = if is_min { i64::MAX } else { i64::MIN };
    for (idx, item) in list.iter().enumerate() {
        match item {
            Value::I64(n) => {
                if (is_min && *n < best_i) || (!is_min && *n > best_i) {
                    best_i = *n;
                }
                let f = *n as f64;
                if (is_min && f < best_f) || (!is_min && f > best_f) {
                    best_f = f;
                }
            }
            other => {
                all_int = false;
                let f = other.as_f64().ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("list extrema: element #{idx} must be a number"),
                    )
                })?;
                if (is_min && f < best_f) || (!is_min && f > best_f) {
                    best_f = f;
                }
            }
        }
    }
    if all_int {
        Ok(Some(Value::I64(best_i)))
    } else {
        Ok(Some(Value::F64(best_f)))
    }
}

fn checked_record(opt: Option<i64>) -> Value {
    let mut rec = BTreeMap::new();
    match opt {
        Some(v) => {
            rec.insert("ok".into(), Value::Bool(true));
            rec.insert("value".into(), Value::I64(v));
        }
        None => {
            rec.insert("ok".into(), Value::Bool(false));
            rec.insert("value".into(), Value::Null);
        }
    }
    Value::Record(rec)
}

fn make_prng_rec(state: u64) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("__prng".into(), Value::Bool(true));
    rec.insert("state".into(), Value::U64(state));
    Value::Record(rec)
}

fn extract_prng_state(
    arg: Option<&Value>,
    ctx: &str,
    span: Span,
) -> Result<u64, Diagnostic> {
    match arg {
        Some(Value::Record(m)) => match m.get("state") {
            Some(Value::U64(u)) => Ok(*u),
            Some(Value::I64(i)) => Ok(*i as u64),
            _ => Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("{ctx}: PRNG record missing 'state'"),
            )),
        },
        Some(Value::U64(u)) => Ok(*u),
        Some(Value::I64(i)) => Ok(*i as u64),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: expected PRNG state record or integer seed"),
        )),
    }
}

fn splitmix64_next(state: u64) -> (u64, u64) {
    let next_state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = next_state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let val = z ^ (z >> 31);
    (val, next_state)
}
