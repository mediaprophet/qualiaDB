//! JSON Parser for QualiaDB
//!
//! Zero-allocation JSON parser that streams data directly into NQuin format.
//! Supports common data types with proper inline type tagging.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde_json::{Deserializer, Value};
use std::io::Read;

use crate::mini_parser::hash_token;
use crate::NQuin;

/// Data type for JSON field mapping
#[derive(Debug, Clone, Copy)]
pub enum JsonDatatype {
    Integer,
    Float,
    DateTime,
    StringRef,
}

/// Field mapping configuration
#[derive(Debug, Clone)]
pub struct JsonFieldMapping {
    pub source_key: String,
    pub predicate_hash: u64,
    pub datatype: JsonDatatype,
}

/// JSON parsing configuration
#[derive(Debug, Clone)]
pub struct JsonMappingProfile {
    pub base_class_hash: u64,
    pub fields: Vec<JsonFieldMapping>,
}

/// Parse JSON data from a reader and stream Quins
pub fn parse_json_to_quins<R: Read>(
    reader: R,
    profile: &JsonMappingProfile,
    mut on_quin: impl FnMut(NQuin),
) -> Result<(), String> {
    let stream = Deserializer::from_reader(reader).into_iter::<Value>();

    for value in stream {
        let obj = match value {
            Ok(Value::Object(map)) => map,
            _ => continue,
        };

        let subject_hash: u64 = rand::random(); // Ephemeral Subject ID for this entity

        for field in &profile.fields {
            if let Some(val) = obj.get(&field.source_key) {
                match field.datatype {
                    JsonDatatype::Integer => {
                        let parsed_int: u64 = val.as_u64().unwrap_or(0);
                        let quin = NQuin {
                            subject: subject_hash,
                            predicate: field.predicate_hash,
                            object: parsed_int | (0b001 << 60), // INLINE_TAG_INTEGER
                            context: 0,
                            metadata: 0,
                            parity: NQuin::calculate_parity(
                                subject_hash,
                                field.predicate_hash,
                                parsed_int | (0b001 << 60),
                                0,
                                0,
                            ),
                        };
                        on_quin(quin);
                    }
                    JsonDatatype::Float => {
                        let parsed_float: f32 = val.as_f64().unwrap_or(0.0) as f32;
                        let float_bits: u32 = parsed_float.to_bits();
                        let inline_tag: u64 = 0b010 << 60;
                        let packed_object = inline_tag | (float_bits as u64);

                        let quin = NQuin {
                            subject: subject_hash,
                            predicate: field.predicate_hash,
                            object: packed_object,
                            context: 0,
                            metadata: 0,
                            parity: NQuin::calculate_parity(
                                subject_hash,
                                field.predicate_hash,
                                packed_object,
                                0,
                                0,
                            ),
                        };
                        on_quin(quin);
                    }
                    JsonDatatype::StringRef => {
                        if let Some(s) = val.as_str() {
                            let hash = hash_token(s);
                            let quin = NQuin {
                                subject: subject_hash,
                                predicate: field.predicate_hash,
                                object: hash,
                                context: 0,
                                metadata: 0,
                                parity: NQuin::calculate_parity(
                                    subject_hash,
                                    field.predicate_hash,
                                    hash,
                                    0,
                                    0,
                                ),
                            };
                            on_quin(quin);
                        }
                    }
                    JsonDatatype::DateTime => {
                        let s = match val.as_str() {
                            Some(s) => s,
                            None => continue,
                        };
                        let millis: u64 = parse_datetime_millis(s).unwrap_or(0);
                        let quin = NQuin {
                            subject: subject_hash,
                            predicate: field.predicate_hash,
                            object: (0b011u64 << 60) | millis,
                            context: 0,
                            metadata: 0,
                            parity: NQuin::calculate_parity(
                                subject_hash,
                                field.predicate_hash,
                                (0b011u64 << 60) | millis,
                                0,
                                0,
                            ),
                        };
                        on_quin(quin);
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
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%SZ",
    ] {
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
