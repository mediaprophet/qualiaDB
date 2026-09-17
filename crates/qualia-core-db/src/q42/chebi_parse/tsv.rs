//! Tab-separated line splitting and field validation for ChEBI compounds.tsv.

use super::error::{bounded_preview, QuarantinedRow};
use super::record::ChebiRecord;

/// Documented ChEBI FTP-style `compounds.tsv` header (seven required columns).
pub const COMPOUNDS_HEADER: &[&str] = &[
    "ID",
    "STATUS",
    "CHEBI_ACCESSION",
    "SOURCE",
    "PARENT_ID",
    "NAME",
    "DEFINITION",
];

/// Minimum columns required; trailing extras are ignored.
pub const REQUIRED_COLUMNS: usize = 7;

/// Split a line on tabs without allocating per empty trailing cell beyond split.
pub fn split_tsv_fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

/// Return true when `fields` starts with the documented header names.
pub fn header_matches(fields: &[&str]) -> bool {
    if fields.len() < REQUIRED_COLUMNS {
        return false;
    }
    COMPOUNDS_HEADER
        .iter()
        .zip(fields.iter().take(REQUIRED_COLUMNS))
        .all(|(expected, got)| *expected == *got)
}

/// Validate accession shape: `CHEBI:` + decimal digits (case-sensitive prefix).
pub fn accession_ok(accession: &str, id: u64) -> bool {
    let Some(rest) = accession.strip_prefix("CHEBI:") else {
        return false;
    };
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    match rest.parse::<u64>() {
        Ok(n) => n == id,
        Err(_) => false,
    }
}

/// Parse one data row into an accepted record or a quarantine entry.
///
/// Deterministic reason codes:
/// - `column_count` — fewer than seven columns
/// - `malformed_id` — ID not a decimal integer
/// - `deleted_status` — STATUS is `D`
/// - `malformed_accession` — accession not `CHEBI:<id>`
/// - `malformed_parent_id` — PARENT_ID present but not an integer
/// - `empty_name` — NAME cell empty after trim of surrounding whitespace only
///   (leading/trailing whitespace on NAME is preserved in the accepted form;
///   emptiness is checked after `trim()` so whitespace-only names quarantine)
pub fn parse_data_row(source_line: u64, line: &str) -> Result<ChebiRecord, QuarantinedRow> {
    let fields = split_tsv_fields(line);
    if fields.len() < REQUIRED_COLUMNS {
        return Err(QuarantinedRow {
            source_line,
            reason: "column_count",
            raw_preview: bounded_preview(line),
        });
    }

    let id_raw = fields[0];
    let status = fields[1];
    let accession = fields[2];
    let source = fields[3];
    let parent_raw = fields[4];
    let name = fields[5];
    let definition = fields[6];
    // Trailing columns (fields[7..]) ignored intentionally.

    let id = match id_raw.parse::<u64>() {
        Ok(n) => n,
        Err(_) => {
            return Err(QuarantinedRow {
                source_line,
                reason: "malformed_id",
                raw_preview: bounded_preview(line),
            });
        }
    };

    if status == "D" {
        return Err(QuarantinedRow {
            source_line,
            reason: "deleted_status",
            raw_preview: bounded_preview(line),
        });
    }

    if !accession_ok(accession, id) {
        return Err(QuarantinedRow {
            source_line,
            reason: "malformed_accession",
            raw_preview: bounded_preview(line),
        });
    }

    let parent_id = if parent_raw.is_empty() {
        None
    } else {
        match parent_raw.parse::<u64>() {
            Ok(n) => Some(n),
            Err(_) => {
                return Err(QuarantinedRow {
                    source_line,
                    reason: "malformed_parent_id",
                    raw_preview: bounded_preview(line),
                });
            }
        }
    };

    if name.trim().is_empty() {
        return Err(QuarantinedRow {
            source_line,
            reason: "empty_name",
            raw_preview: bounded_preview(line),
        });
    }

    Ok(ChebiRecord {
        id,
        status: status.to_string(),
        accession: accession.to_string(),
        source: source.to_string(),
        parent_id,
        name: name.to_string(),
        definition: definition.to_string(),
        source_line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_accepts_trailing_columns() {
        let fields = [
            "ID",
            "STATUS",
            "CHEBI_ACCESSION",
            "SOURCE",
            "PARENT_ID",
            "NAME",
            "DEFINITION",
            "EXTRA",
        ];
        assert!(header_matches(&fields));
    }

    #[test]
    fn accession_must_match_id() {
        assert!(accession_ok("CHEBI:15377", 15377));
        assert!(!accession_ok("CHEBI:1", 15377));
        assert!(!accession_ok("chebi:15377", 15377));
        assert!(!accession_ok("CHEBI:", 0));
    }

    #[test]
    fn empty_parent_is_none() {
        let line = "10\tC\tCHEBI:10\tsrc\t\twater\tdef";
        let rec = parse_data_row(2, line).unwrap();
        assert_eq!(rec.parent_id, None);
        assert_eq!(rec.name, "water");
    }
}
