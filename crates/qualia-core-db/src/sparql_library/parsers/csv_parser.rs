//! CSV Parser for QualiaDB
//!
//! Zero-allocation CSV parser that streams data directly into NQuin format.
//! Supports common data types with proper inline type tagging.

use atoi::atoi;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use csv::ReaderBuilder;
use std::io::Read;

use crate::mini_parser::hash_token;
use crate::NQuin;

/// Data type for CSV field mapping
#[derive(Debug, Clone, Copy)]
pub enum CsvDatatype {
    Integer,
    Float,
    DateTime,
    StringRef,
}

/// Column mapping configuration
#[derive(Debug, Clone)]
pub struct CsvColumnMapping {
    pub source_key: String,
    pub column_index: Option<usize>,
    pub predicate_hash: u64,
    pub datatype: CsvDatatype,
}

/// CSV parsing configuration
#[derive(Debug, Clone)]
pub struct CsvMappingProfile {
    pub base_class_hash: u64,
    pub fields: Vec<CsvColumnMapping>,
}

/// Parse CSV data from a reader and stream Quins
pub fn parse_csv_to_quins<R: Read>(
    reader: R,
    profile: &mut CsvMappingProfile,
    mut on_quin: impl FnMut(NQuin),
) -> Result<(), String> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(reader);
    
    // Resolve header indices
    let headers = rdr.byte_headers().map_err(|e| format!("Failed to read headers: {}", e))?.clone();
    for field in profile.fields.iter_mut() {
        field.column_index = headers.iter().position(|h| h == field.source_key.as_bytes());
    }

    let mut record = csv::ByteRecord::new();
    
    // Zero-allocation stream loop
    while rdr.read_byte_record(&mut record).map_err(|e| format!("CSV read error: {}", e))? {
        let subject_hash: u64 = rand::random(); // Ephemeral Subject ID for the row

        for field in &profile.fields {
            if let Some(idx) = field.column_index {
                if let Some(raw_bytes) = record.get(idx) {
                    // Pack into Quin without String allocation
                    match field.datatype {
                        CsvDatatype::Integer => {
                            let val: u64 = atoi::<u64>(raw_bytes).unwrap_or(0);
                            let quin = NQuin {
                                subject: subject_hash,
                                predicate: field.predicate_hash,
                                object: val | (0b001 << 60), // INLINE_TAG_INTEGER
                                context: 0,
                                metadata: 0,
                                parity: NQuin::calculate_parity(subject_hash, field.predicate_hash, val | (0b001 << 60), 0, 0),
                            };
                            on_quin(quin);
                        },
                        CsvDatatype::Float => {
                            let str_slice = std::str::from_utf8(raw_bytes).unwrap_or("0.0");
                            let float_val: f32 = str_slice.parse::<f32>().unwrap_or(0.0);
                            let float_bits: u32 = float_val.to_bits();
                            let inline_tag: u64 = 0b010 << 60;
                            let packed_object: u64 = inline_tag | (float_bits as u64);

                            let quin = NQuin {
                                subject: subject_hash,
                                predicate: field.predicate_hash,
                                object: packed_object,
                                context: 0,
                                metadata: 0,
                                parity: NQuin::calculate_parity(subject_hash, field.predicate_hash, packed_object, 0, 0),
                            };
                            on_quin(quin);
                        },
                        CsvDatatype::StringRef => {
                            let s = std::str::from_utf8(raw_bytes).unwrap_or("");
                            let quin = NQuin {
                                subject: subject_hash,
                                predicate: field.predicate_hash,
                                object: hash_token(s),
                                context: 0,
                                metadata: 0,
                                parity: NQuin::calculate_parity(subject_hash, field.predicate_hash, hash_token(s), 0, 0),
                            };
                            on_quin(quin);
                        }
                        CsvDatatype::DateTime => {
                            let s = std::str::from_utf8(raw_bytes).unwrap_or("");
                            let millis: u64 = parse_datetime_millis(s).unwrap_or(0);
                            let quin = NQuin {
                                subject: subject_hash,
                                predicate: field.predicate_hash,
                                object: (0b011u64 << 60) | millis,
                                context: 0,
                                metadata: 0,
                                parity: NQuin::calculate_parity(subject_hash, field.predicate_hash, (0b011u64 << 60) | millis, 0, 0),
                            };
                            on_quin(quin);
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Try parsing common datetime string formats into Unix milliseconds.
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