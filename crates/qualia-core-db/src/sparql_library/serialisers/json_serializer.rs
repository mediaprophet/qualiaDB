//! JSON Serializer for QualiaDB
//!
//! Serializes NQuin data to JSON format with proper type handling.

use std::io::Write;

use crate::NQuin;

/// JSON serialization configuration
#[derive(Debug, Clone)]
pub struct JsonSerializationProfile {
    pub field_names: Vec<String>,
    pub predicate_hashes: Vec<u64>,
    pub datatypes: Vec<JsonDatatype>,
}

/// Data type for JSON serialization
#[derive(Debug, Clone, Copy)]
pub enum JsonDatatype {
    Integer,
    Float,
    DateTime,
    StringRef,
}

/// Serialize Quins to JSON format
pub fn serialize_quins_to_json<W: Write>(
    writer: &mut W,
    quins: &[NQuin],
    profile: &JsonSerializationProfile,
) -> Result<(), String> {
    writeln!(writer, "[").map_err(|e| format!("Failed to write JSON array start: {}", e))?;

    // Group quins by subject (assuming each subject represents an object)
    let mut subjects: std::collections::HashMap<u64, Vec<&NQuin>> =
        std::collections::HashMap::new();
    for quin in quins {
        subjects
            .entry(quin.subject)
            .or_insert_with(Vec::new)
            .push(quin);
    }

    let mut first = true;
    for (_subject, row_quins) in subjects {
        if !first {
            writeln!(writer, ",").map_err(|e| format!("Failed to write JSON comma: {}", e))?;
        }
        first = false;

        write!(writer, "  {{").map_err(|e| format!("Failed to write JSON object start: {}", e))?;

        let mut field_first = true;
        for (i, &pred_hash) in profile.predicate_hashes.iter().enumerate() {
            let field_name = &profile.field_names[i];
            let datatype = profile
                .datatypes
                .get(i)
                .copied()
                .unwrap_or(JsonDatatype::StringRef);

            // Find quin with matching predicate
            if let Some(quin) = row_quins.iter().find(|q| q.predicate == pred_hash) {
                if !field_first {
                    write!(writer, ",")
                        .map_err(|e| format!("Failed to write JSON field comma: {}", e))?;
                }
                field_first = false;

                write!(
                    writer,
                    "\"{}\": {}",
                    field_name,
                    format_quin_value(quin, datatype)
                )
                .map_err(|e| format!("Failed to write JSON field: {}", e))?;
            }
        }

        write!(writer, "}}").map_err(|e| format!("Failed to write JSON object end: {}", e))?;
    }

    writeln!(writer, "\n]").map_err(|e| format!("Failed to write JSON array end: {}", e))?;

    Ok(())
}

/// Format a Quin value for JSON output
fn format_quin_value(quin: &NQuin, datatype: JsonDatatype) -> String {
    match datatype {
        JsonDatatype::Integer => {
            let val = quin.object & 0x0FFF_FFFF_FFFF_FFFF; // Remove type tag
            val.to_string()
        }
        JsonDatatype::Float => {
            let tag = quin.object & 0xF000_0000_0000_0000;
            if tag == (0b010 << 60) {
                let bits = (quin.object & 0xFFFF_FFFF) as u32;
                let float_val = f32::from_bits(bits);
                float_val.to_string()
            } else {
                "null".to_string()
            }
        }
        JsonDatatype::DateTime => {
            let tag = quin.object & 0xF000_0000_0000_0000;
            if tag == (0b011 << 60) {
                let millis = quin.object & 0x0FFF_FFFF_FFFF_FFFF;
                let dt = chrono::DateTime::from_timestamp(millis as i64 / 1000, 0);
                let dt_str = dt.map(|d| d.to_rfc3339()).unwrap_or_default();
                format!("\"{}\"", dt_str)
            } else {
                "null".to_string()
            }
        }
        JsonDatatype::StringRef => {
            // For strings, we just output the hash (in practice, you'd want lexicon lookup)
            format!("\"{:x}\"", quin.object)
        }
    }
}
