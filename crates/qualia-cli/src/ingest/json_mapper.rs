use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde_json::{Deserializer, Value};
use std::fs::File;
use std::io::BufReader;
use qualia_core_db::NQuin;

pub fn stream_json_to_quins(json_path: &str, output_path: &str, profile: &super::mapper::MappingProfile) {
    let mut writer = super::writer::SuperBlockWriter::new(std::path::Path::new(output_path)).expect("Failed to create SuperBlockWriter");
    let file = File::open(json_path).expect("Failed to open JSON file");
    let reader = BufReader::new(file);
    
    // Create an iterator that parses exactly one top-level object at a time
    let stream = Deserializer::from_reader(reader).into_iter::<Value>();

    for value in stream {
        // We only care about JSON Objects mapping to Quins. Skip arrays/primitives at the root.
        let obj = match value {
            Ok(Value::Object(map)) => map,
            _ => continue, 
        };

        let subject_hash: u64 = rand::random(); // Ephemeral Subject ID for this entity

        for field in &profile.fields {
            if let Some(val) = obj.get(&field.source_key) {
                match field.datatype {
                    super::mapper::TargetDatatype::Integer => {
                        let parsed_int: u64 = val.as_u64().unwrap_or(0);
                        let quin = NQuin {
                            subject: subject_hash,
                            predicate: field.predicate_hash,
                            object: parsed_int | (0b001 << 60), // INLINE_TAG_INTEGER
                            context: 0,
                            metadata: 0,
                            parity: 0,
                        };
                        writer.push(quin).expect("Failed to write to SuperBlock");
                    },
                    super::mapper::TargetDatatype::Float => {
                        // Cast JSON f64 down to f32 for inline packing
                        let parsed_float: f32 = val.as_f64().unwrap_or(0.0) as f32;
                        
                        // Extract IEEE 754 bits and pack into 64-bit vector
                        let float_bits: u32 = parsed_float.to_bits();
                        let inline_tag: u64 = 0x2000_0000_0000_0000; // 0b0010 << 60
                        let packed_object = inline_tag | (float_bits as u64);
                        
                        let quin = NQuin {
                            subject: subject_hash,
                            predicate: field.predicate_hash,
                            object: packed_object,
                            context: 0,
                            metadata: 0,
                            parity: 0,
                        };
                        writer.push(quin).expect("Failed to write to SuperBlock");
                    },
                    super::mapper::TargetDatatype::StringRef => {
                        if let Some(s) = val.as_str() {
                            let hash = qualia_core_db::mini_parser::hash_token(s);
                            let quin = NQuin {
                                subject: subject_hash,
                                predicate: field.predicate_hash,
                                object: hash,
                                context: 0,
                                metadata: 0,
                                parity: 0,
                            };
                            writer.push(quin).expect("Failed to write to SuperBlock");
                        }
                    }
                    super::mapper::TargetDatatype::DateTime => {
                        let s = match val.as_str() {
                            Some(s) => s,
                            None => continue,
                        };
                        // Parse RFC3339 / ISO8601 datetime → Unix milliseconds.
                        let millis: u64 = parse_datetime_millis(s).unwrap_or(0);
                        let quin = NQuin {
                            subject: subject_hash,
                            predicate: field.predicate_hash,
                            object: (0b011u64 << 60) | millis,
                            context: 0,
                            metadata: 0,
                            parity: 0,
                        };
                        writer.push(quin).expect("Failed to write to SuperBlock");
                    }
                }
            }
        }
        // `obj` goes out of scope here. Memory is instantly freed for the next record.
    }
}

/// Try parsing common datetime string formats into Unix milliseconds.
/// Returns `None` when the string cannot be parsed.
fn parse_datetime_millis(s: &str) -> Option<u64> {
    // RFC3339 / ISO8601 with timezone offset
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis() as u64);
    }
    // Naive datetime (no timezone) → treat as UTC
    for fmt in &["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%SZ"] {
        if let Ok(nd) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(Utc.from_utc_datetime(&nd).timestamp_millis() as u64);
        }
    }
    // Date only
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let nd = d.and_hms_opt(0, 0, 0)?;
        return Some(Utc.from_utc_datetime(&nd).timestamp_millis() as u64);
    }
    None
}
