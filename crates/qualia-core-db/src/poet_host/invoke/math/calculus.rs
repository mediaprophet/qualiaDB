//! Composite Simpson via `solvers::calculus::dense`.

use super::super::args;
use crate::solvers::calculus::dense::simpson;
use vibe::{Diagnostic, Span, Value};

pub fn integrate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a").ok_or_else(|| args::bad(span, "simpson needs a"))?;
    let b = args::rec_f64(args_v, "b").ok_or_else(|| args::bad(span, "simpson needs b"))?;
    let power = args::rec_f64(args_v, "power").unwrap_or(2.0);
    let panels = args::rec_u64(args_v, "panels").unwrap_or(64) as usize;
    let y = simpson(&|x| x.powf(power), a, b, panels);
    Ok(Value::F64(y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn integrate_x_squared_0_1() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(0.0));
        m.insert("b".into(), Value::F64(1.0));
        m.insert("power".into(), Value::F64(2.0));
        m.insert("panels".into(), Value::U64(64));
        let v = integrate(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        match v {
            Value::F64(y) => assert!((y - 1.0 / 3.0).abs() < 1e-6),
            other => panic!("{other:?}"),
        }
    }
}
