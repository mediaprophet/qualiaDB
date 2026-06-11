/// Trig-Star Parser for QualiaDB
use std::io::BufReader;
///
/// Implements RDF-Star (SPARQL 1.2) parsing for Trig syntax with embedded triples.
/// Trig-Star extends Turtle-Star with named graph support via GRAPH {} blocks.

use crate::NQuin;
use crate::lexicon::{generate_embedded_triple_id, generate_60bit_token};
use crate::rdf_star::{RdfStarParser, RdfStarParseError};

/// Trig-Star parser implementation
pub struct TrigStarParser {
    /// Context hash for the current parsing session
    context_hash: u64,
    /// Current named graph hash
    current_graph: u64,
}

impl TrigStarParser {
    /// Create a new Trig-Star parser
    pub fn new(context_hash: u64) -> Self {
        Self {
            context_hash,
            current_graph: 0, // Default graph
        }
    }

    /// Set the current named graph
    pub fn set_current_graph(&mut self, graph_hash: u64) {
        self.current_graph = graph_hash;
    }

    /// Get the current named graph
    pub fn current_graph(&self) -> u64 {
        self.current_graph
    }

    /// Parse a Trig line (subject, predicate, object) in current graph context
    fn parse_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(ParseResult::Comment);
        }

        // Check for GRAPH keyword
        if line.starts_with("GRAPH") {
            return self.parse_graph_block(line);
        }

        // Check for embedded triple start marker
        if line.contains("<<") {
            self.parse_embedded_triple_line(line)
        } else {
            self.parse_triple_line(line)
        }
    }

    /// Parse a GRAPH block declaration
    fn parse_graph_block(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: GRAPH <graph_uri> { ... }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(RdfStarParseError::InvalidSyntax);
        }

        let graph_str = parts[1].trim_start_matches('<').trim_end_matches('>');
        let graph_hash = generate_60bit_token(graph_str.as_bytes());

        Ok(ParseResult::GraphDeclaration { graph_hash })
    }

    /// Parse a regular Trig triple
    fn parse_triple_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: subject predicate object .
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(RdfStarParseError::InvalidSyntax);
        }

        let subject_str = parts[0];
        let predicate_str = parts[1];
        let object_str = parts[2];

        // Strip angle brackets and quotes
        let subject = subject_str.trim_start_matches('<').trim_end_matches('>');
        let predicate = predicate_str.trim_start_matches('<').trim_end_matches('>');
        let object = object_str.trim_start_matches('<').trim_end_matches('>')
            .trim_start_matches('"').trim_end_matches('"');

        let subject_hash = generate_60bit_token(subject.as_bytes());
        let predicate_hash = generate_60bit_token(predicate.as_bytes());
        let object_hash = generate_60bit_token(object.as_bytes());

        Ok(ParseResult::RegularTriple {
            subject: subject_hash,
            predicate: predicate_hash,
            object: object_hash,
            graph_hash: self.current_graph,
        })
    }

    /// Parse an embedded triple line
    fn parse_embedded_triple_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Similar to Turtle-Star but with graph context
        // Find the embedded triple
        let start = line.find("<<").ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;
        let end = line.find(">>").ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;
        
        let embedded_part = &line[start + 2..end];
        let embedded_parts: Vec<&str> = embedded_part.split_whitespace().collect();
        if embedded_parts.len() < 3 {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }

        let subject = embedded_parts[0].trim_start_matches('<').trim_end_matches('>');
        let predicate = embedded_parts[1].trim_start_matches('<').trim_end_matches('>');
        let object = embedded_parts[2].trim_start_matches('<').trim_end_matches('>')
            .trim_start_matches('"').trim_end_matches('"');

        let subject_hash = generate_60bit_token(subject.as_bytes());
        let predicate_hash = generate_60bit_token(predicate.as_bytes());
        let object_hash = generate_60bit_token(object.as_bytes());

        let virtual_id = generate_embedded_triple_id(subject_hash, predicate_hash, object_hash);

        // Parse the outer triple (after >>)
        let remaining = &line[end + 2..];
        let outer_parts: Vec<&str> = remaining.split_whitespace().collect();
        if outer_parts.len() < 2 {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }

        let outer_predicate = outer_parts[0].trim_start_matches('<').trim_end_matches('>');
        let outer_object = outer_parts[1].trim_start_matches('<').trim_end_matches('>')
            .trim_start_matches('"').trim_end_matches('"');

        let outer_predicate_hash = generate_60bit_token(outer_predicate.as_bytes());
        let outer_object_hash = generate_60bit_token(outer_object.as_bytes());

        Ok(ParseResult::EmbeddedTriple {
            virtual_id,
            components: [subject_hash, predicate_hash, object_hash],
            outer_predicate: outer_predicate_hash,
            outer_object: outer_object_hash,
            graph_hash: self.current_graph,
        })
    }
}

impl RdfStarParser for TrigStarParser {
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

    fn parse_quad(&mut self, input: &[u8]) -> Result<(u64, u64, u64, u64), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;
        
        match self.parse_line(line)? {
            ParseResult::RegularTriple { subject, predicate, object, graph_hash } => {
                Ok((subject, predicate, object, graph_hash))
            }
            ParseResult::EmbeddedTriple { outer_predicate, outer_object, graph_hash, .. } => {
                Ok((0, outer_predicate, outer_object, graph_hash))
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
        "Trig-Star"
    }
}

/// Parse result for Trig-Star
enum ParseResult {
    Comment,
    GraphDeclaration {
        graph_hash: u64,
    },
    RegularTriple {
        subject: u64,
        predicate: u64,
        object: u64,
        graph_hash: u64,
    },
    EmbeddedTriple {
        virtual_id: u64,
        components: [u64; 3],
        outer_predicate: u64,
        outer_object: u64,
        graph_hash: u64,
    },
}

/// Parse Trig-Star stream and emit Quins
pub fn parse_trig_star_stream<R: std::io::Read>(
    reader: R,
    context_hash: u64,
    sorter: &mut crate::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    use std::io::BufRead;
    
    let mut parser = TrigStarParser::new(context_hash);
    let mut count = 0;
    let buf_reader = BufReader::new(reader);

    for line in buf_reader.lines() {
        let line = line?;
        match parser.parse_line(&line)? {
            ParseResult::Comment => continue,
            ParseResult::GraphDeclaration { graph_hash } => {
                parser.set_current_graph(graph_hash);
            }
            ParseResult::RegularTriple { subject, predicate, object, graph_hash } => {
                sorter.push(NQuin {
                    subject,
                    predicate,
                    object,
                    context: graph_hash,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
            }
            ParseResult::EmbeddedTriple { virtual_id, components, outer_predicate, outer_object, graph_hash } => {
                sorter.push(NQuin {
                    subject: virtual_id,
                    predicate: outer_predicate,
                    object: outer_object,
                    context: graph_hash,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
                
                sorter.push(NQuin {
                    subject: components[0],
                    predicate: components[1],
                    object: components[2],
                    context: graph_hash,
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
    use crate::rdf_star::{RdfStarParser, RdfStarSerializer};
    use super::*;

    #[test]
    fn test_trig_star_parser_creation() {
        let parser = TrigStarParser::new(0);
        assert_eq!(parser.format_name(), "Trig-Star");
        assert!(parser.supports_quads());
        assert!(parser.supports_named_graphs());
    }

    #[test]
    fn test_set_current_graph() {
        let mut parser = TrigStarParser::new(0);
        parser.set_current_graph(123);
        assert_eq!(parser.current_graph(), 123);
    }

    #[test]
    fn test_parse_regular_triple() {
        let mut parser = TrigStarParser::new(0);
        let input = b"<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .";
        let result = parser.parse_triple(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_embedded_triple() {
        let mut parser = TrigStarParser::new(0);
        let input = b"<<http://example.org/Alice http://example.org/knows http://example.org/Bob>> http://example.org/saidBy http://example.org/Charlie .";
        let result = parser.parse_embedded_triple(input);
        assert!(result.is_ok());
    }
}
