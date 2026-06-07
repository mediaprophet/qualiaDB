use std::io::{BufRead, Error};

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
    Linear,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub id: Option<String>,
    pub rule_type: RuleType,
    pub weight: Option<f32>,
    pub premise: Formula,
    pub conclusion: Formula,
}

#[derive(Debug)]
pub enum N3Event {
    StaticTriple(Triple),
    LogicRule(Rule),
    AspBlock(String),
    DiffuseBlock(String),
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
        let mut in_asp_block = false;
        let mut asp_content = String::new();
        let mut in_diffuse_block = false;
        let mut diffuse_content = String::new();

        while self.reader.read_line(&mut buffer)? > 0 {
            let line = buffer.trim();
            
            if in_asp_block {
                if line == "}" {
                    in_asp_block = false;
                    callback(N3Event::AspBlock(asp_content.clone()))?;
                    asp_content.clear();
                } else {
                    asp_content.push_str(line);
                    asp_content.push('\n');
                }
                buffer.clear();
                continue;
            }
            
            if in_diffuse_block {
                if line == "}" {
                    in_diffuse_block = false;
                    callback(N3Event::DiffuseBlock(diffuse_content.clone()))?;
                    diffuse_content.clear();
                } else {
                    diffuse_content.push_str(line);
                    diffuse_content.push('\n');
                }
                buffer.clear();
                continue;
            }

            if line.starts_with("#asp {") {
                in_asp_block = true;
                buffer.clear();
                continue;
            }
            
            if line.starts_with("qualia:diffuse {") {
                in_diffuse_block = true;
                buffer.clear();
                continue;
            }

            if line.is_empty() || line.starts_with('#') || line.starts_with("@prefix") {
                buffer.clear();
                continue;
            }

            // Heuristic for Rule: contains => or ~> or ^> or -o and braces
            if line.contains("=>") || line.contains("~>") || line.contains("^>") || line.contains("-o") {
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
        let mut weight = None;

        // Extract optional [rule_id]
        if clean_line.starts_with('[') {
            if let Some(end_idx) = clean_line.find(']') {
                id = Some(clean_line[1..end_idx].to_string());
                clean_line = clean_line[end_idx + 1..].trim();
            }
        }

        // Extract optional (weight) e.g. (0.8)
        if clean_line.starts_with('(') {
            if let Some(end_idx) = clean_line.find(')') {
                if let Ok(w) = clean_line[1..end_idx].parse::<f32>() {
                    weight = Some(w);
                }
                clean_line = clean_line[end_idx + 1..].trim();
            }
        }

        let (separator, rule_type) = if clean_line.contains("=>") {
            ("=>", RuleType::Strict)
        } else if clean_line.contains("~>") {
            ("~>", RuleType::Defeasible)
        } else if clean_line.contains("^>") {
            ("^>", RuleType::Defeater)
        } else if clean_line.contains("-o") {
            ("-o", RuleType::Linear)
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

        Some(Rule { id, rule_type, weight, premise, conclusion })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_events(input: &str) -> Vec<N3Event> {
        let cursor = std::io::Cursor::new(input.as_bytes());
        let mut parser = N3Parser::new(cursor);
        let mut events = Vec::new();
        parser.parse_all(|event| {
            events.push(event);
            Ok(())
        }).unwrap();
        events
    }

    #[test]
    fn parses_weighted_rule_with_id_and_defeasible_arrow() {
        let events = collect_events("[r1] (0.8) { ?s a ?t } ~> { ?s a ?t } .");
        match &events[0] {
            N3Event::LogicRule(rule) => {
                assert_eq!(rule.id.as_deref(), Some("r1"));
                assert_eq!(rule.weight, Some(0.8));
                assert_eq!(rule.rule_type, RuleType::Defeasible);
                assert_eq!(rule.premise.triples.len(), 1);
                assert_eq!(rule.conclusion.triples.len(), 1);
            }
            other => panic!("expected logic rule, got {:?}", other),
        }
    }

    #[test]
    fn parses_defeater_and_linear_rules() {
        let defeater = collect_events("{ ?x a <http://ex.org/Exc> } ^> { ?x <http://ex.org/applies> false } .");
        let linear = collect_events("{ ?x <http://ex.org/token> ?t } -o { ?x <http://ex.org/used> true } .");

        match &defeater[0] {
            N3Event::LogicRule(rule) => assert_eq!(rule.rule_type, RuleType::Defeater),
            other => panic!("expected defeater rule, got {:?}", other),
        }
        match &linear[0] {
            N3Event::LogicRule(rule) => assert_eq!(rule.rule_type, RuleType::Linear),
            other => panic!("expected linear rule, got {:?}", other),
        }
    }

    #[test]
    fn parses_asp_and_diffuse_blocks() {
        let events = collect_events("#asp {\nanswer_set.\n}\nqualia:diffuse {\nwavefront.\n}\n");
        assert!(matches!(&events[0], N3Event::AspBlock(body) if body.contains("answer_set.")));
        assert!(matches!(&events[1], N3Event::DiffuseBlock(body) if body.contains("wavefront.")));
    }

    #[test]
    fn static_triple_is_emitted() {
        let events = collect_events("<http://ex.org/Alice> <http://ex.org/knows> <http://ex.org/Bob> .");
        match &events[0] {
            N3Event::StaticTriple(triple) => {
                assert!(matches!(triple.subject, Term::Uri(_)));
                assert!(matches!(triple.predicate, Term::Uri(_)));
                assert!(matches!(triple.object, Term::Uri(_)));
            }
            other => panic!("expected static triple, got {:?}", other),
        }
    }
}
