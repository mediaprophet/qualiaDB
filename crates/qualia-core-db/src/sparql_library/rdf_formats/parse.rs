//! Zero-heap RDF parse dispatch into [`QuinCollector`].

use super::{collector::QuinCollector, QuinSink, RdfFormat};
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfParseError {
    UnknownFormat,
    Io(String),
    Syntax(String),
    BufferFull,
}

impl std::fmt::Display for RdfParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RdfParseError {}

fn map_sink_err(e: Box<dyn std::error::Error>) -> RdfParseError {
    if e.to_string()
        .contains(super::collector::QUIN_BUFFER_FULL_MSG)
    {
        RdfParseError::BufferFull
    } else {
        RdfParseError::Syntax(e.to_string())
    }
}

/// Parse RDF/RDF-Star bytes into a fixed-capacity collector (no per-triple heap).
pub fn parse_rdf<R: Read>(
    format: RdfFormat,
    reader: R,
    context_hash: u64,
    collector: &mut QuinCollector,
) -> Result<u64, RdfParseError> {
    let count = match format {
        RdfFormat::NTriples => {
            crate::sparql_library::parsers::ntriples_star::parse_ntriples_star_into(
                reader,
                context_hash,
                collector,
            )
            .map_err(map_sink_err)?
        }
        RdfFormat::Turtle => crate::sparql_library::parsers::turtle_star::parse_turtle_star_into(
            reader,
            context_hash,
            collector,
        )
        .map_err(map_sink_err)?,
        RdfFormat::NQuads => crate::sparql_library::parsers::nquads_star::parse_nquads_star_into(
            reader,
            context_hash,
            collector,
        )
        .map_err(map_sink_err)?,
        RdfFormat::TriG => crate::sparql_library::parsers::trig_star::parse_trig_star_into(
            reader,
            context_hash,
            collector,
        )
        .map_err(map_sink_err)?,
        RdfFormat::N3 => crate::sparql_library::parsers::n3_star::parse_n3_star_into(
            reader,
            context_hash,
            collector,
        )
        .map_err(map_sink_err)?,
        RdfFormat::JsonLd => crate::sparql_library::parsers::json_ld_stream::parse_json_ld_into(
            reader,
            context_hash,
            collector,
        )
        .map_err(map_sink_err)?,
        RdfFormat::CborLd => crate::sparql_library::parsers::cbor_parser::parse_cbor_ld_into(
            reader,
            context_hash,
            collector,
        )
        .map_err(map_sink_err)?,
    };
    if collector.truncated {
        return Err(RdfParseError::BufferFull);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparql_library::rdf_formats::{RdfFormat, RdfStarMode};
    use std::io::Cursor;

    #[test]
    fn parse_ntriples_into_collector() {
        let input =
            "<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .\n";
        let mut collector = QuinCollector::new();
        let count = parse_rdf(
            RdfFormat::NTriples,
            Cursor::new(input.as_bytes()),
            0,
            &mut collector,
        )
        .expect("parse");
        assert_eq!(count, 1);
        assert_eq!(collector.count, 1);
        assert!(!collector.truncated);
    }

    #[test]
    fn plain_serialize_round_trip() {
        let input =
            "<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .\n";
        let mut collector = QuinCollector::new();
        parse_rdf(
            RdfFormat::NTriples,
            Cursor::new(input.as_bytes()),
            0,
            &mut collector,
        )
        .expect("parse");

        let mut out = Vec::new();
        super::super::serialize::serialize_rdf(
            RdfFormat::NTriples,
            RdfStarMode::Plain,
            collector.as_slice(),
            &mut out,
        )
        .expect("serialize");
        let rendered = String::from_utf8(out).expect("utf8");
        assert!(rendered.contains("quin:hash/"));
        assert!(rendered.ends_with(" .\n"));
    }

    #[test]
    fn rdf_format_from_str_aliases() {
        assert_eq!(RdfFormat::from_str("nt"), Some(RdfFormat::NTriples));
        assert_eq!(RdfFormat::from_str("ttl"), Some(RdfFormat::Turtle));
        assert_eq!(RdfFormat::from_str("json-ld"), Some(RdfFormat::JsonLd));
        assert_eq!(RdfFormat::from_str("unknown"), None);
    }
}
