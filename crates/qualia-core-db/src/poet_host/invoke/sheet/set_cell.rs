//! Bounded numeric cell replacement over existing Vibe grid values.
use super::super::args;
use vibe::{Diagnostic, Span, Value};

const MAX_SIDE: usize = 32;
fn numeric(v: &Value) -> bool {
    match v { Value::I64(_) | Value::U64(_) => true, Value::F64(n) => n.is_finite(), _ => false }
}
fn coordinate(v: Option<&Value>, span: Span) -> Result<usize, Diagnostic> {
    match v {
        Some(Value::U64(n)) => usize::try_from(*n).map_err(|_| args::bad(span, "sheet coordinate is out of range")),
        Some(Value::I64(n)) if *n >= 0 => usize::try_from(*n).map_err(|_| args::bad(span, "sheet coordinate is out of range")),
        _ => Err(args::bad(span, "sheet coordinate must be a nonnegative integer")),
    }
}
/// Returns a new rectangular grid; all untouched numeric Value variants are exact.
pub fn set_cell(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let row = coordinate(args::rec(v, "row"), span)?;
    let col = coordinate(args::rec(v, "col"), span)?;
    let replacement = args::rec(v, "value").filter(|v| numeric(v)).ok_or_else(|| args::bad(span, "sheet replacement must be a finite number"))?;
    let rows = args::rec(v, "grid").and_then(args::list).ok_or_else(|| args::bad(span, "sheet grid must be a numeric list of rows"))?;
    if rows.is_empty() || rows.len() > MAX_SIDE { return Err(args::bad(span, "sheet requires 1 to 32 rows")); }
    let width = args::list(&rows[0]).map(|r| r.len()).ok_or_else(|| args::bad(span, "sheet row must be a numeric list"))?;
    if width == 0 || width > MAX_SIDE { return Err(args::bad(span, "sheet requires 1 to 32 columns")); }
    for candidate in rows {
        let cells = args::list(candidate).ok_or_else(|| args::bad(span, "sheet row must be a numeric list"))?;
        if cells.len() != width || !cells.iter().all(numeric) { return Err(args::bad(span, "sheet rows must be rectangular and contain finite numbers")); }
    }
    if row >= rows.len() || col >= width { return Err(args::bad(span, "sheet cell is outside the existing grid")); }
    let mut updated = rows.to_vec();
    let Value::List(cells) = &mut updated[row] else { unreachable!("row validated above") };
    cells[col] = replacement.clone();
    Ok(Value::List(updated))
}
