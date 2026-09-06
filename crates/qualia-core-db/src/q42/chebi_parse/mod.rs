//! ChEBI compounds.tsv local parser (AST-03).
//!
//! # Format
//!
//! Parses one documented ChEBI FTP-style bulk TSV from a **caller-selected**
//! local byte slice (or path). No network I/O. Tiny synthetic fixtures only in
//! tests.
//!
//! Required header (tab-separated):
//!
//! ```text
//! ID	STATUS	CHEBI_ACCESSION	SOURCE	PARENT_ID	NAME	DEFINITION
//! ```
//!
//! | Column | Notes |
//! |--------|--------|
//! | `ID` | Integer string (e.g. `15377`) |
//! | `STATUS` | Preserved as text (`C` / `E` / `D` …); `D` → quarantine |
//! | `CHEBI_ACCESSION` | e.g. `CHEBI:15377` (must match `ID`) |
//! | `SOURCE` | Free text; may be empty |
//! | `PARENT_ID` | Integer or empty → `None` |
//! | `NAME` | Required non-empty (after trim) for acceptance |
//! | `DEFINITION` | May be empty; preserved |
//!
//! Optional trailing columns are accepted and ignored.
//!
//! # Quarantine vs error
//!
//! - Oversize whole input (`max_bytes`) → [`ParseError::ByteBudgetExceeded`]
//! - Oversize single line (`max_line_bytes`) → [`ParseError::LineTooLong`]
//! - Header mismatch / missing → [`ParseError::BadHeader`]
//! - Cancel flag set → [`ParseError::Cancelled`]
//! - Deleted / malformed / empty-name rows → [`QuarantinedRow`] (others continue)
//!
//! Cold construction: `String` fields are intentional. Hot-path Quin mapping is
//! AST-04.

mod error;
mod parse;
mod record;
mod tsv;

pub use error::{bounded_preview, ParseError, QuarantinedRow, RAW_PREVIEW_MAX};
pub use parse::{parse_compounds_tsv, parse_compounds_tsv_path};
pub use record::{ChebiRecord, ParseBudgets, ParseReport};
pub use tsv::{accession_ok, header_matches, COMPOUNDS_HEADER, REQUIRED_COLUMNS};
