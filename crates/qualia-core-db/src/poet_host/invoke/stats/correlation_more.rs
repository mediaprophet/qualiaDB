//! Correlation invoke seams — `solvers::statistics::correlation`.

use super::super::args;
use crate::solvers::statistics::correlation;
use poet_vibe::{Diagnostic, Span, Value};

/// `Statistics.spearman` — Spearman rank correlation.
/// Args: { x: [f64], y: [f64] }
pub fn spearman(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x")
        .ok_or_else(|| args::bad(span, "Statistics.spearman needs x"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "Statistics.spearman needs y"))?;
    correlation::spearman(&x, &y)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "spearman undefined"))
}

/// `Statistics.kendall` — Kendall's tau rank correlation.
/// Args: { x: [f64], y: [f64] }
pub fn kendall(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x")
        .ok_or_else(|| args::bad(span, "Statistics.kendall needs x"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "Statistics.kendall needs y"))?;
    correlation::kendall(&x, &y)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "kendall undefined"))
}
