use std::io::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Term<'a> {
    Uri(&'a str),
    Variable(&'a str),
    Literal(&'a str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Triple<'a> {
    pub subject: Term<'a>,
    pub predicate: Term<'a>,
    pub object: Term<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Formula<'a> {
    pub triples: Vec<Triple<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleType {
    Strict,
    Defeasible,
    Defeater,
    Linear,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule<'a> {
    pub id: Option<&'a str>,
    pub rule_type: RuleType,
    pub weight: Option<f32>,
    pub premise: Formula<'a>,
    pub conclusion: Formula<'a>,
}

#[derive(Debug)]
pub enum N3Event<'a> {
    StaticTriple(Triple<'a>),
    LogicRule(Rule<'a>),
    AspBlock(&'a str),
    DiffuseBlock(&'a str),
}

#[derive(Debug, Clone)]
pub struct N3ParserError(pub &'static str);

impl fmt::Display for N3ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for N3ParserError {}

/// A highly constrained, native MVP N3 parser.
/// It splits the file into lines, extracts basic triples and implication rules.
/// Fully zero-allocation AST using slices.
pub struct N3Parser<'a> {
    text: &'a str,
}

impl<'a> N3Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        N3Parser { text }
    }

    pub fn parse_all<F>(&mut self, mut callback: F) -> Result<(), N3ParserError>
    where
        F: FnMut(N3Event<'a>) -> Result<(), N3ParserError>,
    {
        let bytes = self.text.as_bytes();
        let len = bytes.len();
        
        let mut i = 0;
        let mut stmt_start = 0;
        let mut brace_depth = 0;
        let mut in_comment = false;
        
        while i < len {
            let c = bytes[i] as char;
            
            if in_comment {
                if c == '\n' { in_comment = false; }
                i += 1;
                continue;
            }
            
            if c == '#' {
                if self.text[i..].starts_with("#asp {") {
                    let end = self.text[i..].find('}').unwrap_or(self.text[i..].len() - i);
                    callback(N3Event::AspBlock(self.text[i + 6 .. i + end].trim()))?;
                    i += end + 1;
                    stmt_start = i;
                    continue;
                } else {
                    in_comment = true;
                    i += 1;
                    continue;
                }
            }
            
            if self.text[i..].starts_with("qualia:diffuse {") {
                let end = self.text[i..].find('}').unwrap_or(self.text[i..].len() - i);
                callback(N3Event::DiffuseBlock(self.text[i + 16 .. i + end].trim()))?;
                i += end + 1;
                stmt_start = i;
                continue;
            }
            
            if c == '{' { brace_depth += 1; }
            if c == '}' { brace_depth -= 1; }
            
            if c == '.' && brace_depth <= 0 {
                let stmt = self.text[stmt_start..=i].trim();
                Self::dispatch_statement(stmt, &mut callback)?;
                stmt_start = i + 1;
            }
            
            i += 1;
        }
        
        let rem = self.text[stmt_start..].trim();
        if !rem.is_empty() && !rem.starts_with('@') && !rem.starts_with('#') {
             Self::dispatch_statement(rem, &mut callback)?;
        }
        
        Ok(())
    }

    fn dispatch_statement<F>(stmt: &'a str, callback: &mut F) -> Result<(), N3ParserError>
    where
        F: FnMut(N3Event<'a>) -> Result<(), N3ParserError>,
    {
        let s = stmt.trim();
        if s.is_empty() { return Ok(()); }
        
        let looks_like_rule = s.contains("=>") || s.contains("~>") || s.contains("^>") || s.contains(" -o ");
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

    fn parse_rule(line: &'a str) -> Option<Rule<'a>> {
        let mut clean_line = line.trim();
        let mut id = None;
        let mut weight = None;

        if clean_line.starts_with('[') {
            if let Some(end_idx) = clean_line.find(']') {
                id = Some(clean_line[1..end_idx].trim());
                clean_line = clean_line[end_idx + 1..].trim();
            }
        }

        if clean_line.starts_with('(') {
            if let Some(end_idx) = clean_line.find(')') {
                let w_str = clean_line[1..end_idx].trim();
                if let Ok(w) = w_str.parse::<f32>() {
                    weight = Some(w);
                }
                clean_line = clean_line[end_idx + 1..].trim();
            }
        }

        let (rule_type, arrow_len, arrow_idx) = if let Some(idx) = clean_line.find("=>") {
            (RuleType::Strict, 2, idx)
        } else if let Some(idx) = clean_line.find("~>") {
            (RuleType::Defeasible, 2, idx)
        } else if let Some(idx) = clean_line.find("^>") {
            (RuleType::Defeater, 2, idx)
        } else if let Some(idx) = clean_line.find(" -o ") {
            (RuleType::Linear, 4, idx)
        } else {
            return None;
        };

        let premise_str = clean_line[..arrow_idx].trim();
        let conclusion_str = clean_line[arrow_idx + arrow_len..].trim().trim_end_matches('.');

        Some(Rule {
            id,
            rule_type,
            weight,
            premise: Formula {
                triples: Self::parse_formula_triples(premise_str),
            },
            conclusion: Formula {
                triples: Self::parse_formula_triples(conclusion_str),
            },
        })
    }

    fn parse_formula_triples(block: &'a str) -> Vec<Triple<'a>> {
        let mut s = block.trim();
        if s.starts_with('{') && s.ends_with('}') {
            s = s[1..s.len() - 1].trim();
        }
        let mut triples = Vec::new();
        let statements = s.split('.');
        for stmt in statements {
            let stmt = stmt.trim();
            if stmt.is_empty() { continue; }
            let mut parts = stmt.split_whitespace();
            let subject_str = match parts.next() {
                Some(p) => p,
                None => continue,
            };
            
            let mut rest = stmt[subject_str.len()..].trim_start();
            let mut current_subject = subject_str;
            
            while !rest.is_empty() {
                let predicate_str = match rest.split_whitespace().next() {
                    Some(p) => p,
                    None => break,
                };
                rest = rest[predicate_str.len()..].trim_start();
                
                let (object_str, remainder) = Self::extract_object(rest);
                if let Some(obj) = object_str {
                    triples.push(Triple {
                        subject: Self::parse_term(current_subject),
                        predicate: Self::parse_term(predicate_str),
                        object: Self::parse_term(obj),
                    });
                }
                
                rest = remainder.trim_start();
                if rest.starts_with(';') {
                    rest = rest[1..].trim_start();
                } else {
                    break; // Proper parser handles `,` too, keeping simple for MVP.
                }
            }
        }
        triples
    }
    
    fn extract_object(s: &'a str) -> (Option<&'a str>, &'a str) {
        if s.is_empty() { return (None, s); }
        if s.starts_with('"') {
            if let Some(end) = s[1..].find('"') {
                let end_idx = end + 2;
                return (Some(&s[..end_idx]), &s[end_idx..]);
            }
        }
        let end = s.find(|c: char| c.is_whitespace() || c == ';' || c == ',').unwrap_or(s.len());
        (Some(&s[..end]), &s[end..])
    }

    fn parse_term(s: &'a str) -> Term<'a> {
        if s.starts_with('?') {
            Term::Variable(s)
        } else if s.starts_with('"') {
            // Strip surrounding quotes so literal VALUES are comparable (zero-copy slice).
            // Without this, `"12"` is stored with quotes and numeric SHACL checks can't
            // `parse::<f64>()` it, silently bypassing the firewall.
            Term::Literal(s.trim_matches('"'))
        } else if s.parse::<f64>().is_ok() {
            Term::Literal(s)
        } else {
            Term::Uri(s)
        }
    }
}
