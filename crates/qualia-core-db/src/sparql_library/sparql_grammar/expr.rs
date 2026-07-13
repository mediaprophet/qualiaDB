//! SPARQL expression parser — precedence-climbing recursive descent over the
//! token stream, producing `Expression` nodes into the `SparqlQueryContext`
//! arena. Used for `FILTER`, `BIND`, and `HAVING` expressions.
//!
//! Precedence (lowest → highest), per the SPARQL 1.1 grammar:
//! `||` < `&&` < (`=` `!=` `<` `<=` `>` `>=`) < (`+` `-`) < (`*` `/`) < unary < primary.
//!
//! ## Encoding note (matches the evaluator's model)
//! `ExpressionEvaluator` maps both `Variable` and `Literal` to
//! `EvalResult::Numeric(u64)`, and comparisons require both operands to be the
//! same `EvalResult` variant. So a constant used in an expression (IRI, string,
//! number, boolean) is encoded as `Expression::Literal(hash_or_value)` — using
//! exactly the same term encoding the triple-pattern parser (`parse_term`) uses
//! — so `?x = <iri>` / `?name = "Alice"` / `?age >= 18` all compare correctly.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::sparql_ast::{
    BinaryOp, Expression, ExpressionId, Function, LiteralTable, SparqlQueryContext, UnaryOp,
};
use crate::sparql_library::sparql_grammar::tokenizer::Token;

thread_local! {
    /// Literal text (`hash -> string`) collected while parsing the current query,
    /// so `geof:*`/text functions can recover it. SPARQL parsing is single-
    /// threaded and non-reentrant here, so a thread-local avoids threading a
    /// `&mut LiteralTable` through every parser function. `parse_sparql` resets
    /// it before and takes it after a parse.
    static PARSE_LITERALS: RefCell<LiteralTable> = RefCell::new(LiteralTable::new());
}

/// Clear the parse-time literal table (call before parsing a query).
pub fn reset_parse_literals() {
    PARSE_LITERALS.with(|l| *l.borrow_mut() = LiteralTable::new());
}

/// Take the literal table collected during the last parse.
pub fn take_parse_literals() -> LiteralTable {
    PARSE_LITERALS.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

fn record_parse_literal_tagged(hash: u64, text: &str, lang: Option<&str>, datatype: Option<&str>) {
    PARSE_LITERALS.with(|l| l.borrow_mut().intern_tagged(hash, text, lang, datatype));
}

/// Parse a full expression from `tokens`, allocating nodes into `ctx`.
/// Returns the root `ExpressionId`. Errors on malformed syntax or arena overflow.
pub fn parse_expression(
    tokens: &[Token],
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<ExpressionId, String> {
    let mut p = ExprParser {
        tokens,
        pos: 0,
        ctx,
        prefixes,
    };
    let id = p.parse_or()?;
    if p.pos != tokens.len() {
        return Err(format!(
            "unexpected trailing tokens in expression at index {}",
            p.pos
        ));
    }
    Ok(id)
}

struct ExprParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    ctx: &'a mut SparqlQueryContext,
    prefixes: &'a HashMap<String, String>,
}

impl<'a> ExprParser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn eat_op(&mut self, op: &str) -> bool {
        if let Some(Token::Op(o)) = self.peek() {
            if *o == op {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn eat_punct(&mut self, c: char) -> bool {
        if let Some(Token::Punct(p)) = self.peek() {
            if *p == c {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn expect_punct(&mut self, c: char) -> Result<(), String> {
        if self.eat_punct(c) {
            Ok(())
        } else {
            Err(format!("expected '{c}' at token {}", self.pos))
        }
    }

    // `||`
    fn parse_or(&mut self) -> Result<ExpressionId, String> {
        let mut left = self.parse_and()?;
        while self.eat_op("||") {
            let right = self.parse_and()?;
            left = self.alloc(Expression::BinaryOp {
                op: BinaryOp::Or,
                left,
                right,
            })?;
        }
        Ok(left)
    }

    // `&&`
    fn parse_and(&mut self) -> Result<ExpressionId, String> {
        let mut left = self.parse_comparison()?;
        while self.eat_op("&&") {
            let right = self.parse_comparison()?;
            left = self.alloc(Expression::BinaryOp {
                op: BinaryOp::And,
                left,
                right,
            })?;
        }
        Ok(left)
    }

    // `=` `!=` `<` `<=` `>` `>=`
    fn parse_comparison(&mut self) -> Result<ExpressionId, String> {
        let left = self.parse_additive()?;
        let op = match self.peek() {
            Some(Token::Op("=")) => Some(BinaryOp::Equal),
            Some(Token::Op("!=")) => Some(BinaryOp::NotEqual),
            Some(Token::Op("<")) => Some(BinaryOp::LessThan),
            Some(Token::Op("<=")) => Some(BinaryOp::LessThanOrEqual),
            Some(Token::Op(">")) => Some(BinaryOp::GreaterThan),
            Some(Token::Op(">=")) => Some(BinaryOp::GreaterThanOrEqual),
            _ => None,
        };
        if let Some(op) = op {
            self.pos += 1;
            let right = self.parse_additive()?;
            return self.alloc(Expression::BinaryOp { op, left, right });
        }
        Ok(left)
    }

    // `+` `-`
    fn parse_additive(&mut self) -> Result<ExpressionId, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.eat_op("+") {
                BinaryOp::Add
            } else if self.eat_op("-") {
                BinaryOp::Subtract
            } else {
                break;
            };
            let right = self.parse_multiplicative()?;
            left = self.alloc(Expression::BinaryOp { op, left, right })?;
        }
        Ok(left)
    }

    // `*` `/`
    fn parse_multiplicative(&mut self) -> Result<ExpressionId, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.eat_op("*") {
                BinaryOp::Multiply
            } else if self.eat_op("/") {
                BinaryOp::Divide
            } else {
                break;
            };
            let right = self.parse_unary()?;
            left = self.alloc(Expression::BinaryOp { op, left, right })?;
        }
        Ok(left)
    }

    // unary `!` `+` `-`
    fn parse_unary(&mut self) -> Result<ExpressionId, String> {
        let op = match self.peek() {
            Some(Token::Op("!")) => Some(UnaryOp::Not),
            Some(Token::Op("+")) => Some(UnaryOp::Plus),
            Some(Token::Op("-")) => Some(UnaryOp::Minus),
            _ => None,
        };
        if let Some(op) = op {
            self.pos += 1;
            let expr = self.parse_unary()?;
            return self.alloc(Expression::UnaryOp { op, expr });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<ExpressionId, String> {
        // `( expr )`
        if self.eat_punct('(') {
            let inner = self.parse_or()?;
            self.expect_punct(')')?;
            return Ok(inner);
        }

        // `<< s p o >>` embedded triple (constant or variable terms).
        if let Some(Token::StarOpen) = self.peek() {
            return self.parse_embedded_triple();
        }

        // `EXISTS { … }` / `NOT EXISTS { … }` inside a bracketed expression
        // (e.g. `FILTER( EXISTS { … } && ?x > 1 )`). The inner group is parsed by
        // the group-graph-pattern parser over the current token slice.
        let is_exists =
            matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case("EXISTS"));
        let is_not_exists =
            matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case("NOT"))
                && matches!(self.tokens.get(self.pos + 1),
                    Some(Token::Word(w)) if w.eq_ignore_ascii_case("EXISTS"));
        if is_exists || is_not_exists {
            let mut negated = false;
            if is_not_exists {
                self.pos += 1; // consume NOT
                negated = true;
            }
            self.pos += 1; // consume EXISTS; self.pos now indexes '{'
            let (pattern, new_pos) = super::pattern::parse_group_tokens(
                self.tokens,
                self.pos,
                self.ctx,
                self.prefixes,
            )?;
            self.pos = new_pos;
            return self.alloc(Expression::Exists { pattern, negated });
        }

        let tok = self
            .bump()
            .ok_or_else(|| "unexpected end of expression".to_string())?
            .clone();

        match tok {
            Token::Var(name) => {
                let vid = self.ctx.register_variable(&name)?;
                self.alloc(Expression::Variable(vid))
            }
            Token::Num(text) => {
                let value = encode_number(&text);
                self.alloc(Expression::Literal(value))
            }
            Token::Bool(b) => self.alloc(Expression::Literal(if b { 1 } else { 0 })),
            Token::Str { value, lang, datatype } => {
                // Expand a prefixed datatype (e.g. `xsd:integer`) against the prefix map
                // so DATATYPE(?x) is comparable to the query's own datatype IRI term.
                let dt = datatype.as_ref().map(|d| {
                    if d.contains("://") {
                        d.clone()
                    } else if let Some((p, local)) = d.split_once(':') {
                        match self.prefixes.get(p) {
                            Some(base) => format!("{base}{local}"),
                            None => d.clone(),
                        }
                    } else {
                        d.clone()
                    }
                });
                // A plain, lang-tagged, and datatyped literal of the same text are
                // distinct terms — hash accordingly so LANG/DATATYPE read the tag back.
                let h = crate::sparql_ast::literal_term_hash(&value, lang.as_deref(), dt.as_deref());
                record_parse_literal_tagged(h, &value, lang.as_deref(), dt.as_deref());
                self.alloc(Expression::Literal(h))
            }
            Token::Iri(iri) => {
                // An IRI directly followed by `(` is an extension function call.
                if matches!(self.peek(), Some(Token::Punct('('))) {
                    return self.parse_custom_call(&iri);
                }
                let h = crate::lexicon::generate_60bit_token(iri.as_bytes());
                self.alloc(Expression::Literal(h))
            }
            Token::Prefixed(prefix, local) => {
                // A prefixed name followed by `(` is an extension function call;
                // otherwise a constant IRI.
                if matches!(self.peek(), Some(Token::Punct('('))) {
                    let iri = self.expand_function_iri(&prefix, &local);
                    return self.parse_custom_call(&iri);
                }
                let expanded = match self.prefixes.get(&prefix) {
                    Some(base) => format!("{base}{local}"),
                    None => format!("{prefix}:{local}"),
                };
                let h = crate::lexicon::generate_60bit_token(expanded.as_bytes());
                self.alloc(Expression::Literal(h))
            }
            Token::Word(word) => self.parse_word_or_call(&word),
            other => Err(format!("unexpected token in expression: {other:?}")),
        }
    }

    /// A bare word is either a builtin function call (`WORD ( args )`) or the
    /// boolean/`a` keyword handled elsewhere. Unknown words error rather than
    /// silently passing.
    fn parse_word_or_call(&mut self, word: &str) -> Result<ExpressionId, String> {
        let func = builtin_function(word)
            .ok_or_else(|| format!("unknown function or identifier '{word}'"))?;
        self.expect_punct('(')?;
        // Parse comma-separated argument expressions.
        let mut arg_ids: Vec<ExpressionId> = Vec::new();
        if !matches!(self.peek(), Some(Token::Punct(')'))) {
            loop {
                arg_ids.push(self.parse_or()?);
                if self.eat_punct(',') {
                    continue;
                }
                break;
            }
        }
        self.expect_punct(')')?;

        // Copy arg ids into the ctx.function_args table; the Function node stores
        // (args_start, args_len) into it.
        let args_start = self.ctx.function_arg_count as u16;
        for id in &arg_ids {
            if (self.ctx.function_arg_count as usize) >= self.ctx.function_args.len() {
                return Err("too many function arguments (arena full)".to_string());
            }
            self.ctx.function_args[self.ctx.function_arg_count as usize] = *id;
            self.ctx.function_arg_count += 1;
        }
        self.alloc(Expression::Function {
            func,
            args_start,
            args_len: arg_ids.len() as u16,
        })
    }

    /// Expand a prefixed function name to its IRI. Known extension prefixes with
    /// no declared mapping fall back to their standard base (currently `geof:`),
    /// so `geof:sfWithin(...)` works even without a `PREFIX geof:` declaration.
    fn expand_function_iri(&self, prefix: &str, local: &str) -> String {
        if let Some(base) = self.prefixes.get(prefix) {
            return format!("{base}{local}");
        }
        if prefix == "geof" {
            if let Some(iri) = crate::sparql_library::geosparql::geo_function_iri(local) {
                return iri.to_string();
            }
            return format!("http://www.opengis.net/def/function/geosparql/{local}");
        }
        format!("{prefix}:{local}")
    }

    /// Parse an extension-function call `IRI ( args )` into
    /// `Expression::Function { func: Function::Custom(q_hash(iri)), … }`.
    fn parse_custom_call(&mut self, iri: &str) -> Result<ExpressionId, String> {
        let func = Function::Custom(crate::lexicon::generate_60bit_token(iri.as_bytes()));
        self.expect_punct('(')?;
        let mut arg_ids: Vec<ExpressionId> = Vec::new();
        if !matches!(self.peek(), Some(Token::Punct(')'))) {
            loop {
                arg_ids.push(self.parse_or()?);
                if self.eat_punct(',') {
                    continue;
                }
                break;
            }
        }
        self.expect_punct(')')?;
        let args_start = self.ctx.function_arg_count as u16;
        for id in &arg_ids {
            if (self.ctx.function_arg_count as usize) >= self.ctx.function_args.len() {
                return Err("too many function arguments (arena full)".to_string());
            }
            self.ctx.function_args[self.ctx.function_arg_count as usize] = *id;
            self.ctx.function_arg_count += 1;
        }
        self.alloc(Expression::Function {
            func,
            args_start,
            args_len: arg_ids.len() as u16,
        })
    }

    fn parse_embedded_triple(&mut self) -> Result<ExpressionId, String> {
        // consume `<<`
        self.bump();
        let s = self.embedded_term()?;
        let p = self.embedded_term()?;
        let o = self.embedded_term()?;
        match self.peek() {
            Some(Token::StarClose) => {
                self.pos += 1;
            }
            _ => return Err("expected '>>' to close embedded triple".to_string()),
        }
        self.alloc(Expression::EmbeddedTriple {
            subject: s,
            predicate: p,
            object: o,
        })
    }

    /// A term inside an embedded triple resolves to a `u64` (constant hash or,
    /// for a variable, its id) — matching the `parse_term` convention.
    fn embedded_term(&mut self) -> Result<u64, String> {
        let tok = self
            .bump()
            .ok_or_else(|| "unexpected end inside embedded triple".to_string())?
            .clone();
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
            Token::Num(text) => Ok(encode_number(&text)),
            other => Err(format!("invalid term inside embedded triple: {other:?}")),
        }
    }

    fn alloc(&mut self, expr: Expression) -> Result<ExpressionId, String> {
        self.ctx.alloc_expression(expr)
    }
}

/// Encode a numeric literal to match the triple-pattern convention: an integer
/// is stored as its raw `u64` value (so numeric comparisons work against
/// raw-encoded object values); a non-integer is interned by text (equality only).
fn encode_number(text: &str) -> u64 {
    if let Ok(n) = text.parse::<u64>() {
        n
    } else if let Ok(n) = text.parse::<i64>() {
        n as u64
    } else {
        crate::lexicon::generate_60bit_token(text.as_bytes())
    }
}

/// Map a builtin function name (case-insensitive) to the `Function` enum.
/// Extension functions (prefixed / IRI names) are handled by the registry slice.
fn builtin_function(word: &str) -> Option<Function> {
    let up = word.to_ascii_uppercase();
    Some(match up.as_str() {
        "STR" => Function::Str,
        "LANG" => Function::Lang,
        "LANGMATCHES" => Function::LangMatches,
        "DATATYPE" => Function::Datatype,
        "BOUND" => Function::Bound,
        "IRI" => Function::Iri,
        "URI" => Function::Uri,
        "BNODE" => Function::Bnode,
        "RAND" => Function::Rand,
        "ABS" => Function::Abs,
        "CEIL" => Function::Ceil,
        "FLOOR" => Function::Floor,
        "ROUND" => Function::Round,
        "CONCAT" => Function::Concat,
        "SUBSTR" => Function::Substring,
        "STRLEN" => Function::Strlen,
        "UCASE" => Function::Ucase,
        "LCASE" => Function::Lcase,
        "ENCODE_FOR_URI" => Function::EncodeForUri,
        "CONTAINS" => Function::Contains,
        "STRSTARTS" => Function::VarStarts,
        "STRENDS" => Function::VarEnds,
        "STRBEFORE" => Function::StrBefore,
        "STRAFTER" => Function::StrAfter,
        "YEAR" => Function::Year,
        "MONTH" => Function::Month,
        "DAY" => Function::Day,
        "HOURS" => Function::Hours,
        "MINUTES" => Function::Minutes,
        "SECONDS" => Function::Seconds,
        "TIMEZONE" => Function::Timezone,
        "TZ" => Function::Tz,
        "NOW" => Function::Now,
        "UUID" => Function::Uuid,
        "STRUUID" => Function::StrUuid,
        "COALESCE" => Function::Coalesce,
        "IF" => Function::If,
        "STRLANG" => Function::StrLang,
        "STRDT" => Function::StrDt,
        "SAMETERM" => Function::SameTerm,
        "ISIRI" => Function::IsIri,
        "ISURI" => Function::IsUri,
        "ISBLANK" => Function::IsBlank,
        "ISLITERAL" => Function::IsLiteral,
        "ISNUMERIC" => Function::IsNumeric,
        "REGEX" => Function::Regex,
        "TRIPLE" => Function::Triple,
        "SUBJECT" => Function::TripleSubject,
        "PREDICATE" => Function::TriplePredicate,
        "OBJECT" => Function::TripleObject,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparql_ast::SparqlQueryContext;
    use crate::sparql_library::sparql_grammar::tokenizer::tokenize;

    fn parse(input: &str) -> (SparqlQueryContext, ExpressionId) {
        let mut ctx = SparqlQueryContext::new();
        let toks = tokenize(input).unwrap();
        let prefixes = HashMap::new();
        let id = parse_expression(&toks, &mut ctx, &prefixes).unwrap();
        (ctx, id)
    }

    #[test]
    fn parses_numeric_comparison() {
        let (ctx, root) = parse("?age >= 18");
        match ctx.expressions[root as usize] {
            Expression::BinaryOp {
                op: BinaryOp::GreaterThanOrEqual,
                left,
                right,
            } => {
                assert!(matches!(ctx.expressions[left as usize], Expression::Variable(_)));
                assert_eq!(ctx.expressions[right as usize], Expression::Literal(18));
            }
            other => panic!("expected >= comparison, got {other:?}"),
        }
    }

    #[test]
    fn respects_and_or_precedence() {
        // a || b && c  parses as  a || (b && c)
        let (ctx, root) = parse("?a || ?b && ?c");
        match ctx.expressions[root as usize] {
            Expression::BinaryOp {
                op: BinaryOp::Or,
                right,
                ..
            } => {
                assert!(matches!(
                    ctx.expressions[right as usize],
                    Expression::BinaryOp {
                        op: BinaryOp::And,
                        ..
                    }
                ));
            }
            other => panic!("expected top-level OR, got {other:?}"),
        }
    }

    #[test]
    fn arithmetic_precedence() {
        // 1 + 2 * 3  →  1 + (2 * 3)
        let (ctx, root) = parse("1 + 2 * 3");
        match ctx.expressions[root as usize] {
            Expression::BinaryOp {
                op: BinaryOp::Add,
                right,
                ..
            } => assert!(matches!(
                ctx.expressions[right as usize],
                Expression::BinaryOp {
                    op: BinaryOp::Multiply,
                    ..
                }
            )),
            other => panic!("expected top-level Add, got {other:?}"),
        }
    }

    #[test]
    fn parses_function_call_with_args() {
        let (ctx, root) = parse("REGEX(?name, \"^A\")");
        match ctx.expressions[root as usize] {
            Expression::Function {
                func: Function::Regex,
                args_start,
                args_len,
            } => {
                assert_eq!(args_len, 2);
                let a0 = ctx.function_args[args_start as usize];
                assert!(matches!(ctx.expressions[a0 as usize], Expression::Variable(_)));
            }
            other => panic!("expected REGEX function, got {other:?}"),
        }
    }

    #[test]
    fn parses_parenthesised_grouping() {
        // (?a || ?b) && ?c  →  top-level AND
        let (ctx, root) = parse("(?a || ?b) && ?c");
        assert!(matches!(
            ctx.expressions[root as usize],
            Expression::BinaryOp {
                op: BinaryOp::And,
                ..
            }
        ));
    }

    #[test]
    fn unknown_function_errors() {
        let mut ctx = SparqlQueryContext::new();
        let toks = tokenize("NOTAFUNC(?x)").unwrap();
        assert!(parse_expression(&toks, &mut ctx, &HashMap::new()).is_err());
    }

    #[test]
    fn parses_embedded_triple_expression() {
        let (ctx, root) = parse("<< ?s ?p ?o >>");
        assert!(matches!(
            ctx.expressions[root as usize],
            Expression::EmbeddedTriple { .. }
        ));
    }
}
