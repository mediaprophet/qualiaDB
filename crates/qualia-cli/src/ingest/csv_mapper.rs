use atoi::atoi;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use csv::ReaderBuilder;
use std::fs::File;
use qualia_core_db::{mini_parser::hash_token, NQuin};

pub fn stream_csv_to_quins(csv_path: &str, output_path: &str, profile: &mut super::mapper::MappingProfile) {
    let mut writer = super::writer::SuperBlockWriter::new(std::path::Path::new(output_path)).expect("Failed to create SuperBlockWriter");
    let file = File::open(csv_path).expect("Failed to open CSV");
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);
    
    // 1. Resolve header indices
    let headers = rdr.byte_headers().expect("Failed to read headers").clone();
    for field in profile.fields.iter_mut() {
        field.column_index = headers.iter().position(|h| h == field.source_key.as_bytes());
    }

    let mut record = csv::ByteRecord::new();
    
    // 2. The Zero-Allocation Stream Loop
    while rdr.read_byte_record(&mut record).expect("CSV read error") {
        let subject_hash: u64 = rand::random(); // Ephemeral Subject ID for the row

        for field in &profile.fields {
            if let Some(idx) = field.column_index {
                if let Some(raw_bytes) = record.get(idx) {
                    // Pack into Quin without String allocation
                    match field.datatype {
                        super::mapper::TargetDatatype::Integer => {
                            let val: u64 = atoi::<u64>(raw_bytes).unwrap_or(0);
                            let quin = NQuin {
                                subject: subject_hash,
                                predicate: field.predicate_hash,
                                object: val | (0b001 << 60), // INLINE_TAG_INTEGER
                                context: 0,
                                metadata: 0,
                                parity: 0,
                            };
                            writer.push(quin).expect("Failed to write to SuperBlock");
                        },
                        super::mapper::TargetDatatype::Float => {
                            // 1. Zero-Allocation String View (Validate UTF-8 without heap copy)
                            let str_slice = std::str::from_utf8(raw_bytes).unwrap_or("0.0");
                            
                            // 2. Parse to single-precision float
                            let float_val: f32 = str_slice.parse::<f32>().unwrap_or(0.0);
                            
                            // 3. Extract pure IEEE 754 bit pattern (u32)
                            let float_bits: u32 = float_val.to_bits();
                            
                            // 4. Pack into the 64-bit Object vector
                            // TAG: 0b010 << 60 (which equals 0x2000_0000_0000_0000)
                            let inline_tag: u64 = 0x2000_0000_0000_0000;
                            
                            // The u32 bits naturally sit in the lowest 32 bits of the u64.
                            // The middle 28 bits remain 0 (padding).
                            let packed_object: u64 = inline_tag | (float_bits as u64);

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
                            // Hash the raw bytes directly — no UTF-8 copy to heap.
                            let s = std::str::from_utf8(raw_bytes).unwrap_or("");
                            let quin = NQuin {
                                subject: subject_hash,
                                predicate: field.predicate_hash,
                                object: hash_token(s),
                                context: 0,
                                metadata: 0,
                                parity: 0,
                            };
                            writer.push(quin).expect("Failed to write to SuperBlock");
                        }
                        super::mapper::TargetDatatype::DateTime => {
                            let s = std::str::from_utf8(raw_bytes).unwrap_or("");
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
        }
    }
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
