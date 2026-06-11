//! N-Triples-Star Parser for QualiaDB
//!
//! Implements RDF-Star (SPARQL 1.2) parsing for N-Triples syntax with embedded triples.
//! N-Triples-Star is a line-based format with strict syntax:
//! - Regular triple: `<subject> <predicate> <object> .`
//! - Embedded triple: `<<<subject> <predicate> <object>>> <predicate> <object> .`

use qualia_core_db::NQuin;
use qualia_core_db::lexicon::{generate_embedded_triple_id, generate_60bit_token};
use qualia_core_db::rdf_star::{RdfStarParser, RdfStarParseError};

/// N-Triples-Star parser implementation
pub struct NTriplesStarParser {
    /// Context hash for the current parsing session
    context_hash: u64,
}

impl NTriplesStarParser {
    /// Create a new N-Triples-Star parser
    pub fn new(context_hash: u64) -> Self {
        Self { context_hash }
    }

    /// Parse an N-Triples line (subject, predicate, object)
    /// 
    /// Handles both regular triples and embedded triples
    fn parse_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(ParseResult::Comment);
        }

        // Check for embedded triple start marker
        if line.starts_with("<<<") {
            self.parse_embedded_triple_line(line)
        } else {
            self.parse_regular_triple_line(line)
        }
    }

    /// Parse a regular N-Triples triple
    fn parse_regular_triple_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: <subject> <predicate> <object> .
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(RdfStarParseError::InvalidSyntax);
        }

        let subject_str = parts[0];
        let predicate_str = parts[1];
        let object_str = parts[2];

        // Strip angle brackets
        let subject = subject_str.trim_start_matches('<').trim_end_matches('>');
        let predicate = predicate_str.trim_start_matches('<').trim_end_matches('>');
        let object = object_str.trim_start_matches('<').trim_end_matches('>');

        let subject_hash = generate_60bit_token(subject.as_bytes());
        let predicate_hash = generate_60bit_token(predicate.as_bytes());
        let object_hash = generate_60bit_token(object.as_bytes());

        Ok(ParseResult::RegularTriple {
            subject: subject_hash,
            predicate: predicate_hash,
            object: object_hash,
            subject_str: subject.to_string(),
            predicate_str: predicate.to_string(),
            object_str: object.to_string(),
        })
    }

    /// Parse an embedded triple line
    fn parse_embedded_triple_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: <<<subject> <predicate> <object>>> <predicate> <object> .
        // This is complex - need to find the closing >>> and then parse the outer triple
        
        // Find the closing >>> for the embedded triple
        let end_embedded = line.find(">>>").ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;
        
        // Extract the embedded triple part
        let embedded_part = &line[3..end_embedded]; // Skip <<<
        
        // Parse the embedded triple components
        let embedded_parts: Vec<&str> = embedded_part.split_whitespace().collect();
        if embedded_parts.len() < 3 {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }

        let subject = embedded_parts[0].trim_start_matches('<').trim_end_matches('>');
        let predicate = embedded_parts[1].trim_start_matches('<').trim_end_matches('>');
        let object = embedded_parts[2].trim_start_matches('<').trim_end_matches('>');

        let subject_hash = generate_60bit_token(subject.as_bytes());
        let predicate_hash = generate_60bit_token(predicate.as_bytes());
        let object_hash = generate_60bit_token(object.as_bytes());

        // Generate Virtual ID for the embedded triple
        let virtual_id = generate_embedded_triple_id(subject_hash, predicate_hash, object_hash);

        // Parse the outer triple (the part after >>>)
        let remaining = &line[end_embedded + 3..]; // Skip >>>
        let outer_parts: Vec<&str> = remaining.split_whitespace().collect();
        if outer_parts.len() < 2 {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }

        let outer_predicate = outer_parts[0].trim_start_matches('<').trim_end_matches('>');
        let outer_object = outer_parts[1].trim_start_matches('<').trim_end_matches('>');

        let outer_predicate_hash = generate_60bit_token(outer_predicate.as_bytes());
        let outer_object_hash = generate_60bit_token(outer_object.as_bytes());

        Ok(ParseResult::EmbeddedTriple {
            virtual_id,
            components: [subject_hash, predicate_hash, object_hash],
            outer_predicate: outer_predicate_hash,
            outer_object: outer_object_hash,
            outer_predicate_str: outer_predicate.to_string(),
            outer_object_str: outer_object.to_string(),
        })
    }
}

impl RdfStarParser for NTriplesStarParser {
    fn parse_embedded_triple(&mut self, input: &[u8]) -> Result<(u64, [u64; 3]), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;
        
        match self.parse_line(line)? {
            ParseResult::EmbeddedTriple { virtual_id, components, .. } => {
                Ok((virtual_id, components))
            }
            _ => Err(RdfStarParseError::MalformedEmbeddedTriple),
        }
    }

    fn parse_triple(&mut self, input: &[u8]) -> Result<(u64, u64, u64), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;
        
        match self.parse_line(line)? {
            ParseResult::RegularTriple { subject, predicate, object, .. } => {
                Ok((subject, predicate, object))
            }
            _ => Err(RdfStarParseError::InvalidSyntax),
        }
    }

    fn parse_quad(&mut self, _input: &[u8]) -> Result<(u64, u64, u64, u64), RdfStarParseError> {
        // N-Triples-Star doesn't support quads natively (use N-Quads-Star for that)
        Err(RdfStarParseError::UnsupportedFeature)
    }

    fn supports_quads(&self) -> bool {
        false
    }

    fn supports_named_graphs(&self) -> bool {
        false
    }

    fn format_name(&self) -> &'static str {
        "N-Triples-Star"
    }
}

/// Parse result for N-Triples-Star
enum ParseResult {
    Comment,
    RegularTriple {
        subject: u64,
        predicate: u64,
        object: u64,
        subject_str: String,
        predicate_str: String,
        object_str: String,
    },
    EmbeddedTriple {
        virtual_id: u64,
        components: [u64; 3],
        outer_predicate: u64,
        outer_object: u64,
        outer_predicate_str: String,
        outer_object_str: String,
    },
}

/// Parse N-Triples-Star stream and emit Quins
pub fn parse_ntriples_star_stream<R: std::io::Read>(
    reader: R,
    context_hash: u64,
    sorter: &mut super::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    use std::io::BufRead;
    
    let mut parser = NTriplesStarParser::new(context_hash);
    let mut count = 0;
    let buf_reader = BufReader::new(reader);

    for line in buf_reader.lines() {
        let line = line?;
        match parser.parse_line(&line)? {
            ParseResult::Comment => continue,
            ParseResult::RegularTriple { subject, predicate, object, .. } => {
                sorter.push(NQuin {
                    subject,
                    predicate,
                    object,
                    context: context_hash,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
            }
            ParseResult::EmbeddedTriple { virtual_id, components, outer_predicate, outer_object, .. } => {
                // Emit the outer triple with the Virtual ID as the subject
                sorter.push(NQuin {
                    subject: virtual_id,
                    predicate: outer_predicate,
                    object: outer_object,
                    context: context_hash,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
                
                // Also emit the embedded triple components for indexing
                sorter.push(NQuin {
                    subject: components[0],
                    predicate: components[1],
                    object: components[2],
                    context: context_hash,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
            }
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ntriples_star_parser_creation() {
        let parser = NTriplesStarParser::new(0);
        assert_eq!(parser.format_name(), "N-Triples-Star");
        assert!(!parser.supports_quads());
        assert!(!parser.supports_named_graphs());
    }

    #[test]
    fn test_parse_regular_triple() {
        let mut parser = NTriplesStarParser::new(0);
        let input = b"<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .";
        let result = parser.parse_triple(input);
        assert!(result.is_ok());
        let (s, p, o) = result.unwrap();
        assert_ne!(s, 0);
        assert_ne!(p, 0);
        assert_ne!(o, 0);
    }

    #[test]
    fn test_parse_embedded_triple() {
        let mut parser = NTriplesStarParser::new(0);
        let input = b"<<<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob>>> <http://example.org/saidBy> <http://example.org/Charlie> .";
        let result = parser.parse_embedded_triple(input);
        assert!(result.is_ok());
        let (virtual_id, components) = result.unwrap();
        assert_ne!(virtual_id, 0);
        assert_ne!(components[0], 0);
        assert_ne!(components[1], 0);
        assert_ne!(components[2], 0);
    }
}