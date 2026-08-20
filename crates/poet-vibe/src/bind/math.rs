//! Pure math bindings.

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

pub fn call_math(path: &str, args: &[Value], span: Span) -> Result<Option<Value>, Diagnostic> {
    match path {
        "math.abs" => {
            let first = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.abs expects a number")
            })?;
            match first {
                Value::I64(n) => Ok(Some(Value::I64(n.checked_abs().unwrap_or(i64::MAX)))),
                Value::U64(n) => Ok(Some(Value::U64(*n))),
                _ => {
                    let a = first.as_f64().ok_or_else(|| {
                        Diagnostic::new(DiagCode::E100, span, "math.abs expects a number")
                    })?;
                    Ok(Some(Value::F64(a.abs())))
                }
            }
        }
        "math.min" => {
            let a = args
                .first()
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math.min expects numbers"))?;
            let b = args
                .get(1)
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math.min expects numbers"))?;
            if let (Value::I64(x), Value::I64(y)) = (a, b) {
                Ok(Some(Value::I64(*x.min(y))))
            } else if let (Value::U64(x), Value::U64(y)) = (a, b) {
                Ok(Some(Value::U64(*x.min(y))))
            } else {
                let x = a.as_f64().ok_or_else(|| {
                    Diagnostic::new(DiagCode::E100, span, "math.min expects numbers")
                })?;
                let y = b.as_f64().ok_or_else(|| {
                    Diagnostic::new(DiagCode::E100, span, "math.min expects numbers")
                })?;
                Ok(Some(Value::F64(f64::min(x, y))))
            }
        }
        "math.max" => {
            let a = args
                .first()
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math.max expects numbers"))?;
            let b = args
                .get(1)
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math.max expects numbers"))?;
            if let (Value::I64(x), Value::I64(y)) = (a, b) {
                Ok(Some(Value::I64(*x.max(y))))
            } else if let (Value::U64(x), Value::U64(y)) = (a, b) {
                Ok(Some(Value::U64(*x.max(y))))
            } else {
                let x = a.as_f64().ok_or_else(|| {
                    Diagnostic::new(DiagCode::E100, span, "math.max expects numbers")
                })?;
                let y = b.as_f64().ok_or_else(|| {
                    Diagnostic::new(DiagCode::E100, span, "math.max expects numbers")
                })?;
                Ok(Some(Value::F64(f64::max(x, y))))
            }
        }
        "math.floor" => {
            let first = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.floor expects a number")
            })?;
            match first {
                Value::I64(n) => Ok(Some(Value::I64(*n))),
                Value::U64(n) => Ok(Some(Value::U64(*n))),
                _ => {
                    let a = first.as_f64().ok_or_else(|| {
                        Diagnostic::new(DiagCode::E100, span, "math.floor expects a number")
                    })?;
                    Ok(Some(Value::F64(a.floor())))
                }
            }
        }
        "math.ceil" => {
            let first = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.ceil expects a number")
            })?;
            match first {
                Value::I64(n) => Ok(Some(Value::I64(*n))),
                Value::U64(n) => Ok(Some(Value::U64(*n))),
                _ => {
                    let a = first.as_f64().ok_or_else(|| {
                        Diagnostic::new(DiagCode::E100, span, "math.ceil expects a number")
                    })?;
                    Ok(Some(Value::F64(a.ceil())))
                }
            }
        }
        "math.round" => {
            let first = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.round expects a number")
            })?;
            match first {
                Value::I64(n) => Ok(Some(Value::I64(*n))),
                Value::U64(n) => Ok(Some(Value::U64(*n))),
                _ => {
                    let a = first.as_f64().ok_or_else(|| {
                        Diagnostic::new(DiagCode::E100, span, "math.round expects a number")
                    })?;
                    Ok(Some(Value::F64(a.round())))
                }
            }
        }
        "math.clamp" => {
            let v = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.clamp expects numbers")
            })?;
            let lo = args.get(1).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.clamp expects numbers")
            })?;
            let hi = args.get(2).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.clamp expects numbers")
            })?;
            if let (Value::I64(v_val), Value::I64(lo_val), Value::I64(hi_val)) = (v, lo, hi) {
                Ok(Some(Value::I64(*v_val.clamp(lo_val, hi_val))))
            } else if let (Value::U64(v_val), Value::U64(lo_val), Value::U64(hi_val)) = (v, lo, hi)
            {
                Ok(Some(Value::U64(*v_val.clamp(lo_val, hi_val))))
            } else {
                let v_f = v.as_f64().ok_or_else(|| {
                    Diagnostic::new(DiagCode::E100, span, "math.clamp expects numbers")
                })?;
                let lo_f = lo.as_f64().unwrap_or(0.0);
                let hi_f = hi.as_f64().unwrap_or(0.0);
                Ok(Some(Value::F64(v_f.clamp(lo_f, hi_f))))
            }
        }
        "math.sqrt" => {
            let a = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.sqrt expects a number")
            })?;
            Ok(Some(Value::F64(a.sqrt())))
        }
        "math.sin" => {
            let a = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.sin expects a number")
            })?;
            Ok(Some(Value::F64(a.sin())))
        }
        "math.cos" => {
            let a = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.cos expects a number")
            })?;
            Ok(Some(Value::F64(a.cos())))
        }
        "math.log" => {
            let a = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.log expects a number")
            })?;
            Ok(Some(Value::F64(a.ln())))
        }
        "math.exp" => {
            let a = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.exp expects a number")
            })?;
            Ok(Some(Value::F64(a.exp())))
        }
        "math.pow" => {
            let base = args
                .first()
                .and_then(Value::as_f64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math.pow expects numbers"))?;
            let exp = args
                .get(1)
                .and_then(Value::as_f64)
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "math.pow expects numbers"))?;
            Ok(Some(Value::F64(base.powf(exp))))
        }
        "math.log10" => {
            let a = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.log10 expects a number")
            })?;
            Ok(Some(Value::F64(a.log10())))
        }
        "math.tan" => {
            let a = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.tan expects a number")
            })?;
            Ok(Some(Value::F64(a.tan())))
        }
        "math.atan" => {
            let a = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.atan expects a number")
            })?;
            Ok(Some(Value::F64(a.atan())))
        }
        "math.atan2" => {
            let y = args.first().and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.atan2 expects numbers")
            })?;
            let x = args.get(1).and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "math.atan2 expects numbers")
            })?;
            Ok(Some(Value::F64(y.atan2(x))))
        }
        _ => Ok(None),
    }
}
