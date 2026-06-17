//! Unified zero-heap RDF / RDF-Star serialization dispatch.

use crate::resolver;
use crate::sparql_library::parsers::turtle_star::{
    JsonLdStarSerializer, N3StarSerializer, NQuadsStarSerializer, TrigStarSerializer,
    TurtleStarSerializer,
};
use crate::sparql_library::rdf_formats::{RdfFormat, RdfStarMode};
use crate::NQuin;
use crate::rdf_star::RdfStarSerializer;
use std::io::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfDispatchError {
    Unsupported,
    Io(String),
}

pub fn serialize_quins<W: Write>(
    format: RdfFormat,
    mode: RdfStarMode,
    quins: &[NQuin],
    out: &mut W,
) -> Result<(), RdfDispatchError> {
    match mode {
        RdfStarMode::Plain => serialize_plain(format, quins, out),
        RdfStarMode::Star => serialize_star(format, quins, out),
    }
}

fn serialize_plain<W: Write>(
    format: RdfFormat,
    quins: &[NQuin],
    out: &mut W,
) -> Result<(), RdfDispatchError> {
    let result = match format {
        RdfFormat::NTriples => resolver::format_ntriples_to(quins, out),
        RdfFormat::NQuads | RdfFormat::TriG => resolver::format_nquads_to(quins, out),
        RdfFormat::Turtle | RdfFormat::N3 | RdfFormat::JsonLd | RdfFormat::CborLd => {
            resolver::format_ntriples_to(quins, out)
        }
    };
    result.map_err(|e| RdfDispatchError::Io(e.to_string()))
}

fn serialize_star<W: Write>(
    format: RdfFormat,
    quins: &[NQuin],
    out: &mut W,
) -> Result<(), RdfDispatchError> {
    match format {
        RdfFormat::NTriples => resolver::format_ntriples_star_to(quins, out)
            .map_err(|e| RdfDispatchError::Io(e.to_string())),
        RdfFormat::NQuads => {
            for q in quins {
                let ser = NQuadsStarSerializer::new();
                let bytes = if q.context != 0 {
                    ser.serialize_quad(q.subject, q.predicate, q.object, q.context)
                } else {
                    ser.serialize_triple(q.subject, q.predicate, q.object)
                }
                .map_err(|e| RdfDispatchError::Io(format!("{e:?}")))?;
                out.write_all(&bytes).map_err(|e| RdfDispatchError::Io(e.to_string()))?;
                out.write_all(b"\n")
                    .map_err(|e| RdfDispatchError::Io(e.to_string()))?;
            }
            Ok(())
        }
        RdfFormat::Turtle => write_with_serializer(quins, out, &TurtleStarSerializer::new()),
        RdfFormat::TriG => {
            let ser = TrigStarSerializer::new();
            write_with_serializer(quins, out, &ser)
        }
        RdfFormat::N3 => {
            let ser = N3StarSerializer::new();
            write_with_serializer(quins, out, &ser)
        }
        RdfFormat::JsonLd => {
            let ser = JsonLdStarSerializer::new();
            write_with_serializer(quins, out, &ser)
        }
        RdfFormat::CborLd => Err(RdfDispatchError::Unsupported),
    }
}

fn write_with_serializer<W: Write, S: RdfStarSerializer>(
    quins: &[NQuin],
    out: &mut W,
    ser: &S,
) -> Result<(), RdfDispatchError> {
    for q in quins {
        let bytes = if q.context != 0 && ser.supports_quads() {
            ser.serialize_quad(q.subject, q.predicate, q.object, q.context)
        } else {
            ser.serialize_triple(q.subject, q.predicate, q.object)
        }
        .map_err(|e| RdfDispatchError::Io(format!("{e:?}")))?;
        out.write_all(&bytes)
            .map_err(|e| RdfDispatchError::Io(e.to_string()))?;
        out.write_all(b"\n")
            .map_err(|e| RdfDispatchError::Io(e.to_string()))?;
    }
    Ok(())
}