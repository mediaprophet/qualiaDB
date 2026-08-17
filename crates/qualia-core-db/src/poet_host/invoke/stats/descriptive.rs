//! Descriptive mean — `solvers::statistics` is the engine source of truth.

use super::super::args;
use crate::solvers::statistics::descriptive::mean;
use poet_vibe::{Diagnostic, Span, Value};

pub fn arithmetic_mean(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::f64s(args_v)
        .or_else(|| args::rec(args_v, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.mean needs a number list"))?;
    mean(&xs)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "mean of empty list"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_of_three() {
        let args = args::f64_list_value(vec![1.0, 2.0, 3.0]);
        assert_eq!(
            arithmetic_mean(&args, Span { start: 0, end: 0 }).unwrap(),
            Value::F64(2.0)
        );
    }
}
