//! Bessel J via `solvers::special_functions`.

use super::super::args;
use crate::solvers::special_functions::bessel::bessel_j;
use poet_vibe::{Diagnostic, Span, Value};

pub fn bessel_jn(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_i64(args_v, "n")
        .or_else(|| args::list(args_v).and_then(|xs| xs.first().and_then(args::as_i64)))
        .ok_or_else(|| args::bad(span, "bessel_j needs n"))?;
    let x = args::rec_f64(args_v, "x")
        .or_else(|| args::list(args_v).and_then(|xs| xs.get(1).and_then(args::as_f64)))
        .ok_or_else(|| args::bad(span, "bessel_j needs x"))?;
    Ok(Value::F64(bessel_j(n as i32, x)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j0_at_zero() {
        let args = Value::List(vec![Value::I64(0), Value::F64(0.0)]);
        match bessel_jn(&args, Span { start: 0, end: 0 }).unwrap() {
            Value::F64(y) => assert!((y - 1.0).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
    }
}
