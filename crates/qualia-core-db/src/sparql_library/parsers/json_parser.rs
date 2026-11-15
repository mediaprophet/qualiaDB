//! JSON Parser for QualiaDB
//!
//! Zero-allocation JSON parser that streams data directly into NQuin format.
//! Supports common data types with proper inline type tagging, deterministic
//! subject identifiers (DOM-AX-7), provenance context/sensitivity packing,
//! and raw invalid lexical retention.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde_json::{Deserializer, Value};
use std::io::Read;

use crate::mini_parser::hash_token;
use crate::NQuin;

pub const INVALID_VALUE_METADATA_FLAG: u64 = 1 << 31;
pub const PRED_RAW_INVALID_LEXICAL: u64 = crate::q_hash("qualia:rawInvalidLexical");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonDatatype {
    Integer,
    Float,
    DateTime,
    StringRef,
}

#[derive(Debug, Clone)]
pub struct JsonFieldMapping {
    pub source_key: String,
    pub predicate_hash: u64,
    pub datatype: JsonDatatype,
}

#[derive(Debug, Clone)]
pub struct JsonMappingProfile {
    pub base_class_hash: u64,
    pub fields: Vec<JsonFieldMapping>,
}

impl JsonMappingProfile {
    pub fn new(base_class_hash: u64, fields: Vec<JsonFieldMapping>) -> Self {
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
fn hash_json_object(obj: &serde_json::Map<String, Value>) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for (k, v) in obj {
        for &b in k.as_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        match v {
            Value::String(s) => { for &b in s.as_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); } }
            Value::Number(n) => {
                if let Some(u) = n.as_u64() { for &b in &u.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); } }
                else if let Some(i) = n.as_i64() { for &b in &i.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); } }
                else if let Some(f) = n.as_f64() { for &b in &f.to_bits().to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); } }
            }
            Value::Bool(b) => { hash ^= if *b { 1 } else { 0 }; hash = hash.wrapping_mul(0x100000001b3); }
            Value::Null => { hash ^= 0xFF; hash = hash.wrapping_mul(0x100000001b3); }
            _ => {}
        }
        hash ^= 0xAA; hash = hash.wrapping_mul(0x100000001b3);
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

#[inline]
fn val_as_str<'a>(val: &'a Value) -> &'a str {
    match val {
        Value::String(s) => s.as_str(),
        Value::Null => "null",
        Value::Bool(true) => "true",
        Value::Bool(false) => "false",
        _ => "",
    }
}

/// Parse JSON data from a reader and stream Quins (backward-compatible signature).
pub fn parse_json_to_quins<R: Read>(
    reader: R,
    profile: &JsonMappingProfile,
    on_quin: impl FnMut(NQuin),
) -> Result<(), String> {
    let source_context_hash = profile.base_class_hash & 0x00FF_FFFF_FFFF_FFFF;
    let sensitivity_class = NQuin::SENSITIVITY_PUBLIC;
    parse_json_internal(reader, profile, source_context_hash, sensitivity_class, on_quin)
}

/// Parse JSON data with explicit provenance. Fails closed if source_context_hash or base_class_hash is 0.
pub fn parse_json_to_quins_with_provenance<R: Read>(
    reader: R,
    profile: &JsonMappingProfile,
    source_context_hash: u64,
    sensitivity_class: u8,
    on_quin: impl FnMut(NQuin),
) -> Result<(), String> {
    if (source_context_hash & 0x00FF_FFFF_FFFF_FFFF) == 0 || profile.base_class_hash == 0 {
        return Err("Strict provenance requires non-zero source_context_hash and base_class_hash".into());
    }
    parse_json_internal(reader, profile, source_context_hash, sensitivity_class, on_quin)
}

fn parse_json_internal<R: Read>(
    reader: R,
    profile: &JsonMappingProfile,
    source_context_hash: u64,
    sensitivity_class: u8,
    mut on_quin: impl FnMut(NQuin),
) -> Result<(), String> {
    let stream = Deserializer::from_reader(reader).into_iter::<Value>();
    let context = ((sensitivity_class as u64) << 56) | (source_context_hash & 0x00FF_FFFF_FFFF_FFFF);
    let mut record_index: u64 = 0;

    let mut emit_object = |obj: &serde_json::Map<String, Value>| {
        record_index += 1;
        let record_key = if let Some(id_val) = obj.get("id").or_else(|| obj.get("_id")).or_else(|| obj.get("key")) {
            match id_val {
                Value::String(s) => hash_token(s),
                Value::Number(n) => n.as_u64().unwrap_or(record_index),
                _ => record_index,
            }
        } else {
            record_index
        };
        let raw_record_hash = hash_json_object(obj);
        let subject_hash = derive_deterministic_subject(profile.base_class_hash, record_key, raw_record_hash);

        for field in &profile.fields {
            if let Some(val) = obj.get(&field.source_key) {
                match field.datatype {
                    JsonDatatype::Integer => {
                        let parsed = match val {
                            Value::Number(n) => n.as_u64().or_else(|| n.as_i64().map(|i| i as u64)),
                            Value::String(s) => {
                                let t = s.trim();
                                if !t.is_empty() { t.parse::<i64>().map(|i| i as u64).ok().or_else(|| t.parse::<u64>().ok()) } else { None }
                            }
                            _ => None,
                        };
                        match parsed {
                            Some(v) => emit_quin(subject_hash, field.predicate_hash, v | (0b001 << 60), context, 0, &mut on_quin),
                            None => emit_invalid(subject_hash, field.predicate_hash, val_as_str(val), context, &mut on_quin),
                        }
                    }
                    JsonDatatype::Float => {
                        let parsed = match val {
                            Value::Number(n) => n.as_f64().map(|f| f as f32).filter(|f| f.is_finite()),
                            Value::String(s) => {
                                let t = s.trim();
                                if !t.is_empty() && !t.eq_ignore_ascii_case("nan") { t.parse::<f32>().ok().filter(|f| f.is_finite()) } else { None }
                            }
                            _ => None,
                        };
                        match parsed {
                            Some(f) => emit_quin(subject_hash, field.predicate_hash, (0b010u64 << 60) | (f.to_bits() as u64), context, 0, &mut on_quin),
                            None => emit_invalid(subject_hash, field.predicate_hash, val_as_str(val), context, &mut on_quin),
                        }
                    }
                    JsonDatatype::StringRef => {
                        if let Some(s) = val.as_str() {
                            let hash = hash_token(s) & 0x0FFF_FFFF_FFFF_FFFF;
                            emit_quin(subject_hash, field.predicate_hash, hash, context, 0, &mut on_quin);
                        }
                    }
                    JsonDatatype::DateTime => {
                        let s_opt = val.as_str();
                        match s_opt.and_then(parse_datetime_millis) {
                            Some(ms) => emit_quin(subject_hash, field.predicate_hash, (0b011u64 << 60) | ms, context, 0, &mut on_quin),
                            None => emit_invalid(subject_hash, field.predicate_hash, s_opt.unwrap_or(""), context, &mut on_quin),
                        }
                    }
                }
            }
        }
    };

    for value in stream {
        match value {
            Ok(Value::Object(obj)) => emit_object(&obj),
            Ok(Value::Array(entries)) => {
                for entry in entries {
                    if let Value::Object(obj) = entry { emit_object(&obj); }
                }
            }
            _ => {}
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
    fn parses_each_object_in_a_top_level_array() {
        let profile = JsonMappingProfile::new(0, vec![
            JsonFieldMapping { source_key: "name".into(), predicate_hash: 42, datatype: JsonDatatype::StringRef },
        ]);
        let mut quins = Vec::new();
        parse_json_to_quins(br#"[{"name":"Alice"},{"name":"Bob"}]"#.as_slice(), &profile, |q| quins.push(q)).unwrap();
        assert_eq!(quins.len(), 2);
        assert!(quins.iter().all(|quin| quin.predicate == 42));
    }

    #[test]
    fn deterministic_json_subjects_across_ingest() {
        let json_data = br#"[{"id":"rec_001","name":"SensorA","val":100},{"id":"rec_002","name":"SensorB","val":200}]"#;
        let profile = JsonMappingProfile::new(4004, vec![
            JsonFieldMapping { source_key: "name".into(), predicate_hash: 10, datatype: JsonDatatype::StringRef },
            JsonFieldMapping { source_key: "val".into(), predicate_hash: 20, datatype: JsonDatatype::Integer },
        ]);
        let mut quins1 = Vec::new();
        parse_json_to_quins(json_data.as_slice(), &profile, |q| quins1.push(q)).unwrap();
        let mut quins2 = Vec::new();
        parse_json_to_quins(json_data.as_slice(), &profile, |q| quins2.push(q)).unwrap();

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
    fn malformed_json_numbers_preserve_raw_lexical() {
        let json_data = br#"[
            {"int_val": "N/A", "flt_val": "bad_float"},
            {"int_val": "-9999_err", "flt_val": "NaN"},
            {"int_val": 42, "flt_val": 3.14}
        ]"#;
        let profile = JsonMappingProfile::new(5005, vec![
            JsonFieldMapping { source_key: "int_val".into(), predicate_hash: 100, datatype: JsonDatatype::Integer },
            JsonFieldMapping { source_key: "flt_val".into(), predicate_hash: 200, datatype: JsonDatatype::Float },
        ]);
        let mut quins = Vec::new();
        parse_json_to_quins(json_data.as_slice(), &profile, |q| quins.push(q)).unwrap();

        let na_quin = quins.iter().find(|q| q.predicate == 100).unwrap();
        assert_eq!(na_quin.metadata & INVALID_VALUE_METADATA_FLAG, INVALID_VALUE_METADATA_FLAG);
        assert_eq!(na_quin.object, hash_token("N/A") & 0x0FFF_FFFF_FFFF_FFFF);

        let bad_flt = quins.iter().find(|q| q.predicate == 200).unwrap();
        assert_eq!(bad_flt.metadata & INVALID_VALUE_METADATA_FLAG, INVALID_VALUE_METADATA_FLAG);
        assert_eq!(bad_flt.object, hash_token("bad_float") & 0x0FFF_FFFF_FFFF_FFFF);

        let err_quin = quins.iter().filter(|q| q.predicate == 100).nth(1).unwrap();
        assert_eq!(err_quin.metadata & INVALID_VALUE_METADATA_FLAG, INVALID_VALUE_METADATA_FLAG);
        assert_eq!(err_quin.object, hash_token("-9999_err") & 0x0FFF_FFFF_FFFF_FFFF);

        let valid_int = quins.iter().filter(|q| q.predicate == 100).nth(2).unwrap();
        assert_eq!(valid_int.metadata & INVALID_VALUE_METADATA_FLAG, 0);
        assert_eq!(valid_int.object, 42 | (0b001 << 60));

        let valid_flt = quins.iter().filter(|q| q.predicate == 200).nth(2).unwrap();
        assert_eq!(valid_flt.metadata & INVALID_VALUE_METADATA_FLAG, 0);

        let raw_lex: Vec<_> = quins.iter().filter(|q| q.predicate == PRED_RAW_INVALID_LEXICAL).collect();
        assert_eq!(raw_lex.len(), 4);
        for q in &quins { assert!(q.verify_ecc_parity()); }
    }

    #[test]
    fn json_provenance_context_and_sensitivity_packing() {
        let json_data = br#"[{"level": 99}]"#;
        let profile = JsonMappingProfile::new(8888, vec![
            JsonFieldMapping { source_key: "level".into(), predicate_hash: 77, datatype: JsonDatatype::Integer },
        ]);
        let source_ctx = 0x00FE_DCBA_9876_5432;
        for &sens in &[NQuin::SENSITIVITY_PUBLIC, NQuin::SENSITIVITY_RESTRICTED, NQuin::SENSITIVITY_CLASSIFIED] {
            let mut quins = Vec::new();
            parse_json_to_quins_with_provenance(json_data.as_slice(), &profile, source_ctx, sens, |q| quins.push(q)).unwrap();
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
        let json_data = br#"[{"level": 99}]"#;
        let mut profile = JsonMappingProfile::new(0, vec![
            JsonFieldMapping { source_key: "level".into(), predicate_hash: 77, datatype: JsonDatatype::Integer },
        ]);
        let res1 = parse_json_to_quins_with_provenance(json_data.as_slice(), &profile, 0x1234, NQuin::SENSITIVITY_PUBLIC, |_| {});
        assert!(res1.is_err());
        assert_eq!(res1.unwrap_err(), "Strict provenance requires non-zero source_context_hash and base_class_hash");

        profile.base_class_hash = 1234;
        let res2 = parse_json_to_quins_with_provenance(json_data.as_slice(), &profile, 0, NQuin::SENSITIVITY_PUBLIC, |_| {});
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err(), "Strict provenance requires non-zero source_context_hash and base_class_hash");
    }

    #[test]
    fn hot_quin_packing_is_zero_heap() {
        assert_zero_alloc("json_hot_quin_packing", || {
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
