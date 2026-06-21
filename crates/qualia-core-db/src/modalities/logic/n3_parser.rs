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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

        // Accumulator for a single (possibly multi-line) N3 statement. A statement
        // terminates on a top-level `.` — i.e. brace depth back to zero — so a
        // multi-triple, multi-line rule premise (e.g. agency.n3 G1) assembles into
        // one `Rule` rather than being lost line-by-line.
        let mut stmt = String::new();
        let mut brace_depth: i32 = 0;

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

            // Block openers are only recognised between statements (never while a
            // brace-balanced accumulation is in progress).
            if stmt.is_empty() && line.starts_with("#asp {") {
                in_asp_block = true;
                buffer.clear();
                continue;
            }
            if stmt.is_empty() && line.starts_with("qualia:diffuse {") {
                in_diffuse_block = true;
                buffer.clear();
                continue;
            }

            // Skip blank / full-line-comment / @prefix lines — including mid-statement
            // (a comment line may sit between a rule's premise triples).
            if line.is_empty() || line.starts_with('#') || line.starts_with("@prefix") {
                buffer.clear();
                continue;
            }

            // Strip a trailing inline comment. URIs/IRIs contain no spaces, so " #"
            // unambiguously begins a comment for the rule/fact shapes we parse.
            let line = match line.find(" #") {
                Some(idx) => line[..idx].trim_end(),
                None => line,
            };

            if !line.is_empty() {
                if !stmt.is_empty() {
                    stmt.push(' ');
                }
                stmt.push_str(line);
                for ch in line.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => brace_depth -= 1,
                        _ => {}
                    }
                }
            }

            // Complete when braces balance AND the statement ends with `.`.
            if brace_depth <= 0 && stmt.trim_end().ends_with('.') {
                Self::dispatch_statement(&stmt, &mut callback)?;
                stmt.clear();
                brace_depth = 0;
            }

            buffer.clear();
        }

        // Best-effort flush of a trailing statement that lacked a final `.`.
        if !stmt.trim().is_empty() {
            Self::dispatch_statement(&stmt, &mut callback)?;
        }
        Ok(())
    }

    /// Classify one fully-accumulated statement and emit the corresponding event(s).
    /// A logic rule (`=>` / `~>` / `^>` / ` -o `) becomes one `LogicRule`; anything
    /// else is parsed as one-or-more `StaticTriple`s (`;`/`,`/`.` lists expanded).
    fn dispatch_statement<F>(stmt: &str, callback: &mut F) -> Result<(), Error>
    where
        F: FnMut(N3Event) -> Result<(), Error>,
    {
        let s = stmt.trim();
        if s.is_empty() {
            return Ok(());
        }
        let looks_like_rule =
            s.contains("=>") || s.contains("~>") || s.contains("^>") || s.contains(" -o ");
        if looks_like_rule {
            if let Some(rule) = Self::parse_rule(s) {
                callback(N3Event::LogicRule(rule))?;
                return Ok(());
            }
        }
        for triple in Self::parse_formula_triples(s.trim_end_matches('.')) {
            callback(N3Event::StaticTriple(triple))?;
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
        let conclusion_str = parts[1]
            .trim()
            .trim_end_matches('.')
            .trim()
            .trim_matches(|c| c == '{' || c == '}');

        let premise = Formula {
            triples: Self::parse_formula_triples(premise_str),
        };
        let conclusion = Formula {
            triples: Self::parse_formula_triples(conclusion_str),
        };

        Some(Rule {
            id,
            rule_type,
            weight,
            premise,
            conclusion,
        })
    }

    /// Parse a formula body into its component triples, expanding Turtle/N3
    /// abbreviations: `.` ends a triple, `;` repeats the subject with a new
    /// predicate-object list, and `,` repeats the subject+predicate with a new
    /// object. Whitespace tokenisation is used so that a `.` inside an IRI
    /// (`<http://ex.org/Alice>`) is part of a single token and never mistaken for a
    /// statement terminator. A bare single triple still yields exactly one triple,
    /// preserving backward compatibility.
    fn parse_formula_triples(content: &str) -> Vec<Triple> {
        let mut triples = Vec::new();
        let tokens: Vec<&str> = content.split_whitespace().collect();

        let mut subject: Option<Term> = None;
        let mut predicate: Option<Term> = None;

        for &tok in &tokens {
            match tok {
                "." => {
                    subject = None;
                    predicate = None;
                }
                ";" => {
                    // keep subject, expect a fresh predicate next
                    predicate = None;
                }
                "," => {
                    // keep subject + predicate, expect a fresh object next
                }
                _ => {
                    if subject.is_none() {
                        subject = Some(Self::parse_term(tok));
                    } else if predicate.is_none() {
                        predicate = Some(Self::parse_term(tok));
                    } else {
                        // An object completes a triple; subject + predicate persist
                        // so a following `,`/`;` list reuses them.
                        triples.push(Triple {
                            subject: subject.clone().unwrap(),
                            predicate: predicate.clone().unwrap(),
                            object: Self::parse_term(tok),
                        });
                    }
                }
            }
        }

        triples
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
        parser
            .parse_all(|event| {
                events.push(event);
                Ok(())
            })
            .unwrap();
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
        let defeater = collect_events(
            "{ ?x a <http://ex.org/Exc> } ^> { ?x <http://ex.org/applies> false } .",
        );
        let linear =
            collect_events("{ ?x <http://ex.org/token> ?t } -o { ?x <http://ex.org/used> true } .");

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
        let events =
            collect_events("<http://ex.org/Alice> <http://ex.org/knows> <http://ex.org/Bob> .");
        match &events[0] {
            N3Event::StaticTriple(triple) => {
                assert!(matches!(triple.subject, Term::Uri(_)));
                assert!(matches!(triple.predicate, Term::Uri(_)));
                assert!(matches!(triple.object, Term::Uri(_)));
            }
            other => panic!("expected static triple, got {:?}", other),
        }
    }

    #[test]
    fn parses_multiline_rule_with_semicolon_and_inner_dot() {
        // The real agency.n3 G1 shape: multi-line, a `;` predicate-list, and a `.`
        // separating premise triples — all inside the braces.
        let events = collect_events(
            "# leading comment\n\
             { ?c a values:CorporatePerson ; values:claims ?r .\n\
               ?r a values:Right ; values:heldBy values:NaturalPerson\n\
             } => { ?c values:flag values:PersonhoodCategoryError } .\n",
        );
        assert_eq!(events.len(), 1, "exactly one rule across the lines");
        match &events[0] {
            N3Event::LogicRule(rule) => {
                assert_eq!(rule.rule_type, RuleType::Strict);
                assert_eq!(
                    rule.premise.triples.len(),
                    4,
                    "premise expands to 4 triples: {:?}",
                    rule.premise.triples
                );
                assert_eq!(rule.conclusion.triples.len(), 1);
                // Spot-check one expanded triple from the `;` list.
                let has_claims = rule.premise.triples.iter().any(|t| {
                    matches!(&t.subject, Term::Variable(v) if v == "?c")
                        && matches!(&t.predicate, Term::Uri(u) if u == "values:claims")
                        && matches!(&t.object, Term::Variable(v) if v == "?r")
                });
                assert!(has_claims, "?c values:claims ?r must be one of the triples");
            }
            other => panic!("expected logic rule, got {:?}", other),
        }
    }

    #[test]
    fn expands_comma_object_list_in_facts() {
        // `,` object list repeats subject+predicate.
        let events = collect_events("ex:s ex:p ex:a , ex:b , ex:c .");
        assert_eq!(events.len(), 3, "three triples from the object list");
        for ev in &events {
            assert!(matches!(ev, N3Event::StaticTriple(_)));
        }
    }

    #[test]
    fn multiline_fact_block_with_semicolons() {
        // A `;`-list fact block spanning lines yields one triple per predicate.
        let events = collect_events(
            "ex:Acme a values:CorporatePerson ;\n  values:claims ex:R1 .\n",
        );
        assert_eq!(events.len(), 2);
    }
}
