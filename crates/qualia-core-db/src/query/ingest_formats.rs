//! Dispatch ingest across RDF serialisations. Line formats seek; Turtle/TriG
//! resume via prefix prolog + offset; XML / JSON-LD / gzip stay skip-N.

use std::io::{BufRead, Cursor, Read};

use rio_api::model::{GraphName, Literal, Quad, Subject, Term, Triple};
use rio_api::parser::{QuadsParser, TriplesParser};
use rio_turtle::{NQuadsParser, NTriplesParser, TriGParser, TurtleParser};
use rio_xml::RdfXmlParser;

use crate::query::cbor_compiler::parse_cbor_ld_to_quin;
use crate::query::ingest_job::IngestRdfFormat;
use crate::q_hash;

/// Object-field hashes keep bits 60–63 free for resolver type tags.
pub const OBJECT_IRI_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

pub fn object_iri_hash(iri: &str) -> u64 {
    q_hash(iri) & OBJECT_IRI_MASK
}

/// Raw string triple from any RDF serialisation. `context` is the graph name hash (0 = default).
#[derive(Debug)]
pub struct RawTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub packed_object: Option<u64>,
    pub context: u64,
}

pub fn format_from_path(path_lower: &str) -> IngestRdfFormat {
    crate::query::ingest_job::infer_rdf_format(path_lower)
}

pub fn pack_rio_triple(t: Triple<'_>, context: u64) -> RawTriple {
    let (object, packed_object) = pack_term(t.object);
    RawTriple {
        subject: pack_subject(t.subject),
        predicate: t.predicate.iri.to_string(),
        object,
        packed_object,
        context,
    }
}

fn pack_subject(s: Subject<'_>) -> String {
    match s {
        Subject::NamedNode(n) => n.iri.to_string(),
        Subject::BlankNode(b) => format!("_:{}", b.id),
        Subject::Triple(inner) => inner.to_string(),
    }
}

fn pack_term(term: Term<'_>) -> (String, Option<u64>) {
    match term {
        Term::NamedNode(n) => (n.iri.to_string(), None),
        Term::BlankNode(b) => (format!("_:{}", b.id), None),
        Term::Literal(Literal::Simple { value }) => (value.to_string(), None),
        Term::Literal(Literal::LanguageTaggedString { value, language }) => {
            (format!("{value}@{language}"), None)
        }
        Term::Literal(Literal::Typed { value, datatype }) => {
            (value.to_string(), pack_typed_literal(value, datatype.iri))
        }
        Term::Triple(inner) => (inner.to_string(), None),
    }
}

/// Catalog RDF/XML often ships `xml:base=""`. Fill it from the stem IRI.
pub fn repair_rdfxml_empty_base(src: &str, base: &str) -> String {
    src.replace("xml:base=\"\"", &format!("xml:base=\"{base}\""))
        .replace("xml:base=''", &format!("xml:base='{base}'"))
}

pub fn pack_rio_quad(q: Quad<'_>) -> RawTriple {
    let context = match q.graph_name {
        Some(GraphName::NamedNode(n)) => q_hash(n.iri),
        Some(GraphName::BlankNode(b)) => q_hash(b.id),
        None => 0,
    };
    pack_rio_triple(
        Triple {
            subject: q.subject,
            predicate: q.predicate,
            object: q.object,
        },
        context,
    )
}

fn pack_typed_literal(value: &str, dt: &str) -> Option<u64> {
    if dt == "http://www.w3.org/2001/XMLSchema#integer" {
        if let Ok(num) = value.parse::<i64>() {
            let max_val = (1i64 << 59) - 1;
            let min_val = -(1i64 << 59);
            if num >= min_val && num <= max_val {
                let unsigned = (num as u64) & crate::resolver::INLINE_VALUE_MASK;
                return Some(crate::resolver::INLINE_TAG_INTEGER | unsigned);
            }
        }
    } else if dt == "http://www.w3.org/2001/XMLSchema#decimal" {
        if let Ok(num) = value.parse::<f64>() {
            let scaled = num * 1_000_000.0;
            let max_val = ((1i64 << 59) - 1) as f64;
            let min_val = (-(1i64 << 59)) as f64;
            if scaled >= min_val && scaled <= max_val {
                let num_i64 = scaled.round() as i64;
                let unsigned = (num_i64 as u64) & crate::resolver::INLINE_VALUE_MASK;
                return Some(crate::resolver::INLINE_TAG_DECIMAL | unsigned);
            }
        }
    } else if dt == "http://www.w3.org/2001/XMLSchema#boolean" {
        if value == "true" || value == "1" {
            return Some(crate::resolver::INLINE_TAG_BOOLEAN | 1);
        }
        if value == "false" || value == "0" {
            return Some(crate::resolver::INLINE_TAG_BOOLEAN | 0);
        }
    }
    None
}

pub fn parse_triples_format<R, F>(
    format: IngestRdfFormat,
    reader: R,
    base_iri: Option<oxiri::Iri<String>>,
    on_triple: &mut F,
) -> Result<(), String>
where
    R: BufRead,
    F: FnMut(RawTriple),
{
    match format {
        IngestRdfFormat::NTriples => {
            let mut p = NTriplesParser::new(reader);
            p.parse_all(&mut |t| {
                on_triple(pack_rio_triple(t, 0));
                Ok(()) as Result<(), rio_turtle::TurtleError>
            })
            .map_err(|e| format!("N-Triples: {e}"))
        }
        IngestRdfFormat::Turtle | IngestRdfFormat::Auto => {
            let mut p = TurtleParser::new(reader, base_iri);
            p.parse_all(&mut |t| {
                on_triple(pack_rio_triple(t, 0));
                Ok(()) as Result<(), rio_turtle::TurtleError>
            })
            .map_err(|e| format!("Turtle: {e}"))
        }
        IngestRdfFormat::NQuads => {
            let mut p = NQuadsParser::new(reader);
            p.parse_all(&mut |q| {
                on_triple(pack_rio_quad(q));
                Ok(()) as Result<(), rio_turtle::TurtleError>
            })
            .map_err(|e| format!("N-Quads: {e}"))
        }
        IngestRdfFormat::TriG => {
            let mut p = TriGParser::new(reader, base_iri);
            p.parse_all(&mut |q| {
                on_triple(pack_rio_quad(q));
                Ok(()) as Result<(), rio_turtle::TurtleError>
            })
            .map_err(|e| format!("TriG: {e}"))
        }
        IngestRdfFormat::RdfXml => {
            let mut p = RdfXmlParser::new(reader, base_iri);
            p.parse_all(&mut |t| {
                on_triple(pack_rio_triple(t, 0));
                Ok(()) as Result<(), rio_xml::RdfXmlError>
            })
            .map_err(|e| format!("RDF/XML: {e}"))
        }
        IngestRdfFormat::JsonLd => parse_jsonld(reader, on_triple),
        IngestRdfFormat::YamlLd => parse_yamlld(reader, on_triple),
        IngestRdfFormat::RdfJson => parse_rdfjson(reader, on_triple),
        IngestRdfFormat::CborLd => parse_cbor_ld_stream(reader, on_triple),
        IngestRdfFormat::N3 => Err("N3 uses the native N3 parser".into()),
    }
}

fn parse_jsonld<R, F>(mut reader: R, on_triple: &mut F) -> Result<(), String>
where
    R: Read,
    F: FnMut(RawTriple),
{
    let v = read_json_document(&mut reader, "JSON-LD")?;
    emit_jsonld_value(&v, on_triple)
}

fn parse_yamlld<R, F>(mut reader: R, on_triple: &mut F) -> Result<(), String>
where
    R: Read,
    F: FnMut(RawTriple),
{
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("YAML-LD read: {e}"))?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("YAML-LD ingest is limited to 32 MiB (use Turtle/N-Triples/N-Quads for dumps)".into());
    }
    let v: serde_json::Value =
        serde_yaml::from_slice(&bytes).map_err(|e| format!("YAML-LD: {e}"))?;
    emit_jsonld_value(&v, on_triple)
}

fn parse_rdfjson<R, F>(mut reader: R, on_triple: &mut F) -> Result<(), String>
where
    R: Read,
    F: FnMut(RawTriple),
{
    let v = read_json_document(&mut reader, "RDF/JSON")?;
    emit_rdfjson_value(&v, on_triple)
}

fn read_json_document<R: Read>(reader: &mut R, label: &str) -> Result<serde_json::Value, String> {
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{label} read: {e}"))?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(format!(
            "{label} ingest is limited to 32 MiB (use Turtle/N-Triples/N-Quads for dumps)"
        ));
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("{label}: {e}"))
}

fn emit_rdfjson_value<F>(v: &serde_json::Value, on_triple: &mut F) -> Result<(), String>
where
    F: FnMut(RawTriple),
{
    let Some(subjects) = v.as_object() else {
        return Err("RDF/JSON must be an object of subject → predicate maps".into());
    };
    for (subj, preds) in subjects {
        let Some(pred_map) = preds.as_object() else {
            continue;
        };
        for (pred, objects) in pred_map {
            let items = if let Some(arr) = objects.as_array() {
                arr.as_slice()
            } else {
                std::slice::from_ref(objects)
            };
            for item in items {
                let Some(obj) = item.as_object() else {
                    continue;
                };
                let kind = obj
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("literal");
                let value = obj
                    .get("value")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                if value.is_empty() {
                    continue;
                }
                let packed_object = match kind {
                    "uri" | "iri" | "bnode" => None,
                    _ => obj
                        .get("datatype")
                        .and_then(|t| t.as_str())
                        .and_then(|dt| pack_typed_literal(&value, dt)),
                };
                let object = if kind == "bnode" && !value.starts_with("_:") {
                    format!("_:{value}")
                } else if let Some(lang) = obj.get("lang").and_then(|t| t.as_str()) {
                    format!("{value}@{lang}")
                } else {
                    value
                };
                on_triple(RawTriple {
                    subject: subj.clone(),
                    predicate: pred.clone(),
                    object,
                    packed_object,
                    context: 0,
                });
            }
        }
    }
    Ok(())
}

fn emit_jsonld_value<F>(v: &serde_json::Value, on_triple: &mut F) -> Result<(), String>
where
    F: FnMut(RawTriple),
{
    if let Ok(flat) = serde_json::from_value::<Vec<FlatTriple>>(v.clone()) {
        for t in flat {
            on_triple(RawTriple {
                subject: t.s,
                predicate: t.p,
                object: t.o,
                packed_object: None,
                context: t.g.map(|g| q_hash(&g)).unwrap_or(0),
            });
        }
        return Ok(());
    }
    let nodes = if let Some(g) = v.get("@graph").and_then(|g| g.as_array()) {
        g.clone()
    } else if v.is_array() {
        v.as_array().cloned().unwrap_or_default()
    } else {
        vec![v.clone()]
    };
    for node in nodes {
        let Some(subj) = node.get("@id").and_then(|x| x.as_str()).map(str::to_string) else {
            continue;
        };
        if let Some(obj) = &node.as_object() {
            for (k, val) in *obj {
                if k.starts_with('@') {
                    continue;
                }
                emit_jsonld_pred(&subj, k, val, 0, on_triple)?;
            }
        }
    }
    Ok(())
}

fn emit_jsonld_pred<F>(
    subj: &str,
    pred: &str,
    val: &serde_json::Value,
    context: u64,
    on_triple: &mut F,
) -> Result<(), String>
where
    F: FnMut(RawTriple),
{
    match val {
        serde_json::Value::Array(items) => {
            for item in items {
                emit_jsonld_pred(subj, pred, item, context, on_triple)?;
            }
        }
        serde_json::Value::Object(map) => {
            let obj = map
                .get("@id")
                .and_then(|x| x.as_str())
                .or_else(|| map.get("@value").and_then(|x| x.as_str()))
                .unwrap_or("")
                .to_string();
            if !obj.is_empty() {
                on_triple(RawTriple {
                    subject: subj.to_string(),
                    predicate: pred.to_string(),
                    object: obj,
                    packed_object: None,
                    context,
                });
            }
        }
        serde_json::Value::String(s) => {
            on_triple(RawTriple {
                subject: subj.to_string(),
                predicate: pred.to_string(),
                object: s.clone(),
                packed_object: None,
                context,
            });
        }
        serde_json::Value::Number(n) => {
            on_triple(RawTriple {
                subject: subj.to_string(),
                predicate: pred.to_string(),
                object: n.to_string(),
                packed_object: None,
                context,
            });
        }
        serde_json::Value::Bool(b) => {
            on_triple(RawTriple {
                subject: subj.to_string(),
                predicate: pred.to_string(),
                object: b.to_string(),
                packed_object: Some(crate::resolver::INLINE_TAG_BOOLEAN | u64::from(*b)),
                context,
            });
        }
        _ => {}
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct FlatTriple {
    s: String,
    p: String,
    o: String,
    g: Option<String>,
}

fn parse_cbor_ld_stream<R, F>(mut reader: R, on_triple: &mut F) -> Result<(), String>
where
    R: Read,
    F: FnMut(RawTriple),
{
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("CBOR-LD read: {e}"))?;
    let mut rest = bytes.as_slice();
    while !rest.is_empty() {
        match parse_cbor_ld_to_quin(rest) {
            Ok(quin) => {
                on_triple(RawTriple {
                    subject: quin.subject.to_string(),
                    predicate: quin.predicate.to_string(),
                    object: quin.object.to_string(),
                    packed_object: Some(quin.object),
                    context: quin.context,
                });
                // Advance at least one byte so we cannot livelock; exact CBOR
                // length is not returned by the quin parser, so we require a
                // concatenated stream of standalone quin arrays decoded via
                // a 1-byte scan for the next array header.
                if let Some(pos) = rest.iter().skip(1).position(|&b| (0x80..=0x97).contains(&b) || b == 0x9f)
                {
                    rest = &rest[pos + 1..];
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

/// Helper for tests / small in-memory sources.
pub fn parse_bytes_to_raw(
    format: IngestRdfFormat,
    bytes: &[u8],
) -> Result<Vec<RawTriple>, String> {
    let mut out = Vec::new();
    parse_triples_format(format, Cursor::new(bytes), None, &mut |t| {
        out.push(t);
    })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntriples_and_nquads_parse() {
        let nt = b"<http://ex/s> <http://ex/p> <http://ex/o> .\n";
        let got = parse_bytes_to_raw(IngestRdfFormat::NTriples, nt).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].subject, "http://ex/s");
        assert_eq!(got[0].predicate, "http://ex/p");
        assert_eq!(got[0].object, "http://ex/o");
        let nq = b"<http://ex/s> <http://ex/p> <http://ex/o> <http://ex/g> .\n";
        let got = parse_bytes_to_raw(IngestRdfFormat::NQuads, nq).unwrap();
        assert_eq!(got.len(), 1);
        assert_ne!(got[0].context, 0);
    }

    #[test]
    fn turtle_and_trig_parse() {
        let ttl = b"@prefix ex: <http://ex/> .\nex:s ex:p ex:o .\n";
        let got = parse_bytes_to_raw(IngestRdfFormat::Turtle, ttl).unwrap();
        assert_eq!(got.len(), 1);
        let trig = b"@prefix ex: <http://ex/> .\ngraph ex:g { ex:s ex:p ex:o . }\n";
        let got = parse_bytes_to_raw(IngestRdfFormat::TriG, trig).unwrap();
        assert_eq!(got.len(), 1);
        assert_ne!(got[0].context, 0);
    }

    #[test]
    fn jsonld_flat_and_graph() {
        let flat = br#"[{"s":"http://ex/s","p":"http://ex/p","o":"http://ex/o"}]"#;
        let got = parse_bytes_to_raw(IngestRdfFormat::JsonLd, flat).unwrap();
        assert_eq!(got.len(), 1);
        let g = br#"{"@graph":[{"@id":"http://ex/s","http://ex/p":{"@id":"http://ex/o"}}]}"#;
        let got = parse_bytes_to_raw(IngestRdfFormat::JsonLd, g).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].object, "http://ex/o");
    }

    #[test]
    fn rdfxml_yamlld_and_rdfjson_parse() {
        let xml = br#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://ex/">
  <rdf:Description rdf:about="http://ex/s">
    <ex:p rdf:resource="http://ex/o"/>
  </rdf:Description>
</rdf:RDF>"#;
        let got = parse_bytes_to_raw(IngestRdfFormat::RdfXml, xml).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].subject, "http://ex/s");
        assert_eq!(got[0].object, "http://ex/o");

        let yaml = br#""@graph":
  - "@id": http://ex/s
    "http://ex/p":
      "@id": http://ex/o
"#;
        let got = parse_bytes_to_raw(IngestRdfFormat::YamlLd, yaml).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].object, "http://ex/o");

        let rj = br#"{"http://ex/s":{"http://ex/p":[{"type":"uri","value":"http://ex/o"}]}}"#;
        let got = parse_bytes_to_raw(IngestRdfFormat::RdfJson, rj).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].subject, "http://ex/s");
        assert_eq!(got[0].object, "http://ex/o");
    }

    #[test]
    fn formats_share_the_same_iri_spelling() {
        let nt = parse_bytes_to_raw(
            IngestRdfFormat::NTriples,
            b"<http://ex/s> <http://ex/p> <http://ex/o> .\n",
        )
        .unwrap();
        let ttl = parse_bytes_to_raw(
            IngestRdfFormat::Turtle,
            b"@prefix ex: <http://ex/> .\nex:s ex:p ex:o .\n",
        )
        .unwrap();
        assert_eq!(nt[0].subject, ttl[0].subject);
        assert_eq!(nt[0].object, ttl[0].object);
        assert_eq!(object_iri_hash(&nt[0].object), object_iri_hash("http://ex/o"));
    }
}
