//! 1-D hill-climb of `(x - target)^2` via `solvers::optimization::metaheuristics`.

use super::super::args;
use crate::solvers::optimization::metaheuristics::hill_climbing;
use vibe::{Diagnostic, Span, Value};

pub fn hill_climb(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let start = args::rec_f64(args_v, "start").unwrap_or(0.0);
    let target = args::rec_f64(args_v, "target")
        .ok_or_else(|| args::bad(span, "hill_climb needs target"))?;
    let step = args::rec_f64(args_v, "step").unwrap_or(0.25);
    let max_iter = args::rec_u64(args_v, "max_iter").unwrap_or(64) as usize;
    let (x, val) = hill_climbing(
        start,
        |cur| vec![cur - step, cur + step],
        |cur| {
            let d = cur - target;
            d * d
        },
        max_iter,
    );
    Ok(args::record([
        ("x", Value::F64(x)),
        ("objective", Value::F64(val)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn climbs_toward_two() {
        let mut m = BTreeMap::new();
        m.insert("start".into(), Value::F64(0.0));
        m.insert("target".into(), Value::F64(2.0));
        m.insert("step".into(), Value::F64(0.5));
        m.insert("max_iter".into(), Value::U64(16));
        let v = hill_climb(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::Record(r) => match r.get("x") {
                Some(Value::F64(x)) => assert!((x - 2.0).abs() < 1e-9),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
