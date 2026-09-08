//! Wave-18 Host binds: Calculus remainder (mechanics / quadrature / JVP·VJP).
//!
//! Pure CPU paths from `solvers::calculus` — no forge / `caps()` / CUDA.

use super::super::args;
use crate::solvers::calculus::analysis::{LinearMap, Vector};
use crate::solvers::calculus::differential::{jvp, vjp};
use crate::solvers::calculus::mechanics::{
    canonical_poisson_bracket, stormer_verlet_step, PhaseState,
};
use crate::solvers::calculus::quadrature::adaptive_gauss_kronrod_15;
use crate::specialized_libs::symbolic_algebra::{self as sa, Expr};
use std::collections::HashMap;
use vibe::{Diagnostic, Span, Value};

fn parse_expr(s: &str, span: Span) -> Result<Expr, Diagnostic> {
    sa::parse(s).map_err(|e| args::bad(span, format!("failed to parse expression '{s}': {e}")))
}

fn vec2(vals: &[f64], span: Span, what: &str) -> Result<Vector<2>, Diagnostic> {
    if vals.len() != 2 {
        return Err(args::bad(span, format!("{what}: need length-2 [f64]")));
    }
    Ok(Vector::new([vals[0], vals[1]]))
}

fn parse_matrix_2x2(a_val: &Value, span: Span, what: &str) -> Result<LinearMap<2, 2>, Diagnostic> {
    let rows = args::list(a_val).ok_or_else(|| args::bad(span, format!("{what}: a must be [[f64;2];2]")))?;
    if rows.len() != 2 {
        return Err(args::bad(span, format!("{what}: need 2×2 matrix")));
    }
    let mut coef = [[0.0; 2]; 2];
    for (i, row) in rows.iter().enumerate() {
        let vals = args::f64s(row).ok_or_else(|| args::bad(span, format!("{what}: row must be [f64]")))?;
        if vals.len() != 2 {
            return Err(args::bad(span, format!("{what}: each row length 2")));
        }
        coef[i][0] = vals[0];
        coef[i][1] = vals[1];
    }
    Ok(LinearMap::new(coef))
}

/// `Calculus.canonical_poisson_bracket` — {df_dq,df_dp,dg_dq,dg_dp: [f64;2]} → f64.
pub fn canonical_poisson_bracket_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let df_dq = args::rec_f64_list(args_v, "df_dq")
        .ok_or_else(|| args::bad(span, "canonical_poisson_bracket needs df_dq"))?;
    let df_dp = args::rec_f64_list(args_v, "df_dp")
        .ok_or_else(|| args::bad(span, "canonical_poisson_bracket needs df_dp"))?;
    let dg_dq = args::rec_f64_list(args_v, "dg_dq")
        .ok_or_else(|| args::bad(span, "canonical_poisson_bracket needs dg_dq"))?;
    let dg_dp = args::rec_f64_list(args_v, "dg_dp")
        .ok_or_else(|| args::bad(span, "canonical_poisson_bracket needs dg_dp"))?;
    let bracket = canonical_poisson_bracket(
        vec2(&df_dq, span, "df_dq")?,
        vec2(&df_dp, span, "df_dp")?,
        vec2(&dg_dq, span, "dg_dq")?,
        vec2(&dg_dp, span, "dg_dp")?,
    )
    .map_err(|e| args::bad(span, format!("canonical_poisson_bracket: {e:?}")))?;
    Ok(Value::F64(bracket))
}

/// `Calculus.stormer_verlet_step` — 1-D harmonic Störmer–Verlet.
/// Args: `{ q, p, h, k?, mass? }`. Out: `{ q, p }`.
pub fn stormer_verlet_step_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = args::rec_f64(args_v, "q").ok_or_else(|| args::bad(span, "needs q"))?;
    let p = args::rec_f64(args_v, "p").ok_or_else(|| args::bad(span, "needs p"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let k = args::rec_f64(args_v, "k").unwrap_or(1.0);
    let mass = args::rec_f64(args_v, "mass").unwrap_or(1.0);
    let mut state = PhaseState {
        q: Vector::new([q]),
        p: Vector::new([p]),
    };
    stormer_verlet_step(
        &mut state,
        h,
        |qq| Vector::new([k * qq.data[0]]),
        |pp| Vector::new([pp.data[0] / mass]),
    )
    .map_err(|e| args::bad(span, format!("stormer_verlet_step: {e:?}")))?;
    Ok(args::record([
        ("q", Value::F64(state.q.data[0])),
        ("p", Value::F64(state.p.data[0])),
    ]))
}

/// `Calculus.adaptive_gauss_kronrod_15` — G7-K15 adaptive quadrature.
/// Args: `{ expr, a, b, var?, tolerance?, max_evaluations? }`.
pub fn adaptive_gauss_kronrod_15_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "adaptive_gauss_kronrod_15 needs expr: string"))?;
    let var_name = args::rec_str(args_v, "var").unwrap_or("x");
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "adaptive_gauss_kronrod_15 needs a"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "adaptive_gauss_kronrod_15 needs b"))?;
    let tol = args::rec_f64(args_v, "tolerance").unwrap_or(1e-8);
    let max_evals = args::rec_i64(args_v, "max_evaluations").unwrap_or(10_000) as u32;
    let expr = parse_expr(expr_str, span)?;
    let f = |x: f64| -> f64 {
        let mut env = HashMap::new();
        env.insert(var_name.to_string(), x);
        expr.eval(&env).unwrap_or(f64::NAN)
    };
    match adaptive_gauss_kronrod_15(f, a, b, tol, max_evals) {
        Ok(res) => Ok(args::record([
            ("value", Value::F64(res.value)),
            ("absolute_error", Value::F64(res.absolute_error)),
            ("evaluations", Value::I64(res.evaluations as i64)),
            ("intervals", Value::I64(res.intervals as i64)),
        ])),
        Err(e) => Err(args::bad(span, format!("adaptive_gauss_kronrod_15: {e:?}"))),
    }
}

/// `Calculus.jvp` — Jacobian-vector product for a fixed 2×2 map.
/// Args: `{ a: [[f64;2];2], v: [f64;2] }`. Out: `{ out: [f64;2] }`.
pub fn jvp_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_val = args::rec(args_v, "a").ok_or_else(|| args::bad(span, "jvp needs a"))?;
    let v = args::rec_f64_list(args_v, "v").ok_or_else(|| args::bad(span, "jvp needs v"))?;
    let map = parse_matrix_2x2(a_val, span, "jvp")?;
    let out = jvp(&map, vec2(&v, span, "v")?)
        .map_err(|e| args::bad(span, format!("jvp: {e:?}")))?;
    Ok(args::record([(
        "out",
        Value::List(vec![Value::F64(out.data[0]), Value::F64(out.data[1])]),
    )]))
}

/// `Calculus.vjp` — vector-Jacobian product for a fixed 2×2 map.
/// Args: `{ a: [[f64;2];2], w: [f64;2] }`. Out: `{ out: [f64;2] }`.
pub fn vjp_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_val = args::rec(args_v, "a").ok_or_else(|| args::bad(span, "vjp needs a"))?;
    let w = args::rec_f64_list(args_v, "w").ok_or_else(|| args::bad(span, "vjp needs w"))?;
    let map = parse_matrix_2x2(a_val, span, "vjp")?;
    let out = vjp(&map, vec2(&w, span, "w")?)
        .map_err(|e| args::bad(span, format!("vjp: {e:?}")))?;
    Ok(args::record([(
        "out",
        Value::List(vec![Value::F64(out.data[0]), Value::F64(out.data[1])]),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave18_canonical_poisson_bracket_antisym() {
        let mut m = BTreeMap::new();
        m.insert("df_dq".into(), args::f64_list_value([1.0, 2.0]));
        m.insert("df_dp".into(), args::f64_list_value([3.0, 4.0]));
        m.insert("dg_dq".into(), args::f64_list_value([-2.0, 1.0]));
        m.insert("dg_dp".into(), args::f64_list_value([0.5, 3.0]));
        let fg = match canonical_poisson_bracket_host(&Value::Record(m.clone()), span()).unwrap() {
            Value::F64(x) => x,
            other => panic!("{other:?}"),
        };
        // Swap f↔g → negate.
        let mut swap = BTreeMap::new();
        swap.insert("df_dq".into(), m.remove("dg_dq").unwrap());
        swap.insert("df_dp".into(), m.remove("dg_dp").unwrap());
        swap.insert("dg_dq".into(), m.remove("df_dq").unwrap());
        swap.insert("dg_dp".into(), m.remove("df_dp").unwrap());
        let gf = match canonical_poisson_bracket_host(&Value::Record(swap), span()).unwrap() {
            Value::F64(x) => x,
            other => panic!("{other:?}"),
        };
        assert!((fg + gf).abs() < 1e-12);
    }

    #[test]
    fn wave18_stormer_verlet_energy_approx() {
        let mut m = BTreeMap::new();
        m.insert("q".into(), Value::F64(1.0));
        m.insert("p".into(), Value::F64(0.0));
        m.insert("h".into(), Value::F64(0.01));
        let out = stormer_verlet_step_host(&Value::Record(m), span()).unwrap();
        let q = args::rec_f64(&out, "q").unwrap();
        let p = args::rec_f64(&out, "p").unwrap();
        let e0 = 0.5;
        let e1 = 0.5 * q * q + 0.5 * p * p;
        assert!((e1 - e0).abs() < 1e-4);
    }

    #[test]
    fn wave18_adaptive_gauss_kronrod_x2() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x^2".into()));
        m.insert("a".into(), Value::F64(0.0));
        m.insert("b".into(), Value::F64(3.0));
        let out = adaptive_gauss_kronrod_15_host(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64(&out, "value").unwrap();
        assert!((v - 9.0).abs() < 1e-6);
    }

    #[test]
    fn wave18_jvp_identity() {
        let mut m = BTreeMap::new();
        m.insert(
            "a".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(1.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(0.0), Value::F64(1.0)]),
            ]),
        );
        m.insert("v".into(), args::f64_list_value([3.0, -1.0]));
        let out = jvp_host(&Value::Record(m), span()).unwrap();
        let o = args::rec_f64_list(&out, "out").unwrap();
        assert!((o[0] - 3.0).abs() < 1e-12);
        assert!((o[1] + 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave18_vjp_transpose() {
        // A = [[1,2],[3,4]], w=[1,0] → Aᵀ w = [1,2]
        let mut m = BTreeMap::new();
        m.insert(
            "a".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(1.0), Value::F64(2.0)]),
                Value::List(vec![Value::F64(3.0), Value::F64(4.0)]),
            ]),
        );
        m.insert("w".into(), args::f64_list_value([1.0, 0.0]));
        let out = vjp_host(&Value::Record(m), span()).unwrap();
        let o = args::rec_f64_list(&out, "out").unwrap();
        assert!((o[0] - 1.0).abs() < 1e-12);
        assert!((o[1] - 2.0).abs() < 1e-12);
    }
}
