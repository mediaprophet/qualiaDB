//! Polynomial eval through the CAS `Expr` tree.

use super::super::args;
use crate::specialized_libs::symbolic_algebra as sa;
use crate::specialized_libs::symbolic_algebra::{add, c, mul, pow, var, Expr};
use std::collections::HashMap;
use vibe::{Diagnostic, Span, Value};

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

/// Symbolic derivative of a parsed expression with respect to a variable, simplified.
pub fn differentiate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.differentiate needs expr: string"))?;
    let var = args::rec_str(args_v, "var")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.differentiate needs var: string"))?;
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let d = sa::simplify(&sa::differentiate(&e, var));
    Ok(args::record([("derivative", Value::String(d.to_string()))]))
}

/// Simplify a parsed expression (constant folding + identity elimination).
pub fn simplify(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.simplify needs expr: string"))?;
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let s = sa::simplify(&e);
    Ok(args::record([("simplified", Value::String(s.to_string()))]))
}

/// Expand a parsed expression (distribute products over sums, expand small powers).
pub fn expand(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.expand needs expr: string"))?;
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let x = sa::expand(&e);
    Ok(args::record([("expanded", Value::String(x.to_string()))]))
}

/// Factor a real quadratic `a·x² + b·x + c` into `a·(x − r₁)·(x − r₂)` when it has
/// real roots. Errors when `a == 0` (not quadratic) or the discriminant is negative.
pub fn factor(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a").ok_or_else(|| args::bad(span, "factor needs a: number"))?;
    let b = args::rec_f64(args_v, "b").ok_or_else(|| args::bad(span, "factor needs b: number"))?;
    let c = args::rec_f64(args_v, "c").ok_or_else(|| args::bad(span, "factor needs c: number"))?;
    let var = args::rec_str(args_v, "var").unwrap_or("x");
    if a == 0.0 {
        return Err(args::bad(span, "factor: a must not be 0 (not a quadratic)"));
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return Err(args::bad(
            span,
            "factor: discriminant is negative (no real roots)",
        ));
    }
    let factored = sa::factor_quadratic(a, b, c, var)
        .ok_or_else(|| args::bad(span, "factor: no real factorisation"))?;
    Ok(args::record([(
        "factored",
        Value::String(factored.to_string()),
    )]))
}

/// Symbolic roots of `a·x² + b·x + c = 0`. Each root is returned as a record with the
/// symbolic expression string and its numeric value (or `null` when non-finite, e.g.
/// a negative discriminant producing an imaginary root).
pub fn solve_quadratic(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "solve_quadratic needs a: number"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "solve_quadratic needs b: number"))?;
    let c = args::rec_f64(args_v, "c")
        .ok_or_else(|| args::bad(span, "solve_quadratic needs c: number"))?;
    let roots = sa::solve_quadratic_symbolic(a, b, c);
    let env: HashMap<String, f64> = HashMap::new();
    let list: Vec<Value> = roots
        .iter()
        .map(|r| {
            let value = match Expr::eval(r, &env) {
                Some(v) if v.is_finite() => Value::F64(v),
                _ => Value::Null,
            };
            args::record([("expr", Value::String(r.to_string())), ("value", value)])
        })
        .collect();
    Ok(Value::List(list))
}

/// `VectorCalculus.gradient` — symbolic gradient of an expression.
/// Args: { expr: string, vars: [string] }
pub fn gradient(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "VectorCalculus.gradient needs expr: string"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "VectorCalculus.gradient needs vars: [string]"))?;
    let expr = sa::parse(expr_str).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    let grad = crate::solvers::vector_calculus::gradient(&expr, &var_refs);
    let grad_strs: Vec<Value> = grad.iter().map(|e| Value::String(e.to_string())).collect();
    Ok(args::record([("gradient", Value::List(grad_strs))]))
}

/// `VectorCalculus.divergence` — symbolic divergence of a vector field.
/// Args: { field: [string], vars: [string] }
pub fn divergence(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let field_strs = args::rec_str_list(args_v, "field")
        .ok_or_else(|| args::bad(span, "VectorCalculus.divergence needs field: [string]"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "VectorCalculus.divergence needs vars: [string]"))?;
    let mut field = Vec::new();
    for s in &field_strs {
        field.push(sa::parse(s).map_err(|e| args::bad(span, format!("parse error: {e}")))?);
    }
    let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    match crate::solvers::vector_calculus::divergence(&field, &var_refs) {
        Ok(div) => Ok(args::record([(
            "divergence",
            Value::String(div.to_string()),
        )])),
        Err(e) => Err(args::bad(span, format!("divergence: {e}"))),
    }
}

/// `VectorCalculus.curl` — symbolic curl of a 3-component vector field.
/// Args: { field: [string], vars: [string] }
pub fn curl(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let field_strs = args::rec_str_list(args_v, "field")
        .ok_or_else(|| args::bad(span, "VectorCalculus.curl needs field: [string]"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "VectorCalculus.curl needs vars: [string]"))?;
    if field_strs.len() != 3 {
        return Err(args::bad(
            span,
            "VectorCalculus.curl: field must have 3 components",
        ));
    }
    let mut field = Vec::new();
    for s in &field_strs {
        field.push(sa::parse(s).map_err(|e| args::bad(span, format!("parse error: {e}")))?);
    }
    let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    match crate::solvers::vector_calculus::curl(&field, &var_refs) {
        Ok(curl_vec) => {
            let curl_strs: Vec<Value> = curl_vec
                .iter()
                .map(|e| Value::String(e.to_string()))
                .collect();
            Ok(args::record([("curl", Value::List(curl_strs))]))
        }
        Err(e) => Err(args::bad(span, format!("curl: {e}"))),
    }
}

/// `VectorCalculus.laplacian` — symbolic Laplacian of an expression.
/// Args: { expr: string, vars: [string] }
pub fn laplacian(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "VectorCalculus.laplacian needs expr: string"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "VectorCalculus.laplacian needs vars: [string]"))?;
    let expr = sa::parse(expr_str).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    match crate::solvers::vector_calculus::laplacian(&expr, &var_refs) {
        Ok(lap) => Ok(args::record([(
            "laplacian",
            Value::String(lap.to_string()),
        )])),
        Err(e) => Err(args::bad(span, format!("laplacian: {e}"))),
    }
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

    #[test]
    fn simplify_x_plus_zero_is_x() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x + 0".into()));
        let v = simplify(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            _ => panic!("expected record"),
        };
        assert_eq!(rec.get("simplified"), Some(&Value::String("x".into())));
    }
}
