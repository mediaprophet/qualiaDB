//! Clean-room first-party bounded `Sheet.eval_formula` parser and evaluator.
//! Cold capability path; no external workbook dependency, persistence, or authorization is implied.
use super::super::{args, ids};
use vibe::{Diagnostic, Span, Value};

const MAX_FORMULA_BYTES: usize = 256;
const MAX_AST_NODES: usize = 64;
const MAX_AST_DEPTH: usize = 16;
const MAX_GRID_SIDE: usize = 32;
const MAX_CALL_ARGS: usize = 8;
const MAX_EXACT_INT: u64 = 1u64 << 53;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellCoord {
    row: usize,
    col: usize,
}

#[derive(Clone, Debug, PartialEq)]
enum RefKind {
    Cell(CellCoord),
    Range(CellCoord, CellCoord),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnOp {
    Plus,
    Neg,
    Percent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Number(f64),
    Ref(RefKind),
    Paren(Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
enum EvalResult {
    Number(f64),
    Error(&'static str),
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    nodes: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
            nodes: 0,
        }
    }

    fn alloc_node(&mut self) -> Result<(), &'static str> {
        self.nodes += 1;
        if self.nodes > MAX_AST_NODES {
            return Err("formula exceeds node budget");
        }
        Ok(())
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_formula(mut self) -> Result<Expr, &'static str> {
        self.skip_ws();
        if self.peek() != Some(b'=') {
            return Err("formula must start with =");
        }
        self.pos += 1;
        let expr = self.parse_add_sub(1)?;
        self.skip_ws();
        if self.pos != self.bytes.len() {
            return Err("unexpected trailing tokens in formula");
        }
        Ok(expr)
    }

    fn parse_add_sub(&mut self, depth: usize) -> Result<Expr, &'static str> {
        if depth > MAX_AST_DEPTH {
            return Err("formula exceeds complexity budget");
        }
        let mut left = self.parse_mul_div(depth)?;
        loop {
            let op = match self.peek() {
                Some(b'+') => BinOp::Add,
                Some(b'-') => BinOp::Sub,
                _ => break,
            };
            self.pos += 1;
            self.alloc_node()?;
            let right = self.parse_mul_div(depth + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_mul_div(&mut self, depth: usize) -> Result<Expr, &'static str> {
        if depth > MAX_AST_DEPTH {
            return Err("formula exceeds complexity budget");
        }
        let mut left = self.parse_unary(depth)?;
        loop {
            let op = match self.peek() {
                Some(b'*') => BinOp::Mul,
                Some(b'/') => BinOp::Div,
                _ => break,
            };
            self.pos += 1;
            self.alloc_node()?;
            let right = self.parse_unary(depth + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self, depth: usize) -> Result<Expr, &'static str> {
        if depth > MAX_AST_DEPTH {
            return Err("formula exceeds complexity budget");
        }
        match self.peek() {
            Some(b'+') => {
                self.pos += 1;
                self.alloc_node()?;
                let inner = self.parse_unary(depth + 1)?;
                Ok(Expr::Unary(UnOp::Plus, Box::new(inner)))
            }
            Some(b'-') => {
                self.pos += 1;
                self.alloc_node()?;
                let inner = self.parse_unary(depth + 1)?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(inner)))
            }
            _ => self.parse_postfix(depth),
        }
    }

    fn parse_postfix(&mut self, depth: usize) -> Result<Expr, &'static str> {
        let mut expr = self.parse_primary(depth)?;
        while self.peek() == Some(b'%') {
            self.pos += 1;
            self.alloc_node()?;
            expr = Expr::Unary(UnOp::Percent, Box::new(expr));
        }
        Ok(expr)
    }

    fn parse_primary(&mut self, depth: usize) -> Result<Expr, &'static str> {
        if depth > MAX_AST_DEPTH {
            return Err("formula exceeds complexity budget");
        }
        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                self.alloc_node()?;
                let inner = self.parse_add_sub(depth + 1)?;
                if self.peek() != Some(b')') {
                    return Err("unclosed parenthesis in formula");
                }
                self.pos += 1;
                Ok(Expr::Paren(Box::new(inner)))
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                self.alloc_node()?;
                let start = self.pos;
                let mut seen_dot = false;
                while self.pos < self.bytes.len() {
                    let b = self.bytes[self.pos];
                    if b.is_ascii_digit() {
                        self.pos += 1;
                    } else if b == b'.' && !seen_dot {
                        seen_dot = true;
                        self.pos += 1;
                    } else if (b == b'e' || b == b'E') && self.pos > start {
                        self.pos += 1;
                        if self.pos < self.bytes.len()
                            && (self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-')
                        {
                            self.pos += 1;
                        }
                        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                            self.pos += 1;
                        }
                        break;
                    } else {
                        break;
                    }
                }
                let text = std::str::from_utf8(&self.bytes[start..self.pos])
                    .map_err(|_| "invalid numeric literal")?;
                let value: f64 = text.parse().map_err(|_| "invalid numeric literal")?;
                if !value.is_finite() {
                    return Err("non-finite numeric literal");
                }
                Ok(Expr::Number(value))
            }
            Some(c) if c.is_ascii_alphabetic() || c == b'$' => self.parse_ident_or_ref(depth),
            _ => Err("formula uses unsupported or invalid syntax"),
        }
    }

    fn parse_cell_coord(&mut self) -> Result<CellCoord, &'static str> {
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'$' {
            self.pos += 1;
        }
        let col_start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        let col_slice = &self.bytes[col_start..self.pos];
        if col_slice.is_empty() || col_slice.len() > 3 {
            return Err("invalid cell column reference");
        }
        let mut col_1based = 0usize;
        for &b in col_slice {
            col_1based = col_1based
                .checked_mul(26)
                .and_then(|v| v.checked_add((b.to_ascii_uppercase() - b'A') as usize + 1))
                .ok_or("column index overflow")?;
        }
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'$' {
            self.pos += 1;
        }
        let row_start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let row_slice = &self.bytes[row_start..self.pos];
        if row_slice.is_empty() {
            return Err("whole-column or incomplete cell reference is not supported");
        }
        let row_str =
            std::str::from_utf8(row_slice).map_err(|_| "invalid cell row reference")?;
        let row_1based: usize = row_str
            .parse()
            .map_err(|_| "formula reference exceeds the bounded grid")?;
        if row_1based == 0 || col_1based == 0 {
            return Err("cell coordinates are 1-based");
        }
        let row = row_1based - 1;
        let col = col_1based - 1;
        if row >= MAX_GRID_SIDE || col >= MAX_GRID_SIDE {
            return Err("formula reference exceeds the bounded grid");
        }
        Ok(CellCoord { row, col })
    }

    fn parse_ident_or_ref(&mut self, depth: usize) -> Result<Expr, &'static str> {
        let saved = self.pos;
        if self.bytes[self.pos] != b'$' {
            let ident_start = self.pos;
            while self.pos < self.bytes.len()
                && (self.bytes[self.pos].is_ascii_alphanumeric() || self.bytes[self.pos] == b'_')
            {
                self.pos += 1;
            }
            let ident = std::str::from_utf8(&self.bytes[ident_start..self.pos])
                .map_err(|_| "invalid identifier")?
                .to_ascii_uppercase();
            self.skip_ws();
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b'(' {
                self.pos += 1;
                self.alloc_node()?;
                if !matches!(
                    ident.as_str(),
                    "SUM" | "AVERAGE" | "MIN" | "MAX" | "COUNT"
                ) {
                    return Err("formula uses a feature not supported by this bounded Sheet evaluator");
                }
                let mut args = Vec::new();
                if self.peek() == Some(b')') {
                    return Err("function call requires at least one argument");
                }
                loop {
                    if args.len() >= MAX_CALL_ARGS {
                        return Err("function call exceeds argument budget");
                    }
                    args.push(self.parse_add_sub(depth + 1)?);
                    match self.peek() {
                        Some(b',') => {
                            self.pos += 1;
                        }
                        Some(b')') => {
                            self.pos += 1;
                            break;
                        }
                        _ => return Err("unclosed or malformed function call"),
                    }
                }
                return Ok(Expr::Call(ident, args));
            }
        }
        self.pos = saved;
        self.alloc_node()?;
        let first = self.parse_cell_coord()?;
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'!' {
            return Err("cross-sheet references are not supported");
        }
        if self.peek() == Some(b':') {
            self.pos += 1;
            let second = self.parse_cell_coord()?;
            Ok(Expr::Ref(RefKind::Range(first, second)))
        } else {
            Ok(Expr::Ref(RefKind::Cell(first)))
        }
    }
}

fn bounded_number(value: &Value) -> Option<f64> {
    match value {
        Value::F64(n) if n.is_finite() => Some(*n),
        Value::I64(n) if n.unsigned_abs() <= MAX_EXACT_INT => Some(*n as f64),
        Value::U64(n) if *n <= MAX_EXACT_INT => Some(*n as f64),
        _ => None,
    }
}

fn analyze(expr: &Expr, depth: usize) -> Result<(usize, bool), &'static str> {
    if depth > MAX_AST_DEPTH {
        return Err("formula exceeds complexity budget");
    }
    match expr {
        Expr::Number(_) => Ok((depth, false)),
        Expr::Ref(_) => Ok((depth, true)),
        Expr::Paren(inner) | Expr::Unary(_, inner) => analyze(inner, depth + 1),
        Expr::Binary(_, left, right) => {
            let (dl, rl) = analyze(left, depth + 1)?;
            let (dr, rr) = analyze(right, depth + 1)?;
            Ok((dl.max(dr), rl || rr))
        }
        Expr::Call(_, args) => {
            let mut max_d = depth;
            let mut has_ref = false;
            for arg in args {
                let (d, r) = analyze(arg, depth + 1)?;
                max_d = max_d.max(d);
                has_ref |= r;
            }
            Ok((max_d, has_ref))
        }
    }
}

fn eval_expr(expr: &Expr, grid: &[Vec<Option<f64>>]) -> EvalResult {
    let cell_val = |c: CellCoord| -> f64 {
        grid.get(c.row)
            .and_then(|r| r.get(c.col))
            .copied()
            .flatten()
            .unwrap_or(0.0)
    };
    match expr {
        Expr::Number(n) => EvalResult::Number(*n),
        Expr::Ref(RefKind::Cell(c)) => EvalResult::Number(cell_val(*c)),
        Expr::Ref(RefKind::Range(_, _)) => EvalResult::Error("#VALUE!"),
        Expr::Paren(inner) => eval_expr(inner, grid),
        Expr::Unary(op, inner) => match eval_expr(inner, grid) {
            EvalResult::Number(n) => {
                let v = match op {
                    UnOp::Plus => n,
                    UnOp::Neg => -n,
                    UnOp::Percent => n / 100.0,
                };
                if v.is_finite() {
                    EvalResult::Number(v)
                } else {
                    EvalResult::Error("#NUM!")
                }
            }
            err => err,
        },
        Expr::Binary(op, left, right) => match (eval_expr(left, grid), eval_expr(right, grid)) {
            (EvalResult::Error(e), _) | (_, EvalResult::Error(e)) => EvalResult::Error(e),
            (EvalResult::Number(a), EvalResult::Number(b)) => {
                if *op == BinOp::Div && b == 0.0 {
                    return EvalResult::Error("#DIV/0!");
                }
                let v = match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div => a / b,
                };
                if v.is_finite() {
                    EvalResult::Number(v)
                } else {
                    EvalResult::Error("#NUM!")
                }
            }
        },
        Expr::Call(name, args) => {
            let mut values = Vec::new();
            for arg in args {
                match arg {
                    Expr::Ref(RefKind::Range(a, b)) => {
                        let r0 = a.row.min(b.row);
                        let r1 = a.row.max(b.row);
                        let c0 = a.col.min(b.col);
                        let c1 = a.col.max(b.col);
                        for r in r0..=r1 {
                            for c in c0..=c1 {
                                if let Some(Some(v)) = grid.get(r).and_then(|row| row.get(c)) {
                                    values.push(*v);
                                }
                            }
                        }
                    }
                    Expr::Ref(RefKind::Cell(c)) => {
                        if let Some(Some(v)) = grid.get(c.row).and_then(|row| row.get(c.col)) {
                            values.push(*v);
                        }
                    }
                    other => match eval_expr(other, grid) {
                        EvalResult::Number(v) => values.push(v),
                        err @ EvalResult::Error(_) => return err,
                    },
                }
            }
            match name.as_str() {
                "SUM" => {
                    let s: f64 = values.iter().sum();
                    if s.is_finite() {
                        EvalResult::Number(s)
                    } else {
                        EvalResult::Error("#NUM!")
                    }
                }
                "COUNT" => EvalResult::Number(values.len() as f64),
                "AVERAGE" => {
                    if values.is_empty() {
                        EvalResult::Error("#DIV/0!")
                    } else {
                        let avg = values.iter().sum::<f64>() / (values.len() as f64);
                        if avg.is_finite() {
                            EvalResult::Number(avg)
                        } else {
                            EvalResult::Error("#NUM!")
                        }
                    }
                }
                "MIN" => {
                    let m = values
                        .iter()
                        .copied()
                        .fold(f64::INFINITY, f64::min);
                    EvalResult::Number(if m.is_finite() { m } else { 0.0 })
                }
                "MAX" => {
                    let m = values
                        .iter()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max);
                    EvalResult::Number(if m.is_finite() { m } else { 0.0 })
                }
                _ => EvalResult::Error("#NAME?"),
            }
        }
    }
}

/// Evaluates a restricted scalar formula over a caller-supplied numeric Vibe
/// grid using QualiaDB's clean-room first-party parser and evaluator. Grid is
/// optional for formulas with no cell references. Unsupported/volatile expressions fail.
pub fn evaluate_formula(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let formula = args::rec_str(args_v, "formula")
        .ok_or_else(|| args::bad(span, "Sheet.eval_formula needs a formula"))?;
    if formula.len() > MAX_FORMULA_BYTES || !formula.starts_with('=') {
        return Err(args::bad(
            span,
            "formula must start with = and fit the 256-byte budget",
        ));
    }
    let expression = Parser::new(formula)
        .parse_formula()
        .map_err(|e| args::bad(span, format!("formula parse failed: {e}")))?;
    let (depth, references) = analyze(&expression, 1).map_err(|e| args::bad(span, e))?;
    let mut matrix: Vec<Vec<Option<f64>>> = Vec::new();
    if let Some(grid) = args::rec(args_v, "grid") {
        let rows = args::list(grid).ok_or_else(|| args::bad(span, "grid must contain numeric rows"))?;
        if rows.len() > MAX_GRID_SIDE {
            return Err(args::bad(span, "grid exceeds 32 rows"));
        }
        matrix.reserve(rows.len());
        for row in rows {
            let cells = args::list(row)
                .ok_or_else(|| args::bad(span, "grid row must be a numeric list"))?;
            if cells.len() > MAX_GRID_SIDE {
                return Err(args::bad(span, "grid exceeds 32 columns"));
            }
            let mut row_vals = Vec::with_capacity(cells.len());
            for cell in cells {
                let n = bounded_number(cell).ok_or_else(|| {
                    args::bad(
                        span,
                        "grid contains non-finite, non-numeric or inexact integer data",
                    )
                })?;
                row_vals.push(Some(n));
            }
            matrix.push(row_vals);
        }
    } else if references {
        return Err(args::bad(
            span,
            "formula references require an explicit numeric grid",
        ));
    }
    let evaluated = eval_expr(&expression, &matrix);
    let (result_type, value) = match evaluated {
        EvalResult::Number(n) if n.is_finite() => ("number", Value::F64(n)),
        EvalResult::Error(e) => ("error", Value::String(e.into())),
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
        if let Some(grid) = grid {
            fields.insert("grid".into(), grid);
        }
        Value::Record(fields)
    }
    fn value(rows: &[&[f64]]) -> Value {
        Value::List(
            rows.iter()
                .map(|row| Value::List(row.iter().copied().map(Value::F64).collect()))
                .collect(),
        )
    }
    #[test]
    fn actual_formula_evaluation_and_spreadsheet_errors_are_reported() {
        let span = Span { start: 0, end: 0 };
        let Value::Record(result) = evaluate_formula(
            &request("=SUM(A1:B2)*1.15", Some(value(&[&[1.0, 2.0], &[3.0, 4.0]]))),
            span,
        )
        .unwrap() else {
            panic!("record")
        };
        assert_eq!(result.get("value"), Some(&Value::F64(11.5)));
        assert_eq!(result.get("cached"), Some(&Value::Bool(false)));
        let Value::Record(error) = evaluate_formula(&request("=1/0", None), span).unwrap() else {
            panic!("record")
        };
        assert_eq!(
            error.get("result_type"),
            Some(&Value::String("error".into()))
        );
        assert_eq!(error.get("value"), Some(&Value::String("#DIV/0!".into())));
        let Value::Record(ragged) =
            evaluate_formula(&request("=SUM(A1:B2)", Some(value(&[&[2.0], &[]]))), span).unwrap()
        else {
            panic!("record")
        };
        assert_eq!(ragged.get("value"), Some(&Value::F64(2.0)));
    }
    #[test]
    fn formula_bounds_and_unsupported_features_fail_closed() {
        let span = Span { start: 0, end: 0 };
        for f in [
            "=NOW()",
            "=SUM(A1:ZZ999999)",
            "=A:A",
            "=SUM(A1:B2)",
            "=1^1000",
            "=SUM(",
        ] {
            assert!(evaluate_formula(&request(f, None), span).is_err(), "{f}");
        }
        let grid = Some(value(&[&[1.0]]));
        for f in ["=NOW()", "=SUM(A1:AG1)", "=A33", "=1^1000"] {
            assert!(
                evaluate_formula(&request(f, grid.clone()), span).is_err(),
                "{f}"
            );
        }
        assert!(
            evaluate_formula(&request("=SUM(A1:B2)", Some(value(&[&[f64::NAN]]))), span).is_err()
        );
        assert!(evaluate_formula(&request(&format!("={}", "1+".repeat(200)), None), span).is_err());
        let large = Value::List(vec![Value::List(vec![Value::U64(MAX_EXACT_INT + 1)])]);
        assert!(evaluate_formula(&request("=A1", Some(large)), span).is_err());
        let too_many = Value::List(vec![Value::List(vec![]); MAX_GRID_SIDE + 1]);
        assert!(evaluate_formula(&request("=1", Some(too_many)), span).is_err());
    }
}
