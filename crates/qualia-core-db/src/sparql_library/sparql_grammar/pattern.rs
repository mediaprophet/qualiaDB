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
//! ## Engine semantics
//! An `OPTIONAL` group child lowers to a real left-join and a `MINUS` child to a
//! real anti-join (SPARQL 1.1: a left solution survives MINUS unless a right
//! solution is compatible with it *and* shares a bound variable). Filter/BIND
//! scoping over a group is simplified (both apply over the whole group's join
//! result — see below), which is the remaining deliberate simplification here.

use std::collections::HashMap;

use crate::sparql_ast::{
    Expression, ExpressionId, Pattern, PatternId, SparqlQuery, SparqlQueryContext, VariableId,
};

/// Render a token slice back to a SPARQL fragment. Used to hand a sub-`SELECT`'s
/// tokens to the (string-based) SELECT parser. Whitespace-joined, which is safe
/// for SPARQL. Datatype IRIs are re-bracketed; a prefixed datatype is left as
/// `prefix:local` for the parser to expand against the same prefix map.
fn render_tokens(tokens: &[Token]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(tokens.len());
    for t in tokens {
        parts.push(match t {
            Token::Var(v) => v.clone(),
            Token::Iri(i) => format!("<{i}>"),
            Token::Prefixed(p, l) => format!("{p}:{l}"),
            Token::Str {
                value,
                lang,
                datatype,
            } => {
                let mut s = format!("\"{value}\"");
                if let Some(l) = lang {
                    s.push('@');
                    s.push_str(l);
                } else if let Some(d) = datatype {
                    if d.contains("://") {
                        s.push_str(&format!("^^<{d}>"));
                    } else {
                        s.push_str(&format!("^^{d}"));
                    }
                }
                s
            }
            Token::Num(n) => n.clone(),
            Token::Bool(b) => (if *b { "true" } else { "false" }).to_string(),
            Token::Word(w) => w.clone(),
            Token::Punct(c) => c.to_string(),
            Token::Op(o) => (*o).to_string(),
            Token::StarOpen => "<<".to_string(),
            Token::StarClose => ">>".to_string(),
        });
    }
    parts.join(" ")
}
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

/// Parse a `{ … }` group graph pattern directly from a token slice starting at
/// `pos` (which must index the opening `{`). Returns the root `PatternId` and
/// the position just past the closing `}`. Used by the FILTER-expression parser
/// to parse an `EXISTS { … }` group embedded in a bracketed expression.
pub(crate) fn parse_group_tokens(
    tokens: &[Token],
    pos: usize,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<(PatternId, usize), String> {
    let mut p = PatternParser {
        tokens,
        pos,
        ctx,
        prefixes,
    };
    let id = p.parse_group()?;
    Ok((id, p.pos))
}

/// One direct child of a group, allocated as exactly one arena node in the
/// contiguous batch at group close (inner sub-patterns already allocated).
enum ChildSpec {
    Triple {
        s: u64,
        p: u64,
        o: u64,
    },
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
    Service {
        endpoint: u64,
        inner: PatternId,
    },
    Graph {
        graph_var_or_id: u64,
        inner: PatternId,
    },
    SubSelect {
        query_id: u16,
    },
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
        let (specs, filters, binds) = self.parse_group_body()?;
        let mut root = self.materialize(specs)?;
        // BINDs wrap the group first (in parse order — innermost = first), so a
        // later BIND and any FILTER can see an earlier BIND's variable. Scoping
        // is simplified the same way FILTER is (see module doc): both apply over
        // the whole group's join result.
        for (expr_id, var) in binds {
            root = self.ctx.alloc_pattern(Pattern::Bind {
                pattern: root,
                var,
                expression: expr_id,
            })?;
        }
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
    #[allow(clippy::type_complexity)]
    fn parse_group_body(
        &mut self,
    ) -> Result<
        (
            Vec<ChildSpec>,
            Vec<ExpressionId>,
            Vec<(ExpressionId, VariableId)>,
        ),
        String,
    > {
        let mut specs: Vec<ChildSpec> = Vec::new();
        let mut filters: Vec<ExpressionId> = Vec::new();
        let mut binds: Vec<(ExpressionId, VariableId)> = Vec::new();

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
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("SERVICE") => {
                    self.pos += 1;
                    // Optional SILENT keyword.
                    if self.peek_word_ci("SILENT") {
                        self.pos += 1;
                    }
                    let endpoint = self.service_endpoint()?;
                    let inner = self.parse_group()?;
                    specs.push(ChildSpec::Service { endpoint, inner });
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("GRAPH") => {
                    self.pos += 1;
                    // `GRAPH <iri> { … }` (a named graph) or `GRAPH ?g { … }`
                    // (bind ?g to each matching named graph). `term()` reads the
                    // graph term the same way as any triple term — a variable is
                    // registered and returned as its id, an IRI as its hash.
                    let graph_var_or_id = self.term()?;
                    let inner = self.parse_group()?;
                    specs.push(ChildSpec::Graph {
                        graph_var_or_id,
                        inner,
                    });
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("FILTER") => {
                    self.pos += 1;
                    let expr_id = self.parse_filter_expr()?;
                    filters.push(expr_id);
                }
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("BIND") => {
                    self.pos += 1;
                    let (expr_id, var_id) = self.parse_bind()?;
                    binds.push((expr_id, var_id));
                }
                Some(Token::Punct('{'))
                    if matches!(self.tokens.get(self.pos + 1),
                        Some(Token::Word(w)) if w.eq_ignore_ascii_case("SELECT")) =>
                {
                    // `{ SELECT … }` sub-select.
                    let query_id = self.parse_sub_select()?;
                    specs.push(ChildSpec::SubSelect { query_id });
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
                        self.flatten_group_into(left, &mut specs, &mut filters, &mut binds);
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
        Ok((specs, filters, binds))
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
                ChildSpec::StarTriple { is, ip, io, op, oo } => {
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
                ChildSpec::Service { endpoint, inner } => {
                    self.ctx.alloc_pattern(Pattern::Service {
                        endpoint_did_id: endpoint,
                        inner_pattern: inner,
                    })?;
                }
                ChildSpec::Graph {
                    graph_var_or_id,
                    inner,
                } => {
                    self.ctx.alloc_pattern(Pattern::Graph {
                        graph_var_or_id,
                        inner,
                    })?;
                }
                ChildSpec::SubSelect { query_id } => {
                    self.ctx.alloc_pattern(Pattern::SubSelect { query_id })?;
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
        // `FILTER EXISTS { … }` / `FILTER NOT EXISTS { … }` — an unparenthesised
        // built-in constraint (SPARQL 1.1 [69] Constraint → BuiltInCall).
        let not_exists = self.peek_word_ci("NOT")
            && matches!(self.tokens.get(self.pos + 1),
                Some(Token::Word(w)) if w.eq_ignore_ascii_case("EXISTS"));
        if self.peek_word_ci("EXISTS") || not_exists {
            return self.parse_exists_constraint();
        }

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

    /// Parse `EXISTS { … }` or `NOT EXISTS { … }` (positioned at `NOT`/`EXISTS`)
    /// into an `Expression::Exists` over the inner group graph pattern.
    fn parse_exists_constraint(&mut self) -> Result<ExpressionId, String> {
        let mut negated = false;
        if self.peek_word_ci("NOT") {
            self.pos += 1;
            negated = true;
        }
        if !self.peek_word_ci("EXISTS") {
            return Err("expected EXISTS".to_string());
        }
        self.pos += 1;
        let pattern = self.parse_group()?;
        self.ctx
            .alloc_expression(Expression::Exists { pattern, negated })
    }

    /// Index of the `}` token that closes the `{` at `open` (balanced).
    fn matching_brace(&self, open: usize) -> Result<usize, String> {
        let mut depth = 0i32;
        for (i, t) in self.tokens.iter().enumerate().skip(open) {
            match t {
                Token::Punct('{') => depth += 1,
                Token::Punct('}') => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(i);
                    }
                }
                _ => {}
            }
        }
        Err("unbalanced braces in sub-SELECT".to_string())
    }

    /// Parse `{ SELECT … }` (positioned at the opening `{`) into a stored
    /// subquery, returning its id. The inner tokens are rendered back to a
    /// SPARQL string and handed to the full SELECT parser (which shares this
    /// `ctx`, so the sub-select's variables interned by name line up with the
    /// enclosing scope), giving sub-selects the same feature set as a top-level
    /// query (projection, DISTINCT, GROUP BY, ORDER BY, LIMIT).
    fn parse_sub_select(&mut self) -> Result<u16, String> {
        let open = self.pos;
        let close = self.matching_brace(open)?;
        let query_str = render_tokens(&self.tokens[open + 1..close]);
        self.pos = close + 1; // consume through the closing '}'
        let select = crate::sparql_library::sparql_parser::parse_select_query(
            &query_str,
            self.ctx,
            self.prefixes,
        )?;
        self.ctx.alloc_subquery(SparqlQuery::Select(select))
    }

    /// Parse `BIND ( expr AS ?var )` → (expression id, target variable id).
    ///
    /// The value-producing case — numeric / boolean / term / already-interned
    /// string results (`?a + ?b`, `STRLEN(?x)`, `IF(...)`, `?x`) — binds a real
    /// `u64`. A string-*producing* expression (`CONCAT`/`SUBSTR`/…) evaluates to
    /// an error at runtime because the zero-heap arena has no channel to intern
    /// a new string; per SPARQL, that error leaves the variable unbound rather
    /// than failing the query.
    fn parse_bind(&mut self) -> Result<(ExpressionId, VariableId), String> {
        if !matches!(self.peek(), Some(Token::Punct('('))) {
            return Err("expected '(' after BIND".to_string());
        }
        // Collect the balanced-paren span (same approach as parse_filter_expr).
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
            return Err("unbalanced parentheses in BIND".to_string());
        }
        let span = &self.tokens[start..i];
        self.pos = i + 1; // past ')'

        // Split on the top-level `AS` keyword: `expr AS ?var`.
        let as_pos = span
            .iter()
            .position(|t| matches!(t, Token::Word(w) if w.eq_ignore_ascii_case("AS")))
            .ok_or("BIND requires the form `BIND(expr AS ?var)`")?;
        let expr_tokens = &span[..as_pos];
        let var_tokens = &span[as_pos + 1..];
        if expr_tokens.is_empty() {
            return Err("BIND has no expression before AS".to_string());
        }
        let var_name = match var_tokens {
            [Token::Var(name)] => name.clone(),
            _ => return Err("BIND target must be a single ?variable".to_string()),
        };
        let expr_id = parse_expression(expr_tokens, self.ctx, self.prefixes)?;
        let var_id = self.ctx.register_variable(&var_name)?;
        Ok((expr_id, var_id))
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

    /// Resolve a SERVICE endpoint. Local endpoints (`local:` / `qualia:` / a
    /// `did:` IRI) get the executor's `0x8` DID prefix so its `Service` operator
    /// runs the inner pattern against the local graph. A remote `http(s)`
    /// endpoint is rejected with an honest error — real network egress is a
    /// governance decision and is intentionally not wired (see the plan). A
    /// variable endpoint (dynamic SERVICE) is likewise deferred.
    fn service_endpoint(&mut self) -> Result<u64, String> {
        const DID_PREFIX: u64 = 0x8000_0000_0000_0000;
        let tok = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| "expected SERVICE endpoint".to_string())?
            .clone();
        self.pos += 1;
        let iri = match tok {
            Token::Iri(iri) => iri,
            Token::Prefixed(prefix, local) => match self.prefixes.get(&prefix) {
                Some(base) => format!("{base}{local}"),
                None => format!("{prefix}:{local}"),
            },
            Token::Var(_) => {
                return Err("dynamic SERVICE endpoint (a variable) is not supported yet".to_string())
            }
            other => return Err(format!("invalid SERVICE endpoint token: {other:?}")),
        };
        let lower = iri.to_ascii_lowercase();
        if lower.starts_with("local:") || lower.starts_with("qualia:") || lower.starts_with("did:")
        {
            Ok(crate::lexicon::generate_60bit_token(iri.as_bytes()) | DID_PREFIX)
        } else if lower.starts_with("http://") || lower.starts_with("https://") {
            Err(format!(
                "remote SERVICE endpoint <{iri}> is not supported — network egress is \
                 governance-gated; use a local: or qualia: endpoint"
            ))
        } else {
            // Unknown scheme: treat as a local/opaque endpoint (local execution).
            Ok(crate::lexicon::generate_60bit_token(iri.as_bytes()) | DID_PREFIX)
        }
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
        Ok(ChildSpec::StarTriple { is, ip, io, op, oo })
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
        binds: &mut Vec<(ExpressionId, VariableId)>,
    ) {
        let pat = self.ctx.patterns[id as usize];
        match pat {
            Pattern::Group { start_idx, len } => {
                for i in start_idx..(start_idx + len) {
                    self.flatten_group_into(i, specs, filters, binds);
                }
            }
            Pattern::Filter {
                pattern,
                expression,
            } => {
                filters.push(expression);
                self.flatten_group_into(pattern, specs, filters, binds);
            }
            Pattern::Bind {
                pattern,
                var,
                expression,
            } => {
                binds.push((expression, var));
                self.flatten_group_into(pattern, specs, filters, binds);
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
            Pattern::Graph {
                graph_var_or_id,
                inner,
            } => specs.push(ChildSpec::Graph {
                graph_var_or_id,
                inner,
            }),
            Pattern::SubSelect { query_id } => specs.push(ChildSpec::SubSelect { query_id }),
            // Anything else (Service/PropertyPath/AsOf) is already a single
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
            Token::Num(text) => Ok(text
                .parse::<u64>()
                .unwrap_or_else(|_| crate::lexicon::generate_60bit_token(text.as_bytes()))),
            Token::Bool(b) => Ok(crate::lexicon::generate_60bit_token(if b {
                b"true"
            } else {
                b"false"
            })),
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
    fn parses_local_service() {
        let (_ctx, pat) = root_pattern("{ SERVICE <local:health> { ?s ?p ?o } }");
        assert!(matches!(pat, Pattern::Service { .. }), "got {pat:?}");
    }

    #[test]
    fn remote_service_is_rejected() {
        let mut ctx = SparqlQueryContext::new();
        let err = parse_where_group(
            "{ SERVICE <https://dbpedia.org/sparql> { ?s ?p ?o } }",
            &mut ctx,
            &HashMap::new(),
        )
        .unwrap_err();
        assert!(err.contains("egress"), "got {err}");
    }

    #[test]
    fn parses_bind_into_bind_node() {
        let (ctx, pat) = root_pattern("{ ?s ?p ?o . BIND(?o AS ?x) }");
        // BIND wraps the group in a Pattern::Bind whose target var is registered.
        match pat {
            Pattern::Bind { var, .. } => {
                // ?x is the last variable registered (?s ?p ?o then ?x).
                let x = ctx
                    .variable_hashes
                    .iter()
                    .position(|h| *h == crate::lexicon::generate_60bit_token(b"?x"))
                    .unwrap();
                assert_eq!(var as usize, x);
            }
            other => panic!("expected Pattern::Bind, got {other:?}"),
        }
    }

    #[test]
    fn bind_requires_as_var_form() {
        let mut ctx = SparqlQueryContext::new();
        let err =
            parse_where_group("{ ?s ?p ?o . BIND(?o) }", &mut ctx, &HashMap::new()).unwrap_err();
        assert!(err.contains("AS"), "got {err}");
    }
}
