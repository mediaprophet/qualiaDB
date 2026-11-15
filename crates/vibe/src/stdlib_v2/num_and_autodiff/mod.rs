//! VibeScript v0.02 Standard Library — Strict Numeric Parsers, Exact Decimal Math,
//! Euclidean/Checked Arithmetic, Seeded SplitMix64 PRNG & Forward-Mode Autodiff.

pub mod autodiff;
pub mod decimal;
pub mod math_ext;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// All stdlib v0.02 bindings provided by `num_and_autodiff`.
pub const NUM_AND_AUTODIFF_BINDINGS: &[&str] = &[
    // Strict non-coercing numeric parsers (§9)
    "num.parse_i64",
    "num.parse_u64",
    "num.parse_f64",
    // Exact base-10 Decimal math (§2, §11)
    "decimal.from_str",
    "decimal.from_parts",
    "decimal.add",
    "decimal.sub",
    "decimal.mul",
    "decimal.div",
    "decimal.round",
    "decimal.to_fixed",
    "decimal.eq",
    "decimal.cmp",
    // Euclidean, checked, saturating & compensated math (§11, §22)
    "math.idiv",
    "math.imod",
    "math.divmod",
    "math.gcd",
    "math.lcm",
    "math.lerp",
    "math.sum_precise",
    "math.min_list",
    "math.max_list",
    "math.add_checked",
    "math.sub_checked",
    "math.mul_checked",
    "math.add_saturating",
    "math.sub_saturating",
    "math.mul_saturating",
    "math.round_mode",
    // Deterministic seeded SplitMix64 PRNG (§6, §11)
    "math.prng_seed",
    "math.prng_next_u64",
    "math.prng_next_f64",
    "math.prng_range",
    // First-class forward-mode Automatic Differentiation (§16)
    "autodiff.dual",
    "autodiff.dual_add",
    "autodiff.dual_sub",
    "autodiff.dual_mul",
    "autodiff.dual_div",
    "autodiff.dual_sin",
    "autodiff.dual_cos",
    "autodiff.dual_exp",
    "autodiff.dual_pow",
    "autodiff.value_and_grad",
    "autodiff.grad",
    "autodiff.jacobian",
];

/// Route a `num.*`, `decimal.*`, `math.*`, or `autodiff.*` call.
pub fn call_num_and_autodiff(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if let Some(v) = decimal::call_decimal(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = math_ext::call_math_ext(path, args, span)? {
        return Ok(Some(v));
    }
    autodiff::call_autodiff(path, args, span)
}

pub(crate) fn extract_str(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<String, Diagnostic> {
    match args.get(idx) {
        Some(Value::String(s)) | Some(Value::Iri(s)) => Ok(s.clone()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a String"),
        )),
    }
}

pub(crate) fn extract_i64(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<i64, Diagnostic> {
    args.get(idx).and_then(Value::as_i64).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be an I64"),
        )
    })
}

pub(crate) fn extract_f64(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<f64, Diagnostic> {
    args.get(idx).and_then(Value::as_f64).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a number"),
        )
    })
}

pub(crate) fn extract_list(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<Vec<Value>, Diagnostic> {
    match args.get(idx) {
        Some(Value::List(items)) => Ok(items.clone()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a List"),
        )),
    }
}

pub(crate) fn opt_i64(args: &[Value], idx: usize, default: i64) -> i64 {
    args.get(idx).and_then(Value::as_i64).unwrap_or(default)
}

pub(crate) fn opt_str(args: &[Value], idx: usize, default: &str) -> String {
    match args.get(idx) {
        Some(Value::String(s)) | Some(Value::Iri(s)) => s.clone(),
        _ => default.to_string(),
    }
}
