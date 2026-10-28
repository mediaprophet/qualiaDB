//! Bounded, typed Sheet.eval_formula adapter over GridCraft's actual evaluator.
//! Cold capability path; no workbook persistence or authorization is implied.
use super::super::{args, ids};
use gridcraft_core::{CellRef, Value as CellValue};
use gridcraft_formula::{BinOp, Expr, RefKind, SheetSel, UnOp};
use gridcraft_model::Workbook;
use vibe::{Diagnostic, Span, Value};

const MAX_FORMULA_BYTES: usize = 256;
const MAX_AST_NODES: usize = 64;
const MAX_AST_DEPTH: usize = 16;
const MAX_GRID_SIDE: usize = 32;
const MAX_CALL_ARGS: usize = 8;
const MAX_EXACT_INT: u64 = 1u64 << 53;

fn bounded_number(value: &Value) -> Option<f64> {
    match value {
        Value::F64(n) if n.is_finite() => Some(*n),
        Value::I64(n) if n.unsigned_abs() <= MAX_EXACT_INT => Some(*n as f64),
        Value::U64(n) if *n <= MAX_EXACT_INT => Some(*n as f64),
        _ => None,
    }
}

fn push<'a>(stack: &mut [Option<(&'a Expr, usize)>; MAX_AST_NODES], len: &mut usize, e: &'a Expr, depth: usize) -> Result<(), &'static str> {
    if *len == stack.len() { return Err("formula exceeds node budget"); }
    stack[*len] = Some((e, depth));
    *len += 1;
    Ok(())
}

fn validate(expr: &Expr) -> Result<(usize, bool), &'static str> {
    let mut pending = [None; MAX_AST_NODES];
    let mut len = 0;
    push(&mut pending, &mut len, expr, 1)?;
    let (mut visited, mut depth_seen, mut has_reference) = (0usize, 0usize, false);
    while len > 0 {
        len -= 1;
        let (node, depth) = pending[len].take().ok_or("invalid formula traversal")?;
        visited += 1;
        if visited > MAX_AST_NODES || depth > MAX_AST_DEPTH { return Err("formula exceeds complexity budget"); }
        depth_seen = depth_seen.max(depth);
        match node {
            Expr::Number(n) if n.is_finite() => {}
            Expr::Ref(reference) if reference.sheet == SheetSel::Current => {
                has_reference = true;
                let valid = |a: &gridcraft_formula::Anchor| (a.row as usize) < MAX_GRID_SIDE && (a.col as usize) < MAX_GRID_SIDE;
                match &reference.kind {
                    RefKind::Cell(a) if valid(a) => {}
                    RefKind::Range(a, b) if valid(a) && valid(b) => {}
                    _ => return Err("formula reference exceeds the bounded grid"),
                }
            }
            Expr::Unary(UnOp::Plus | UnOp::Neg | UnOp::Percent, inner) | Expr::Paren(inner) => {
                push(&mut pending, &mut len, inner, depth + 1)?;
            }
            Expr::Binary(BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div, left, right) => {
                push(&mut pending, &mut len, left, depth + 1)?;
                push(&mut pending, &mut len, right, depth + 1)?;
            }
            Expr::Call(name, arguments) if matches!(name.as_str(), "SUM" | "AVERAGE" | "MIN" | "MAX" | "COUNT") && !arguments.is_empty() && arguments.len() <= MAX_CALL_ARGS => {
                for argument in arguments { push(&mut pending, &mut len, argument, depth + 1)?; }
            }
            _ => return Err("formula uses a feature not supported by this bounded Sheet evaluator"),
        }
    }
    Ok((depth_seen, has_reference))
}

/// Evaluates a restricted scalar formula over a caller-supplied numeric Vibe
/// grid using Craft's existing typed parser and evaluator. Grid is optional for
/// formulas with no cell references. Unsupported/volatile expressions fail.
pub fn evaluate_formula(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let formula = args::rec_str(args_v, "formula").ok_or_else(|| args::bad(span, "Sheet.eval_formula needs a formula"))?;
    if formula.len() > MAX_FORMULA_BYTES || !formula.starts_with('=') {
        return Err(args::bad(span, "formula must start with = and fit the 256-byte budget"));
    }
    let expression = gridcraft_formula::parse(formula).map_err(|e| args::bad(span, format!("formula parse failed: {e}")))?;
    let (depth, references) = validate(&expression).map_err(|e| args::bad(span, e))?;
    let mut workbook = Workbook::new();
    if let Some(grid) = args::rec(args_v, "grid") {
        let rows = args::list(grid).ok_or_else(|| args::bad(span, "grid must contain numeric rows"))?;
        if rows.len() > MAX_GRID_SIDE { return Err(args::bad(span, "grid exceeds 32 rows")); }
        let sheet = workbook.sheet_mut(0).ok_or_else(|| args::bad(span, "Craft workbook has no initial sheet"))?;
        for (r, row) in rows.iter().enumerate() {
            let cells = args::list(row).ok_or_else(|| args::bad(span, "grid row must be a numeric list"))?;
            if cells.len() > MAX_GRID_SIDE { return Err(args::bad(span, "grid exceeds 32 columns")); }
            for (c, cell) in cells.iter().enumerate() {
                let n = bounded_number(cell).ok_or_else(|| args::bad(span, "grid contains non-finite, non-numeric or inexact integer data"))?;
                sheet.set_value(CellRef::new(r as u32, c as u32), CellValue::Number(n));
            }
        }
    } else if references {
        return Err(args::bad(span, "formula references require an explicit numeric grid"));
    }
    let evaluated = gridcraft_calc::recalc::evaluate_expr(&workbook, 0, CellRef::new(0, 0), &expression);
    let (result_type, value) = match evaluated {
        CellValue::Number(n) if n.is_finite() => ("number", Value::F64(n)),
        CellValue::Error(e) => ("error", Value::String(e.as_str().into())),
        _ => return Err(args::bad(span, "formula did not produce a finite scalar result")),
    };
    Ok(args::record([
        ("op", Value::String("sheet_eval_formula".into())),
        ("id", Value::String(ids::SHEET_EVAL_FORMULA.into())),
        ("formula", Value::String(formula.into())),
        ("ast_depth", Value::U64(depth as u64)),
        ("result_type", Value::String(result_type.into())),
        ("value", value),
        ("cached", Value::Bool(false)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn request(formula: &str, grid: Option<Value>) -> Value {
        let mut fields = BTreeMap::from([("formula".into(), Value::String(formula.into()))]);
        if let Some(grid) = grid { fields.insert("grid".into(), grid); }
        Value::Record(fields)
    }
    fn value(rows: &[&[f64]]) -> Value {
        Value::List(rows.iter().map(|row| Value::List(row.iter().copied().map(Value::F64).collect())).collect())
    }
    #[test]
    fn actual_formula_evaluation_and_spreadsheet_errors_are_reported() {
        let span = Span { start: 0, end: 0 };
        let Value::Record(result) = evaluate_formula(&request("=SUM(A1:B2)*1.15", Some(value(&[&[1.0,2.0],&[3.0,4.0]]))), span).unwrap() else { panic!("record") };
        assert_eq!(result.get("value"), Some(&Value::F64(11.5)));
        assert_eq!(result.get("cached"), Some(&Value::Bool(false)));
        let Value::Record(error) = evaluate_formula(&request("=1/0", None), span).unwrap() else { panic!("record") };
        assert_eq!(error.get("result_type"), Some(&Value::String("error".into())));
        assert_eq!(error.get("value"), Some(&Value::String("#DIV/0!".into())));
        let Value::Record(ragged) = evaluate_formula(&request("=SUM(A1:B2)", Some(value(&[&[2.0], &[]]))), span).unwrap() else { panic!("record") };
        assert_eq!(ragged.get("value"), Some(&Value::F64(2.0)));
    }
    #[test]
    fn formula_bounds_and_unsupported_features_fail_closed() {
        let span = Span { start: 0, end: 0 };
        for f in ["=NOW()", "=SUM(A1:ZZ999999)", "=A:A", "=SUM(A1:B2)", "=1^1000", "=SUM("] { assert!(evaluate_formula(&request(f, None), span).is_err(), "{f}"); }
        let grid = Some(value(&[&[1.0]]));
        for f in ["=NOW()", "=SUM(A1:AG1)", "=A33", "=1^1000"] { assert!(evaluate_formula(&request(f, grid.clone()), span).is_err(), "{f}"); }
        assert!(evaluate_formula(&request("=SUM(A1:B2)", Some(value(&[&[f64::NAN]]))), span).is_err());
        assert!(evaluate_formula(&request(&format!("={}", "1+".repeat(200)), None), span).is_err());
        let large = Value::List(vec![Value::List(vec![Value::U64(MAX_EXACT_INT + 1)])]);
        assert!(evaluate_formula(&request("=A1", Some(large)), span).is_err());
        let too_many = Value::List(vec![Value::List(vec![]); MAX_GRID_SIDE + 1]);
        assert!(evaluate_formula(&request("=1", Some(too_many)), span).is_err());
    }
}
