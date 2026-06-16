//! CSV Serializer for QualiaDB
//!
//! Serializes NQuin data to CSV format with proper type handling.

use std::io::Write;

use crate::NQuin;

/// CSV serialization configuration
#[derive(Debug, Clone)]
pub struct CsvSerializationProfile {
    pub headers: Vec<String>,
    pub predicate_hashes: Vec<u64>,
    pub datatypes: Vec<CsvDatatype>,
}

/// Data type for CSV serialization
#[derive(Debug, Clone, Copy)]
pub enum CsvDatatype {
    Integer,
    Float,
    DateTime,
    StringRef,
}

/// Serialize Quins to CSV format
pub fn serialize_quins_to_csv<W: Write>(
    writer: &mut W,
    quins: &[NQuin],
    profile: &CsvSerializationProfile,
) -> Result<(), String> {
    // Write headers
    writeln!(writer, "{}", profile.headers.join(","))
        .map_err(|e| format!("Failed to write CSV headers: {}", e))?;

    // Group quins by subject (assuming each subject represents a row)
    let mut subjects: std::collections::HashMap<u64, Vec<&NQuin>> = std::collections::HashMap::new();
    for quin in quins {
        subjects.entry(quin.subject).or_insert_with(Vec::new).push(quin);
    }

    // Write data rows
    for (_subject, row_quins) in subjects {
        let mut row_values = Vec::with_capacity(profile.headers.len());
        
        for (i, &pred_hash) in profile.predicate_hashes.iter().enumerate() {
            let datatype = profile.datatypes.get(i).copied().unwrap_or(CsvDatatype::StringRef);
            
            // Find quin with matching predicate
            let value = row_quins.iter()
                .find(|q| q.predicate == pred_hash)
                .map(|quin| format_quin_value(quin, datatype))
                .unwrap_or(String::new());
            
            row_values.push(value);
        }
        
        writeln!(writer, "{}", row_values.join(","))
            .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    Ok(())
}

/// Format a Quin value for CSV output
fn format_quin_value(quin: &NQuin, datatype: CsvDatatype) -> String {
    match datatype {
        CsvDatatype::Integer => {
            let val = quin.object & 0x0FFF_FFFF_FFFF_FFFF; // Remove type tag
            val.to_string()
        },
        CsvDatatype::Float => {
            let tag = quin.object & 0xF000_0000_0000_0000;
            if tag == (0b010 << 60) {
                let bits = (quin.object & 0xFFFF_FFFF) as u32;
                let float_val = f32::from_bits(bits);
                float_val.to_string()
            } else {
                String::new()
            }
        },
        CsvDatatype::DateTime => {
            let tag = quin.object & 0xF000_0000_0000_0000;
            if tag == (0b011 << 60) {
                let millis = quin.object & 0x0FFF_FFFF_FFFF_FFFF;
                let dt = chrono::DateTime::from_timestamp(millis as i64 / 1000, 0);
                dt.map(|d| d.to_rfc3339()).unwrap_or_default()
            } else {
                String::new()
            }
        },
        CsvDatatype::StringRef => {
            // For strings, we just output the hash (in practice, you'd want lexicon lookup)
            format!("{:x}", quin.object)
        }
    }
}