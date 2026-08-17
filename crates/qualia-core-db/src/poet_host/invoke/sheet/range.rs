//! Spreadsheet start: stats over a numeric grid + A1:B2 range sum.

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};

fn sum_slice(xs: &[f64]) -> f64 {
    xs.iter().copied().sum()
}

fn mean_slice(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        None
    } else {
        Some(sum_slice(xs) / xs.len() as f64)
    }
}

pub fn stats(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let cells = flatten(args_v, span)?;
    if cells.is_empty() {
        return Err(args::bad(span, "Sheet.stats needs a numeric grid"));
    }
    let min = cells.iter().copied().fold(f64::INFINITY, f64::min);
    let max = cells.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Ok(args::record([
        ("count", Value::U64(cells.len() as u64)),
        ("sum", Value::F64(sum_slice(&cells))),
        (
            "mean",
            Value::F64(mean_slice(&cells).unwrap_or(0.0)),
        ),
        ("min", Value::F64(min)),
        ("max", Value::F64(max)),
    ]))
}

pub fn sum_range(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let grid = grid(args_v, span)?;
    let spec = args::rec_str(args_v, "range")
        .or_else(|| args::rec_str(args_v, "cells"))
        .ok_or_else(|| args::bad(span, "Sheet.sum_range needs range: \"A1:B2\""))?;
    let (c0, r0, c1, r1) = parse_range(spec).ok_or_else(|| args::bad(span, "bad A1 range"))?;
    let mut acc = 0.0;
    let mut n = 0u64;
    for r in r0..=r1 {
        for c in c0..=c1 {
            if let Some(v) = grid.get(r).and_then(|row| row.get(c)) {
                acc += *v;
                n += 1;
            }
        }
    }
    Ok(args::record([
        ("sum", Value::F64(acc)),
        ("count", Value::U64(n)),
    ]))
}

fn grid(args_v: &Value, span: Span) -> Result<Vec<Vec<f64>>, Diagnostic> {
    let rows = args::rec(args_v, "grid")
        .or(Some(args_v))
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "sheet needs grid: [[...], ...]"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let cells = args::f64s(row).ok_or_else(|| args::bad(span, "grid row must be numbers"))?;
        out.push(cells);
    }
    Ok(out)
}

fn flatten(args_v: &Value, span: Span) -> Result<Vec<f64>, Diagnostic> {
    if let Some(xs) = args::f64s(args_v) {
        return Ok(xs);
    }
    Ok(grid(args_v, span)?.into_iter().flatten().collect())
}

fn parse_a1(s: &str) -> Option<(usize, usize)> {
    let s = s.trim();
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut col = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        col = col * 26 + (bytes[i].to_ascii_uppercase() - b'A' + 1) as usize;
        i += 1;
    }
    if i == 0 || i == bytes.len() {
        return None;
    }
    let row: usize = s[i..].parse().ok()?;
    if row == 0 || col == 0 {
        return None;
    }
    Some((col - 1, row - 1))
}

fn parse_range(s: &str) -> Option<(usize, usize, usize, usize)> {
    let (a, b) = s.split_once(':')?;
    let (c0, r0) = parse_a1(a)?;
    let (c1, r1) = parse_a1(b)?;
    Some((c0.min(c1), r0.min(r1), c0.max(c1), r0.max(r1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn sum_a1_b2() {
        let mut m = BTreeMap::new();
        m.insert(
            "grid".into(),
            Value::List(vec![
                args::f64_list_value([1.0, 2.0]),
                args::f64_list_value([3.0, 4.0]),
            ]),
        );
        m.insert("range".into(), Value::String("A1:B2".into()));
        match sum_range(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => assert_eq!(r.get("sum"), Some(&Value::F64(10.0))),
            other => panic!("{other:?}"),
        }
    }
}
