//! SPARQL group-graph-pattern parser — the WHERE-clause grammar.
//!
//! Parses `{ … }` groups into the `Pattern` arena the planner/executor already
//! consume: basic triple patterns, `OPTIONAL { … }`, `{ … } UNION { … }`,
//! `MINUS { … }`, nested groups, `<< s p o >>` quoted triples, and `FILTER(…)`
//! (whose expression is parsed by `super::expr`).
//!
//! ## The arena-contiguity constraint
//! `Pattern::Group { start_idx, len }` is planned by joining the *contiguous*
//! range `[start_idx, start_idx+len)` of the pattern arena. So a group's direct
//! children must be allocated as one uninterrupted batch. We therefore parse a
//! group into a list of `ChildSpec`s — building any inner sub-patterns
//! (OPTIONAL/UNION/MINUS inners) eagerly and referencing them by id — and only
//! then allocate the direct-child nodes contiguously. Plain nested groups are
//! flattened into the parent (group nesting is join-associative), which also
//! sidesteps the contiguity problem for them.
//!
//! ## Known engine simplifications (not parser bugs)
//! The planner currently lowers `Optional` to a plain join and `Minus` to its
//! inner (see `sparql_planner`), so left-join / anti-join *semantics* are an
//! engine TODO. This parser produces the correct `Pattern` nodes; when the
//! planner is upgraded, they will gain full semantics with no parser change.

use std::collections::HashMap;

use crate::sparql_ast::{ExpressionId, Pattern, PatternId, SparqlQueryContext};
use crate::sparql_library::sparql_grammar::expr::parse_expression;
use crate::sparql_library::sparql_grammar::tokenizer::{tokenize, Token};

/// Parse a WHERE group graph pattern from a fragment beginning at (or before)
/// the opening `{`. Returns the root `PatternId`.
pub fn parse_where_group(
    input: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<PatternId, String> {
    let tokens = tokenize(input)?;
    let mut p = PatternParser {
        tokens: &tokens,
        pos: 0,
        ctx,
        prefixes,
    };
    // Skip a leading WHERE keyword if the caller included it.
    if let Some(Token::Word(w)) = p.peek() {
        if w.eq_ignore_ascii_case("WHERE") {
            p.pos += 1;
        }
    }
    p.parse_group()
}

/// One direct child of a group, allocated as exactly one arena node in the
/// contiguous batch at group close (inner sub-patterns already allocated).
enum ChildSpec {
    Triple { s: u64, p: u64, o: u64 },
    StarTriple {
        is: u64,
        ip: u64,
        io: u64,
        op: u64,
        oo: u64,
    },
    Optional(PatternId),
    Union(PatternId, PatternId),
    Minus(PatternId),
}

struct PatternParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    ctx: &'a mut SparqlQueryContext,
    prefixes: &'a HashMap<String, String>,
}

impl<'a> PatternParser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn eat_punct(&mut self, c: char) -> bool {
        if matches!(self.peek(), Some(Token::Punct(p)) if *p == c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_punct(&mut self, c: char) -> Result<(), String> {
        if self.eat_punct(c) {
            Ok(())
        } else {
            Err(format!("expected '{c}' at token {}", self.pos))
        }
    }

    fn peek_word_ci(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    /// Parse `{ … }` and return a single `PatternId`.
    fn parse_group(&mut self) -> Result<PatternId, String> {
        self.expect_punct('{')?;
        let (specs, filters) = self.parse_group_body()?;
        let root = self.materialize(specs)?;
        // Filters wrap the whole group (scoping is simplified — see module doc).
        let mut root = root;
        for expr_id in filters {
            root = self.ctx.alloc_pattern(Pattern::Filter {
                pattern: root,
                expression: expr_id,
            })?;
        }
        Ok(root)
    }

    /// Parse the body of a group up to (and consuming) the closing `}`.
    /// Returns the direct-child specs and any FILTER expression ids (hoisted to
    /// the group). Plain nested groups are flattened in.
    fn parse_group_body(&mut self) -> Result<(Vec<ChildSpec>, Vec<ExpressionId>), String> {
        let mut specs: Vec<ChildSpec> = Vec::new();
        let mut filters: Vec<ExpressionId> = Vec::new();

        loop {
            match self.peek() {
                None => return Err("unterminated group (missing '}')".to_string()),
                Some(Token::Punct('}')) => {
                    self.pos += 1;
                    break;
                }
                Some(Token::Punct('.')) => {
                    self.pos += 1; // triple separator between items
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("OPTIONAL") => {
                    self.pos += 1;
                    let inner = self.parse_group()?;
                    specs.push(ChildSpec::Optional(inner));
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("MINUS") => {
                    self.pos += 1;
                    let inner = self.parse_group()?;
                    specs.push(ChildSpec::Minus(inner));
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("FILTER") => {
                    self.pos += 1;
                    let expr_id = self.parse_filter_expr()?;
                    filters.push(expr_id);
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("BIND") => {
                    return Err(
                        "BIND requires engine support (no Bind pattern node yet) — deferred"
                            .to_string(),
                    );
                }
                Some(Token::Punct('{')) => {
                    // A nested group: either the left side of `{ } UNION { }`, or
                    // a plain nested group (flattened into this one).
                    let left = self.parse_group()?;
                    if self.peek_word_ci("UNION") {
                        self.pos += 1;
                        let right = self.parse_group()?;
                        specs.push(ChildSpec::Union(left, right));
                    } else {
                        // Plain nested group → flatten its children into this
                        // group (group nesting is join-associative). Any inner
                        // FILTER is hoisted to this group's filter list — filter
                        // scoping is thereby simplified (see module doc).
                        self.flatten_group_into(left, &mut specs, &mut filters);
                    }
                }
                Some(Token::StarOpen) => {
                    let spec = self.parse_star_triple()?;
                    specs.push(spec);
                }
                _ => {
                    let spec = self.parse_triple()?;
                    specs.push(spec);
                }
            }
        }
        Ok((specs, filters))
    }

    /// Allocate the direct-child specs as a contiguous arena batch and return a
    /// single root: the child itself if there is one, else a `Group`.
    fn materialize(&mut self, specs: Vec<ChildSpec>) -> Result<PatternId, String> {
        if specs.is_empty() {
            return Err("empty group graph pattern".to_string());
        }
        let start = self.ctx.pattern_count as u16;
        let len = specs.len() as u16;
        for spec in specs {
            match spec {
                ChildSpec::Triple { s, p, o } => {
                    self.ctx.alloc_pattern(Pattern::Triple {
                        subject: s,
                        predicate: p,
                        object: o,
                    })?;
                }
                ChildSpec::StarTriple {
                    is,
                    ip,
                    io,
                    op,
                    oo,
                } => {
                    self.ctx.alloc_pattern(Pattern::StarTriple {
                        inner_subject: is,
                        inner_predicate: ip,
                        inner_object: io,
                        outer_predicate: op,
                        outer_object: oo,
                    })?;
                }
                ChildSpec::Optional(inner) => {
                    self.ctx.alloc_pattern(Pattern::Optional { inner })?;
                }
                ChildSpec::Union(left, right) => {
                    self.ctx.alloc_pattern(Pattern::Union { left, right })?;
                }
                ChildSpec::Minus(inner) => {
                    self.ctx.alloc_pattern(Pattern::Minus { inner })?;
                }
            }
        }
        if len == 1 {
            Ok(start)
        } else {
            self.ctx.alloc_pattern(Pattern::Group {
                start_idx: start,
                len,
            })
        }
    }

    /// Parse a `FILTER` expression: either `( expr )` or a bare `WORD(args)`
    /// builtin. Collects the balanced-paren token span and parses it.
    fn parse_filter_expr(&mut self) -> Result<ExpressionId, String> {
        // Collect the token span of a parenthesised expression.
        if !matches!(self.peek(), Some(Token::Punct('('))) {
            return Err("expected '(' after FILTER".to_string());
        }
        let start = self.pos + 1;
        let mut depth = 0i32;
        let mut i = self.pos;
        while i < self.tokens.len() {
            match &self.tokens[i] {
                Token::Punct('(') => depth += 1,
                Token::Punct(')') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            return Err("unbalanced parentheses in FILTER".to_string());
        }
        let expr_tokens = &self.tokens[start..i];
        self.pos = i + 1; // past ')'
        parse_expression(expr_tokens, self.ctx, self.prefixes)
    }

    fn parse_triple(&mut self) -> Result<ChildSpec, String> {
        let s = self.term()?;
        let p = self.term()?;
        // Consume a trailing property-path quantifier (`+` / `*`) on the
        // predicate. Full property-path semantics (Pattern::PropertyPath) are a
        // later slice; for now the base predicate is used, matching the legacy
        // parser's behaviour (which hashed the whole token including the `+`).
        while matches!(self.peek(), Some(Token::Op(o)) if *o == "+" || *o == "*") {
            self.pos += 1;
        }
        let o = self.term()?;
        Ok(ChildSpec::Triple { s, p, o })
    }

    fn parse_star_triple(&mut self) -> Result<ChildSpec, String> {
        // consume `<<`
        self.pos += 1;
        let is = self.term()?;
        let ip = self.term()?;
        let io = self.term()?;
        if !matches!(self.peek(), Some(Token::StarClose)) {
            return Err("expected '>>' in quoted triple pattern".to_string());
        }
        self.pos += 1;
        let op = self.term()?;
        let oo = self.term()?;
        Ok(ChildSpec::StarTriple {
            is,
            ip,
            io,
            op,
            oo,
        })
    }

    /// Flatten an already-built plain nested group `id` into the parent's spec
    /// list. A multi-child `Group` contributes each of its children; a `Filter`
    /// hoists its expression and recurses into its inner pattern; any other
    /// single node contributes one spec.
    fn flatten_group_into(
        &mut self,
        id: PatternId,
        specs: &mut Vec<ChildSpec>,
        filters: &mut Vec<ExpressionId>,
    ) {
        let pat = self.ctx.patterns[id as usize];
        match pat {
            Pattern::Group { start_idx, len } => {
                for i in start_idx..(start_idx + len) {
                    self.flatten_group_into(i, specs, filters);
                }
            }
            Pattern::Filter {
                pattern,
                expression,
            } => {
                filters.push(expression);
                self.flatten_group_into(pattern, specs, filters);
            }
            Pattern::Triple {
                subject,
                predicate,
                object,
            } => specs.push(ChildSpec::Triple {
                s: subject,
                p: predicate,
                o: object,
            }),
            Pattern::StarTriple {
                inner_subject,
                inner_predicate,
                inner_object,
                outer_predicate,
                outer_object,
            } => specs.push(ChildSpec::StarTriple {
                is: inner_subject,
                ip: inner_predicate,
                io: inner_object,
                op: outer_predicate,
                oo: outer_object,
            }),
            Pattern::Union { left, right } => specs.push(ChildSpec::Union(left, right)),
            Pattern::Optional { inner } => specs.push(ChildSpec::Optional(inner)),
            Pattern::Minus { inner } => specs.push(ChildSpec::Minus(inner)),
            // Anything else (Graph/Service/PropertyPath/AsOf) is already a single
            // built node; carry it as an Optional inner, which the current
            // join-only planner treats as a plain join (see module doc).
            _ => specs.push(ChildSpec::Optional(id)),
        }
    }

    /// Resolve one term token to a `u64`, matching `parse_term`'s convention
    /// (variables → their id, IRIs/literals → 60-bit token, `a` → rdf:type).
    fn term(&mut self) -> Result<u64, String> {
        let tok = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| "unexpected end of pattern (expected a term)".to_string())?
            .clone();
        self.pos += 1;
        match tok {
            Token::Var(name) => Ok(self.ctx.register_variable(&name)? as u64),
            Token::Iri(iri) => Ok(crate::lexicon::generate_60bit_token(iri.as_bytes())),
            Token::Prefixed(prefix, local) => {
                let expanded = match self.prefixes.get(&prefix) {
                    Some(base) => format!("{base}{local}"),
                    None => format!("{prefix}:{local}"),
                };
                Ok(crate::lexicon::generate_60bit_token(expanded.as_bytes()))
            }
            Token::Str { value, .. } => Ok(crate::lexicon::generate_60bit_token(value.as_bytes())),
            Token::Num(text) => Ok(text.parse::<u64>().unwrap_or_else(|_| {
                crate::lexicon::generate_60bit_token(text.as_bytes())
            })),
            Token::Bool(b) => Ok(crate::lexicon::generate_60bit_token(
                if b { b"true" } else { b"false" },
            )),
            Token::Word(w) if w == "a" => Ok(crate::lexicon::generate_60bit_token(
                b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
            )),
            // Any other bareword is treated as an IRI token, matching the legacy
            // `parse_term`'s permissive fallthrough (e.g. `?s knows ?o`).
            Token::Word(w) => Ok(crate::lexicon::generate_60bit_token(w.as_bytes())),
            other => Err(format!("invalid term token: {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparql_ast::SparqlQueryContext;

    fn root_pattern(input: &str) -> (SparqlQueryContext, Pattern) {
        let mut ctx = SparqlQueryContext::new();
        let id = parse_where_group(input, &mut ctx, &HashMap::new()).unwrap();
        let pat = ctx.patterns[id as usize];
        (ctx, pat)
    }

    #[test]
    fn parses_optional() {
        let (_ctx, pat) = root_pattern("{ ?s ?p ?o . OPTIONAL { ?s ?x ?y } }");
        // Root is a Group of [Triple, Optional].
        assert!(matches!(pat, Pattern::Group { len: 2, .. }));
    }

    #[test]
    fn parses_union() {
        let (ctx, pat) = root_pattern("{ { ?s ?p ?o } UNION { ?a ?b ?c } }");
        // A single UNION child → the group collapses to the Union node.
        assert!(matches!(pat, Pattern::Union { .. }), "got {pat:?}");
        let _ = ctx;
    }

    #[test]
    fn parses_minus() {
        let (_ctx, pat) = root_pattern("{ ?s ?p ?o . MINUS { ?s ?x ?y } }");
        assert!(matches!(pat, Pattern::Group { len: 2, .. }));
    }

    #[test]
    fn parses_filter_in_group() {
        let (_ctx, pat) = root_pattern("{ ?s ?p ?o . FILTER(?o >= 18) }");
        assert!(matches!(pat, Pattern::Filter { .. }));
    }

    #[test]
    fn parses_plain_bgp() {
        let (_ctx, pat) = root_pattern("{ ?s ?p ?o . ?a ?b ?c }");
        assert!(matches!(pat, Pattern::Group { len: 2, .. }));
    }

    #[test]
    fn bind_is_deferred_with_clear_error() {
        let mut ctx = SparqlQueryContext::new();
        let err = parse_where_group("{ ?s ?p ?o . BIND(?o AS ?x) }", &mut ctx, &HashMap::new())
            .unwrap_err();
        assert!(err.contains("BIND"), "got {err}");
    }
}
