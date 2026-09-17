//! Serializable spreadsheet state and coordinate helpers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SHEET_STATE_KEY: &str = "sheet.state.v1";
pub const MIN_ROWS: usize = 12;
pub const MIN_COLS: usize = 8;
pub const MAX_ROWS: usize = 200;
pub const MAX_COLS: usize = 52;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetState {
    pub rows: usize,
    pub cols: usize,
    #[serde(default)]
    pub cells: BTreeMap<String, String>,
}

impl Default for SheetState {
    fn default() -> Self {
        Self {
            rows: MIN_ROWS,
            cols: MIN_COLS,
            cells: BTreeMap::new(),
        }
    }
}

impl SheetState {
    pub fn from_settings(settings: &BTreeMap<String, String>) -> Self {
        let mut state: Self = settings
            .get(SHEET_STATE_KEY)
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();
        state.rows = state.rows.clamp(MIN_ROWS, MAX_ROWS);
        state.cols = state.cols.clamp(MIN_COLS, MAX_COLS);
        state.cells.retain(|cell, value| {
            !value.is_empty()
                && parse_cell_ref(cell)
                    .is_some_and(|(col, row)| col < state.cols && row < state.rows)
        });
        state
    }

    pub fn raw(&self, cell: &str) -> &str {
        self.cells.get(cell).map(String::as_str).unwrap_or("")
    }

    pub fn set(&mut self, cell: &str, value: String) {
        let value = value.trim_end_matches(['\r', '\n']).to_string();
        if value.is_empty() {
            self.cells.remove(cell);
        } else {
            self.cells.insert(cell.to_ascii_uppercase(), value);
        }
    }

    pub fn add_row(&mut self) -> bool {
        if self.rows >= MAX_ROWS {
            return false;
        }
        self.rows += 1;
        true
    }

    pub fn add_col(&mut self) -> bool {
        if self.cols >= MAX_COLS {
            return false;
        }
        self.cols += 1;
        true
    }

    /// Paste tab/newline-delimited values from the selected origin cell.
    pub fn paste_tsv(&mut self, origin: &str, text: &str) -> usize {
        let Some((origin_col, origin_row)) = parse_cell_ref(origin) else {
            return 0;
        };
        let mut written = 0;
        for (row_offset, line) in text.replace("\r\n", "\n").split('\n').enumerate() {
            if line.is_empty() && row_offset + 1 == text.lines().count() {
                continue;
            }
            let row = origin_row + row_offset;
            if row >= MAX_ROWS {
                break;
            }
            for (col_offset, value) in line.split('\t').enumerate() {
                let col = origin_col + col_offset;
                if col >= MAX_COLS {
                    break;
                }
                self.rows = self.rows.max(row + 1);
                self.cols = self.cols.max(col + 1);
                self.set(&cell_ref(col, row), value.to_string());
                written += 1;
            }
        }
        written
    }
}

pub fn cell_ref(col: usize, row: usize) -> String {
    format!("{}{}", col_label(col), row + 1)
}

pub fn col_label(mut col: usize) -> String {
    let mut label = String::new();
    loop {
        label.insert(0, (b'A' + (col % 26) as u8) as char);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    label
}

pub fn parse_cell_ref(value: &str) -> Option<(usize, usize)> {
    let value = value.trim();
    let split = value.find(|ch: char| ch.is_ascii_digit())?;
    if split == 0 || split == value.len() {
        return None;
    }
    let (letters, digits) = value.split_at(split);
    if !letters.chars().all(|ch| ch.is_ascii_alphabetic())
        || !digits.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    let mut col = 0usize;
    for ch in letters.bytes() {
        col = col.checked_mul(26)?;
        col = col.checked_add((ch.to_ascii_uppercase() - b'A' + 1) as usize)?;
    }
    let row = digits.parse::<usize>().ok()?;
    (row > 0).then_some((col - 1, row - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_round_trip_includes_double_letter_columns() {
        for (col, label) in [(0, "A1"), (25, "Z1"), (26, "AA1"), (51, "AZ1")] {
            assert_eq!(cell_ref(col, 0), label);
            assert_eq!(parse_cell_ref(label), Some((col, 0)));
        }
    }

    #[test]
    fn tsv_paste_expands_and_preserves_formulas() {
        let mut state = SheetState::default();
        assert_eq!(state.paste_tsv("B2", "1\t2\n3\t=SUM(B2:C2)"), 4);
        assert_eq!(state.raw("B2"), "1");
        assert_eq!(state.raw("C3"), "=SUM(B2:C2)");
    }
}
