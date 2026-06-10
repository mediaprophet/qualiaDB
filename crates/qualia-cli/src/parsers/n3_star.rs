//! N3-Star Parser for QualiaDB
//!
//! Implements RDF-Star (SPARQL 1.2) parsing for N3 syntax with embedded triples.
//! N3-Star extends Turtle-Star with formulae, variables, and rules.

use qualia_core_db::QualiaQuin;
use qualia_core_db::lexicon::{generate_embedded_triple_id, generate_60bit_token};
use qualia_core_db::rdf_star::{RdfStarParser, RdfStarParseError};

/// N3-Star parser implementation
pub struct N3StarParser {
    /// Context hash for the current parsing session
    context_hash: u64,
    /// Variable bindings
    variables: std::collections::HashMap<String, u64>,
}

impl N3StarParser {
    /// Create a new N3-Star parser
    pub fn new(context_hash: u64) -> Self {
        Self {
            context_hash,
            variables: std::collections::HashMap::new(),
        }
    }

    /// Bind a variable to a value
    pub fn bind_variable(&mut self, var_name: &str, value: u64) {
        self.variables.insert(var_name.to_string(), value);
    }

    /// Get a variable binding
    pub fn get_variable(&self, var_name: &str) -> Option<u64> {
        self.variables.get(var_name).copied()
    }

    /// Parse an N3 line
    fn parse_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(ParseResult::Comment);
        }

        // Check for rule implications
        if line.contains("=>") || line.contains("~>") || line.contains("^>") || line.contains("-o") {
            self.parse_rule(line)
        } else if line.contains("{") {
            self.parse_formula(line)
        } else if line.contains("<<") {
            self.parse_embedded_triple_line(line)
        } else {
            self.parse_triple_line(line)
        }
    }

    /// Parse a rule with implication
    fn parse_rule(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: { premise } => { conclusion }
        // For now, we'll just recognize it and return a placeholder
        let rule_type = if line.contains("~>") {
            RuleType::Defeasible
        } else if line.contains("^>") {
            RuleType::Defeater
        } else if line.contains("-o") {
            RuleType::Linear
        } else {
            RuleType::Strict
        };

        Ok(ParseResult::Rule { rule_type })
    }

    /// Parse a formula (collection of triples)
    fn parse_formula(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        // Format: { s p o . s2 p2 o2 . }
        // For now, just recognize it
        Ok(ParseResult::Formula)
    }

    /// Parse a regular N3 triple
    fn parse_triple_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(RdfStarParseError::InvalidSyntax);
        }

        let subject = self.resolve_term(parts[0])?;
        let predicate = self.resolve_term(parts[1])?;
        let object = self.resolve_term(parts[2])?;

        Ok(ParseResult::RegularTriple {
            subject,
            predicate,
            object,
            graph_hash: self.context_hash,
        })
    }

    /// Parse an embedded triple line
    fn parse_embedded_triple_line(&self, line: &str) -> Result<ParseResult, RdfStarParseError> {
        let start = line.find("<<").ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;
        let end = line.find(">>>").ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;
        
        let embedded_part = &line[start + 2..end];
        let embedded_parts: Vec<&str> = embedded_part.split_whitespace().collect();
        if embedded_parts.len() < 3 {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }

        let subject = self.resolve_term(embedded_parts[0])?;
        let predicate = self.resolve_term(embedded_parts[1])?;
        let object = self.resolve_term(embedded_parts[2])?;

        let virtual_id = generate_embedded_triple_id(subject, predicate, object);

        let remaining = &line[end + 3..];
        let outer_parts: Vec<&str> = remaining.split_whitespace().collect();
        if outer_parts.len() < 2 {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }

        let outer_predicate = self.resolve_term(outer_parts[0])?;
        let outer_object = self.resolve_term(outer_parts[1])?;

        Ok(ParseResult::EmbeddedTriple {
            virtual_id,
            components: [subject, predicate, object],
            outer_predicate,
            outer_object,
            graph_hash: self.context_hash,
        })
    }

    /// Resolve a term (IRI, blank node, or variable)
    fn resolve_term(&self, term: &str) -> Result<u64, RdfStarParseError> {
        // Check for variable
        if term.starts_with('?') {
            if let Some(value) = self.get_variable(term) {
                return Ok(value);
            }
            // Generate a placeholder hash for unbound variables
            return Ok(generate_60bit_token(term.as_bytes()));
        }

        // Strip angle brackets and quotes
        let term = term.trim_start_matches('<').trim_end_matches('>')
            .trim_start_matches('"').trim_end_matches('"');

        Ok(generate_60bit_token(term.as_bytes()))
    }
}

impl RdfStarParser for N3StarParser {
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
        false // N3 doesn't have named graphs like Trig
    }

    fn format_name(&self) -> &'static str {
        "N3-Star"
    }
}

/// Rule types in N3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleType {
    /// Classical modus ponens (=>)
    Strict,
    /// Defeasible (~>)
    Defeasible,
    /// Defeater (^>)
    Defeater,
    /// Linear logic (-o)
    Linear,
}

/// Parse result for N3-Star
enum ParseResult {
    Comment,
    Formula,
    Rule {
        rule_type: RuleType,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_n3_star_parser_creation() {
        let parser = N3StarParser::new(0);
        assert_eq!(parser.format_name(), "N3-Star");
        assert!(parser.supports_quads());
        assert!(!parser.supports_named_graphs());
    }

    #[test]
    fn test_variable_binding() {
        let mut parser = N3StarParser::new(0);
        parser.bind_variable("?x", 123);
        assert_eq!(parser.get_variable("?x"), Some(123));
    }

    #[test]
    fn test_parse_regular_triple() {
        let parser = N3StarParser::new(0);
        let input = b"<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .";
        let result = parser.parse_triple(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_embedded_triple() {
        let parser = N3StarParser::new(0);
        let input = b"<<http://example.org/Alice http://example.org/knows http://example.org/Bob>> http://example.org/saidBy http://example.org/Charlie .";
        let result = parser.parse_embedded_triple(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_rule() {
        let parser = N3StarParser::new(0);
        let input = b"{ ?x a :Person } => { ?x :hasName ?name } .";
        let result = parser.parse_line(std::str::from_utf8(input).unwrap());
        assert!(result.is_ok());
    }
}