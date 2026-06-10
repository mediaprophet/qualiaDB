//! N-Quads-Star Parser for QualiaDB
//!
//! Implements RDF-Star (SPARQL 1.2) parsing for N-Quads syntax with embedded triples.
//! N-Quads-Star extends N-Triples-Star with a fourth component (graph/context).
//! Format: `<subject> <predicate> <object> <graph> .`

use qualia_core_db::QualiaQuin;
use qualia_core_db::lexicon::{generate_embedded_triple_id, generate_60bit_token};
use qualia_core_db::rdf_star::{RdfStarParser, RdfStarParseError};

/// N-Quads-Star parser implementation
pub struct NQuadsStarParser {
    /// Context hash for the current parsing session
    context_hash: u64,
}

impl NQuadsStarParser {
    /// Create a new N-Quads-Star parser
    pub fn new(context_hash: u64) -> Self {
        Self { context_hash }
    }

    /// Parse an N-Quads line (subject, predicate, object, graph)
    fn parse_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(ParseResult::Comment);
        }

        // Check for embedded triple start marker
        if line.starts_with("<<<") {
            self.parse_embedded_triple_line(line)
        } else {
            self.parse_quad_line(line)
        }
    }

    /// Parse a regular N-Quads quad
    fn parse_quad_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: <subject> <predicate> <object> <graph> .
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(RdfStarParseError::InvalidSyntax);
        }

        let subject_str = parts[0];
        let predicate_str = parts[1];
        let object_str = parts[2];
        let graph_str = parts[3];

        // Strip angle brackets
        let subject = subject_str.trim_start_matches('<').trim_end_matches('>');
        let predicate = predicate_str.trim_start_matches('<').trim_end_matches('>');
        let object = object_str.trim_start_matches('<').trim_end_matches('>');
        let graph = graph_str.trim_start_matches('<').trim_end_matches('>');

        let subject_hash = generate_60bit_token(subject.as_bytes());
        let predicate_hash = generate_60bit_token(predicate.as_bytes());
        let object_hash = generate_60bit_token(object.as_bytes());
        let graph_hash = generate_60bit_token(graph.as_bytes());

        Ok(ParseResult::RegularQuad {
            subject: subject_hash,
            predicate: predicate_hash,
            object: object_hash,
            graph: graph_hash,
            subject_str: subject.to_string(),
            predicate_str: predicate.to_string(),
            object_str: object.to_string(),
            graph_str: graph.to_string(),
        })
    }

    /// Parse an embedded triple line with graph context
    fn parse_embedded_triple_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: <<<subject> <predicate> <object>>> <predicate> <object> <graph> .
        
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
        if outer_parts.len() < 3 {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }

        let outer_predicate = outer_parts[0].trim_start_matches('<').trim_end_matches('>');
        let outer_object = outer_parts[1].trim_start_matches('<').trim_end_matches('>');
        let outer_graph = outer_parts[2].trim_start_matches('<').trim_end_matches('>');

        let outer_predicate_hash = generate_60bit_token(outer_predicate.as_bytes());
        let outer_object_hash = generate_60bit_token(outer_object.as_bytes());
        let outer_graph_hash = generate_60bit_token(outer_graph.as_bytes());

        Ok(ParseResult::EmbeddedQuad {
            virtual_id,
            components: [subject_hash, predicate_hash, object_hash],
            outer_predicate: outer_predicate_hash,
            outer_object: outer_object_hash,
            outer_graph: outer_graph_hash,
            outer_predicate_str: outer_predicate.to_string(),
            outer_object_str: outer_object.to_string(),
            outer_graph_str: outer_graph.to_string(),
        })
    }
}

impl RdfStarParser for NQuadsStarParser {
    fn parse_embedded_triple(&mut self, input: &[u8]) -> Result<(u64, [u64; 3]), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;
        
        match self.parse_line(line)? {
            ParseResult::EmbeddedQuad { virtual_id, components, .. } => {
                Ok((virtual_id, components))
            }
            _ => Err(RdfStarParseError::MalformedEmbeddedTriple),
        }
    }

    fn parse_triple(&mut self, input: &[u8]) -> Result<(u64, u64, u64), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;
        
        match self.parse_line(line)? {
            ParseResult::RegularQuad { subject, predicate, object, .. } => {
                Ok((subject, predicate, object))
            }
            _ => Err(RdfStarParseError::InvalidSyntax),
        }
    }

    fn parse_quad(&mut self, input: &[u8]) -> Result<(u64, u64, u64, u64), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;
        
        match self.parse_line(line)? {
            ParseResult::RegularQuad { subject, predicate, object, graph, .. } => {
                Ok((subject, predicate, object, graph))
            }
            ParseResult::EmbeddedQuad { outer_predicate, outer_object, outer_graph, .. } => {
                Ok((0, outer_predicate, outer_object, outer_graph))
            }
            _ => Err(RdfStarParseError::InvalidSyntax),
        }
    }

    fn supports_quads(&self) -> bool {
        true
    }

    fn supports_named_graphs(&self) -> bool {
        true
    }

    fn format_name(&self) -> &'static str {
        "N-Quads-Star"
    }
}

/// Parse result for N-Quads-Star
enum ParseResult {
    Comment,
    RegularQuad {
        subject: u64,
        predicate: u64,
        object: u64,
        graph: u64,
        subject_str: String,
        predicate_str: String,
        object_str: String,
        graph_str: String,
    },
    EmbeddedQuad {
        virtual_id: u64,
        components: [u64; 3],
        outer_predicate: u64,
        outer_object: u64,
        outer_graph: u64,
        outer_predicate_str: String,
        outer_object_str: String,
        outer_graph_str: String,
    },
}

/// Parse N-Quads-Star stream and emit Quins
pub fn parse_nquads_star_stream<R: std::io::Read>(
    reader: R,
    context_hash: u64,
    sorter: &mut super::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    use std::io::BufRead;
    
    let mut parser = NQuadsStarParser::new(context_hash);
    let mut count = 0;
    let buf_reader = BufReader::new(reader);

    for line in buf_reader.lines() {
        let line = line?;
        match parser.parse_line(&line)? {
            ParseResult::Comment => continue,
            ParseResult::RegularQuad { subject, predicate, object, graph, .. } => {
                sorter.push(QualiaQuin {
                    subject,
                    predicate,
                    object,
                    context: graph,  // Use graph as context in QualiaQuin
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
            }
            ParseResult::EmbeddedQuad { virtual_id, components, outer_predicate, outer_object, outer_graph, .. } => {
                // Emit the outer quad with the Virtual ID as the subject
                sorter.push(QualiaQuin {
                    subject: virtual_id,
                    predicate: outer_predicate,
                    object: outer_object,
                    context: outer_graph,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
                
                // Also emit the embedded triple components for indexing
                sorter.push(QualiaQuin {
                    subject: components[0],
                    predicate: components[1],
                    object: components[2],
                    context: outer_graph,
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
    fn test_nquads_star_parser_creation() {
        let parser = NQuadsStarParser::new(0);
        assert_eq!(parser.format_name(), "N-Quads-Star");
        assert!(parser.supports_quads());
        assert!(parser.supports_named_graphs());
    }

    #[test]
    fn test_parse_regular_quad() {
        let mut parser = NQuadsStarParser::new(0);
        let input = b"<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> <http://example.org/Graph1> .";
        let result = parser.parse_quad(input);
        assert!(result.is_ok());
        let (s, p, o, g) = result.unwrap();
        assert_ne!(s, 0);
        assert_ne!(p, 0);
        assert_ne!(o, 0);
        assert_ne!(g, 0);
    }

    #[test]
    fn test_parse_embedded_quad() {
        let mut parser = NQuadsStarParser::new(0);
        let input = b"<<<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob>>> <http://example.org/saidBy> <http://example.org/Charlie> <http://example.org/Graph1> .";
        let result = parser.parse_embedded_triple(input);
        assert!(result.is_ok());
        let (virtual_id, components) = result.unwrap();
        assert_ne!(virtual_id, 0);
        assert_ne!(components[0], 0);
        assert_ne!(components[1], 0);
        assert_ne!(components[2], 0);
    }
}