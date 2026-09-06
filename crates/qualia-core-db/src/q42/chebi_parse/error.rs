//! Parse and quarantine errors for ChEBI `compounds.tsv` (AST-03).

use std::fmt;

/// Fail-closed errors for the cold-construction ChEBI TSV parser.
#[derive(Debug)]
pub enum ParseError {
    /// Header row missing or column names do not match the documented layout.
    BadHeader,
    /// Whole input exceeds [`super::ParseBudgets::max_bytes`].
    ByteBudgetExceeded { size: u64, max: u64 },
    /// A single line exceeds [`super::ParseBudgets::max_line_bytes`].
    LineTooLong { source_line: u64, len: u64, max: u64 },
    /// Accepted + quarantined rows would exceed [`super::ParseBudgets::max_records`].
    RecordBudgetExceeded { used: u64, max: u64 },
    /// Budget fields must be strictly positive.
    InvalidBudgets,
    /// Caller set the cancel flag mid-parse.
    Cancelled,
    /// Local filesystem read failed (path helper only; no network).
    Io(std::io::Error),
}

impl PartialEq for ParseError {
    fn eq(&self, other: &Self) -> bool {
        use ParseError::*;
        match (self, other) {
            (BadHeader, BadHeader) | (InvalidBudgets, InvalidBudgets) | (Cancelled, Cancelled) => {
                true
            }
            (ByteBudgetExceeded { size: a, max: b }, ByteBudgetExceeded { size: c, max: d }) => {
                a == c && b == d
            }
            (
                LineTooLong {
                    source_line: a,
                    len: b,
                    max: c,
                },
                LineTooLong {
                    source_line: d,
                    len: e,
                    max: f,
                },
            ) => a == d && b == e && c == f,
            (RecordBudgetExceeded { used: a, max: b }, RecordBudgetExceeded { used: c, max: d }) => {
                a == c && b == d
            }
            (Io(a), Io(b)) => a.kind() == b.kind(),
            _ => false,
        }
    }
}

impl Eq for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadHeader => write!(f, "compounds.tsv header missing or mismatched"),
            Self::ByteBudgetExceeded { size, max } => {
                write!(f, "input exceeds max_bytes (size={size}, max={max})")
            }
            Self::LineTooLong {
                source_line,
                len,
                max,
            } => write!(
                f,
                "line {source_line} exceeds max_line_bytes (len={len}, max={max})"
            ),
            Self::RecordBudgetExceeded { used, max } => {
                write!(f, "record budget exceeded (used={used}, max={max})")
            }
            Self::InvalidBudgets => write!(f, "parse budgets must be positive"),
            Self::Cancelled => write!(f, "chebi parse cancelled"),
            Self::Io(e) => write!(f, "chebi parse i/o: {e}"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// One quarantined source row (not accepted into the normalized set).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantinedRow {
    /// 1-based file line number (header is line 1).
    pub source_line: u64,
    /// Stable reason code for deterministic reports.
    pub reason: &'static str,
    /// Bounded UTF-8 preview of the raw line (lossy if needed).
    pub raw_preview: String,
}

/// Maximum bytes retained in [`QuarantinedRow::raw_preview`].
pub const RAW_PREVIEW_MAX: usize = 160;

/// Build a bounded preview from a raw line (no NUL expansion; chars truncated).
pub fn bounded_preview(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.len() + ch.len_utf8() > RAW_PREVIEW_MAX {
            break;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_respects_byte_budget() {
        let long = "x".repeat(RAW_PREVIEW_MAX + 40);
        let p = bounded_preview(&long);
        assert!(p.len() <= RAW_PREVIEW_MAX);
        assert_eq!(p.len(), RAW_PREVIEW_MAX);
    }
}
