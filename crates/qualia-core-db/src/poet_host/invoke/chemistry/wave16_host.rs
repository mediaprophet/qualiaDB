//! Wave-16 Host binds: Chemistry SCF linear-algebra helpers (stack DIIS path).
//!
//! Wraps `gaussian_elimination` for fixed 2×2 / 3×3 — no GPU / forge.

use super::super::args;
use crate::specialized_libs::chemistry_modeling::scf::gaussian_elimination;
use crate::specialized_libs::shared::zero_heap_algebra::ZeroHeapMatrix;
use vibe::{Diagnostic, Span, Value};

/// `Chemistry.gaussian_elimination` — solve A x = b (N∈{2,3}).
/// Args: `{ a: [[f64;N];N], b: [f64;N] }`. Out: `{ x: [f64] }`.
pub fn gaussian_elimination_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_val = args::rec(args_v, "a")
        .ok_or_else(|| args::bad(span, "gaussian_elimination needs a: matrix"))?;
    let rows = match a_val {
        Value::List(l) => l,
        _ => {
            return Err(args::bad(
                span,
                "gaussian_elimination: a must be a list of rows",
            ))
        }
    };
    let n = rows.len();
    let b = args::rec_f64_list(args_v, "b")
        .ok_or_else(|| args::bad(span, "gaussian_elimination needs b: [f64]"))?;
    if b.len() != n {
        return Err(args::bad(
            span,
            "gaussian_elimination: b length must match a",
        ));
    }
    let x = match n {
        2 => {
            let mut mat = ZeroHeapMatrix::<f64, 2, 2>::zeros();
            for (i, row) in rows.iter().enumerate() {
                let vals = args::f64s(row).ok_or_else(|| {
                    args::bad(span, "gaussian_elimination: row must be [f64]")
                })?;
                if vals.len() != 2 {
                    return Err(args::bad(
                        span,
                        "gaussian_elimination: 2×2 rows need length 2",
                    ));
                }
                mat.set(i, 0, vals[0]);
                mat.set(i, 1, vals[1]);
            }
            let bb = [b[0], b[1]];
            gaussian_elimination(mat, bb).map(|x| x.to_vec())
        }
        3 => {
            let mut mat = ZeroHeapMatrix::<f64, 3, 3>::zeros();
            for (i, row) in rows.iter().enumerate() {
                let vals = args::f64s(row).ok_or_else(|| {
                    args::bad(span, "gaussian_elimination: row must be [f64]")
                })?;
                if vals.len() != 3 {
                    return Err(args::bad(
                        span,
                        "gaussian_elimination: 3×3 rows need length 3",
                    ));
                }
                for j in 0..3 {
                    mat.set(i, j, vals[j]);
                }
            }
            let bb = [b[0], b[1], b[2]];
            gaussian_elimination(mat, bb).map(|x| x.to_vec())
        }
        _ => {
            return Err(args::bad(
                span,
                "gaussian_elimination: only 2×2 or 3×3 supported",
            ))
        }
    };
    match x {
        Ok(xs) => Ok(args::record([(
            "x",
            Value::List(xs.into_iter().map(Value::F64).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("gaussian_elimination: {e:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave16_gaussian_elimination_2x2() {
        // [[2,0],[0,3]] [x,y]=[4,9] → [2,3]
        let mut m = BTreeMap::new();
        m.insert(
            "a".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(2.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(0.0), Value::F64(3.0)]),
            ]),
        );
        m.insert(
            "b".into(),
            Value::List(vec![Value::F64(4.0), Value::F64(9.0)]),
        );
        let out = gaussian_elimination_host(&Value::Record(m), span()).unwrap();
        let x = args::rec_f64_list(&out, "x").unwrap();
        assert!((x[0] - 2.0).abs() < 1e-12);
        assert!((x[1] - 3.0).abs() < 1e-12);
    }
}
