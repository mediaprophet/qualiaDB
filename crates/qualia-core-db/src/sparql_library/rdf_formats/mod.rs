//! Unified RDF / RDF-Star format dispatch with zero-heap parse collection.

mod collector;
mod parse;
mod serialize;

pub use collector::{QuinCollector, MAX_RDF_QUINS};
pub use crate::sparql_library::quin_sink::QuinSink;
pub use parse::{parse_rdf, RdfParseError};
pub use serialize::{serialize_rdf, RdfSerializeError, RdfStarMode};


/// Supported RDF surface syntaxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdfFormat {
    NTriples,
    Turtle,
    NQuads,
    TriG,
    N3,
    JsonLd,
    CborLd,
}

impl RdfFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "nt" | "ntriples" | "n-triples" => Some(Self::NTriples),
            "turtle" | "ttl" => Some(Self::Turtle),
            "nquads" | "n-quads" => Some(Self::NQuads),
            "trig" => Some(Self::TriG),
            "n3" => Some(Self::N3),
            "jsonld" | "json-ld" => Some(Self::JsonLd),
            "cbor" | "cbor-ld" | "cborld" => Some(Self::CborLd),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::NTriples => "ntriples",
            Self::Turtle => "turtle",
            Self::NQuads => "nquads",
            Self::TriG => "trig",
            Self::N3 => "n3",
            Self::JsonLd => "jsonld",
            Self::CborLd => "cborld",
        }
    }

    pub fn supports_quads(self) -> bool {
        matches!(self, Self::NQuads | Self::TriG | Self::JsonLd | Self::CborLd)
    }
}