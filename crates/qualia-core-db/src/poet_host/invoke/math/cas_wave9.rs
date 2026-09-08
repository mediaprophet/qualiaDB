//! Wave-9 Host binds: remaining CAS expression constructors.
//!
//! Host-missing `pub fn`s from `specialized_libs::symbolic_algebra` not bound in
//! waves 1–8: `c`, `var`, `add`, `sub`, `mul`, `div`
//! (wave 8 covered `pow`/`neg`/`sqrt`/`exp`/`ln`/`sin`/`cos`/`tan`).

use super::super::args;
use crate::specialized_libs::symbolic_algebra as sa;
use vibe::{Diagnostic, Span, Value};

fn parse_expr(s: &str, span: Span) -> Result<sa::Expr, Diagnostic> {
    sa::parse(s).map_err(|e| args::bad(span, format!("parse error: {e}")))
}

fn binary(
    args_v: &Value,
    span: Span,
    id: &str,
    op: fn(sa::Expr, sa::Expr) -> sa::Expr,
) -> Result<Value, Diagnostic> {
    let a = args::rec_str(args_v, "a")
        .ok_or_else(|| args::bad(span, format!("{id} needs a: string")))?;
    let b = args::rec_str(args_v, "b")
        .ok_or_else(|| args::bad(span, format!("{id} needs b: string")))?;
    let ea = parse_expr(a, span)?;
    let eb = parse_expr(b, span)?;
    Ok(args::record([("expr", Value::String(op(ea, eb).to_string()))]))
}

/// `SymbolicAlgebra.c` — constant leaf. Args: `{ value }`. Out: `{ expr }`.
pub fn c(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let value = args::rec_f64(args_v, "value")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.c needs value: f64"))?;
    Ok(args::record([(
        "expr",
        Value::String(sa::c(value).to_string()),
    )]))
}

/// `SymbolicAlgebra.var` — named variable leaf. Args: `{ name }`. Out: `{ expr }`.
pub fn var(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "name")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.var needs name: string"))?;
    if name.is_empty() {
        return Err(args::bad(span, "SymbolicAlgebra.var: name must be non-empty"));
    }
    Ok(args::record([(
        "expr",
        Value::String(sa::var(name).to_string()),
    )]))
}

/// `SymbolicAlgebra.add` — `a + b` node. Args: `{ a, b }` (expr strings). Out: `{ expr }`.
pub fn add(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    binary(args_v, span, "SymbolicAlgebra.add", sa::add)
}

/// `SymbolicAlgebra.sub` — `a − b` node. Args: `{ a, b }` (expr strings). Out: `{ expr }`.
pub fn sub(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    binary(args_v, span, "SymbolicAlgebra.sub", sa::sub)
}

/// `SymbolicAlgebra.mul` — `a * b` node. Args: `{ a, b }` (expr strings). Out: `{ expr }`.
pub fn mul(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    binary(args_v, span, "SymbolicAlgebra.mul", sa::mul)
}

/// `SymbolicAlgebra.div` — `a / b` node. Args: `{ a, b }` (expr strings). Out: `{ expr }`.
pub fn div(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    binary(args_v, span, "SymbolicAlgebra.div", sa::div)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn pair(a: &str, b: &str) -> Value {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::String(a.into()));
        m.insert("b".into(), Value::String(b.into()));
        Value::Record(m)
    }

    #[test]
    fn wave9_c_const_leaf() {
        let mut m = BTreeMap::new();
        m.insert("value".into(), Value::F64(3.0));
        let out = c(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_str(&out, "expr"), Some("3"));
    }

    #[test]
    fn wave9_var_leaf() {
        let mut m = BTreeMap::new();
        m.insert("name".into(), Value::String("x".into()));
        let out = var(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_str(&out, "expr"), Some("x"));
    }

    #[test]
    fn wave9_add_sub_nodes() {
        assert_eq!(
            args::rec_str(&add(&pair("x", "1"), span()).unwrap(), "expr"),
            Some("(x + 1)")
        );
        assert_eq!(
            args::rec_str(&sub(&pair("x", "1"), span()).unwrap(), "expr"),
            Some("(x - 1)")
        );
    }

    #[test]
    fn wave9_mul_div_nodes() {
        assert_eq!(
            args::rec_str(&mul(&pair("x", "2"), span()).unwrap(), "expr"),
            Some("(x * 2)")
        );
        assert_eq!(
            args::rec_str(&div(&pair("x", "2"), span()).unwrap(), "expr"),
            Some("(x / 2)")
        );
    }
}
