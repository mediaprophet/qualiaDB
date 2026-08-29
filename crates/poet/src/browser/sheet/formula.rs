//! Small deterministic formula evaluator for the Poet sheet surface.

use std::collections::BTreeSet;

use super::model::{cell_ref, parse_cell_ref, SheetState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FormulaError {
    Circular,
    DivZero,
    Name,
    Ref,
    Value,
}

impl FormulaError {
    fn label(self) -> &'static str {
        match self {
            Self::Circular => "#CIRC!",
            Self::DivZero => "#DIV/0!",
            Self::Name => "#NAME?",
            Self::Ref => "#REF!",
            Self::Value => "#VALUE!",
        }
    }
}

pub fn display_value(state: &SheetState, cell: &str) -> String {
    let raw = state.raw(cell);
    let Some(expression) = raw.strip_prefix('=') else {
        return raw.to_string();
    };
    let mut visiting = BTreeSet::new();
    visiting.insert(cell.to_ascii_uppercase());
    match evaluate(expression, state, &mut visiting) {
        Ok(value) => format_number(value),
        Err(error) => error.label().to_string(),
    }
}

fn evaluate(
    expression: &str,
    state: &SheetState,
    visiting: &mut BTreeSet<String>,
) -> Result<f64, FormulaError> {
    let mut parser = Parser {
        bytes: expression.as_bytes(),
        pos: 0,
        state,
        visiting,
    };
    let value = parser.expression()?;
    parser.spaces();
    if parser.pos == parser.bytes.len() {
        Ok(value)
    } else {
        Err(FormulaError::Value)
    }
}

struct Parser<'a, 'b> {
    bytes: &'a [u8],
    pos: usize,
    state: &'a SheetState,
    visiting: &'b mut BTreeSet<String>,
}

impl Parser<'_, '_> {
    fn expression(&mut self) -> Result<f64, FormulaError> {
        let mut value = self.term()?;
        loop {
            self.spaces();
            match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    value += self.term()?;
                }
                Some(b'-') => {
                    self.pos += 1;
                    value -= self.term()?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn term(&mut self) -> Result<f64, FormulaError> {
        let mut value = self.factor()?;
        loop {
            self.spaces();
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    value *= self.factor()?;
                }
                Some(b'/') => {
                    self.pos += 1;
                    let divisor = self.factor()?;
                    if divisor == 0.0 {
                        return Err(FormulaError::DivZero);
                    }
                    value /= divisor;
                }
                _ => return Ok(value),
            }
        }
    }

    fn factor(&mut self) -> Result<f64, FormulaError> {
        self.spaces();
        match self.peek() {
            Some(b'+') => {
                self.pos += 1;
                self.factor()
            }
            Some(b'-') => {
                self.pos += 1;
                Ok(-self.factor()?)
            }
            Some(b'(') => {
                self.pos += 1;
                let value = self.expression()?;
                self.spaces();
                self.consume(b')')?;
                Ok(value)
            }
            Some(ch) if ch.is_ascii_digit() || ch == b'.' => self.number(),
            Some(ch) if ch.is_ascii_alphabetic() => self.reference_or_function(),
            _ => Err(FormulaError::Value),
        }
    }

    fn number(&mut self) -> Result<f64, FormulaError> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|ch| ch.is_ascii_digit() || ch == b'.')
        {
            self.pos += 1;
        }
        std::str::from_utf8(&self.bytes[start..self.pos])
            .ok()
            .and_then(|value| value.parse().ok())
            .ok_or(FormulaError::Value)
    }

    fn reference_or_function(&mut self) -> Result<f64, FormulaError> {
        let start = self.pos;
        while self.peek().is_some_and(|ch| ch.is_ascii_alphabetic()) {
            self.pos += 1;
        }
        let letters = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| FormulaError::Value)?
            .to_ascii_uppercase();
        self.spaces();
        if self.peek() == Some(b'(') {
            self.pos += 1;
            return self.function(&letters);
        }
        let digit_start = self.pos;
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.pos += 1;
        }
        if digit_start == self.pos {
            return Err(FormulaError::Name);
        }
        let digits = std::str::from_utf8(&self.bytes[digit_start..self.pos])
            .map_err(|_| FormulaError::Ref)?;
        self.cell_value(&format!("{letters}{digits}"))
    }

    fn function(&mut self, name: &str) -> Result<f64, FormulaError> {
        let args_start = self.pos;
        let mut depth = 1usize;
        while self.pos < self.bytes.len() && depth > 0 {
            match self.bytes[self.pos] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            self.pos += 1;
        }
        if depth != 0 {
            return Err(FormulaError::Value);
        }
        let args = std::str::from_utf8(&self.bytes[args_start..self.pos - 1])
            .map_err(|_| FormulaError::Value)?;
        let mut values = Vec::new();
        for arg in split_arguments(args) {
            if let Some((start, end)) = arg.split_once(':') {
                values.extend(self.range_values(start, end)?);
            } else if !arg.trim().is_empty() {
                values.push(evaluate(arg, self.state, self.visiting)?);
            }
        }
        match name {
            "SUM" => Ok(values.iter().sum()),
            "AVERAGE" | "AVG" if !values.is_empty() => {
                Ok(values.iter().sum::<f64>() / values.len() as f64)
            }
            "MIN" if !values.is_empty() => Ok(values.into_iter().fold(f64::INFINITY, f64::min)),
            "MAX" if !values.is_empty() => Ok(values.into_iter().fold(f64::NEG_INFINITY, f64::max)),
            "COUNT" => Ok(values.len() as f64),
            "AVERAGE" | "AVG" | "MIN" | "MAX" => Err(FormulaError::Value),
            _ => Err(FormulaError::Name),
        }
    }

    fn range_values(&mut self, start: &str, end: &str) -> Result<Vec<f64>, FormulaError> {
        let (start_col, start_row) = parse_cell_ref(start).ok_or(FormulaError::Ref)?;
        let (end_col, end_row) = parse_cell_ref(end).ok_or(FormulaError::Ref)?;
        let mut values = Vec::new();
        for row in start_row.min(end_row)..=start_row.max(end_row) {
            for col in start_col.min(end_col)..=start_col.max(end_col) {
                values.push(self.cell_value(&cell_ref(col, row))?);
            }
        }
        Ok(values)
    }

    fn cell_value(&mut self, cell: &str) -> Result<f64, FormulaError> {
        let cell = cell.to_ascii_uppercase();
        if !self.visiting.insert(cell.clone()) {
            return Err(FormulaError::Circular);
        }
        let raw = self.state.raw(&cell).trim();
        let result = if raw.is_empty() {
            Ok(0.0)
        } else if let Some(expression) = raw.strip_prefix('=') {
            evaluate(expression, self.state, self.visiting)
        } else {
            raw.parse().map_err(|_| FormulaError::Value)
        };
        self.visiting.remove(&cell);
        result
    }

    fn spaces(&mut self) {
        while self.peek().is_some_and(|ch| ch.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn consume(&mut self, expected: u8) -> Result<(), FormulaError> {
        if self.peek() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(FormulaError::Value)
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
}

fn split_arguments(value: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out
}

fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "#NUM!".into();
    }
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        let value = format!("{value:.8}");
        value
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(entries: &[(&str, &str)]) -> SheetState {
        let mut state = SheetState::default();
        for (cell, value) in entries {
            state.set(cell, (*value).into());
        }
        state
    }

    #[test]
    fn arithmetic_has_normal_precedence() {
        let state = state(&[("A1", "2"), ("A2", "3"), ("B1", "=A1+A2*4")]);
        assert_eq!(display_value(&state, "B1"), "14");
    }

    #[test]
    fn aggregates_ranges_and_recalculates() {
        let mut state = state(&[
            ("A1", "2"),
            ("A2", "4"),
            ("A3", "6"),
            ("B1", "=SUM(A1:A3)"),
            ("B2", "=AVERAGE(A1:A3)"),
        ]);
        assert_eq!(display_value(&state, "B1"), "12");
        assert_eq!(display_value(&state, "B2"), "4");
        state.set("A2", "10".into());
        assert_eq!(display_value(&state, "B1"), "18");
    }

    #[test]
    fn circular_and_divide_by_zero_errors_are_visible() {
        let state = state(&[("A1", "=B1"), ("B1", "=A1"), ("C1", "=2/0")]);
        assert_eq!(display_value(&state, "A1"), "#CIRC!");
        assert_eq!(display_value(&state, "C1"), "#DIV/0!");
    }
}
