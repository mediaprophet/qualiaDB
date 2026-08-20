//! Nucleotide Smith–Waterman — `domains::biological::bioinformatics`.

use super::super::args;
use crate::domains::biological::bioinformatics::align_nucleotide;
use poet_vibe::{Diagnostic, Span, Value};

pub fn align(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (q, t) = pair_seq(args_v, span)?;
    let r = align_nucleotide(q.as_bytes(), t.as_bytes());
    Ok(args::record([
        ("score", Value::I64(r.score as i64)),
        ("identity_pct", Value::F64(r.identity_pct as f64)),
        ("gaps", Value::U64(r.num_gaps as u64)),
    ]))
}

fn pair_seq(args_v: &Value, span: Span) -> Result<(String, String), Diagnostic> {
    if let Some(xs) = args::list(args_v) {
        let q = xs
            .first()
            .and_then(args::as_str)
            .ok_or_else(|| args::bad(span, "align needs [query, target]"))?;
        let t = xs
            .get(1)
            .and_then(args::as_str)
            .ok_or_else(|| args::bad(span, "align needs [query, target]"))?;
        return Ok((q.to_string(), t.to_string()));
    }
    let q = args::rec_str(args_v, "query").ok_or_else(|| args::bad(span, "align needs query"))?;
    let t = args::rec_str(args_v, "target").ok_or_else(|| args::bad(span, "align needs target"))?;
    Ok((q.to_string(), t.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_scores_positive() {
        let args = Value::List(vec![
            Value::String("ACGT".into()),
            Value::String("ACGT".into()),
        ]);
        match align(&args, Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => match r.get("score") {
                Some(Value::I64(s)) => assert!(*s > 0),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
