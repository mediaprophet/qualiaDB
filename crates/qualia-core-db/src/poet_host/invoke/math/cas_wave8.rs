//! Wave-8 Host binds: remaining CAS expression constructors.
//!
//! Host-missing `pub fn`s from `specialized_libs::symbolic_algebra` not bound in
//! waves 1–7: `pow`, `neg`, `sqrt`, `exp`, `ln`, `sin`, `cos`, `tan`.

use super::super::args;
use crate::specialized_libs::symbolic_algebra as sa;
use vibe::{Diagnostic, Span, Value};

fn parse_expr(s: &str, span: Span) -> Result<sa::Expr, Diagnostic> {
    sa::parse(s).map_err(|e| args::bad(span, format!("parse error: {e}")))
}

fn unary(
    args_v: &Value,
    span: Span,
    id: &str,
    op: fn(sa::Expr) -> sa::Expr,
) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, format!("{id} needs expr: string")))?;
    let e = parse_expr(expr, span)?;
    Ok(args::record([("expr", Value::String(op(e).to_string()))]))
}

/// `SymbolicAlgebra.pow` — `base ^ exp` with integer exponent.
/// Args: `{ expr, exp }`. Out: `{ expr }`.
pub fn pow(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.pow needs expr: string"))?;
    let exp = args::rec_i64(args_v, "exp")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.pow needs exp: integer"))?;
    if exp < i32::MIN as i64 || exp > i32::MAX as i64 {
        return Err(args::bad(span, "SymbolicAlgebra.pow: exp out of i32 range"));
    }
    let e = parse_expr(expr, span)?;
    Ok(args::record([(
        "expr",
        Value::String(sa::pow(e, exp as i32).to_string()),
    )]))
}

/// `SymbolicAlgebra.neg` — unary minus.
/// Args: `{ expr }`. Out: `{ expr }`.
pub fn neg(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    unary(args_v, span, "SymbolicAlgebra.neg", sa::neg)
}

/// `SymbolicAlgebra.sqrt` — square root node.
/// Args: `{ expr }`. Out: `{ expr }`.
pub fn sqrt(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    unary(args_v, span, "SymbolicAlgebra.sqrt", sa::sqrt)
}

/// `SymbolicAlgebra.exp` — exponential node.
/// Args: `{ expr }`. Out: `{ expr }`.
pub fn exp(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    unary(args_v, span, "SymbolicAlgebra.exp", sa::exp)
}

/// `SymbolicAlgebra.ln` — natural log node.
/// Args: `{ expr }`. Out: `{ expr }`.
pub fn ln(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    unary(args_v, span, "SymbolicAlgebra.ln", sa::ln)
}

/// `SymbolicAlgebra.sin` — sine node.
/// Args: `{ expr }`. Out: `{ expr }`.
pub fn sin(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    unary(args_v, span, "SymbolicAlgebra.sin", sa::sin)
}

/// `SymbolicAlgebra.cos` — cosine node.
/// Args: `{ expr }`. Out: `{ expr }`.
pub fn cos(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    unary(args_v, span, "SymbolicAlgebra.cos", sa::cos)
}

/// `SymbolicAlgebra.tan` — tangent node.
/// Args: `{ expr }`. Out: `{ expr }`.
pub fn tan(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    unary(args_v, span, "SymbolicAlgebra.tan", sa::tan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn expr_arg(s: &str) -> Value {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String(s.into()));
        Value::Record(m)
    }

    #[test]
    fn wave8_pow_x_squared() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x".into()));
        m.insert("exp".into(), Value::I64(2));
        let out = pow(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_str(&out, "expr"), Some("(x^2)"));
    }

    #[test]
    fn wave8_neg_of_const() {
        let out = neg(&expr_arg("3"), span()).unwrap();
        assert_eq!(args::rec_str(&out, "expr"), Some("(-3)"));
    }

    #[test]
    fn wave8_sqrt_of_x() {
        let out = sqrt(&expr_arg("x"), span()).unwrap();
        assert_eq!(args::rec_str(&out, "expr"), Some("sqrt(x)"));
    }

    #[test]
    fn wave8_exp_ln_nodes() {
        assert_eq!(
            args::rec_str(&exp(&expr_arg("x"), span()).unwrap(), "expr"),
            Some("exp(x)")
        );
        assert_eq!(
            args::rec_str(&ln(&expr_arg("x"), span()).unwrap(), "expr"),
            Some("ln(x)")
        );
    }

    #[test]
    fn wave8_trig_nodes() {
        assert_eq!(
            args::rec_str(&sin(&expr_arg("x"), span()).unwrap(), "expr"),
            Some("sin(x)")
        );
        assert_eq!(
            args::rec_str(&cos(&expr_arg("x"), span()).unwrap(), "expr"),
            Some("cos(x)")
        );
        assert_eq!(
            args::rec_str(&tan(&expr_arg("x"), span()).unwrap(), "expr"),
            Some("tan(x)")
        );
    }
}
