//! Unified RDF / RDF-Star format dispatch with zero-heap parse collection.

mod collector;
mod jsonld_context;
#[cfg(test)]
mod jsonld_vendor_roundtrip;
mod package_manifest;
mod parse;
mod rdfc;
mod serialize;
mod solid_media;
mod sparql_ldjson_adapters;

pub use crate::sparql_library::quin_sink::QuinSink;
pub use collector::{QuinCollector, MAX_RDF_QUINS};
pub use jsonld_context::{
    context_digest_hex, digest_hex, pinned_context_bytes, QUALIA_JSONLD_CONTEXT_ID,
    QUALIA_JSONLD_CONTEXT_V1,
};
pub use package_manifest::{package_exposure_manifest, PackageExposureManifest, VIBE_AST_TAG};
pub use parse::{parse_rdf, RdfParseError};
pub use rdfc::{
    provisional_spo_digest_hex, rdfc10_available, rdfc10_hash_hex, RdfcError,
    PROVISIONAL_SPO_DIGEST_PROFILE, RDFC10_PROFILE,
};
pub use serialize::{serialize_rdf, RdfSerializeError, RdfStarMode};
pub use solid_media::{
    negotiate_solid_accept, SolidRdfMedia, SOLID_DEFAULT_CONTENT_TYPE, SOLID_RDF_MEDIA,
};
pub use sparql_ldjson_adapters::{construct_results_as_ld_json, daemon_quin_graph_ld_json};

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
            "nt" | "ntriples" | "n-triples" | "application/n-triples" => Some(Self::NTriples),
            "turtle" | "ttl" | "text/turtle" => Some(Self::Turtle),
            "nquads" | "n-quads" | "application/n-quads" => Some(Self::NQuads),
            "trig" | "application/trig" => Some(Self::TriG),
            "n3" | "text/n3" | "text/rdf+n3" => Some(Self::N3),
            "jsonld" | "json-ld" | "application/ld+json" => Some(Self::JsonLd),
            "cbor" | "cbor-ld" | "cborld" | "application/vnd.qualia.nquin-cbor" => {
                Some(Self::CborLd)
            }
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
        matches!(
            self,
            Self::NQuads | Self::TriG | Self::JsonLd | Self::CborLd
        )
    }
}
