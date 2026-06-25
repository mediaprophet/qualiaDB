//! Native, zero-copy N3 (Notation3) parser.
//!
//! Two complementary front-ends over the same tokenizer:
//!
//! * **Borrowing AST** ([`Formula`]/[`Rule`], `Vec`-backed) — the convenient
//!   cold-path form consumed by [`crate::modalities::logic::n3_compiler`], which
//!   immediately lowers it to a zero-heap `CompiledRule`.
//! * **Zero-allocation AST** ([`StackFormula`]/[`StackRule`], fixed arrays) +
//!   [`N3Parser::parse_all_zero_heap`] — parses N3 with **no heap allocation**
//!   at all (proven by a `dhat` test), for edge hardware and the hot path.
//!
//! Supported N3 surface: triples with `;` (predicate lists) and `,` (object
//! lists); the `a` (rdf:type) keyword, kept as the engine's bare `a` token;
//! implication rules
//! (`=>` strict, `~>` defeasible, `^>` defeater, ` -o ` linear) with optional
//! `[id]` and `(weight)` annotations; `{ … }` **formula quoting / reification**
//! (a quoted graph used as a term, identified by the canonical hash of its
//! text); `#asp { … }` and `qualia:diffuse { … }` blocks; `#` comments.
//!
//! Resource caps (anti-DoS): brace nesting is bounded by
//! [`MAX_PARSE_BRACE_DEPTH`] and statement count by [`MAX_PARSE_STATEMENTS`];
//! a quoted/parsed formula yields at most [`MAX_STACK_TRIPLES`] triples. The
//! *evaluation* caps (forward-chaining fixpoint rounds, premise depth) live in
//! [`crate::webizen`] (`fire_guard_rules`).

use std::fmt;

/// Max triples in a zero-allocation [`StackFormula`] (matches the compiler's
/// `CompiledFormula` capacity).
pub const MAX_STACK_TRIPLES: usize = 8;
/// Max `{` nesting depth accepted by [`N3Parser::parse_all`] before failing
/// closed — prevents pathological-input parser state explosion.
pub const MAX_PARSE_BRACE_DEPTH: usize = 16;
/// Max top-level statements accepted in a single parse — anti-DoS bound.
pub const MAX_PARSE_STATEMENTS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Term<'a> {
    Uri(&'a str),
    Variable(&'a str),
    Literal(&'a str),
    /// A quoted N3 formula `{ … }` used as a term (graph quoting / reification).
    /// Holds the trimmed inner text; its identity is [`q_hash_formula`] of that
    /// text, so an nquin can refer to (reason about) another statement.
    Formula(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triple<'a> {
    pub subject: Term<'a>,
    pub predicate: Term<'a>,
    pub object: Term<'a>,
}

/// Heap-backed formula (cold-path convenience; the compiler lowers it to a
/// fixed-size `CompiledFormula`).
#[derive(Debug, Clone, PartialEq)]
pub struct Formula<'a> {
    pub triples: Vec<Triple<'a>>,
}

/// Zero-allocation formula: borrowed triples in a fixed stack array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackFormula<'a> {
    pub triples: [Triple<'a>; MAX_STACK_TRIPLES],
    pub len: usize,
}

impl<'a> StackFormula<'a> {
    #[inline]
    pub fn new() -> Self {
        Self {
            triples: [empty_triple(); MAX_STACK_TRIPLES],
            len: 0,
        }
    }
    /// The populated prefix.
    #[inline]
    pub fn as_slice(&self) -> &[Triple<'a>] {
        &self.triples[..self.len]
    }
}

impl Default for StackFormula<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleType {
    Strict,
    Defeasible,
    Defeater,
    Linear,
}

/// Heap-backed rule (cold-path convenience).
#[derive(Debug, Clone, PartialEq)]
pub struct Rule<'a> {
    pub id: Option<&'a str>,
    pub rule_type: RuleType,
    pub weight: Option<f32>,
    pub premise: Formula<'a>,
    pub conclusion: Formula<'a>,
}

/// Zero-allocation rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackRule<'a> {
    pub id: Option<&'a str>,
    pub rule_type: RuleType,
    pub weight: Option<f32>,
    pub premise: StackFormula<'a>,
    pub conclusion: StackFormula<'a>,
}

/// Streamed event from the heap front-end ([`N3Parser::parse_all`]).
#[derive(Debug)]
pub enum N3Event<'a> {
    StaticTriple(Triple<'a>),
    LogicRule(Rule<'a>),
    AspBlock(&'a str),
    DiffuseBlock(&'a str),
}

/// Streamed event from the zero-allocation front-end
/// ([`N3Parser::parse_all_zero_heap`]). `Copy`, no heap.
#[derive(Debug, Clone, Copy)]
pub enum StackEvent<'a> {
    StaticTriple(Triple<'a>),
    LogicRule(StackRule<'a>),
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

#[inline]
fn empty_triple<'a>() -> Triple<'a> {
    Triple {
        subject: Term::Uri(""),
        predicate: Term::Uri(""),
        object: Term::Uri(""),
    }
}

/// Canonical 64-bit FNV-1a hash of a quoted-formula's text, with runs of ASCII
/// whitespace collapsed to a single space (and leading/trailing whitespace
/// dropped). This gives a **stable identity** for a quoted statement so that
/// `{ :a :b :c }` and `{  :a  :b  :c  }` denote the same node — the handle other
/// nquins use to reason about that statement (reification). Zero-allocation.
pub fn q_hash_formula(text: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x0000_0100_0000_01B3;
    let mut h = OFFSET;
    // Defer the separator space until a following token confirms it, so leading
    // AND trailing whitespace are both dropped (only interior runs collapse).
    let mut pending_space = false;
    let mut started = false;
    for &b in text.as_bytes() {
        if b.is_ascii_whitespace() {
            if started {
                pending_space = true;
            }
        } else {
            if pending_space {
                h ^= 0x20;
                h = h.wrapping_mul(PRIME);
                pending_space = false;
            }
            h ^= b as u64;
            h = h.wrapping_mul(PRIME);
            started = true;
        }
    }
    h
}

/// Tokenizer over one N3 statement-block. Yields, in order:
/// * a `{ … }` quoted formula (balanced braces) as a single token (incl. braces),
/// * a `"…"` string literal as a single token (incl. quotes),
/// * the punctuation tokens `;`, `,`, `.` (one char each),
/// * otherwise a run up to the next whitespace / punctuation / `{` / `"`.
///
/// A `.` between two digits (a decimal, e.g. `3.14`) is kept inside the token.
struct TripleTokenizer<'a> {
    s: &'a str,
    b: &'a [u8],
    i: usize,
}

impl<'a> TripleTokenizer<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            s,
            b: s.as_bytes(),
            i: 0,
        }
    }
}

impl<'a> Iterator for TripleTokenizer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let len = self.b.len();
        // Skip whitespace and `#...\n` comments (a comment is only a comment when
        // it starts a token - a `#` inside a `<...>` URI or string literal is
        // consumed as part of that token below).
        loop {
            while self.i < len && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            if self.i < len && self.b[self.i] == b'#' {
                while self.i < len && self.b[self.i] != b'\n' {
                    self.i += 1;
                }
                continue;
            }
            break;
        }
        if self.i >= len {
            return None;
        }
        let start = self.i;
        let c = self.b[self.i];

        // Quoted formula: balanced braces.
        if c == b'{' {
            let mut depth = 0i32;
            while self.i < len {
                match self.b[self.i] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            self.i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                self.i += 1;
            }
            return Some(&self.s[start..self.i]);
        }

        // String literal.
        if c == b'"' {
            self.i += 1;
            while self.i < len && self.b[self.i] != b'"' {
                self.i += 1;
            }
            if self.i < len {
                self.i += 1; // include closing quote
            }
            return Some(&self.s[start..self.i]);
        }

        // Single-char punctuation.
        if c == b';' || c == b',' || c == b'.' {
            self.i += 1;
            return Some(&self.s[start..self.i]);
        }

        // General token.
        while self.i < len {
            let d = self.b[self.i];
            if d == b'.' {
                // Keep a decimal point (digit '.' digit) inside the token.
                let prev_digit = self.i > start && self.b[self.i - 1].is_ascii_digit();
                let next_digit = self.i + 1 < len && self.b[self.i + 1].is_ascii_digit();
                if prev_digit && next_digit {
                    self.i += 1;
                    continue;
                }
                break;
            }
            if d.is_ascii_whitespace() || d == b';' || d == b',' || d == b'{' || d == b'"' {
                break;
            }
            self.i += 1;
        }
        Some(&self.s[start..self.i])
    }
}

/// Parse the triples of one block, emitting each via `emit`. The block may be a
/// bare formula body or a `{ … }`-wrapped one; multiple `.`-separated statements
/// are handled, as are `;` (predicate) and `,` (object) lists. Zero-allocation.
///
/// `emit` returns `Ok(true)` to continue, `Ok(false)` to stop early (e.g. a
/// caller buffer filled), or `Err` to abort.
fn for_each_triple<'a>(
    block: &'a str,
    mut emit: impl FnMut(Triple<'a>) -> Result<bool, N3ParserError>,
) -> Result<(), N3ParserError> {
    let mut s = block.trim();
    if s.starts_with('{') && s.ends_with('}') {
        s = s[1..s.len() - 1].trim();
    }

    let mut subject: Option<Term<'a>> = None;
    let mut predicate: Option<Term<'a>> = None;

    for tok in TripleTokenizer::new(s) {
        match tok {
            "." => {
                subject = None;
                predicate = None;
            }
            ";" => {
                predicate = None; // same subject, new predicate
            }
            "," => {
                // same subject + predicate, new object: state already correct.
            }
            node => {
                let term = N3Parser::parse_term(node);
                if subject.is_none() {
                    subject = Some(term);
                } else if predicate.is_none() {
                    predicate = Some(term);
                } else {
                    let triple = Triple {
                        subject: subject.unwrap(),
                        predicate: predicate.unwrap(),
                        object: term,
                    };
                    if !emit(triple)? {
                        return Ok(());
                    }
                    // Stay ready for `,` (another object) / `;` (new predicate) /
                    // `.` (new statement); a bare following node is treated as a
                    // comma-implied object.
                }
            }
        }
    }
    Ok(())
}

/// A highly capable, native N3 parser over a borrowed `&str` (zero-copy terms).
pub struct N3Parser<'a> {
    text: &'a str,
}

impl<'a> N3Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        N3Parser { text }
    }

    // ── Heap front-end (cold-path convenience) ──────────────────────────────

    pub fn parse_all<F>(&mut self, callback: F) -> Result<(), N3ParserError>
    where
        F: FnMut(N3Event<'a>) -> Result<(), N3ParserError>,
    {
        self.scan(callback, dispatch_statement_heap, emit_block_heap)
    }

    /// Zero-allocation streaming parse: emits [`StackEvent`]s with no heap use.
    pub fn parse_all_zero_heap<F>(&mut self, callback: F) -> Result<(), N3ParserError>
    where
        F: FnMut(StackEvent<'a>) -> Result<(), N3ParserError>,
    {
        self.scan(callback, dispatch_statement_stack, emit_block_stack)
    }

    /// Shared statement-splitter for both front-ends. Tracks comments, the two
    /// special `{ … }` blocks, brace depth (capped), and `.`-termination at
    /// brace depth 0, dispatching each statement via `dispatch` and each special
    /// block via `emit_block`. `F` (the caller's event type) is opaque here —
    /// only the per-front-end function pointers know how to build events.
    fn scan<F>(
        &mut self,
        mut callback: F,
        dispatch: fn(&'a str, &mut F) -> Result<(), N3ParserError>,
        emit_block: fn(BlockKind, &'a str, &mut F) -> Result<(), N3ParserError>,
    ) -> Result<(), N3ParserError> {
        let bytes = self.text.as_bytes();
        let len = bytes.len();

        let mut i = 0;
        let mut stmt_start = 0;
        let mut brace_depth: i32 = 0;
        let mut in_comment = false;
        let mut statements = 0usize;

        while i < len {
            let c = bytes[i] as char;

            if in_comment {
                if c == '\n' {
                    in_comment = false;
                }
                i += 1;
                continue;
            }

            if c == '#' {
                if self.text[i..].starts_with("#asp {") {
                    let end = self.text[i..].find('}').unwrap_or(self.text[i..].len());
                    emit_block(BlockKind::Asp, self.text[i + 6..i + end].trim(), &mut callback)?;
                    i += end + 1;
                    stmt_start = i;
                    continue;
                } else {
                    in_comment = true;
                    i += 1;
                    continue;
                }
            }

            // Gate on the ASCII lead byte 'q' so `self.text[i..]` is only sliced
            // on a UTF-8 char boundary (continuation bytes of multi-byte chars
            // in URIs/strings/comments must never reach a `str` slice).
            if c == 'q' && self.text[i..].starts_with("qualia:diffuse {") {
                let end = self.text[i..].find('}').unwrap_or(self.text[i..].len());
                emit_block(BlockKind::Diffuse, self.text[i + 16..i + end].trim(), &mut callback)?;
                i += end + 1;
                stmt_start = i;
                continue;
            }

            if c == '{' {
                brace_depth += 1;
                if brace_depth as usize > MAX_PARSE_BRACE_DEPTH {
                    return Err(N3ParserError("N3 brace nesting exceeds MAX_PARSE_BRACE_DEPTH"));
                }
            }
            if c == '}' {
                brace_depth -= 1;
                if brace_depth < 0 {
                    return Err(N3ParserError("unbalanced '}' in N3 input"));
                }
            }

            if c == '.' && brace_depth <= 0 {
                let stmt = self.text[stmt_start..=i].trim();
                statements += 1;
                if statements > MAX_PARSE_STATEMENTS {
                    return Err(N3ParserError("N3 statement count exceeds MAX_PARSE_STATEMENTS"));
                }
                dispatch(stmt, &mut callback)?;
                stmt_start = i + 1;
            }

            i += 1;
        }

        if brace_depth != 0 {
            return Err(N3ParserError("unbalanced '{' in N3 input"));
        }

        let rem = self.text[stmt_start..].trim();
        if !rem.is_empty() && !rem.starts_with('@') && !rem.starts_with('#') {
            dispatch(rem, &mut callback)?;
        }

        Ok(())
    }

    // ── Zero-allocation helpers ─────────────────────────────────────────────

    /// Parse a block's triples into a caller-provided buffer; returns the count
    /// written (capped at `out.len()`). Zero-allocation.
    pub fn parse_triples_into(block: &'a str, out: &mut [Triple<'a>]) -> usize {
        let mut n = 0usize;
        let _ = for_each_triple(block, |t| {
            if n < out.len() {
                out[n] = t;
                n += 1;
                Ok(n < out.len())
            } else {
                Ok(false)
            }
        });
        n
    }

    /// Parse one rule line into a zero-allocation [`StackRule`].
    pub fn parse_rule_zero_heap(line: &'a str) -> Option<StackRule<'a>> {
        let (id, weight, rule_type, premise_str, conclusion_str) = split_rule(line)?;
        let mut premise = StackFormula::new();
        premise.len = Self::parse_triples_into(premise_str, &mut premise.triples);
        let mut conclusion = StackFormula::new();
        conclusion.len = Self::parse_triples_into(conclusion_str, &mut conclusion.triples);
        Some(StackRule {
            id,
            rule_type,
            weight,
            premise,
            conclusion,
        })
    }

    // ── Heap helpers (kept for existing consumers) ──────────────────────────

    fn parse_rule(line: &'a str) -> Option<Rule<'a>> {
        let (id, weight, rule_type, premise_str, conclusion_str) = split_rule(line)?;
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
        let mut triples = Vec::new();
        let _ = for_each_triple(block, |t| {
            triples.push(t);
            Ok(true)
        });
        triples
    }

    fn parse_term(s: &'a str) -> Term<'a> {
        let s = s.trim();
        if s.is_empty() {
            return Term::Uri("");
        }
        // NOTE: the N3 `a` keyword (rdf:type) is left as the bare token `a`,
        // which is the engine's established type predicate (`q_hash("a")`, used
        // uniformly by the agency/values guards and fact ingestion). It is *not*
        // expanded to `rdf:type` here — doing so would desync rules from facts.
        if let Some(rest) = s.strip_prefix('?') {
            let _ = rest;
            return Term::Variable(s);
        }
        if s.starts_with('{') {
            let inner = s
                .strip_prefix('{')
                .and_then(|x| x.strip_suffix('}'))
                .unwrap_or(s)
                .trim();
            return Term::Formula(inner);
        }
        if s.starts_with('"') {
            // Strip quotes so literal VALUES are comparable / numerically parseable.
            return Term::Literal(s.trim_matches('"'));
        }
        if s.parse::<f64>().is_ok() {
            return Term::Literal(s);
        }
        Term::Uri(s)
    }
}

/// True if a statement looks like an implication rule.
fn looks_like_rule(s: &str) -> bool {
    s.contains("=>") || s.contains("~>") || s.contains("^>") || s.contains(" -o ")
}

/// Split a rule line into `(id, weight, type, premise_str, conclusion_str)`,
/// all zero-copy slices. Returns `None` if no arrow is present.
fn split_rule(line: &str) -> Option<(Option<&str>, Option<f32>, RuleType, &str, &str)> {
    let mut clean = line.trim();
    let mut id = None;
    let mut weight = None;

    if clean.starts_with('[') {
        if let Some(end) = clean.find(']') {
            id = Some(clean[1..end].trim());
            clean = clean[end + 1..].trim();
        }
    }
    if clean.starts_with('(') {
        if let Some(end) = clean.find(')') {
            if let Ok(w) = clean[1..end].trim().parse::<f32>() {
                weight = Some(w);
            }
            clean = clean[end + 1..].trim();
        }
    }

    let (rule_type, arrow_len, arrow_idx) = if let Some(idx) = clean.find("=>") {
        (RuleType::Strict, 2, idx)
    } else if let Some(idx) = clean.find("~>") {
        (RuleType::Defeasible, 2, idx)
    } else if let Some(idx) = clean.find("^>") {
        (RuleType::Defeater, 2, idx)
    } else if let Some(idx) = clean.find(" -o ") {
        (RuleType::Linear, 4, idx)
    } else {
        return None;
    };

    let premise = clean[..arrow_idx].trim();
    let conclusion = clean[arrow_idx + arrow_len..].trim().trim_end_matches('.');
    Some((id, weight, rule_type, premise, conclusion))
}

fn trim_leading_comment_lines(mut s: &str) -> &str {
    loop {
        let trimmed = s.trim_start();
        let Some(rest) = trimmed.strip_prefix('#') else {
            return trimmed;
        };
        let Some(newline) = rest.find('\n') else {
            return "";
        };
        s = &rest[newline + 1..];
    }
}

// ── Statement dispatchers (one per front-end) ───────────────────────────────

enum BlockKind {
    Asp,
    Diffuse,
}

fn emit_block_heap<'a, F>(kind: BlockKind, body: &'a str, callback: &mut F) -> Result<(), N3ParserError>
where
    F: FnMut(N3Event<'a>) -> Result<(), N3ParserError>,
{
    match kind {
        BlockKind::Asp => callback(N3Event::AspBlock(body)),
        BlockKind::Diffuse => callback(N3Event::DiffuseBlock(body)),
    }
}

fn emit_block_stack<'a, F>(kind: BlockKind, body: &'a str, callback: &mut F) -> Result<(), N3ParserError>
where
    F: FnMut(StackEvent<'a>) -> Result<(), N3ParserError>,
{
    match kind {
        BlockKind::Asp => callback(StackEvent::AspBlock(body)),
        BlockKind::Diffuse => callback(StackEvent::DiffuseBlock(body)),
    }
}

fn dispatch_statement_heap<'a, F>(stmt: &'a str, callback: &mut F) -> Result<(), N3ParserError>
where
    F: FnMut(N3Event<'a>) -> Result<(), N3ParserError>,
{
    let s = trim_leading_comment_lines(stmt);
    if s.is_empty() {
        return Ok(());
    }
    if looks_like_rule(s) {
        if let Some(rule) = N3Parser::parse_rule(s) {
            return callback(N3Event::LogicRule(rule));
        }
    }
    let triples = N3Parser::parse_formula_triples(s.trim_end_matches('.'));
    for triple in triples {
        callback(N3Event::StaticTriple(triple))?;
    }
    Ok(())
}

fn dispatch_statement_stack<'a, F>(stmt: &'a str, callback: &mut F) -> Result<(), N3ParserError>
where
    F: FnMut(StackEvent<'a>) -> Result<(), N3ParserError>,
{
    let s = trim_leading_comment_lines(stmt);
    if s.is_empty() {
        return Ok(());
    }
    if looks_like_rule(s) {
        if let Some(rule) = N3Parser::parse_rule_zero_heap(s) {
            return callback(StackEvent::LogicRule(rule));
        }
    }
    let mut err: Option<N3ParserError> = None;
    for_each_triple(s.trim_end_matches('.'), |t| match callback(StackEvent::StaticTriple(t)) {
        Ok(()) => Ok(true),
        Err(e) => {
            err = Some(e);
            Ok(false)
        }
    })?;
    if let Some(e) = err {
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Faithful parsing (item 2) ───────────────────────────────────────────

    #[test]
    fn keyword_a_is_the_engine_type_token() {
        // The N3 `a` keyword is kept as the bare token the engine uses uniformly
        // for rdf:type (`q_hash("a")`) — NOT expanded to `rdf:type`, which would
        // desync parsed rules from facts asserted with `vh("a")`.
        let mut buf = [empty_triple(); MAX_STACK_TRIPLES];
        let n = N3Parser::parse_triples_into(":x a :Y .", &mut buf);
        assert_eq!(n, 1);
        assert_eq!(buf[0].predicate, Term::Uri("a"));
        assert_eq!(buf[0].subject, Term::Uri(":x"));
        assert_eq!(buf[0].object, Term::Uri(":Y"));
    }

    #[test]
    fn object_lists_and_predicate_lists() {
        let mut buf = [empty_triple(); MAX_STACK_TRIPLES];
        // comma = object list, semicolon = predicate list
        let n = N3Parser::parse_triples_into(":x :p :a, :b ; :q :c .", &mut buf);
        assert_eq!(n, 3);
        assert_eq!(buf[0], Triple { subject: Term::Uri(":x"), predicate: Term::Uri(":p"), object: Term::Uri(":a") });
        assert_eq!(buf[1], Triple { subject: Term::Uri(":x"), predicate: Term::Uri(":p"), object: Term::Uri(":b") });
        assert_eq!(buf[2], Triple { subject: Term::Uri(":x"), predicate: Term::Uri(":q"), object: Term::Uri(":c") });
    }

    #[test]
    fn leading_comment_before_rule_preserves_premise() {
        let text = r#"
# (G1) Corporate-capture guard.
{ ?c a values:CorporatePerson ; values:claims ?r .
  ?r a values:Right ; values:heldBy values:NaturalPerson
} => { ?c values:flag values:PersonhoodCategoryError } .
"#;
        let mut rules = Vec::new();
        let mut parser = N3Parser::new(text);
        parser
            .parse_all(|ev| {
                if let N3Event::LogicRule(rule) = ev {
                    rules.push(rule);
                }
                Ok(())
            })
            .unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].premise.triples.len(), 4);
        assert_eq!(rules[0].conclusion.triples.len(), 1);
        assert_eq!(rules[0].premise.triples[0].subject, Term::Variable("?c"));
        assert_eq!(rules[0].premise.triples[0].predicate, Term::Uri("a"));
        assert_eq!(
            rules[0].premise.triples[0].object,
            Term::Uri("values:CorporatePerson")
        );
    }

    #[test]
    fn decimal_literal_is_one_token() {
        let mut buf = [empty_triple(); MAX_STACK_TRIPLES];
        let n = N3Parser::parse_triples_into(":x :p 3.14 .", &mut buf);
        assert_eq!(n, 1);
        assert_eq!(buf[0].object, Term::Literal("3.14"));
    }

    #[test]
    fn parses_implication_rule_zero_heap() {
        let rule = N3Parser::parse_rule_zero_heap("{ ?x a ?y } => { ?y a ?z } .").unwrap();
        assert_eq!(rule.rule_type, RuleType::Strict);
        assert_eq!(rule.premise.len, 1);
        assert_eq!(rule.conclusion.len, 1);
        let p = rule.premise.triples[0];
        assert_eq!(p.subject, Term::Variable("?x"));
        assert_eq!(p.predicate, Term::Uri("a"));
        assert_eq!(p.object, Term::Variable("?y"));
    }

    // ── Quoting / reification (item 3) ──────────────────────────────────────

    #[test]
    fn quoted_formula_term_is_captured() {
        let mut buf = [empty_triple(); MAX_STACK_TRIPLES];
        let n = N3Parser::parse_triples_into("{ :alice :says :x } :trustedBy :bob .", &mut buf);
        assert_eq!(n, 1);
        match buf[0].subject {
            Term::Formula(body) => assert!(body.contains(":alice :says :x")),
            other => panic!("expected a quoted formula subject, got {other:?}"),
        }
        assert_eq!(buf[0].predicate, Term::Uri(":trustedBy"));
        assert_eq!(buf[0].object, Term::Uri(":bob"));
    }

    #[test]
    fn formula_with_internal_period_is_one_term() {
        let mut buf = [empty_triple(); MAX_STACK_TRIPLES];
        let n = N3Parser::parse_triples_into("{ :a :b :c . :d :e :f } :g :h .", &mut buf);
        assert_eq!(n, 1);
        assert!(matches!(buf[0].subject, Term::Formula(_)));
        assert_eq!(buf[0].object, Term::Uri(":h"));
    }

    #[test]
    fn reification_handle_is_whitespace_canonical() {
        // The same statement with different spacing denotes the same node.
        assert_eq!(q_hash_formula(":a :b :c"), q_hash_formula("  :a   :b  :c "));
        assert_ne!(q_hash_formula(":a :b :c"), q_hash_formula(":a :b :d"));
    }

    // ── Resource caps (item 4, parser level) ────────────────────────────────

    #[test]
    fn rejects_excessive_brace_nesting() {
        let deep = "{".repeat(MAX_PARSE_BRACE_DEPTH + 2);
        let mut parser = N3Parser::new(&deep);
        let r = parser.parse_all(|_| Ok(()));
        assert!(r.is_err());
    }

    #[test]
    fn rejects_unbalanced_braces() {
        let mut parser = N3Parser::new("} :a :b :c .");
        assert!(parser.parse_all(|_| Ok(())).is_err());
    }

    #[test]
    fn parses_multibyte_utf8_without_panicking() {
        // Multi-byte UTF-8 outside comments (literals, URIs) must not cause a
        // mid-character `str` slice panic in the byte-scanning loop.
        let doc = ":x :label \"café — déjà ➜ vu\" .\n\
                   <http://例え.example/Ω> :note \":naïve\" .\n";
        let mut parser = N3Parser::new(doc);
        let mut n = 0usize;
        parser
            .parse_all(|ev| {
                if let N3Event::StaticTriple(_) = ev {
                    n += 1;
                }
                Ok(())
            })
            .unwrap();
        assert!(n >= 2, "expected both UTF-8 triples, got {n}");
    }

    // ── Zero-allocation guarantee (item 1) ──────────────────────────────────

    #[test]
    fn parse_all_zero_heap_allocates_nothing() {
        let doc = "\
            :alice a :Person .\n\
            :alice :knows :bob, :carol .\n\
            { ?x a :Person } => { ?x a :Agent } .\n\
            { :alice :says :hi } :assertedBy :alice .\n";
        let _profiler = dhat::Profiler::builder().testing().build();

        let mut parser = N3Parser::new(doc);
        let mut triples = 0usize;
        let mut rules = 0usize;
        parser
            .parse_all_zero_heap(|ev| {
                match ev {
                    StackEvent::StaticTriple(_) => triples += 1,
                    StackEvent::LogicRule(_) => rules += 1,
                    _ => {}
                }
                Ok(())
            })
            .unwrap();

        let stats = dhat::HeapStats::get();
        assert_eq!(stats.curr_blocks, 0, "parse_all_zero_heap must not allocate");
        assert_eq!(stats.curr_bytes, 0);
        assert!(triples >= 4, "expected the static + object-list triples");
        assert_eq!(rules, 1);
    }

    // ── End-to-end: parse → compile → bytecode (item 2) ─────────────────────

    #[test]
    fn implication_compiles_to_sentinel_bytecode() {
        use crate::modalities::logic::n3_compiler::{compile_rule_to_opcodes, compile_rule_to_zero_heap, MAX_COMPILED_OPCODES};
        use crate::webizen::SlgOpcode;

        // Parse via the heap front-end (the compiler consumes `Rule`).
        let mut parser = N3Parser::new("{ ?x a ?y } => { ?y a ?z } .");
        let mut compiled_ok = false;
        parser
            .parse_all(|ev| {
                if let N3Event::LogicRule(rule) = ev {
                    let compiled = compile_rule_to_zero_heap(&rule);
                    let mut ops = [SlgOpcode::Call; MAX_COMPILED_OPCODES];
                    let n = compile_rule_to_opcodes(&compiled, &mut ops).unwrap();
                    assert!(n >= 3);
                    assert_eq!(ops[0], SlgOpcode::Unify);
                    compiled_ok = true;
                }
                Ok(())
            })
            .unwrap();
        assert!(compiled_ok, "the implication rule did not reach the compiler");
    }
}
