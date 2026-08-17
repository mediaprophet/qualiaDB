//! Polynomial eval through the CAS `Expr` tree.

use super::super::args;
use crate::specialized_libs::symbolic_algebra::{add, c, mul, pow, var, Expr};
use poet_vibe::{Diagnostic, Span, Value};
use std::collections::HashMap;

pub fn eval_poly(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let coeffs = args::rec(args_v, "coeffs")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.eval needs coeffs: [c0, c1, ...]"))?;
    if coeffs.is_empty() {
        return Err(args::bad(span, "coeffs must not be empty"));
    }
    let x = args::rec_f64(args_v, "x").ok_or_else(|| args::bad(span, "eval needs x"))?;
    let mut expr = c(coeffs[0]);
    for (i, coeff) in coeffs.iter().enumerate().skip(1) {
        if *coeff == 0.0 {
            continue;
        }
        let term = mul(c(*coeff), pow(var("x"), i as i32));
        expr = add(expr, term);
    }
    let mut env = HashMap::new();
    env.insert("x".into(), x);
    let y = Expr::eval(&expr, &env).ok_or_else(|| args::bad(span, "expression was non-finite"))?;
    Ok(Value::F64(y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn eval_x_squared_plus_one() {
        let mut m = BTreeMap::new();
        m.insert("coeffs".into(), args::f64_list_value(vec![1.0, 0.0, 1.0]));
        m.insert("x".into(), Value::F64(3.0));
        assert_eq!(
            eval_poly(&Value::Record(m), Span { start: 0, end: 0 }).unwrap(),
            Value::F64(10.0)
        );
    }
}
