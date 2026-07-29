//! WASM-bindgen API — dataio domain (split from wasm_bridge.rs; verbatim, no behaviour change).
//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//!
//! All functions are `#[cfg(target_arch = "wasm32")]` and only compiled into
//! the browser/OPFS build.  Native desktop builds use direct Rust FFI.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── Economics: Monte Carlo VaR ──────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonLdFlatTriple {
    pub s: String,
    pub p: String,
    pub o: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_json_wasm(payload: &str) -> JsValue {
    if let Ok(triples) = serde_json::from_str::<Vec<JsonLdFlatTriple>>(payload) {
        #[derive(Serialize)]
        struct QOut {
            subject: String,
            predicate: String,
            object: String,
        }

        let mut out = Vec::new();
        for t in triples {
            out.push(QOut {
                subject: t.s,
                predicate: t.p,
                object: t.o,
            });
        }
        serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
    } else {
        JsValue::NULL
    }
}

// ─── LWW CRDT ────────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize, Serialize, Clone)]
pub struct QuinJson {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

// --- Data Format: CSV Parser -----------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct CsvParseParams {
    pub csv_data: String,
    pub base_class_hash: u64,
    pub field_mappings: Vec<CsvFieldMapping>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct CsvFieldMapping {
    pub source_key: String,
    pub predicate_hash: u64,
    pub datatype: String, // "integer", "float", "datetime", "string"
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_csv_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::parsers::csv_parser::{
        parse_csv_to_quins, CsvColumnMapping, CsvDatatype, CsvMappingProfile,
    };
    use std::io::Cursor;

    let p: CsvParseParams = serde_wasm_bindgen::from_value(val)?;

    let mut profile = CsvMappingProfile {
        base_class_hash: p.base_class_hash,
        fields: p
            .field_mappings
            .iter()
            .map(|f| CsvColumnMapping {
                source_key: f.source_key.clone(),
                column_index: None,
                predicate_hash: f.predicate_hash,
                datatype: match f.datatype.as_str() {
                    "integer" => CsvDatatype::Integer,
                    "float" => CsvDatatype::Float,
                    "datetime" => CsvDatatype::DateTime,
                    _ => CsvDatatype::StringRef,
                },
            })
            .collect(),
    };

    let mut quins = Vec::new();
    let cursor = Cursor::new(p.csv_data.as_bytes());

    parse_csv_to_quins(cursor, &mut profile, |quin| {
        quins.push(quin);
    })
    .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct ParseResult {
        quin_count: usize,
        quins: Vec<[u64; 6]>, // Serialize NQuin as array of 6 u64
    }

    let quin_arrays: Vec<[u64; 6]> = quins
        .iter()
        .map(|q| {
            [
                q.subject,
                q.predicate,
                q.object,
                q.context,
                q.metadata,
                q.parity,
            ]
        })
        .collect();

    Ok(serde_wasm_bindgen::to_value(&ParseResult {
        quin_count: quins.len(),
        quins: quin_arrays,
    })?)
}

// --- Data Format: JSON Parser ----------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonParseParams {
    pub json_data: String,
    pub base_class_hash: u64,
    pub field_mappings: Vec<JsonFieldMapping>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonFieldMapping {
    pub source_key: String,
    pub predicate_hash: u64,
    pub datatype: String, // "integer", "float", "datetime", "string"
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_json_mapping_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::parsers::json_parser::{
        parse_json_to_quins, JsonDatatype, JsonFieldMapping as CoreJsonFieldMapping,
        JsonMappingProfile,
    };
    use std::io::Cursor;

    let p: JsonParseParams = serde_wasm_bindgen::from_value(val)?;

    let profile = JsonMappingProfile {
        base_class_hash: p.base_class_hash,
        fields: p
            .field_mappings
            .iter()
            .map(|f| CoreJsonFieldMapping {
                source_key: f.source_key.clone(),
                predicate_hash: f.predicate_hash,
                datatype: match f.datatype.as_str() {
                    "integer" => JsonDatatype::Integer,
                    "float" => JsonDatatype::Float,
                    "datetime" => JsonDatatype::DateTime,
                    _ => JsonDatatype::StringRef,
                },
            })
            .collect(),
    };

    let mut quins = Vec::new();
    let cursor = Cursor::new(p.json_data.as_bytes());

    parse_json_to_quins(cursor, &profile, |quin| {
        quins.push(quin);
    })
    .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct ParseResult {
        quin_count: usize,
        quins: Vec<[u64; 6]>,
    }

    let quin_arrays: Vec<[u64; 6]> = quins
        .iter()
        .map(|q| {
            [
                q.subject,
                q.predicate,
                q.object,
                q.context,
                q.metadata,
                q.parity,
            ]
        })
        .collect();

    Ok(serde_wasm_bindgen::to_value(&ParseResult {
        quin_count: quins.len(),
        quins: quin_arrays,
    })?)
}

// --- Data Format: CSV Serializer -------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct CsvSerializeParams {
    pub quins: Vec<[u64; 6]>,
    pub field_names: Vec<String>,
    pub predicate_hashes: Vec<u64>,
    pub datatypes: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn serialize_csv_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::serialisers::csv_serializer::{
        serialize_quins_to_csv, CsvDatatype as CoreCsvDatatype, CsvSerializationProfile,
    };
    use crate::NQuin;

    let p: CsvSerializeParams = serde_wasm_bindgen::from_value(val)?;

    let quins: Vec<NQuin> = p
        .quins
        .iter()
        .map(|arr| NQuin {
            subject: arr[0],
            predicate: arr[1],
            object: arr[2],
            context: arr[3],
            metadata: arr[4],
            parity: arr[5],
        })
        .collect();

    let profile = CsvSerializationProfile {
        headers: p.field_names,
        predicate_hashes: p.predicate_hashes,
        datatypes: p
            .datatypes
            .iter()
            .map(|d| match d.as_str() {
                "integer" => CoreCsvDatatype::Integer,
                "float" => CoreCsvDatatype::Float,
                "datetime" => CoreCsvDatatype::DateTime,
                _ => CoreCsvDatatype::StringRef,
            })
            .collect(),
    };

    let mut csv_output = Vec::new();
    serialize_quins_to_csv(&mut csv_output, &quins, &profile).map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct SerializeResult {
        csv_data: String,
    }

    Ok(serde_wasm_bindgen::to_value(&SerializeResult {
        csv_data: String::from_utf8(csv_output).map_err(|e| JsValue::from_str(&e.to_string()))?,
    })?)
}

// --- Data Format: JSON Serializer ------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonSerializeParams {
    pub quins: Vec<[u64; 6]>,
    pub field_names: Vec<String>,
    pub predicate_hashes: Vec<u64>,
    pub datatypes: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn serialize_json_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::serialisers::json_serializer::{
        serialize_quins_to_json, JsonDatatype as CoreJsonDatatype, JsonSerializationProfile,
    };
    use crate::NQuin;

    let p: JsonSerializeParams = serde_wasm_bindgen::from_value(val)?;

    let quins: Vec<NQuin> = p
        .quins
        .iter()
        .map(|arr| NQuin {
            subject: arr[0],
            predicate: arr[1],
            object: arr[2],
            context: arr[3],
            metadata: arr[4],
            parity: arr[5],
        })
        .collect();

    let profile = JsonSerializationProfile {
        field_names: p.field_names,
        predicate_hashes: p.predicate_hashes,
        datatypes: p
            .datatypes
            .iter()
            .map(|d| match d.as_str() {
                "integer" => CoreJsonDatatype::Integer,
                "float" => CoreJsonDatatype::Float,
                "datetime" => CoreJsonDatatype::DateTime,
                _ => CoreJsonDatatype::StringRef,
            })
            .collect(),
    };

    let mut json_output = Vec::new();
    serialize_quins_to_json(&mut json_output, &quins, &profile)
        .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct SerializeResult {
        json_data: String,
    }

    Ok(serde_wasm_bindgen::to_value(&SerializeResult {
        json_data: String::from_utf8(json_output).map_err(|e| JsValue::from_str(&e.to_string()))?,
    })?)
}
