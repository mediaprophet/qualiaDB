//! First-Class Forward-Mode Automatic Differentiation (`autodiff.*`) for VibeScript v0.02.
//!
//! Provides exact Dual numbers (`x + x' ε` with `ε² = 0`), elementary transcendental
//! chain-rule operators, univariate `value_and_grad` / `grad`, and multivariate
//! exact `jacobian` evaluation (§16 Julia/Mojo).

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::{extract_f64, extract_list};

/// Forward-mode dual number `val + deriv * ε`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Dual {
    val: f64,
    deriv: f64,
}

impl Dual {
    const fn new(val: f64, deriv: f64) -> Self {
        Self { val, deriv }
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(self.val + rhs.val, self.deriv + rhs.deriv)
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.val - rhs.val, self.deriv - rhs.deriv)
    }

    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.val * rhs.val,
            self.val * rhs.deriv + self.deriv * rhs.val,
        )
    }

    fn div(self, rhs: Self, span: Span) -> Result<Self, Diagnostic> {
        if rhs.val.abs() < 1e-15 {
            return Err(Diagnostic::new(
                DiagCode::E600,
                span,
                "autodiff.dual_div: division by zero",
            ));
        }
        Ok(Self::new(
            self.val / rhs.val,
            (self.deriv * rhs.val - self.val * rhs.deriv) / (rhs.val * rhs.val),
        ))
    }

    fn sin(self) -> Self {
        Self::new(self.val.sin(), self.val.cos() * self.deriv)
    }

    fn cos(self) -> Self {
        Self::new(self.val.cos(), -self.val.sin() * self.deriv)
    }

    fn exp(self) -> Self {
        let e = self.val.exp();
        Self::new(e, e * self.deriv)
    }

    fn pow(self, n: f64) -> Self {
        if n == 0.0 {
            Self::new(1.0, 0.0)
        } else if (n - 1.0).abs() < 1e-15 {
            self
        } else {
            Self::new(self.val.powf(n), n * self.val.powf(n - 1.0) * self.deriv)
        }
    }
}

/// Dispatch an `autodiff.*` call.
pub fn call_autodiff(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "autodiff.dual" => {
            let val = extract_f64(args, 0, "autodiff.dual(val, deriv)", span)?;
            let deriv = args.get(1).and_then(Value::as_f64).unwrap_or(1.0);
            Ok(Some(dual_to_record(Dual::new(val, deriv))))
        }
        "autodiff.dual_add" => {
            let a = coerce_dual(args.first(), "autodiff.dual_add", span)?;
            let b = coerce_dual(args.get(1), "autodiff.dual_add", span)?;
            Ok(Some(dual_to_record(a.add(b))))
        }
        "autodiff.dual_sub" => {
            let a = coerce_dual(args.first(), "autodiff.dual_sub", span)?;
            let b = coerce_dual(args.get(1), "autodiff.dual_sub", span)?;
            Ok(Some(dual_to_record(a.sub(b))))
        }
        "autodiff.dual_mul" => {
            let a = coerce_dual(args.first(), "autodiff.dual_mul", span)?;
            let b = coerce_dual(args.get(1), "autodiff.dual_mul", span)?;
            Ok(Some(dual_to_record(a.mul(b))))
        }
        "autodiff.dual_div" => {
            let a = coerce_dual(args.first(), "autodiff.dual_div", span)?;
            let b = coerce_dual(args.get(1), "autodiff.dual_div", span)?;
            Ok(Some(dual_to_record(a.div(b, span)?)))
        }
        "autodiff.dual_sin" => {
            let a = coerce_dual(args.first(), "autodiff.dual_sin", span)?;
            Ok(Some(dual_to_record(a.sin())))
        }
        "autodiff.dual_cos" => {
            let a = coerce_dual(args.first(), "autodiff.dual_cos", span)?;
            Ok(Some(dual_to_record(a.cos())))
        }
        "autodiff.dual_exp" => {
            let a = coerce_dual(args.first(), "autodiff.dual_exp", span)?;
            Ok(Some(dual_to_record(a.exp())))
        }
        "autodiff.dual_pow" => {
            let a = coerce_dual(args.first(), "autodiff.dual_pow", span)?;
            let n = extract_f64(args, 1, "autodiff.dual_pow(a, n)", span)?;
            Ok(Some(dual_to_record(a.pow(n))))
        }
        "autodiff.value_and_grad" => {
            let spec = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "autodiff.value_and_grad: missing spec")
            })?;
            let x = extract_f64(args, 1, "autodiff.value_and_grad(spec, x)", span)?;
            let d = eval_univariate_spec(spec, Dual::new(x, 1.0), span)?;
            let mut rec = BTreeMap::new();
            rec.insert("value".into(), Value::F64(d.val));
            rec.insert("grad".into(), Value::F64(d.deriv));
            Ok(Some(Value::Record(rec)))
        }
        "autodiff.grad" => {
            let spec = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "autodiff.grad: missing spec")
            })?;
            let x = extract_f64(args, 1, "autodiff.grad(spec, x)", span)?;
            let d = eval_univariate_spec(spec, Dual::new(x, 1.0), span)?;
            Ok(Some(Value::F64(d.deriv)))
        }
        "autodiff.jacobian" => eval_jacobian(args, span),
        _ => Ok(None),
    }
}

fn eval_univariate_spec(spec: &Value, x: Dual, span: Span) -> Result<Dual, Diagnostic> {
    match spec {
        Value::List(coeffs) => eval_poly_dual(coeffs, x, span),
        Value::String(op) => apply_named_op(op, x, span),
        Value::Record(map) => {
            let inner = if let Some(Value::List(coeffs)) = map.get("coeffs") {
                eval_poly_dual(coeffs, x, span)?
            } else {
                x
            };
            if let Some(Value::String(op)) = map.get("op") {
                apply_named_op(op, inner, span)
            } else {
                Ok(inner)
            }
        }
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "autodiff: spec must be a polynomial coefficient List, op String, or Record",
        )),
    }
}

fn eval_poly_dual(coeffs: &[Value], x: Dual, span: Span) -> Result<Dual, Diagnostic> {
    let mut acc = Dual::new(0.0, 0.0);
    let mut x_pow = Dual::new(1.0, 0.0);
    for (idx, c_val) in coeffs.iter().enumerate() {
        let c = c_val.as_f64().ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                format!("autodiff polynomial: coefficient #{idx} must be a number"),
            )
        })?;
        acc = acc.add(Dual::new(c, 0.0).mul(x_pow));
        x_pow = x_pow.mul(x);
    }
    Ok(acc)
}

fn apply_named_op(op: &str, x: Dual, span: Span) -> Result<Dual, Diagnostic> {
    match op {
        "identity" | "linear" => Ok(x),
        "sin" => Ok(x.sin()),
        "cos" => Ok(x.cos()),
        "exp" => Ok(x.exp()),
        "square" | "x^2" => Ok(x.mul(x)),
        "cube" | "x^3" => Ok(x.pow(3.0)),
        "sigmoid" => {
            let neg_x = Dual::new(-x.val, -x.deriv);
            let denom = Dual::new(1.0, 0.0).add(neg_x.exp());
            Dual::new(1.0, 0.0).div(denom, span)
        }
        "softplus" => {
            let e = x.val.exp();
            let val = (1.0 + e).ln();
            let sig = e / (1.0 + e);
            Ok(Dual::new(val, sig * x.deriv))
        }
        "tanh" => {
            let t = x.val.tanh();
            Ok(Dual::new(t, (1.0 - t * t) * x.deriv))
        }
        "rosenbrock_1d" => {
            // (1 - x)^2 + 100 * (x^2 - x)^2
            let one_minus_x = Dual::new(1.0, 0.0).sub(x);
            let term1 = one_minus_x.mul(one_minus_x);
            let x2_minus_x = x.mul(x).sub(x);
            let term2 = Dual::new(100.0, 0.0).mul(x2_minus_x.mul(x2_minus_x));
            Ok(term1.add(term2))
        }
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("autodiff: unsupported function op '{op}'"),
        )),
    }
}

fn eval_jacobian(args: &[Value], span: Span) -> Result<Option<Value>, Diagnostic> {
    let rows_spec = extract_list(args, 0, "autodiff.jacobian(rows_spec, x_vec)", span)?;
    let x_list = extract_list(args, 1, "autodiff.jacobian(rows_spec, x_vec)", span)?;
    let mut x_vec = Vec::with_capacity(x_list.len());
    for (j, v) in x_list.iter().enumerate() {
        let f = v.as_f64().ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                format!("autodiff.jacobian: x_vec[{j}] must be a number"),
            )
        })?;
        x_vec.push(f);
    }

    let rows = rows_spec.len();
    let cols = x_vec.len();
    let mut values_out = Vec::with_capacity(rows);
    let mut jac_out = Vec::with_capacity(rows);

    for row_spec in &rows_spec {
        let mut row_grad = Vec::with_capacity(cols);
        let mut row_val = 0.0;
        if cols == 0 {
            let d = eval_row_spec(row_spec, &[], span)?;
            row_val = d.val;
        } else {
            for j in 0..cols {
                let dual_x: Vec<Dual> = x_vec
                    .iter()
                    .enumerate()
                    .map(|(k, &xv)| Dual::new(xv, if k == j { 1.0 } else { 0.0 }))
                    .collect();
                let d = eval_row_spec(row_spec, &dual_x, span)?;
                if j == 0 {
                    row_val = d.val;
                }
                row_grad.push(Value::F64(d.deriv));
            }
        }
        values_out.push(Value::F64(row_val));
        jac_out.push(Value::List(row_grad));
    }

    let mut rec = BTreeMap::new();
    rec.insert("values".into(), Value::List(values_out));
    rec.insert("jacobian".into(), Value::List(jac_out));
    rec.insert("rows".into(), Value::I64(rows as i64));
    rec.insert("cols".into(), Value::I64(cols as i64));
    Ok(Some(Value::Record(rec)))
}

fn eval_row_spec(spec: &Value, x: &[Dual], span: Span) -> Result<Dual, Diagnostic> {
    match spec {
        Value::List(weights) => {
            let mut acc = Dual::new(0.0, 0.0);
            for (j, w_val) in weights.iter().enumerate() {
                if let Some(&xj) = x.get(j) {
                    let w = w_val.as_f64().unwrap_or(0.0);
                    acc = acc.add(Dual::new(w, 0.0).mul(xj));
                }
            }
            Ok(acc)
        }
        Value::Record(map) => {
            let bias = map.get("bias").and_then(Value::as_f64).unwrap_or(0.0);
            let mut acc = Dual::new(bias, 0.0);
            let powers = map.get("powers").and_then(|v| match v {
                Value::List(xs) => Some(xs),
                _ => None,
            });
            if let Some(Value::List(weights)) = map.get("weights") {
                for (j, w_val) in weights.iter().enumerate() {
                    if let Some(&xj) = x.get(j) {
                        let w = w_val.as_f64().unwrap_or(0.0);
                        let p = powers
                            .and_then(|ps| ps.get(j))
                            .and_then(Value::as_f64)
                            .unwrap_or(1.0);
                        acc = acc.add(Dual::new(w, 0.0).mul(xj.pow(p)));
                    }
                }
            }
            if let Some(Value::String(op)) = map.get("op") {
                apply_named_op(op, acc, span)
            } else {
                Ok(acc)
            }
        }
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "autodiff.jacobian: each row spec must be a List of weights or a Record",
        )),
    }
}

fn coerce_dual(val: Option<&Value>, ctx: &str, span: Span) -> Result<Dual, Diagnostic> {
    match val {
        Some(Value::Record(m)) => {
            let v = m.get("val").and_then(Value::as_f64).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, format!("{ctx}: Dual missing 'val'"))
            })?;
            let d = m.get("deriv").and_then(Value::as_f64).unwrap_or(0.0);
            Ok(Dual::new(v, d))
        }
        Some(other) => {
            let v = other.as_f64().ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("{ctx}: expected Dual record or number"),
                )
            })?;
            Ok(Dual::new(v, 0.0))
        }
        None => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: missing Dual argument"),
        )),
    }
}

fn dual_to_record(d: Dual) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("__dual".into(), Value::Bool(true));
    rec.insert("val".into(), Value::F64(d.val));
    rec.insert("deriv".into(), Value::F64(d.deriv));
    Value::Record(rec)
}
