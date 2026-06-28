//! Full-parameter MCP implementations for parse/serialize data-format tools.
//!
//! Cold path: JSON args via serde. Hot path: `parse_*_to_quins` / `serialize_*` streaming
//! callbacks write into a fixed `[NQuin; MAX_MCP_FORMAT_QUINS]` buffer — no per-row heap.

use super::mcp_tool_impls::{parse_quin_slice, parse_tool_args};
use super::McpSystemError;
use crate::NQuin;
use serde_json::{json, Value};

/// Upper bound on quins collected per parse/serialize MCP call.
pub const MAX_MCP_FORMAT_QUINS: usize = 8_192;

fn json_str<'a>(v: &'a Value, key: &str, default: &'a str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn json_u64(v: &Value, key: &str, default: u64) -> u64 {
    v.get(key)
        .and_then(|x| x.as_u64().or_else(|| x.as_i64().map(|n| n as u64)))
        .unwrap_or(default)
}

fn json_bool(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn quin_to_json(q: &NQuin) -> Value {
    json!({
        "subject": q.subject,
        "predicate": q.predicate,
        "object": q.object,
        "context": q.context,
        "metadata": q.metadata,
        "parity": q.parity,
    })
}

fn apply_context(q: &mut NQuin, context_hash: u64) {
    if context_hash != 0 {
        q.context = context_hash;
    }
    if q.parity == 0 {
        q.parity = NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata);
    }
}

fn resolve_predicate_hash(field: &Value) -> Result<u64, McpSystemError> {
    if let Some(h) = field.get("predicate_hash").and_then(|x| x.as_u64()) {
        return Ok(h);
    }
    if let Some(iri) = field.get("predicate").and_then(Value::as_str) {
        if !iri.is_empty() {
            return Ok(crate::q_hash(iri));
        }
    }
    Err(McpSystemError::InvalidParameters)
}

fn parse_csv_datatype(s: &str) -> crate::sparql_library::parsers::csv_parser::CsvDatatype {
    use crate::sparql_library::parsers::csv_parser::CsvDatatype;
    match s {
        "integer" => CsvDatatype::Integer,
        "float" => CsvDatatype::Float,
        "datetime" => CsvDatatype::DateTime,
        _ => CsvDatatype::StringRef,
    }
}

fn parse_json_datatype(s: &str) -> crate::sparql_library::parsers::json_parser::JsonDatatype {
    use crate::sparql_library::parsers::json_parser::JsonDatatype;
    match s {
        "integer" => JsonDatatype::Integer,
        "float" => JsonDatatype::Float,
        "datetime" => JsonDatatype::DateTime,
        _ => JsonDatatype::StringRef,
    }
}

fn parse_csv_profile(
    v: &Value,
) -> Result<crate::sparql_library::parsers::csv_parser::CsvMappingProfile, McpSystemError> {
    use crate::sparql_library::parsers::csv_parser::{CsvColumnMapping, CsvMappingProfile};
    let mappings = v
        .get("field_mappings")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    if mappings.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let mut fields = Vec::with_capacity(mappings.len());
    for m in mappings {
        let source_key = m
            .get("source_key")
            .and_then(Value::as_str)
            .ok_or(McpSystemError::InvalidParameters)?
            .to_string();
        fields.push(CsvColumnMapping {
            source_key,
            column_index: None,
            predicate_hash: resolve_predicate_hash(m)?,
            datatype: parse_csv_datatype(
                m.get("datatype")
                    .and_then(Value::as_str)
                    .unwrap_or("string"),
            ),
        });
    }
    Ok(CsvMappingProfile {
        base_class_hash: json_u64(v, "base_class_hash", 0),
        fields,
    })
}

fn parse_json_profile(
    v: &Value,
) -> Result<crate::sparql_library::parsers::json_parser::JsonMappingProfile, McpSystemError> {
    use crate::sparql_library::parsers::json_parser::{JsonFieldMapping, JsonMappingProfile};
    let mappings = v
        .get("field_mappings")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    if mappings.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let mut fields = Vec::with_capacity(mappings.len());
    for m in mappings {
        let source_key = m
            .get("source_key")
            .and_then(Value::as_str)
            .ok_or(McpSystemError::InvalidParameters)?
            .to_string();
        fields.push(JsonFieldMapping {
            source_key,
            predicate_hash: resolve_predicate_hash(m)?,
            datatype: parse_json_datatype(
                m.get("datatype")
                    .and_then(Value::as_str)
                    .unwrap_or("string"),
            ),
        });
    }
    Ok(JsonMappingProfile {
        base_class_hash: json_u64(v, "base_class_hash", 0),
        fields,
    })
}

fn read_input_bytes(v: &Value, inline_key: &str) -> Result<Vec<u8>, McpSystemError> {
    if let Some(inline) = v.get(inline_key).and_then(Value::as_str) {
        return Ok(inline.as_bytes().to_vec());
    }
    if let Some(path) = v.get("file_path").and_then(Value::as_str) {
        if path.is_empty() {
            return Err(McpSystemError::InvalidParameters);
        }
        return std::fs::read(path).map_err(|_| McpSystemError::InvalidParameters);
    }
    Err(McpSystemError::InvalidParameters)
}

struct QuinCollector {
    buf: [NQuin; MAX_MCP_FORMAT_QUINS],
    count: usize,
    truncated: bool,
}

impl QuinCollector {
    fn new() -> Self {
        Self {
            buf: [NQuin::default(); MAX_MCP_FORMAT_QUINS],
            count: 0,
            truncated: false,
        }
    }

    fn push(&mut self, mut q: NQuin, context_hash: u64) {
        apply_context(&mut q, context_hash);
        if self.count < MAX_MCP_FORMAT_QUINS {
            self.buf[self.count] = q;
            self.count += 1;
        } else {
            self.truncated = true;
        }
    }

    fn as_slice(&self) -> &[NQuin] {
        &self.buf[..self.count]
    }
}

fn resolve_input_quins(v: &Value) -> Result<Vec<NQuin>, McpSystemError> {
    if json_bool(v, "use_graph", false) {
        let guard = crate::daemon_graph::graph_read_guard();
        let slice = guard.as_slice();
        let ctx = json_u64(v, "context_hash", 0);
        let max = v
            .get("max_quins")
            .and_then(|x| x.as_u64())
            .unwrap_or(MAX_MCP_FORMAT_QUINS as u64) as usize;
        let mut out = Vec::new();
        for q in slice {
            if ctx != 0 && q.context != ctx {
                continue;
            }
            out.push(*q);
            if out.len() >= max {
                break;
            }
        }
        return Ok(out);
    }
    parse_quin_slice(v, "quins")
}

fn csv_serialize_profile(
    v: &Value,
) -> Result<
    crate::sparql_library::serialisers::csv_serializer::CsvSerializationProfile,
    McpSystemError,
> {
    use crate::sparql_library::serialisers::csv_serializer::{
        CsvDatatype, CsvSerializationProfile,
    };
    let headers = v
        .get("headers")
        .or_else(|| v.get("field_names"))
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let predicate_hashes = v
        .get("predicate_hashes")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let datatypes = v.get("datatypes").and_then(Value::as_array);
    if headers.len() != predicate_hashes.len() {
        return Err(McpSystemError::InvalidParameters);
    }
    let mut hdrs = Vec::with_capacity(headers.len());
    let mut preds = Vec::with_capacity(headers.len());
    let mut dts = Vec::with_capacity(headers.len());
    for (i, h) in headers.iter().enumerate() {
        hdrs.push(
            h.as_str()
                .ok_or(McpSystemError::InvalidParameters)?
                .to_string(),
        );
        preds.push(
            predicate_hashes[i]
                .as_u64()
                .ok_or(McpSystemError::InvalidParameters)?,
        );
        let dt = datatypes
            .and_then(|a| a.get(i))
            .and_then(Value::as_str)
            .unwrap_or("string");
        dts.push(match dt {
            "integer" => CsvDatatype::Integer,
            "float" => CsvDatatype::Float,
            "datetime" => CsvDatatype::DateTime,
            _ => CsvDatatype::StringRef,
        });
    }
    Ok(CsvSerializationProfile {
        headers: hdrs,
        predicate_hashes: preds,
        datatypes: dts,
    })
}

fn json_serialize_profile(
    v: &Value,
) -> Result<
    crate::sparql_library::serialisers::json_serializer::JsonSerializationProfile,
    McpSystemError,
> {
    use crate::sparql_library::serialisers::json_serializer::{
        JsonDatatype, JsonSerializationProfile,
    };
    let field_names = v
        .get("field_names")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let predicate_hashes = v
        .get("predicate_hashes")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let datatypes = v.get("datatypes").and_then(Value::as_array);
    if field_names.len() != predicate_hashes.len() {
        return Err(McpSystemError::InvalidParameters);
    }
    let mut names = Vec::with_capacity(field_names.len());
    let mut preds = Vec::with_capacity(field_names.len());
    let mut dts = Vec::with_capacity(field_names.len());
    for (i, n) in field_names.iter().enumerate() {
        names.push(
            n.as_str()
                .ok_or(McpSystemError::InvalidParameters)?
                .to_string(),
        );
        preds.push(
            predicate_hashes[i]
                .as_u64()
                .ok_or(McpSystemError::InvalidParameters)?,
        );
        let dt = datatypes
            .and_then(|a| a.get(i))
            .and_then(Value::as_str)
            .unwrap_or("string");
        dts.push(match dt {
            "integer" => JsonDatatype::Integer,
            "float" => JsonDatatype::Float,
            "datetime" => JsonDatatype::DateTime,
            _ => JsonDatatype::StringRef,
        });
    }
    Ok(JsonSerializationProfile {
        field_names: names,
        predicate_hashes: preds,
        datatypes: dts,
    })
}

fn write_or_inline_output(
    v: &Value,
    inline_key: &str,
    bytes: Vec<u8>,
) -> Result<Value, McpSystemError> {
    let output = json_str(v, "output", "inline");
    if output == "file" {
        let path = v
            .get("file_path")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .ok_or(McpSystemError::InvalidParameters)?;
        std::fs::write(path, &bytes).map_err(|_| McpSystemError::InvalidParameters)?;
        Ok(json!({
            "output": "file",
            "filePath": path,
            "byteLength": bytes.len(),
        }))
    } else {
        let text = String::from_utf8(bytes).map_err(|_| McpSystemError::ParseError)?;
        Ok(json!({
            "output": "inline",
            inline_key: text,
            "byteLength": text.len(),
        }))
    }
}

// ── Parse ────────────────────────────────────────────────────────────────────

pub fn parse_csv(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::sparql_library::parsers::csv_parser::parse_csv_to_quins;
    use std::io::Cursor;

    let v = parse_tool_args(args)?;
    let mut profile = parse_csv_profile(&v)?;
    let context_hash = json_u64(&v, "context_hash", 0);
    let data = read_input_bytes(&v, "csv_data")?;

    let mut collector = QuinCollector::new();
    parse_csv_to_quins(Cursor::new(data), &mut profile, |q| {
        collector.push(q, context_hash);
    })
    .map_err(|_| McpSystemError::ParseError)?;

    let ingested = if json_bool(&v, "ingest_to_graph", false) {
        let n = collector.count;
        crate::daemon_graph::extend_with_ontology_quins_slice(collector.as_slice());
        n
    } else {
        0
    };

    let payload = json!({
        "quinCount": collector.count,
        "truncated": collector.truncated,
        "maxQuins": MAX_MCP_FORMAT_QUINS,
        "ingestedToGraph": ingested,
        "quins": collector.as_slice().iter().map(quin_to_json).collect::<Vec<_>>(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub fn parse_json(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::sparql_library::parsers::json_parser::parse_json_to_quins;
    use std::io::Cursor;

    let v = parse_tool_args(args)?;
    let profile = parse_json_profile(&v)?;
    let context_hash = json_u64(&v, "context_hash", 0);
    let data = read_input_bytes(&v, "json_data")?;

    let mut collector = QuinCollector::new();
    parse_json_to_quins(Cursor::new(data), &profile, |q| {
        collector.push(q, context_hash);
    })
    .map_err(|_| McpSystemError::ParseError)?;

    let ingested = if json_bool(&v, "ingest_to_graph", false) {
        let n = collector.count;
        crate::daemon_graph::extend_with_ontology_quins_slice(collector.as_slice());
        n
    } else {
        0
    };

    let payload = json!({
        "quinCount": collector.count,
        "truncated": collector.truncated,
        "maxQuins": MAX_MCP_FORMAT_QUINS,
        "ingestedToGraph": ingested,
        "quins": collector.as_slice().iter().map(quin_to_json).collect::<Vec<_>>(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

// ── Serialize ────────────────────────────────────────────────────────────────

pub fn serialize_csv(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::sparql_library::serialisers::csv_serializer::serialize_quins_to_csv;

    let v = parse_tool_args(args)?;
    let quins = resolve_input_quins(&v)?;
    if quins.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let profile = csv_serialize_profile(&v)?;

    let mut out = Vec::new();
    serialize_quins_to_csv(&mut out, &quins, &profile).map_err(|_| McpSystemError::ParseError)?;

    let result = write_or_inline_output(&v, "csv_data", out)?;
    let payload = json!({
        "quinCount": quins.len(),
        "result": result,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub fn serialize_json(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::sparql_library::serialisers::json_serializer::serialize_quins_to_json;

    let v = parse_tool_args(args)?;
    let quins = resolve_input_quins(&v)?;
    if quins.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let profile = json_serialize_profile(&v)?;

    let mut out = Vec::new();
    serialize_quins_to_json(&mut out, &quins, &profile).map_err(|_| McpSystemError::ParseError)?;

    let result = write_or_inline_output(&v, "json_data", out)?;
    let payload = json!({
        "quinCount": quins.len(),
        "result": result,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub fn parse_rdf(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::sparql_library::rdf_formats::{
        parse_rdf as dispatch_parse, QuinCollector, RdfFormat,
    };
    use std::io::Cursor;

    let v = parse_tool_args(args)?;
    let format_str = json_str(&v, "format", "nt");
    let format = RdfFormat::from_str(format_str).ok_or(McpSystemError::InvalidParameters)?;
    let context_hash = json_u64(&v, "context_hash", 0);
    let data = read_input_bytes(&v, "rdf_data")?;

    let mut collector = QuinCollector::new();
    let count = dispatch_parse(format, Cursor::new(data), context_hash, &mut collector).map_err(
        |e| match e {
            crate::sparql_library::rdf_formats::RdfParseError::BufferFull => {
                McpSystemError::ParseError
            }
            _ => McpSystemError::InvalidParameters,
        },
    )?;

    let ingested = if json_bool(&v, "ingest_to_graph", false) {
        let n = collector.count;
        crate::daemon_graph::extend_with_ontology_quins_slice(collector.as_slice());
        n
    } else {
        0
    };

    let payload = json!({
        "format": format.as_str(),
        "quinCount": count,
        "truncated": collector.truncated,
        "maxQuins": crate::sparql_library::rdf_formats::MAX_RDF_QUINS,
        "ingestedToGraph": ingested,
        "quins": collector.as_slice().iter().map(quin_to_json).collect::<Vec<_>>(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub fn serialize_rdf(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::sparql_library::rdf_formats::{
        serialize_rdf as dispatch_serialize, RdfFormat, RdfStarMode,
    };

    let v = parse_tool_args(args)?;
    let quins = resolve_input_quins(&v)?;
    if quins.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let format_str = json_str(&v, "format", "nt");
    let format = RdfFormat::from_str(format_str).ok_or(McpSystemError::InvalidParameters)?;
    let mode = if json_bool(&v, "rdf_star", true) && !json_bool(&v, "plain", false) {
        RdfStarMode::Star
    } else if json_bool(&v, "star", false) {
        RdfStarMode::Star
    } else {
        RdfStarMode::Plain
    };

    let mut out = Vec::new();
    dispatch_serialize(format, mode, &quins, &mut out).map_err(|_| McpSystemError::ParseError)?;

    let result = write_or_inline_output(&v, "rdf_data", out)?;
    let payload = json!({
        "quinCount": quins.len(),
        "format": format.as_str(),
        "rdfStar": matches!(mode, RdfStarMode::Star),
        "result": result,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn parse_csv_inline_produces_quins() {
        let args = serde_json::to_string(&json!({
            "csv_data": "name,score\nalice,42\n",
            "field_mappings": [
                {"source_key": "name", "predicate": "schema:name", "datatype": "string"},
                {"source_key": "score", "predicate": "schema:score", "datatype": "integer"}
            ]
        }))
        .unwrap();
        let out = parse_csv(args.as_bytes()).expect("parse");
        let v: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["quinCount"], 2);
        assert_eq!(v["truncated"], false);
    }

    #[test]
    fn serialize_csv_round_trip() {
        let name_pred = q_hash("schema:name");
        let score_pred = q_hash("schema:score");
        let subj = 0xABCD_u64;
        let quins = vec![
            NQuin {
                subject: subj,
                predicate: name_pred,
                object: q_hash("alice"),
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: subj,
                predicate: score_pred,
                object: 42 | (0b001 << 60),
                context: 0,
                metadata: 0,
                parity: 0,
            },
        ];
        let args = serde_json::to_string(&json!({
            "quins": quins.iter().map(quin_to_json).collect::<Vec<_>>(),
            "headers": ["name", "score"],
            "predicate_hashes": [name_pred, score_pred],
            "datatypes": ["string", "integer"]
        }))
        .unwrap();
        let out = serialize_csv(args.as_bytes()).expect("serialize");
        let v: Value = serde_json::from_str(&out).expect("json");
        let csv = v["result"]["csv_data"].as_str().expect("csv");
        assert!(csv.contains("name,score"));
        assert!(csv.contains("42"));
    }

    #[test]
    fn parse_rdf_ntriples_inline() {
        let args = serde_json::to_string(&json!({
            "format": "nt",
            "rdf_data": "<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .\n"
        }))
        .unwrap();
        let out = parse_rdf(args.as_bytes()).expect("parse");
        let v: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["quinCount"], 1);
        assert_eq!(v["truncated"], false);
        assert_eq!(v["format"], "ntriples");
    }

    #[test]
    fn serialize_rdf_ntriples_inline() {
        let subj = q_hash("ex:Alice");
        let pred = q_hash("ex:age");
        let obj = 30u64 | (0b001 << 60);
        let args = serde_json::to_string(&json!({
            "quins": [{"subject": subj, "predicate": pred, "object": obj, "context": 0, "metadata": 0, "parity": 0}],
            "format": "nt"
        }))
        .unwrap();
        let out = serialize_rdf(args.as_bytes()).expect("rdf");
        let v: Value = serde_json::from_str(&out).expect("json");
        let rdf = v["result"]["rdf_data"].as_str().expect("rdf");
        assert!(rdf.contains('.'));
    }
}
