//! CSV Parser for QualiaDB
//!
//! Zero-allocation CSV parser that streams data directly into NQuin format.
//! Supports common data types with proper inline type tagging, deterministic
//! subject identifiers (DOM-AX-7), provenance context/sensitivity packing,
//! and raw invalid lexical retention.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use csv::ReaderBuilder;
use std::io::Read;

use crate::mini_parser::hash_token;
use crate::NQuin;

pub const INVALID_VALUE_METADATA_FLAG: u64 = 1 << 31;
pub const PRED_RAW_INVALID_LEXICAL: u64 = crate::q_hash("qualia:rawInvalidLexical");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvDatatype {
    Integer,
    Float,
    DateTime,
    StringRef,
}

#[derive(Debug, Clone)]
pub struct CsvColumnMapping {
    pub source_key: String,
    pub column_index: Option<usize>,
    pub predicate_hash: u64,
    pub datatype: CsvDatatype,
}

#[derive(Debug, Clone)]
pub struct CsvMappingProfile {
    pub base_class_hash: u64,
    pub fields: Vec<CsvColumnMapping>,
}

impl CsvMappingProfile {
    pub fn new(base_class_hash: u64, fields: Vec<CsvColumnMapping>) -> Self {
        Self { base_class_hash, fields }
    }
}

/// Derive a deterministic 63-bit subject hash from `(base_class_or_ns, row_or_key, raw_record_hash)` (DOM-AX-7).
#[inline]
pub fn derive_deterministic_subject(base_class_or_ns: u64, row_or_key: u64, raw_record_hash: u64) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in &base_class_or_ns.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
    for &b in &row_or_key.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
    for &b in &raw_record_hash.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
    hash & 0x7FFF_FFFF_FFFF_FFFF
}

#[inline]
fn hash_record(record: &csv::ByteRecord) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for field in record.iter() {
        for &b in field { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash ^= 0xFF; hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[inline]
fn emit_quin(s: u64, p: u64, o: u64, c: u64, m: u64, on_quin: &mut impl FnMut(NQuin)) {
    on_quin(NQuin { subject: s, predicate: p, object: o, context: c, metadata: m, parity: NQuin::calculate_parity(s, p, o, c, m) });
}

#[inline]
fn emit_invalid(s: u64, p: u64, raw_str: &str, c: u64, on_quin: &mut impl FnMut(NQuin)) {
    let raw_hash = hash_token(raw_str) & 0x0FFF_FFFF_FFFF_FFFF;
    emit_quin(s, p, raw_hash, c, INVALID_VALUE_METADATA_FLAG, on_quin);
    emit_quin(s, PRED_RAW_INVALID_LEXICAL, raw_hash, c, INVALID_VALUE_METADATA_FLAG, on_quin);
}

/// Parse CSV data from a reader and stream Quins (backward-compatible signature).
pub fn parse_csv_to_quins<R: Read>(
    reader: R,
    profile: &mut CsvMappingProfile,
    on_quin: impl FnMut(NQuin),
) -> Result<(), String> {
    let source_context_hash = profile.base_class_hash & 0x00FF_FFFF_FFFF_FFFF;
    let sensitivity_class = NQuin::SENSITIVITY_PUBLIC;
    parse_csv_internal(reader, profile, source_context_hash, sensitivity_class, on_quin)
}

/// Parse CSV data with explicit provenance. Fails closed if source_context_hash or base_class_hash is 0.
pub fn parse_csv_to_quins_with_provenance<R: Read>(
    reader: R,
    profile: &mut CsvMappingProfile,
    source_context_hash: u64,
    sensitivity_class: u8,
    on_quin: impl FnMut(NQuin),
) -> Result<(), String> {
    if (source_context_hash & 0x00FF_FFFF_FFFF_FFFF) == 0 || profile.base_class_hash == 0 {
        return Err("Strict provenance requires non-zero source_context_hash and base_class_hash".into());
    }
    parse_csv_internal(reader, profile, source_context_hash, sensitivity_class, on_quin)
}

fn parse_csv_internal<R: Read>(
    reader: R,
    profile: &mut CsvMappingProfile,
    source_context_hash: u64,
    sensitivity_class: u8,
    mut on_quin: impl FnMut(NQuin),
) -> Result<(), String> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(reader);
    let headers = rdr.byte_headers().map_err(|e| format!("Failed to read headers: {}", e))?.clone();
    for field in profile.fields.iter_mut() {
        field.column_index = headers.iter().position(|h| h == field.source_key.as_bytes());
    }
    let mut record = csv::ByteRecord::new();
    let mut row_index: u64 = 0;
    let context = ((sensitivity_class as u64) << 56) | (source_context_hash & 0x00FF_FFFF_FFFF_FFFF);

    while rdr.read_byte_record(&mut record).map_err(|e| format!("CSV read error: {}", e))? {
        row_index += 1;
        let raw_record_hash = hash_record(&record);
        let subject_hash = derive_deterministic_subject(profile.base_class_hash, row_index, raw_record_hash);

        for field in &profile.fields {
            if let Some(idx) = field.column_index {
                if let Some(raw_bytes) = record.get(idx) {
                    match field.datatype {
                        CsvDatatype::Integer => {
                            let s = std::str::from_utf8(raw_bytes).unwrap_or("").trim();
                            let parsed = if !s.is_empty() {
                                s.parse::<i64>().map(|i| i as u64).ok().or_else(|| s.parse::<u64>().ok())
                            } else {
                                None
                            };
                            match parsed {
                                Some(v) => emit_quin(subject_hash, field.predicate_hash, v | (0b001 << 60), context, 0, &mut on_quin),
                                None => emit_invalid(subject_hash, field.predicate_hash, s, context, &mut on_quin),
                            }
                        }
                        CsvDatatype::Float => {
                            let s = std::str::from_utf8(raw_bytes).unwrap_or("").trim();
                            let parsed = if !s.is_empty() && !s.eq_ignore_ascii_case("nan") {
                                s.parse::<f32>().ok().filter(|f| f.is_finite())
                            } else {
                                None
                            };
                            match parsed {
                                Some(f) => emit_quin(subject_hash, field.predicate_hash, (0b010u64 << 60) | (f.to_bits() as u64), context, 0, &mut on_quin),
                                None => emit_invalid(subject_hash, field.predicate_hash, s, context, &mut on_quin),
                            }
                        }
                        CsvDatatype::StringRef => {
                            let s = std::str::from_utf8(raw_bytes).unwrap_or("");
                            let hash = hash_token(s) & 0x0FFF_FFFF_FFFF_FFFF;
                            emit_quin(subject_hash, field.predicate_hash, hash, context, 0, &mut on_quin);
                        }
                        CsvDatatype::DateTime => {
                            let s = std::str::from_utf8(raw_bytes).unwrap_or("").trim();
                            match parse_datetime_millis(s) {
                                Some(ms) => emit_quin(subject_hash, field.predicate_hash, (0b011u64 << 60) | ms, context, 0, &mut on_quin),
                                None => emit_invalid(subject_hash, field.predicate_hash, s, context, &mut on_quin),
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_datetime_millis(s: &str) -> Option<u64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis() as u64);
    }
    for fmt in &["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%SZ"] {
        if let Ok(nd) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(Utc.from_utc_datetime(&nd).timestamp_millis() as u64);
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let nd = d.and_hms_opt(0, 0, 0)?;
        return Some(Utc.from_utc_datetime(&nd).timestamp_millis() as u64);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn deterministic_csv_subjects_across_ingest() {
        let csv_data = "name,count\nAlice,10\nBob,20\n";
        let mut profile = CsvMappingProfile::new(1001, vec![
            CsvColumnMapping { source_key: "name".into(), column_index: None, predicate_hash: 10, datatype: CsvDatatype::StringRef },
            CsvColumnMapping { source_key: "count".into(), column_index: None, predicate_hash: 20, datatype: CsvDatatype::Integer },
        ]);
        let mut quins1 = Vec::new();
        parse_csv_to_quins(csv_data.as_bytes(), &mut profile, |q| quins1.push(q)).unwrap();
        let mut profile2 = profile.clone();
        let mut quins2 = Vec::new();
        parse_csv_to_quins(csv_data.as_bytes(), &mut profile2, |q| quins2.push(q)).unwrap();

        assert_eq!(quins1, quins2);
        assert_eq!(quins1.len(), 4);
        for q in &quins1 {
            assert_ne!(q.subject, 0);
            assert_eq!(q.subject >> 63, 0);
            assert!(q.verify_ecc_parity());
        }
        assert_eq!(quins1[0].subject, quins1[1].subject);
        assert_eq!(quins1[2].subject, quins1[3].subject);
        assert_ne!(quins1[0].subject, quins1[2].subject);
    }

    #[test]
    fn malformed_csv_numbers_preserve_raw_lexical() {
        let csv_data = "int_col,float_col\nN/A,bad_float\n-9999_err,NaN\n100,42.5\n";
        let mut profile = CsvMappingProfile::new(2002, vec![
            CsvColumnMapping { source_key: "int_col".into(), column_index: None, predicate_hash: 100, datatype: CsvDatatype::Integer },
            CsvColumnMapping { source_key: "float_col".into(), column_index: None, predicate_hash: 200, datatype: CsvDatatype::Float },
        ]);
        let mut quins = Vec::new();
        parse_csv_to_quins(csv_data.as_bytes(), &mut profile, |q| quins.push(q)).unwrap();

        let na_quin = quins.iter().find(|q| q.predicate == 100).unwrap();
        assert_eq!(na_quin.metadata & INVALID_VALUE_METADATA_FLAG, INVALID_VALUE_METADATA_FLAG);
        assert_eq!(na_quin.object, hash_token("N/A") & 0x0FFF_FFFF_FFFF_FFFF);

        let bad_float = quins.iter().find(|q| q.predicate == 200).unwrap();
        assert_eq!(bad_float.metadata & INVALID_VALUE_METADATA_FLAG, INVALID_VALUE_METADATA_FLAG);
        assert_eq!(bad_float.object, hash_token("bad_float") & 0x0FFF_FFFF_FFFF_FFFF);

        let err_quin = quins.iter().filter(|q| q.predicate == 100).nth(1).unwrap();
        assert_eq!(err_quin.metadata & INVALID_VALUE_METADATA_FLAG, INVALID_VALUE_METADATA_FLAG);
        assert_eq!(err_quin.object, hash_token("-9999_err") & 0x0FFF_FFFF_FFFF_FFFF);

        let valid_int = quins.iter().filter(|q| q.predicate == 100).nth(2).unwrap();
        assert_eq!(valid_int.metadata & INVALID_VALUE_METADATA_FLAG, 0);
        assert_eq!(valid_int.object, 100 | (0b001 << 60));

        let valid_float = quins.iter().filter(|q| q.predicate == 200).nth(2).unwrap();
        assert_eq!(valid_float.metadata & INVALID_VALUE_METADATA_FLAG, 0);

        let raw_lex: Vec<_> = quins.iter().filter(|q| q.predicate == PRED_RAW_INVALID_LEXICAL).collect();
        assert_eq!(raw_lex.len(), 4);
        for q in &quins { assert!(q.verify_ecc_parity()); }
    }

    #[test]
    fn csv_provenance_context_and_sensitivity_packing() {
        let csv_data = "score\n50\n";
        let mut profile = CsvMappingProfile::new(9999, vec![
            CsvColumnMapping { source_key: "score".into(), column_index: None, predicate_hash: 55, datatype: CsvDatatype::Integer },
        ]);
        let source_ctx = 0x00A1_B2C3_D4E5_F600;
        for &sens in &[NQuin::SENSITIVITY_PUBLIC, NQuin::SENSITIVITY_RESTRICTED, NQuin::SENSITIVITY_CLASSIFIED] {
            let mut quins = Vec::new();
            parse_csv_to_quins_with_provenance(csv_data.as_bytes(), &mut profile, source_ctx, sens, |q| quins.push(q)).unwrap();
            assert_eq!(quins.len(), 1);
            let q = quins[0];
            assert_eq!(q.get_sensitivity_byte(), sens);
            assert_eq!(q.context >> 56, sens as u64);
            assert_eq!(q.context & 0x00FF_FFFF_FFFF_FFFF, source_ctx & 0x00FF_FFFF_FFFF_FFFF);
            assert!(q.verify_ecc_parity());
        }
    }

    #[test]
    fn missing_namespace_or_context_rejected_in_strict_provenance() {
        let csv_data = "score\n50\n";
        let mut profile = CsvMappingProfile::new(0, vec![
            CsvColumnMapping { source_key: "score".into(), column_index: None, predicate_hash: 55, datatype: CsvDatatype::Integer },
        ]);
        let res1 = parse_csv_to_quins_with_provenance(csv_data.as_bytes(), &mut profile, 0x1234, NQuin::SENSITIVITY_PUBLIC, |_| {});
        assert!(res1.is_err());
        assert_eq!(res1.unwrap_err(), "Strict provenance requires non-zero source_context_hash and base_class_hash");

        profile.base_class_hash = 1234;
        let res2 = parse_csv_to_quins_with_provenance(csv_data.as_bytes(), &mut profile, 0, NQuin::SENSITIVITY_PUBLIC, |_| {});
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err(), "Strict provenance requires non-zero source_context_hash and base_class_hash");
    }

    #[test]
    fn hot_quin_packing_is_zero_heap() {
        assert_zero_alloc("csv_hot_quin_packing", || {
            let mut sink = [NQuin::default(); 4];
            let mut count = 0;
            let subject = derive_deterministic_subject(0x1234, 1, 0xABCD);
            emit_quin(subject, 42, 100 | (0b001 << 60), 0x5678, 0, &mut |q| { sink[count] = q; count += 1; });
            emit_invalid(subject, 43, "N/A", 0x5678, &mut |q| { sink[count] = q; count += 1; });
            assert_eq!(count, 3);
            assert!(sink[0].verify_ecc_parity());
            assert!(sink[1].verify_ecc_parity());
            assert!(sink[2].verify_ecc_parity());
        });
    }
}
