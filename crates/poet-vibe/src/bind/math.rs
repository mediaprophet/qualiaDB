//! Pure math bindings.

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

pub fn call_math(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    let two = |op: fn(f64, f64) -> f64| -> Result<Option<Value>, Diagnostic> {
        let a = args
            .first()
            .and_then(Value::as_f64)
            .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math expects numbers"))?;
        let b = args
            .get(1)
            .and_then(Value::as_f64)
            .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math expects numbers"))?;
        Ok(Some(Value::F64(op(a, b))))
    };
    match path {
        "math.abs" => {
            let a = args
                .first()
                .and_then(Value::as_f64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math.abs expects a number"))?;
            Ok(Some(Value::F64(a.abs())))
        }
        "math.min" => two(f64::min),
        "math.max" => two(f64::max),
        "math.clamp" => {
            let v = args
                .first()
                .and_then(Value::as_f64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math.clamp expects numbers"))?;
            let lo = args.get(1).and_then(Value::as_f64).unwrap_or(0.0);
            let hi = args.get(2).and_then(Value::as_f64).unwrap_or(0.0);
            Ok(Some(Value::F64(v.clamp(lo, hi))))
        }
        _ => Ok(None),
    }
}
