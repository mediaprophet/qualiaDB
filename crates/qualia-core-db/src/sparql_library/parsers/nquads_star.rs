use crate::lexicon::{generate_60bit_token, generate_embedded_triple_id};
use crate::rdf_star::{RdfStarParseError, RdfStarParser};
/// N-Quads-Star Parser for QualiaDB
///
/// Implements RDF-Star (SPARQL 1.2) parsing for N-Quads syntax with embedded triples.
/// N-Quads-Star extends N-Triples-Star with a fourth component (graph/context).
/// Format: `<subject> <predicate> <object> <graph> .`
use crate::NQuin;

/// Human-readable term strings from the most recent successful parse.
#[derive(Debug, Clone, Default)]
pub struct NQuadsLineTerms {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub graph: String,
    pub outer_predicate: String,
    pub outer_object: String,
    pub outer_graph: String,
}

/// N-Quads-Star parser implementation
pub struct NQuadsStarParser {
    /// Context hash for the current parsing session
    context_hash: u64,
    last_terms: Option<NQuadsLineTerms>,
}

impl NQuadsStarParser {
    /// Create a new N-Quads-Star parser
    pub fn new(context_hash: u64) -> Self {
        Self {
            context_hash,
            last_terms: None,
        }
    }

    /// Session default graph hash (used when a line omits an explicit graph).
    pub fn session_context(&self) -> u64 {
        self.context_hash
    }

    /// Term strings from the last successfully parsed line.
    pub fn last_line_terms(&self) -> Option<&NQuadsLineTerms> {
        self.last_terms.as_ref()
    }

    fn graph_hash_from_token(&self, graph: &str) -> u64 {
        if graph.is_empty() {
            self.context_hash
        } else {
            generate_60bit_token(graph.as_bytes())
        }
    }

    fn store_regular_terms(
        &mut self,
        subject_str: &str,
        predicate_str: &str,
        object_str: &str,
        graph_str: &str,
    ) {
        self.last_terms = Some(NQuadsLineTerms {
            subject: subject_str.to_string(),
            predicate: predicate_str.to_string(),
            object: object_str.to_string(),
            graph: graph_str.to_string(),
            ..Default::default()
        });
    }

    fn store_embedded_terms(
        &mut self,
        outer_predicate_str: &str,
        outer_object_str: &str,
        outer_graph_str: &str,
    ) {
        self.last_terms = Some(NQuadsLineTerms {
            outer_predicate: outer_predicate_str.to_string(),
            outer_object: outer_object_str.to_string(),
            outer_graph: outer_graph_str.to_string(),
            ..Default::default()
        });
    }

    /// Parse an N-Quads line (subject, predicate, object, graph)
    fn parse_line(&mut self, line: &str) -> Result<ParseResult, RdfStarParseError> {
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

    /// Parse a regular N-Quads line.
    ///
    /// Accepts standards-valid default-graph triples (`s p o .`) and named-graph
    /// quads (`s p o g .`). A trailing `.` may be a separate token or omitted when
    /// tools emit bare terms (rio-compatible).
    fn parse_quad_line(&mut self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        let parts = tokenize_nquads_terms(line)?;
        if parts.is_empty() {
            return Err(RdfStarParseError::InvalidSyntax);
        }
        let ends_dot = parts.last().copied() == Some(".");
        let terms = if ends_dot {
            &parts[..parts.len() - 1]
        } else {
            &parts[..]
        };
        // Triple in the default graph, or quad with an explicit graph IRI.
        let (subject_str, predicate_str, object_str, graph_str) = match terms.len() {
            3 => (terms[0], terms[1], terms[2], ""),
            4 => (terms[0], terms[1], terms[2], terms[3]),
            _ => return Err(RdfStarParseError::InvalidSyntax),
        };

        let subject = normalise_term(subject_str);
        let predicate = normalise_term(predicate_str);
        let object = normalise_term(object_str);
        let graph = normalise_term(graph_str);

        let subject_hash = generate_60bit_token(subject.as_bytes());
        let predicate_hash = generate_60bit_token(predicate.as_bytes());
        let object_hash = generate_60bit_token(object.as_bytes());
        let graph_hash = self.graph_hash_from_token(graph);
        self.store_regular_terms(subject, predicate, object, graph);

        Ok(ParseResult::RegularQuad {
            subject: subject_hash,
            predicate: predicate_hash,
            object: object_hash,
            graph: graph_hash,
        })
    }

    /// Parse an embedded triple line with graph context
    fn parse_embedded_triple_line(&mut self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: <<<subject> <predicate> <object>>> <predicate> <object> <graph> .

        // Find the closing >>> for the embedded triple
        let end_embedded = line
            .find(">>>")
            .ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;

        // Extract the embedded triple part
        let embedded_part = &line[3..end_embedded]; // Skip <<<

        // Parse the embedded triple components
        let embedded_parts: Vec<&str> = embedded_part.split_whitespace().collect();
        if embedded_parts.len() < 3 {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }

        let subject = embedded_parts[0]
            .trim_start_matches('<')
            .trim_end_matches('>');
        let predicate = embedded_parts[1]
            .trim_start_matches('<')
            .trim_end_matches('>');
        let object = embedded_parts[2]
            .trim_start_matches('<')
            .trim_end_matches('>');

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
        let outer_graph_hash = self.graph_hash_from_token(outer_graph);
        self.store_embedded_terms(outer_predicate, outer_object, outer_graph);

        Ok(ParseResult::EmbeddedQuad {
            virtual_id,
            components: [subject_hash, predicate_hash, object_hash],
            outer_predicate: outer_predicate_hash,
            outer_object: outer_object_hash,
            outer_graph: outer_graph_hash,
        })
    }
}

/// Return N-Quads terms without splitting the lexical form of a literal.
///
/// A literal can contain spaces and escaped quotes, and can be followed by a
/// language tag or datatype IRI. Those components are one RDF term and must
/// remain one Q42LEX entry.
fn tokenize_nquads_terms(line: &str) -> Result<Vec<&str>, RdfStarParseError> {
    let bytes = line.as_bytes();
    let mut terms = Vec::new();
    let mut cursor = 0;

    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] == b'#' {
            break;
        }

        let start = cursor;
        match bytes[cursor] {
            b'"' => {
                cursor += 1;
                let mut escaped = false;
                let mut closed = false;
                while cursor < bytes.len() {
                    let byte = bytes[cursor];
                    cursor += 1;
                    if escaped {
                        escaped = false;
                    } else if byte == b'\\' {
                        escaped = true;
                    } else if byte == b'"' {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err(RdfStarParseError::InvalidSyntax);
                }
                if cursor < bytes.len() && bytes[cursor] == b'@' {
                    cursor += 1;
                    while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
                        cursor += 1;
                    }
                } else if cursor + 1 < bytes.len()
                    && bytes[cursor] == b'^'
                    && bytes[cursor + 1] == b'^'
                {
                    cursor += 2;
                    if cursor >= bytes.len() || bytes[cursor] != b'<' {
                        return Err(RdfStarParseError::InvalidSyntax);
                    }
                    cursor += 1;
                    while cursor < bytes.len() && bytes[cursor] != b'>' {
                        cursor += 1;
                    }
                    if cursor >= bytes.len() {
                        return Err(RdfStarParseError::InvalidSyntax);
                    }
                    cursor += 1;
                }
            }
            b'<' => {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor] != b'>' {
                    cursor += 1;
                }
                if cursor >= bytes.len() {
                    return Err(RdfStarParseError::InvalidSyntax);
                }
                cursor += 1;
            }
            _ => {
                while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
            }
        }
        terms.push(&line[start..cursor]);
    }

    Ok(terms)
}

fn normalise_term(term: &str) -> &str {
    term.strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
        .unwrap_or(term)
}

impl RdfStarParser for NQuadsStarParser {
    fn parse_embedded_triple(
        &mut self,
        input: &[u8],
    ) -> Result<(u64, [u64; 3]), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;

        match self.parse_line(line)? {
            ParseResult::EmbeddedQuad {
                virtual_id,
                components,
                ..
            } => Ok((virtual_id, components)),
            _ => Err(RdfStarParseError::MalformedEmbeddedTriple),
        }
    }

    fn parse_triple(&mut self, input: &[u8]) -> Result<(u64, u64, u64), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;

        match self.parse_line(line)? {
            ParseResult::RegularQuad {
                subject,
                predicate,
                object,
                ..
            } => Ok((subject, predicate, object)),
            _ => Err(RdfStarParseError::InvalidSyntax),
        }
    }

    fn parse_quad(&mut self, input: &[u8]) -> Result<(u64, u64, u64, u64), RdfStarParseError> {
        let line = std::str::from_utf8(input).map_err(|_| RdfStarParseError::InvalidUtf8)?;

        match self.parse_line(line)? {
            ParseResult::RegularQuad {
                subject,
                predicate,
                object,
                graph,
                ..
            } => Ok((subject, predicate, object, graph)),
            ParseResult::EmbeddedQuad {
                outer_predicate,
                outer_object,
                outer_graph,
                ..
            } => Ok((0, outer_predicate, outer_object, outer_graph)),
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
    },
    EmbeddedQuad {
        virtual_id: u64,
        components: [u64; 3],
        outer_predicate: u64,
        outer_object: u64,
        outer_graph: u64,
    },
}

/// Parse N-Quads-Star into any `QuinSink()`.
pub fn parse_nquads_star_into<R: std::io::Read, S: crate::sparql_library::quin_sink::QuinSink>(
    reader: R,
    context_hash: u64,
    sink: &mut S,
) -> Result<u64, Box<dyn std::error::Error>> {
    use std::io::BufRead;

    let mut parser = NQuadsStarParser::new(context_hash);
    let mut count = 0;
    let buf_reader = BufReader::new(reader);
    let mut line_no: u32 = 0;

    for line in buf_reader.lines() {
        let line = line?;
        line_no = line_no.saturating_add(1);
        let parsed = match parser.parse_line(&line) {
            Ok(p) => p,
            Err(RdfStarParseError::InvalidSyntax) => {
                let snippet: String = line.chars().take(120).collect();
                return Err(format!(
                    "N-Quads InvalidSyntax at line {line_no}: expected `s p o .` (default graph) or `s p o g .` (named graph); got: {snippet}"
                )
                .into());
            }
            Err(e) => return Err(e.into()),
        };
        match parsed {
            ParseResult::Comment => continue,
            ParseResult::RegularQuad {
                subject,
                predicate,
                object,
                graph,
                ..
            } => {
                if let Some(t) = parser.last_line_terms() {
                    sink.push_lex(subject, &t.subject);
                    sink.push_lex(predicate, &t.predicate);
                    sink.push_lex(object, &t.object);
                    if !t.graph.is_empty() {
                        sink.push_lex(graph, &t.graph);
                    }
                }
                let metadata = 0b10 << 61;
                sink.push(NQuin {
                    subject,
                    predicate,
                    object,
                    context: graph, // Use graph as context in NQuin
                    metadata,
                    parity: NQuin::calculate_parity(subject, predicate, object, graph, metadata),
                })?;
                count += 1;
            }
            ParseResult::EmbeddedQuad {
                virtual_id,
                components,
                outer_predicate,
                outer_object,
                outer_graph,
                ..
            } => {
                if let Some(t) = parser.last_line_terms() {
                    if !t.outer_predicate.is_empty() {
                        sink.push_lex(outer_predicate, &t.outer_predicate);
                    }
                    if !t.outer_object.is_empty() {
                        sink.push_lex(outer_object, &t.outer_object);
                    }
                    if !t.outer_graph.is_empty() {
                        sink.push_lex(outer_graph, &t.outer_graph);
                    }
                }
                let metadata = 0b10 << 61;
                sink.push(NQuin {
                    subject: virtual_id,
                    predicate: outer_predicate,
                    object: outer_object,
                    context: outer_graph,
                    metadata,
                    parity: NQuin::calculate_parity(
                        virtual_id,
                        outer_predicate,
                        outer_object,
                        outer_graph,
                        metadata,
                    ),
                })?;
                count += 1;

                sink.push(NQuin {
                    subject: components[0],
                    predicate: components[1],
                    object: components[2],
                    context: outer_graph,
                    metadata,
                    parity: NQuin::calculate_parity(
                        components[0],
                        components[1],
                        components[2],
                        outer_graph,
                        metadata,
                    ),
                })?;
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Parse N-Quads-Star stream via external sort.
pub fn parse_nquads_star_stream<R: std::io::Read>(
    reader: R,
    context_hash: u64,
    sorter: &mut crate::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    parse_nquads_star_into(reader, context_hash, sorter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdf_star::RdfStarParser;

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
    fn accepts_triple_only_default_graph_nquads() {
        // RDF dataset interchange often ships default-graph triples inside .nq files.
        let mut parser = NQuadsStarParser::new(0xABCDu64);
        let input = b"<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .";
        let (s, p, o, g) = parser.parse_quad(input).expect("triple-only N-Quads");
        assert_ne!(s, 0);
        assert_ne!(p, 0);
        assert_ne!(o, 0);
        assert_eq!(g, 0xABCDu64, "omitted graph uses session default");
    }

    #[test]
    fn rejects_malformed_nquads_with_clear_failure() {
        let mut parser = NQuadsStarParser::new(0);
        assert!(parser
            .parse_quad(b"<http://example.org/Alice> <http://example.org/knows> .")
            .is_err());
    }

    #[test]
    fn accepts_literals_with_spaces_language_and_datatype() {
        let mut parser = NQuadsStarParser::new(0);
        for input in [
            b"<https://example.org/s> <https://example.org/p> \"A plain literal\" .".as_slice(),
            b"<https://example.org/s> <https://example.org/p> \"Bonjour le monde\"@fr .".as_slice(),
            b"<https://example.org/s> <https://example.org/p> \"2026\"^^<http://www.w3.org/2001/XMLSchema#integer> .".as_slice(),
        ] {
            let (_, _, object, _) = parser.parse_quad(input).expect("literal N-Quads");
            assert_ne!(object, 0);
            assert!(parser.last_line_terms().unwrap().object.starts_with('\"'));
        }
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
use std::io::BufReader;
