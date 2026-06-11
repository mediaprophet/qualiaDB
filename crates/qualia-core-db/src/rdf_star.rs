//! RDF-Star Parsing and Serialization Infrastructure
//!
//! This module provides trait definitions and shared infrastructure for
//! parsing and serializing RDF-Star (SPARQL 1.2) data across multiple formats.
//!
//! All parsers converge on Virtual IDs (TAG_EMBEDDED | 60-bit hash).
//! All serializers diverge from Virtual IDs to format-specific syntax.

use crate::lexicon::{generate_embedded_triple_id, TAG_EMBEDDED};

/// Error type for RDF-Star parsing operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfStarParseError {
    /// Invalid syntax for the format
    InvalidSyntax,
    /// Malformed embedded triple
    MalformedEmbeddedTriple,
    /// Buffer overflow during parsing
    BufferOverflow,
    /// Unsupported feature for this format
    UnsupportedFeature,
    /// Invalid UTF-8 encoding
    InvalidUtf8,
    /// Lexicon lookup failed
    LexiconError,
}

/// Error type for RDF-Star serialization operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfStarSerializeError {
    /// Virtual ID not found in lexicon
    VirtualIdNotFound,
    /// Component IDs not found in lexicon
    ComponentNotFound,
    /// Buffer too small for output
    BufferTooSmall,
    /// Unsupported feature for this format
    UnsupportedFeature,
}

/// Trait for RDF-Star parsers across multiple formats
///
/// All parsers must implement this trait to ensure consistent
/// conversion from format-specific syntax to Virtual IDs.
pub trait RdfStarParser {
    /// Parse an embedded triple from format-specific syntax
    /// 
    /// Returns (Virtual ID, [subject_id, predicate_id, object_id])
    fn parse_embedded_triple(&mut self, input: &[u8]) -> Result<(u64, [u64; 3]), RdfStarParseError>;
    
    /// Parse a regular triple (subject, predicate, object)
    fn parse_triple(&mut self, input: &[u8]) -> Result<(u64, u64, u64), RdfStarParseError>;
    
    /// Parse a quad (subject, predicate, object, graph)
    /// 
    /// Returns None if the format doesn't support quads
    fn parse_quad(&mut self, input: &[u8]) -> Result<(u64, u64, u64, u64), RdfStarParseError>;
    
    /// Check if this parser supports quad formats (named graphs)
    fn supports_quads(&self) -> bool;
    
    /// Check if this parser supports named graphs
    fn supports_named_graphs(&self) -> bool;
    
    /// Get the format name for error reporting
    fn format_name(&self) -> &'static str;
}

/// Trait for RDF-Star serializers across multiple formats
///
/// All serializers must implement this trait to ensure consistent
/// conversion from Virtual IDs to format-specific syntax.
pub trait RdfStarSerializer {
    /// Serialize a Virtual ID back to format-specific embedded triple syntax
    /// 
    /// Takes the Virtual ID and its component IDs (retrieved from lexicon)
    fn serialize_embedded_triple(
        &self,
        virtual_id: u64,
        components: &[u64; 3],
    ) -> Result<Vec<u8>, RdfStarSerializeError>;
    
    /// Serialize a regular triple
    fn serialize_triple(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Result<Vec<u8>, RdfStarSerializeError>;
    
    /// Serialize a quad (subject, predicate, object, graph)
    fn serialize_quad(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
        graph: u64,
    ) -> Result<Vec<u8>, RdfStarSerializeError>;
    
    /// Check if this serializer supports quad formats
    fn supports_quads(&self) -> bool;
    
    /// Get the format name for error reporting
    fn format_name(&self) -> &'static str;
}

/// Helper function to create a Virtual ID from component IDs
/// 
/// This is a convenience wrapper around generate_embedded_triple_id
/// that also validates the TAG_EMBEDDED bit is set correctly.
#[inline(always)]
pub fn create_virtual_id(subject: u64, predicate: u64, object: u64) -> u64 {
    let virtual_id = generate_embedded_triple_id(subject, predicate, object);
    debug_assert_eq!(virtual_id & TAG_EMBEDDED, TAG_EMBEDDED, "TAG_EMBEDDED bit should be set");
    virtual_id
}

/// Check if a u64 is a Virtual ID (has TAG_EMBEDDED bit set)
#[inline(always)]
pub fn is_virtual_id(value: u64) -> bool {
    (value & TAG_EMBEDDED) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_virtual_id_sets_tag() {
        let vid = create_virtual_id(1, 2, 3);
        assert_eq!(vid & TAG_EMBEDDED, TAG_EMBEDDED);
    }

    #[test]
    fn test_is_virtual_id() {
        let vid = create_virtual_id(1, 2, 3);
        assert!(is_virtual_id(vid));
        
        let regular_hash = 12345u64;
        assert!(!is_virtual_id(regular_hash));
    }

    #[test]
    fn test_create_virtual_id_deterministic() {
        let vid1 = create_virtual_id(1, 2, 3);
        let vid2 = create_virtual_id(1, 2, 3);
        assert_eq!(vid1, vid2);
    }
}

/// SPARQL-Star BIND Functions
/// 
/// These functions are used in SPARQL queries to extract components from
/// embedded triples using the BIND keyword.
/// 
/// Example: BIND (SUBJECT(?triple) AS ?subject)

/// Extract the subject component from an embedded triple.
///
/// Given a Virtual ID, performs a binary search in the Q42LEX data and returns
/// the subject component ID stored in the embedded triple entry.
pub fn subject_of_virtual_id(
    virtual_id: u64,
    lex_data: &[u8],
    _blob_base: usize,
) -> Result<u64, RdfStarParseError> {
    if !is_virtual_id(virtual_id) {
        return Err(RdfStarParseError::MalformedEmbeddedTriple);
    }
    let lex = crate::q42_lex::Q42LexMmap::from_bytes(lex_data)
        .map_err(|_| RdfStarParseError::LexiconError)?;
    lex.lookup_embedded_triple(virtual_id)
        .map(|t| t[0])
        .ok_or(RdfStarParseError::LexiconError)
}

/// Extract the predicate component from an embedded triple.
pub fn predicate_of_virtual_id(
    virtual_id: u64,
    lex_data: &[u8],
    _blob_base: usize,
) -> Result<u64, RdfStarParseError> {
    if !is_virtual_id(virtual_id) {
        return Err(RdfStarParseError::MalformedEmbeddedTriple);
    }
    let lex = crate::q42_lex::Q42LexMmap::from_bytes(lex_data)
        .map_err(|_| RdfStarParseError::LexiconError)?;
    lex.lookup_embedded_triple(virtual_id)
        .map(|t| t[1])
        .ok_or(RdfStarParseError::LexiconError)
}

/// Extract the object component from an embedded triple.
pub fn object_of_virtual_id(
    virtual_id: u64,
    lex_data: &[u8],
    _blob_base: usize,
) -> Result<u64, RdfStarParseError> {
    if !is_virtual_id(virtual_id) {
        return Err(RdfStarParseError::MalformedEmbeddedTriple);
    }
    let lex = crate::q42_lex::Q42LexMmap::from_bytes(lex_data)
        .map_err(|_| RdfStarParseError::LexiconError)?;
    lex.lookup_embedded_triple(virtual_id)
        .map(|t| t[2])
        .ok_or(RdfStarParseError::LexiconError)
}

/// Construct a Virtual ID from three component IDs
/// 
/// This is the inverse of the extraction functions.
pub fn triple_from_components(
    subject: u64,
    predicate: u64,
    object: u64,
) -> u64 {
    create_virtual_id(subject, predicate, object)
}
