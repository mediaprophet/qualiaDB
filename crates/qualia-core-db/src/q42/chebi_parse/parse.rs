//! High-level ChEBI `compounds.tsv` parse with budgets and cancel.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::error::ParseError;
use super::record::{ParseBudgets, ParseReport};
use super::tsv::{header_matches, parse_data_row, split_tsv_fields};

/// Parse ChEBI FTP-style `compounds.tsv` bytes into accepted records and
/// quarantined rows.
///
/// # Format
///
/// Tab-separated, header row required:
/// `ID STATUS CHEBI_ACCESSION SOURCE PARENT_ID NAME DEFINITION`
/// (tabs between columns). Optional trailing columns are ignored.
///
/// # Budgets
///
/// - Whole input larger than `budgets.max_bytes` → [`ParseError::ByteBudgetExceeded`].
/// - Any line longer than `budgets.max_line_bytes` → [`ParseError::LineTooLong`].
/// - Accepted + quarantined count exceeding `max_records` →
///   [`ParseError::RecordBudgetExceeded`].
///
/// # Cancellation
///
/// When `cancel` is set, returns [`ParseError::Cancelled`] at the next
/// non-empty data-line boundary (header is still validated first).
pub fn parse_compounds_tsv(
    input: &[u8],
    budgets: ParseBudgets,
    release_label: &str,
    cancel: &AtomicBool,
) -> Result<ParseReport, ParseError> {
    let budgets = budgets.validate()?;
    let size = input.len() as u64;
    if size > budgets.max_bytes {
        return Err(ParseError::ByteBudgetExceeded {
            size,
            max: budgets.max_bytes,
        });
    }

    match std::str::from_utf8(input) {
        Ok(text) => parse_compounds_text(text, size, budgets, release_label, cancel),
        Err(_) => {
            let owned = String::from_utf8_lossy(input).into_owned();
            parse_compounds_text(&owned, size, budgets, release_label, cancel)
        }
    }
}

/// Convenience: read a caller-selected local path then parse (no network).
pub fn parse_compounds_tsv_path(
    path: &Path,
    budgets: ParseBudgets,
    release_label: &str,
    cancel: &AtomicBool,
) -> Result<ParseReport, ParseError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(ParseError::Cancelled);
    }
    let budgets = budgets.validate()?;
    let meta_len = std::fs::metadata(path)?.len();
    if meta_len > budgets.max_bytes {
        return Err(ParseError::ByteBudgetExceeded {
            size: meta_len,
            max: budgets.max_bytes,
        });
    }
    let bytes = std::fs::read(path)?;
    parse_compounds_tsv(&bytes, budgets, release_label, cancel)
}

fn parse_compounds_text(
    text: &str,
    bytes_read: u64,
    budgets: ParseBudgets,
    release_label: &str,
    cancel: &AtomicBool,
) -> Result<ParseReport, ParseError> {
    let mut lines = text.split('\n').enumerate();
    let Some((header_idx, header_raw)) = lines.next() else {
        return Err(ParseError::BadHeader);
    };
    let header_line = strip_cr(header_raw);
    if header_line.is_empty() {
        return Err(ParseError::BadHeader);
    }
    check_line_len(header_idx as u64 + 1, header_line, budgets.max_line_bytes)?;
    let header_fields = split_tsv_fields(header_line);
    if !header_matches(&header_fields) {
        return Err(ParseError::BadHeader);
    }

    let mut accepted = Vec::new();
    let mut quarantined = Vec::new();

    for (idx, raw) in lines {
        let source_line = idx as u64 + 1;
        let line = strip_cr(raw);
        if line.is_empty() {
            continue;
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(ParseError::Cancelled);
        }
        check_line_len(source_line, line, budgets.max_line_bytes)?;

        let used = (accepted.len() + quarantined.len()) as u64;
        if used >= budgets.max_records {
            return Err(ParseError::RecordBudgetExceeded {
                used,
                max: budgets.max_records,
            });
        }

        match parse_data_row(source_line, line) {
            Ok(rec) => accepted.push(rec),
            Err(q) => quarantined.push(q),
        }
    }

    Ok(ParseReport {
        accepted,
        quarantined,
        bytes_read,
        release_label: release_label.to_string(),
    })
}

fn strip_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

fn check_line_len(source_line: u64, line: &str, max: u64) -> Result<(), ParseError> {
    let len = line.len() as u64;
    if len > max {
        return Err(ParseError::LineTooLong {
            source_line,
            len,
            max,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budgets(max_bytes: u64, max_records: u64, max_line_bytes: u64) -> ParseBudgets {
        ParseBudgets {
            max_bytes,
            max_records,
            max_line_bytes,
        }
    }

    fn header() -> &'static str {
        "ID\tSTATUS\tCHEBI_ACCESSION\tSOURCE\tPARENT_ID\tNAME\tDEFINITION"
    }

    fn cancel_flag(v: bool) -> AtomicBool {
        AtomicBool::new(v)
    }

    #[test]
    fn happy_path_accepts_synthetic_compounds() {
        let tsv = format!(
            "{}\n\
             15377\tC\tCHEBI:15377\tChEBI\t\twater\toxidane\n\
             16236\tC\tCHEBI:16236\tChEBI\t15377\tethanol\t\n\
             17790\tE\tCHEBI:17790\t\t\tmethanol\ta simple alcohol\n",
            header()
        );
        let report = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(10_000, 100, 2_000),
            "chebi-rel-synthetic-1",
            &cancel_flag(false),
        )
        .unwrap();
        assert_eq!(report.accepted.len(), 3);
        assert!(report.quarantined.is_empty());
        assert_eq!(report.total_rows(), 3);
        assert_eq!(report.release_label, "chebi-rel-synthetic-1");
        assert_eq!(report.bytes_read, tsv.len() as u64);
        assert_eq!(report.accepted[0].id, 15377);
        assert_eq!(report.accepted[0].parent_id, None);
        assert_eq!(report.accepted[1].parent_id, Some(15377));
        assert_eq!(report.accepted[2].definition, "a simple alcohol");
    }

    #[test]
    fn malformed_row_quarantined_others_accepted() {
        let tsv = format!(
            "{}\n\
             1\tC\tCHEBI:1\ts\t\talpha\td1\n\
             not-an-id\tC\tCHEBI:2\ts\t\tbeta\td2\n\
             3\tC\tCHEBI:3\ts\t\tgamma\td3\n",
            header()
        );
        let report = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(10_000, 100, 2_000),
            "rel",
            &cancel_flag(false),
        )
        .unwrap();
        assert_eq!(report.accepted.len(), 2);
        assert_eq!(report.quarantined.len(), 1);
        assert_eq!(report.quarantined[0].reason, "malformed_id");
        assert_eq!(report.quarantined[0].source_line, 3);
        assert_eq!(report.total_rows(), 3);
    }

    #[test]
    fn deleted_status_quarantined() {
        let tsv = format!(
            "{}\n\
             9\tD\tCHEBI:9\ts\t\tgone\told\n\
             10\tC\tCHEBI:10\ts\t\tkept\t\n",
            header()
        );
        let report = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(10_000, 100, 2_000),
            "rel",
            &cancel_flag(false),
        )
        .unwrap();
        assert_eq!(report.accepted.len(), 1);
        assert_eq!(report.accepted[0].id, 10);
        assert_eq!(report.quarantined.len(), 1);
        assert_eq!(report.quarantined[0].reason, "deleted_status");
    }

    #[test]
    fn oversize_max_bytes_errors() {
        let tsv = format!("{}\n1\tC\tCHEBI:1\t\t\tn\td\n", header());
        let err = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(8, 100, 2_000),
            "rel",
            &cancel_flag(false),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ParseError::ByteBudgetExceeded {
                size: tsv.len() as u64,
                max: 8
            }
        );
    }

    #[test]
    fn cancellation_mid_parse() {
        // Cancel is observed at the data-line boundary after a valid header
        // (deterministic mid-parse; no thread race).
        let tsv = format!(
            "{}\n\
             1\tC\tCHEBI:1\t\t\ta\t\n\
             2\tC\tCHEBI:2\t\t\tb\t\n",
            header()
        );
        assert_eq!(
            parse_compounds_tsv(
                tsv.as_bytes(),
                budgets(10_000, 100, 2_000),
                "rel",
                &cancel_flag(true),
            )
            .unwrap_err(),
            ParseError::Cancelled
        );

        // Header-only input with cancel set still succeeds (no data boundary).
        let header_only = format!("{}\n", header());
        let report = parse_compounds_tsv(
            header_only.as_bytes(),
            budgets(10_000, 100, 2_000),
            "rel",
            &cancel_flag(true),
        )
        .unwrap();
        assert!(report.accepted.is_empty());
        assert!(report.quarantined.is_empty());
    }

    #[test]
    fn deterministic_mapping() {
        let tsv = format!(
            "{}\n\
             5\tC\tCHEBI:5\tx\t\tok\td\n\
             bad\tC\tCHEBI:6\tx\t\tnook\td\n\
             7\tD\tCHEBI:7\tx\t\tdel\td\n",
            header()
        );
        let a = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(10_000, 100, 2_000),
            "same-label",
            &cancel_flag(false),
        )
        .unwrap();
        let b = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(10_000, 100, 2_000),
            "same-label",
            &cancel_flag(false),
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn empty_parent_id_is_none() {
        let tsv = format!("{}\n42\tC\tCHEBI:42\t\t\tnamed\t\n", header());
        let report = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(10_000, 100, 2_000),
            "rel",
            &cancel_flag(false),
        )
        .unwrap();
        assert_eq!(report.accepted[0].parent_id, None);
    }

    #[test]
    fn missing_header_is_bad_header() {
        let tsv = "1\tC\tCHEBI:1\t\t\tn\td\n";
        let err = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(10_000, 100, 2_000),
            "rel",
            &cancel_flag(false),
        )
        .unwrap_err();
        assert_eq!(err, ParseError::BadHeader);

        let empty = b"";
        assert_eq!(
            parse_compounds_tsv(empty, budgets(10_000, 100, 2_000), "rel", &cancel_flag(false))
                .unwrap_err(),
            ParseError::BadHeader
        );
    }

    #[test]
    fn trailing_columns_ignored() {
        let tsv = format!(
            "{}\tEXTRA\n\
             1\tC\tCHEBI:1\ts\t\tn\td\tx\n",
            header()
        );
        let report = parse_compounds_tsv(
            tsv.as_bytes(),
            budgets(10_000, 100, 2_000),
            "rel",
            &cancel_flag(false),
        )
        .unwrap();
        assert_eq!(report.accepted.len(), 1);
        assert_eq!(report.accepted[0].name, "n");
    }
}
