//! Zero-heap RDF serialize dispatch via resolver + star formatters.

use super::RdfFormat;
use crate::NQuin;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdfStarMode {
    Plain,
    Star,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfSerializeError {
    UnknownFormat,
    UnsupportedFeature,
    Io(String),
}

impl std::fmt::Display for RdfSerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RdfSerializeError {}

/// Serialize quins to RDF/RDF-Star syntax, writing directly to `out` (zero-heap hot path).
pub fn serialize_rdf<W: Write>(
    format: RdfFormat,
    mode: RdfStarMode,
    quins: &[NQuin],
    out: &mut W,
) -> Result<(), RdfSerializeError> {
    crate::sparql_library::serialisers::rdf_dispatch::serialize_quins(format, mode, quins, out)
        .map_err(|e| match e {
            crate::sparql_library::serialisers::rdf_dispatch::RdfDispatchError::Unsupported => {
                RdfSerializeError::UnsupportedFeature
            }
            crate::sparql_library::serialisers::rdf_dispatch::RdfDispatchError::Io(msg) => {
                RdfSerializeError::Io(msg)
            }
        })
}
