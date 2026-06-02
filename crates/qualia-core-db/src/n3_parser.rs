use std::io::{BufRead, Error, ErrorKind};

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Uri(String),
    Variable(String),
    Literal(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Triple {
    pub subject: Term,
    pub predicate: Term,
    pub object: Term,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    pub triples: Vec<Triple>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleType {
    Strict,
    Defeasible,
    Defeater,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub id: Option<String>,
    pub rule_type: RuleType,
    pub premise: Formula,
    pub conclusion: Formula,
}

#[derive(Debug)]
pub enum N3Event {
    StaticTriple(Triple),
    LogicRule(Rule),
}

/// A highly constrained, native MVP N3 parser.
/// It splits the file into lines, extracts basic triples and implication rules.
/// In a production environment, this would be a full recursive descent AST parser.
pub struct N3Parser<R: BufRead> {
    reader: R,
}

impl<R: BufRead> N3Parser<R> {
    pub fn new(reader: R) -> Self {
        N3Parser { reader }
    }

    pub fn parse_all<F>(&mut self, mut callback: F) -> Result<(), Error>
    where
        F: FnMut(N3Event) -> Result<(), Error>,
    {
        let mut buffer = String::new();
        while self.reader.read_line(&mut buffer)? > 0 {
            let line = buffer.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("@prefix") {
                buffer.clear();
                continue;
            }

            // Heuristic for Rule: contains => or ~> or ^> and braces
            if line.contains("=>") || line.contains("~>") || line.contains("^>") {
                if let Some(rule) = Self::parse_rule(line) {
                    callback(N3Event::LogicRule(rule))?;
                }
            } else {
                // Heuristic for standard Triple
                if let Some(triple) = Self::parse_triple(line) {
                    callback(N3Event::StaticTriple(triple))?;
                }
            }
            buffer.clear();
        }
        Ok(())
    }

    fn parse_rule(line: &str) -> Option<Rule> {
        let mut clean_line = line.trim();
        let mut id = None;

        // Extract optional [rule_id]
        if clean_line.starts_with('[') {
            if let Some(end_idx) = clean_line.find(']') {
                id = Some(clean_line[1..end_idx].to_string());
                clean_line = clean_line[end_idx + 1..].trim();
            }
        }

        let (separator, rule_type) = if clean_line.contains("=>") {
            ("=>", RuleType::Strict)
        } else if clean_line.contains("~>") {
            ("~>", RuleType::Defeasible)
        } else if clean_line.contains("^>") {
            ("^>", RuleType::Defeater)
        } else {
            return None;
        };

        let parts: Vec<&str> = clean_line.split(separator).collect();
        if parts.len() != 2 {
            return None;
        }

        let premise_str = parts[0].trim().trim_matches(|c| c == '{' || c == '}');
        let conclusion_str = parts[1].trim().trim_end_matches('.').trim().trim_matches(|c| c == '{' || c == '}');

        let premise = Formula { triples: Self::parse_formula_triples(premise_str) };
        let conclusion = Formula { triples: Self::parse_formula_triples(conclusion_str) };

        Some(Rule { id, rule_type, premise, conclusion })
    }

    fn parse_formula_triples(content: &str) -> Vec<Triple> {
        let mut triples = Vec::new();
        // Just extract single triple for MVP (this assumes one triple per formula for now)
        if let Some(t) = Self::parse_triple(content) {
            triples.push(t);
        }
        triples
    }

    fn parse_triple(line: &str) -> Option<Triple> {
        let clean_line = line.trim_end_matches('.').trim();
        let tokens: Vec<&str> = clean_line.split_whitespace().collect();
        if tokens.len() >= 3 {
            Some(Triple {
                subject: Self::parse_term(tokens[0]),
                predicate: Self::parse_term(tokens[1]),
                object: Self::parse_term(tokens[2]),
            })
        } else {
            None
        }
    }

    fn parse_term(token: &str) -> Term {
        if token.starts_with('?') {
            Term::Variable(token.to_string())
        } else if token.starts_with('"') {
            Term::Literal(token.to_string())
        } else {
            Term::Uri(token.to_string())
        }
    }
}
