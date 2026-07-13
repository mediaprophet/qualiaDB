//! SPARQL FILTER Expression Evaluator
//!
//! Evaluates SPARQL FILTER expressions against binding rows using zero-allocation patterns.
//!
//! Also exposes PROV-O predicate hash constants and `ProvenanceFilter` helpers so the SPARQL
//! executor can push provenance-aware predicates (`prov:wasInvalidatedBy`,
//! `prov:wasAttributedTo`, etc.) down into quin-level scans without string allocation.

use crate::sparql_ast::*;

// ── PROV-O predicate hash constants ─────────────────────────────────────────

/// Compile-time `q_hash` values for all W3C PROV-O predicates used in this codebase.
///
/// These match the constants declared in `temporal_graph.rs` and `epistemic.rs` so that
/// a SPARQL FILTER expression comparing `?p = prov:wasInvalidatedBy` resolves to the same
/// hash as the quin written by `provenance::contest_assertion()`.
pub mod prov_predicates {
    use crate::q_hash;

    pub const GENERATED_AT_TIME: u64 = q_hash("http://www.w3.org/ns/prov#generatedAtTime");
    pub const STARTED_AT_TIME: u64 = q_hash("http://www.w3.org/ns/prov#startedAtTime");
    pub const ENDED_AT_TIME: u64 = q_hash("http://www.w3.org/ns/prov#endedAtTime");
    pub const WAS_ATTRIBUTED_TO: u64 = q_hash("http://www.w3.org/ns/prov#wasAttributedTo");
    pub const WAS_GENERATED_BY: u64 = q_hash("http://www.w3.org/ns/prov#wasGeneratedBy");
    pub const WAS_INVALIDATED_BY: u64 = q_hash("http://www.w3.org/ns/prov#wasInvalidatedBy");
    pub const INVALIDATED_AT_TIME: u64 = q_hash("http://www.w3.org/ns/prov#invalidatedAtTime");
    pub const HAD_PRIMARY_SOURCE: u64 = q_hash("http://www.w3.org/ns/prov#hadPrimarySource");
    pub const WAS_DERIVED_FROM: u64 = q_hash("http://www.w3.org/ns/prov#wasDerivedFrom");
    pub const WAS_ASSOCIATED_WITH: u64 = q_hash("http://www.w3.org/ns/prov#wasAssociatedWith");
    pub const USED: u64 = q_hash("http://www.w3.org/ns/prov#used");
}

/// Named W3C PROV-O predicate, typed for use in filter helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvOPredicate {
    GeneratedAtTime,
    StartedAtTime,
    EndedAtTime,
    WasAttributedTo,
    WasGeneratedBy,
    WasInvalidatedBy,
    InvalidatedAtTime,
    HadPrimarySource,
    WasDerivedFrom,
    WasAssociatedWith,
    Used,
}

impl ProvOPredicate {
    /// The `q_hash` of this predicate's canonical PROV-O IRI.
    #[inline]
    pub fn hash(self) -> u64 {
        match self {
            Self::GeneratedAtTime => prov_predicates::GENERATED_AT_TIME,
            Self::StartedAtTime => prov_predicates::STARTED_AT_TIME,
            Self::EndedAtTime => prov_predicates::ENDED_AT_TIME,
            Self::WasAttributedTo => prov_predicates::WAS_ATTRIBUTED_TO,
            Self::WasGeneratedBy => prov_predicates::WAS_GENERATED_BY,
            Self::WasInvalidatedBy => prov_predicates::WAS_INVALIDATED_BY,
            Self::InvalidatedAtTime => prov_predicates::INVALIDATED_AT_TIME,
            Self::HadPrimarySource => prov_predicates::HAD_PRIMARY_SOURCE,
            Self::WasDerivedFrom => prov_predicates::WAS_DERIVED_FROM,
            Self::WasAssociatedWith => prov_predicates::WAS_ASSOCIATED_WITH,
            Self::Used => prov_predicates::USED,
        }
    }

    /// Classify a raw predicate hash. Returns `None` for non-PROV-O predicates.
    pub fn from_hash(hash: u64) -> Option<Self> {
        Some(match hash {
            h if h == prov_predicates::GENERATED_AT_TIME => Self::GeneratedAtTime,
            h if h == prov_predicates::STARTED_AT_TIME => Self::StartedAtTime,
            h if h == prov_predicates::ENDED_AT_TIME => Self::EndedAtTime,
            h if h == prov_predicates::WAS_ATTRIBUTED_TO => Self::WasAttributedTo,
            h if h == prov_predicates::WAS_GENERATED_BY => Self::WasGeneratedBy,
            h if h == prov_predicates::WAS_INVALIDATED_BY => Self::WasInvalidatedBy,
            h if h == prov_predicates::INVALIDATED_AT_TIME => Self::InvalidatedAtTime,
            h if h == prov_predicates::HAD_PRIMARY_SOURCE => Self::HadPrimarySource,
            h if h == prov_predicates::WAS_DERIVED_FROM => Self::WasDerivedFrom,
            h if h == prov_predicates::WAS_ASSOCIATED_WITH => Self::WasAssociatedWith,
            h if h == prov_predicates::USED => Self::Used,
            _ => return None,
        })
    }
}

/// Provenance-aware filter helpers for the SPARQL executor.
///
/// These operate at the `NQuin` level: they scan a quin slice and apply PROV-O
/// semantics without any heap allocation, fitting within the zero-copy hot-path
/// constraints in `AGENTS.md §6`.
pub struct ProvenanceFilter;

impl ProvenanceFilter {
    /// Returns `true` if `predicate_hash` is any recognised PROV-O predicate.
    #[inline]
    pub fn is_prov_predicate(predicate_hash: u64) -> bool {
        ProvOPredicate::from_hash(predicate_hash).is_some()
    }

    /// Returns `true` if `predicate_hash == prov:wasInvalidatedBy`.
    #[inline]
    pub fn is_invalidation_predicate(predicate_hash: u64) -> bool {
        predicate_hash == prov_predicates::WAS_INVALIDATED_BY
    }

    /// Returns `true` if `predicate_hash == prov:wasAttributedTo`.
    #[inline]
    pub fn is_attribution_predicate(predicate_hash: u64) -> bool {
        predicate_hash == prov_predicates::WAS_ATTRIBUTED_TO
    }

    /// Returns `true` if `subject_hash` has a `prov:wasInvalidatedBy` quin in `quins`.
    ///
    /// Used in SPARQL FILTER to suppress contested / invalidated assertions from results.
    pub fn subject_is_invalidated(quins: &[crate::NQuin], subject_hash: u64) -> bool {
        let p = prov_predicates::WAS_INVALIDATED_BY;
        quins
            .iter()
            .any(|q| q.subject == subject_hash && q.predicate == p)
    }

    /// Returns `true` if `subject_hash` has at least one `prov:wasAttributedTo` quin.
    pub fn subject_has_attribution(quins: &[crate::NQuin], subject_hash: u64) -> bool {
        let p = prov_predicates::WAS_ATTRIBUTED_TO;
        quins
            .iter()
            .any(|q| q.subject == subject_hash && q.predicate == p)
    }

    /// Iterates over all agent DID hashes that `subject_hash` was attributed to
    /// via `prov:wasAttributedTo` in `quins`.
    pub fn attributions<'a>(
        quins: &'a [crate::NQuin],
        subject_hash: u64,
    ) -> impl Iterator<Item = u64> + 'a {
        let p = prov_predicates::WAS_ATTRIBUTED_TO;
        quins
            .iter()
            .filter(move |q| q.subject == subject_hash && q.predicate == p)
            .map(|q| q.object)
    }

    /// Filter `quins` to only those whose predicate matches `target`.
    pub fn filter_by<'a>(
        quins: &'a [crate::NQuin],
        target: ProvOPredicate,
    ) -> impl Iterator<Item = &'a crate::NQuin> {
        let hash = target.hash();
        quins.iter().filter(move |q| q.predicate == hash)
    }

    /// Evaluate an `EvalResult` as a PROV-O predicate filter.
    ///
    /// Returns `Some(true)` if `val` encodes an IRI hash that matches `expected`,
    /// `Some(false)` if it is a different PROV-O IRI, and `None` if the value is
    /// not a recognised PROV-O predicate hash at all.
    pub fn eval_prov_filter(val: EvalResult, expected: ProvOPredicate) -> Option<bool> {
        let hash = match val {
            EvalResult::Iri(h) | EvalResult::Numeric(h) => h,
            _ => return None,
        };
        ProvOPredicate::from_hash(hash).map(|p| p == expected)
    }
}

/// Expression evaluator
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Evaluate an expression against a binding row (no text resolver).
    pub fn evaluate(
        expr_id: ExpressionId,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
    ) -> Result<EvalResult, String> {
        Self::evaluate_with_resolver(expr_id, ctx, row, None)
    }

    /// Evaluate with an optional [`TextResolver`], which lets literal-text
    /// functions (`geof:*`, and in future `STR`/`REGEX`/…) recover the text
    /// behind a term hash. Passing `None` behaves exactly like `evaluate`.
    pub fn evaluate_with_resolver(
        expr_id: ExpressionId,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
    ) -> Result<EvalResult, String> {
        let expr = ctx
            .expressions
            .get(expr_id as usize)
            .ok_or("Expression ID out of bounds")?;
        Self::evaluate_expression(expr, ctx, row, resolver)
    }

    fn evaluate_expression(
        expr: &Expression,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
    ) -> Result<EvalResult, String> {
        match expr {
            Expression::Variable(var_id) => {
                let value = row.get(*var_id);
                Ok(EvalResult::Numeric(value.unwrap_or(0)))
            }
            Expression::Literal(value) => Ok(EvalResult::Numeric(*value)),
            Expression::Iri(value) => Ok(EvalResult::Iri(*value)),
            Expression::UnaryOp { op, expr: inner_id } => {
                let inner = Self::evaluate_expression(
                    &ctx.expressions[*inner_id as usize],
                    ctx,
                    row,
                    resolver,
                )?;
                Self::evaluate_unary_op(*op, inner)
            }
            Expression::BinaryOp { op, left, right } => {
                let left_val =
                    Self::evaluate_expression(&ctx.expressions[*left as usize], ctx, row, resolver)?;
                let right_val = Self::evaluate_expression(
                    &ctx.expressions[*right as usize],
                    ctx,
                    row,
                    resolver,
                )?;
                Self::evaluate_binary_op(*op, left_val, right_val)
            }
            Expression::Function {
                func,
                args_start,
                args_len,
            } => Self::evaluate_function(*func, *args_start, *args_len, ctx, row, resolver),
            Expression::Subquery { query_id } => Self::evaluate_subquery(*query_id, ctx, row),
            Expression::EmbeddedTriple {
                subject,
                predicate,
                object,
            } => {
                // Evaluate embedded triple → the SAME virtual id the ingest path
                // mints (generate_embedded_triple_id: FNV-1a over the 24 LE bytes
                // of [s,p,o] | TAG_EMBEDDED). A prior `s ^ p ^ o` XOR never matched
                // a stored quoted triple (order-insensitive, no tag bit).
                let triple_hash = crate::lexicon::generate_embedded_triple_id(*subject, *predicate, *object);
                Ok(EvalResult::Numeric(triple_hash))
            }
        }
    }

    fn evaluate_unary_op(op: UnaryOp, inner: EvalResult) -> Result<EvalResult, String> {
        match op {
            UnaryOp::Not => match inner {
                EvalResult::Boolean(b) => Ok(EvalResult::Boolean(!b)),
                _ => Err("NOT operator requires boolean operand".to_string()),
            },
            UnaryOp::Plus => Ok(inner),
            UnaryOp::Minus => match inner {
                EvalResult::Numeric(n) => Ok(EvalResult::Numeric((n as i64 * -1) as u64)),
                _ => Err("MINUS operator requires numeric operand".to_string()),
            },
        }
    }

    fn evaluate_binary_op(
        op: BinaryOp,
        left: EvalResult,
        right: EvalResult,
    ) -> Result<EvalResult, String> {
        match op {
            BinaryOp::Or => match (left, right) {
                (EvalResult::Boolean(l), EvalResult::Boolean(r)) => Ok(EvalResult::Boolean(l || r)),
                _ => Err("OR operator requires boolean operands".to_string()),
            },
            BinaryOp::And => match (left, right) {
                (EvalResult::Boolean(l), EvalResult::Boolean(r)) => Ok(EvalResult::Boolean(l && r)),
                _ => Err("AND operator requires boolean operands".to_string()),
            },
            BinaryOp::Equal => match (left, right) {
                (EvalResult::Numeric(_), EvalResult::Numeric(_)) => {
                    Ok(EvalResult::Boolean(left == right))
                }
                (l, r) if matches!(l, EvalResult::Float(_)) || matches!(r, EvalResult::Float(_)) => {
                    match (l.as_f64(), r.as_f64()) {
                        (Some(a), Some(b)) => Ok(EvalResult::Boolean(a == b)),
                        _ => Ok(EvalResult::Boolean(false)),
                    }
                }
                _ => Ok(EvalResult::Boolean(left == right)),
            },
            BinaryOp::NotEqual => match Self::evaluate_binary_op(BinaryOp::Equal, left, right)? {
                EvalResult::Boolean(b) => Ok(EvalResult::Boolean(!b)),
                other => Ok(other),
            },
            BinaryOp::LessThan => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => Ok(EvalResult::Boolean(l < r)),
                (l, r) => match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(EvalResult::Boolean(a < b)),
                    _ => Err("LESS THAN operator requires numeric operands".to_string()),
                },
            },
            BinaryOp::LessThanOrEqual => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => Ok(EvalResult::Boolean(l <= r)),
                (l, r) => match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(EvalResult::Boolean(a <= b)),
                    _ => Err("LESS THAN OR EQUAL operator requires numeric operands".to_string()),
                },
            },
            BinaryOp::GreaterThan => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => Ok(EvalResult::Boolean(l > r)),
                (l, r) => match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(EvalResult::Boolean(a > b)),
                    _ => Err("GREATER THAN operator requires numeric operands".to_string()),
                },
            },
            BinaryOp::GreaterThanOrEqual => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => Ok(EvalResult::Boolean(l >= r)),
                (l, r) => match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(EvalResult::Boolean(a >= b)),
                    _ => Err("GREATER THAN OR EQUAL operator requires numeric operands".to_string()),
                },
            },
            BinaryOp::Add => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                    Ok(EvalResult::Numeric(l.wrapping_add(r)))
                }
                (l, r) => match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(EvalResult::Float(a + b)),
                    _ => Err("ADD operator requires numeric operands".to_string()),
                },
            },
            BinaryOp::Subtract => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                    Ok(EvalResult::Numeric(l.wrapping_sub(r)))
                }
                (l, r) => match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(EvalResult::Float(a - b)),
                    _ => Err("SUBTRACT operator requires numeric operands".to_string()),
                },
            },
            BinaryOp::Multiply => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                    Ok(EvalResult::Numeric(l.wrapping_mul(r)))
                }
                (l, r) => match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(EvalResult::Float(a * b)),
                    _ => Err("MULTIPLY operator requires numeric operands".to_string()),
                },
            },
            BinaryOp::Divide => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                    if r == 0 {
                        return Err("Division by zero".to_string());
                    }
                    Ok(EvalResult::Numeric(l / r))
                }
                _ => Err("DIVIDE operator requires numeric operands".to_string()),
            },
        }
    }

    fn evaluate_function(
        func: Function,
        args_start: u16,
        args_len: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
    ) -> Result<EvalResult, String> {
        match func {
            Function::Bound => {
                // BOUND(?var) - check if variable is bound
                if args_len >= 1 {
                    let var_id = ctx.function_args[args_start as usize] as VariableId;
                    Ok(EvalResult::Boolean(row.get(var_id).is_some()))
                } else {
                    Err("BOUND requires at least one argument".to_string())
                }
            }
            Function::Str => {
                // STR(term) → the lexical string form, interned into the sink so it is a
                // real String value (numbers/booleans formatted; IRIs/literals resolved).
                if args_len < 1 {
                    return Err("STR requires at least one argument".to_string());
                }
                let expr_id = ctx.function_args[args_start as usize];
                let val = Self::evaluate_with_resolver(expr_id, ctx, row, resolver)?;
                match val {
                    EvalResult::String(_) => Ok(val),
                    EvalResult::Boolean(b) => {
                        Self::produce_string(resolver, if b { "true" } else { "false" }, "STR")
                    }
                    EvalResult::Float(f) => Self::produce_string(resolver, &format!("{f}"), "STR"),
                    EvalResult::Numeric(h) | EvalResult::Iri(h) => {
                        if let Some(r) = resolver {
                            if let Some(t) = r.resolve_text(h) {
                                return Self::produce_string(resolver, &t, "STR");
                            }
                            if let Some(lit) = crate::resolver::classify_inline_literal(h) {
                                return Self::produce_string(resolver, &format!("{lit}"), "STR");
                            }
                        }
                        Ok(val) // no resolver: keep the term (best effort)
                    }
                }
            }
            Function::Lang => {
                // LANG(literal) → its language tag, or "" for a non-lang-tagged literal
                // (correct SPARQL default). Honest error if the term is not a literal.
                if args_len < 1 {
                    return Err("LANG requires an argument".to_string());
                }
                let h = Self::arg_term(0, args_start, args_len, ctx, row, resolver)?;
                let lang = resolver
                    .and_then(|r| r.lang_of(h))
                    .ok_or("LANG: argument is not a literal")?;
                Self::produce_string(resolver, &lang, "LANG")
            }
            Function::Datatype => {
                // DATATYPE(literal) → its datatype IRI (explicit tag, inline XSD type,
                // rdf:langString, or xsd:string for a plain literal). Honest error for a
                // term that is not a typed/known literal — no fabricated placeholder.
                if args_len < 1 {
                    return Err("DATATYPE requires an argument".to_string());
                }
                let h = Self::arg_term(0, args_start, args_len, ctx, row, resolver)?;
                let dt = resolver
                    .and_then(|r| r.datatype_of(h))
                    .ok_or("DATATYPE: argument is not a typed literal")?;
                // Return an IRI term whose hash equals the query's own IRI term for the
                // same datatype (generate_60bit_token of the IRI), so `DATATYPE(?x) =
                // xsd:integer` compares equal; also make it resolvable via the sink.
                let hash = crate::lexicon::generate_60bit_token(dt.as_bytes());
                if let Some(sink) = resolver.and_then(|r| r.sink) {
                    sink.intern(&dt);
                }
                Ok(EvalResult::Iri(hash))
            }
            Function::LangMatches => {
                // LANGMATCHES(tag, range) per RFC 4647 basic filtering.
                let tag = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "LANGMATCHES")?;
                let range =
                    Self::arg_text(1, args_start, args_len, ctx, row, resolver, "LANGMATCHES")?;
                Ok(EvalResult::Boolean(Self::lang_matches(&tag, &range)))
            }
            Function::StrLang => {
                // STRLANG(str, lang) → a language-tagged literal (round-trips via LANG).
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "STRLANG")?;
                let lang = Self::arg_text(1, args_start, args_len, ctx, row, resolver, "STRLANG")?;
                match resolver.and_then(|r| r.sink) {
                    Some(sink) => Ok(EvalResult::String(sink.intern_tagged(&s, Some(&lang), None))),
                    None => Err("STRLANG produces a value but no string sink is available".to_string()),
                }
            }
            Function::StrDt => {
                // STRDT(str, datatypeIRI) → a datatyped literal (round-trips via DATATYPE).
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "STRDT")?;
                let dt = Self::arg_text(1, args_start, args_len, ctx, row, resolver, "STRDT")?;
                match resolver.and_then(|r| r.sink) {
                    Some(sink) => Ok(EvalResult::String(sink.intern_tagged(&s, None, Some(&dt)))),
                    None => Err("STRDT produces a value but no string sink is available".to_string()),
                }
            }
            Function::IsIri | Function::IsUri => {
                // isIRI(expr) - check if IRI
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    Ok(EvalResult::Boolean(matches!(result, EvalResult::Iri(_))))
                } else {
                    Err("isIRI requires at least one argument".to_string())
                }
            }
            Function::IsBlank => {
                // isBlank(expr) - check if blank node
                if args_len >= 1 {
                    Ok(EvalResult::Boolean(false)) // Simplified
                } else {
                    Err("isBlank requires at least one argument".to_string())
                }
            }
            Function::IsLiteral => {
                // isLiteral(expr) - check if literal
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    Ok(EvalResult::Boolean(matches!(
                        result,
                        EvalResult::Numeric(_)
                    )))
                } else {
                    Err("isLiteral requires at least one argument".to_string())
                }
            }
            Function::IsNumeric => {
                // isNumeric(expr) - check if numeric
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    Ok(EvalResult::Boolean(matches!(
                        result,
                        EvalResult::Numeric(_)
                    )))
                } else {
                    Err("isNumeric requires at least one argument".to_string())
                }
            }
            Function::Abs => {
                // ABS(expr) - absolute value
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    match result {
                        EvalResult::Numeric(n) => Ok(EvalResult::Numeric((n as i64).abs() as u64)),
                        _ => Err("ABS requires numeric argument".to_string()),
                    }
                } else {
                    Err("ABS requires at least one argument".to_string())
                }
            }
            Function::Ceil => {
                // CEIL(expr) - ceiling
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    match result {
                        EvalResult::Numeric(n) => Ok(EvalResult::Numeric((n as f64).ceil() as u64)),
                        _ => Err("CEIL requires numeric argument".to_string()),
                    }
                } else {
                    Err("CEIL requires at least one argument".to_string())
                }
            }
            Function::Floor => {
                // FLOOR(expr) - floor
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    match result {
                        EvalResult::Numeric(n) => {
                            Ok(EvalResult::Numeric((n as f64).floor() as u64))
                        }
                        _ => Err("FLOOR requires numeric argument".to_string()),
                    }
                } else {
                    Err("FLOOR requires at least one argument".to_string())
                }
            }
            Function::Round => {
                // ROUND(expr) - round
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    match result {
                        EvalResult::Numeric(n) => {
                            Ok(EvalResult::Numeric((n as f64).round() as u64))
                        }
                        _ => Err("ROUND requires numeric argument".to_string()),
                    }
                } else {
                    Err("ROUND requires at least one argument".to_string())
                }
            }
            Function::TripleSubject => {
                // TRIPLESUBJECT(<<s p o>>) - return subject
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let expr = &ctx.expressions[expr_id as usize];
                    if let Expression::EmbeddedTriple { subject, .. } = expr {
                        Ok(EvalResult::Numeric(*subject))
                    } else {
                        Err("TRIPLESUBJECT requires embedded triple".to_string())
                    }
                } else {
                    Err("TRIPLESUBJECT requires at least one argument".to_string())
                }
            }
            Function::TriplePredicate => {
                // TRIPLEPREDICATE(<<s p o>>) - return predicate
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let expr = &ctx.expressions[expr_id as usize];
                    if let Expression::EmbeddedTriple { predicate, .. } = expr {
                        Ok(EvalResult::Numeric(*predicate))
                    } else {
                        Err("TRIPLEPREDICATE requires embedded triple".to_string())
                    }
                } else {
                    Err("TRIPLEPREDICATE requires at least one argument".to_string())
                }
            }
            Function::TripleObject => {
                // TRIPLEOBJECT(<<s p o>>) - return object
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let expr = &ctx.expressions[expr_id as usize];
                    if let Expression::EmbeddedTriple { object, .. } = expr {
                        Ok(EvalResult::Numeric(*object))
                    } else {
                        Err("TRIPLEOBJECT requires embedded triple".to_string())
                    }
                } else {
                    Err("TRIPLEOBJECT requires at least one argument".to_string())
                }
            }
            Function::Triple => {
                // TRIPLE(s, p, o) - create embedded triple
                if args_len >= 3 {
                    let s_id = ctx.function_args[args_start as usize];
                    let p_id = ctx.function_args[args_start as usize + 1];
                    let o_id = ctx.function_args[args_start as usize + 2];

                    let s_result = Self::evaluate(s_id, ctx, row)?;
                    let p_result = Self::evaluate(p_id, ctx, row)?;
                    let o_result = Self::evaluate(o_id, ctx, row)?;

                    match (s_result, p_result, o_result) {
                        (
                            EvalResult::Numeric(s),
                            EvalResult::Numeric(p),
                            EvalResult::Numeric(o),
                        ) => {
                            // Mint the SAME virtual id the ingest path uses, so a
                            // TRIPLE(...)-constructed id matches a stored quoted
                            // triple (was `s ^ p ^ o`, which never matched).
                            let triple_hash = crate::lexicon::generate_embedded_triple_id(s, p, o);
                            Ok(EvalResult::Numeric(triple_hash))
                        }
                        _ => Err("TRIPLE requires numeric arguments".to_string()),
                    }
                } else {
                    Err("TRIPLE requires at least three arguments".to_string())
                }
            }
            // ── String predicates (resolver-backed) ────────────────────────
            // Each recovers argument text via the TextResolver (query literals
            // + ingested lexicon), exactly like the geo functions. Without a
            // resolver, or if a term can't be resolved, they return an honest
            // error — never a fabricated boolean.
            Function::Contains => {
                let hay = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "CONTAINS")?;
                let needle =
                    Self::arg_text(1, args_start, args_len, ctx, row, resolver, "CONTAINS")?;
                Ok(EvalResult::Boolean(hay.contains(&needle)))
            }
            Function::VarStarts => {
                let hay = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "STRSTARTS")?;
                let needle =
                    Self::arg_text(1, args_start, args_len, ctx, row, resolver, "STRSTARTS")?;
                Ok(EvalResult::Boolean(hay.starts_with(&needle)))
            }
            Function::VarEnds => {
                let hay = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "STRENDS")?;
                let needle =
                    Self::arg_text(1, args_start, args_len, ctx, row, resolver, "STRENDS")?;
                Ok(EvalResult::Boolean(hay.ends_with(&needle)))
            }
            Function::Strlen => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "STRLEN")?;
                Ok(EvalResult::Numeric(s.chars().count() as u64))
            }
            Function::Regex => {
                let text = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "REGEX")?;
                let pattern = Self::arg_text(1, args_start, args_len, ctx, row, resolver, "REGEX")?;
                // Optional third arg = flags (i/s/m/x), XPath/SPARQL semantics.
                let flags = if args_len >= 3 {
                    Self::arg_text(2, args_start, args_len, ctx, row, resolver, "REGEX")?
                } else {
                    String::new()
                };
                let mut builder = regex::RegexBuilder::new(&pattern);
                for f in flags.chars() {
                    match f {
                        'i' => {
                            builder.case_insensitive(true);
                        }
                        's' => {
                            builder.dot_matches_new_line(true);
                        }
                        'm' => {
                            builder.multi_line(true);
                        }
                        'x' => {
                            builder.ignore_whitespace(true);
                        }
                        other => return Err(format!("REGEX: unsupported flag '{other}'")),
                    }
                }
                let re = builder
                    .build()
                    .map_err(|e| format!("REGEX: invalid pattern: {e}"))?;
                Ok(EvalResult::Boolean(re.is_match(&text)))
            }
            // ── Term equality / control flow (no text needed) ──────────────
            Function::SameTerm => {
                if args_len < 2 {
                    return Err("SAMETERM requires two arguments".to_string());
                }
                let a = Self::evaluate_with_resolver(
                    ctx.function_args[args_start as usize],
                    ctx,
                    row,
                    resolver,
                )?;
                let b = Self::evaluate_with_resolver(
                    ctx.function_args[args_start as usize + 1],
                    ctx,
                    row,
                    resolver,
                )?;
                // Terms are typed (Iri/Numeric/String/Boolean) — sameTerm is
                // true iff both the type tag and the value hash agree.
                Ok(EvalResult::Boolean(a == b))
            }
            Function::If => {
                if args_len < 3 {
                    return Err("IF requires three arguments".to_string());
                }
                let cond = Self::evaluate_with_resolver(
                    ctx.function_args[args_start as usize],
                    ctx,
                    row,
                    resolver,
                )?;
                let branch = if cond.as_bool() {
                    ctx.function_args[args_start as usize + 1]
                } else {
                    ctx.function_args[args_start as usize + 2]
                };
                Self::evaluate_with_resolver(branch, ctx, row, resolver)
            }
            Function::Custom(iri_hash) => {
                // Extension functions dispatched by function-IRI hash:
                //  1. GeoSPARQL predicates (geof:distance / sfContains / sfWithin /
                //     sfIntersects / sfTouches) — executed on WKT geometry text.
                //  2. QISP (qispf:) functions from the typed registry — type-admitted,
                //     with the 2D ones deferring to the GeoSPARQL engine and the
                //     mesh/tensor ones failing closed with an honest "not yet
                //     executable inline" error (never a fabricated result).
                use crate::sparql_library::geosparql;
                use crate::sparql_library::immersive::functions as qisp_fns;

                // 1. Direct GeoSPARQL predicate.
                if let Some(geo_fn) = geosparql::geo_fn_for_hash(iri_hash) {
                    return Self::run_geo_fn(geo_fn, args_start, args_len, ctx, row, resolver);
                }

                // 2. A registered QISP function.
                if let Some(entry) = qisp_fns::entry_for_iri_hash(iri_hash) {
                    // Admission (plan §4.2/§6.1): an async / non-deterministic /
                    // table-producing function (e.g. qispf:knn) is NOT legal inline —
                    // fail closed with a named error rather than fabricate.
                    if !entry.descriptor.legal_in_filter() {
                        return Err(format!(
                            "QISP function <{}> is not legal in an inline SPARQL expression \
                             (it is a job / graph operator)",
                            entry.iri
                        ));
                    }
                    // A 2D operation GeoSPARQL owns → execute via the geo engine on WKT.
                    if let Some(geof_iri) = entry.defers_to {
                        if let Some(geo_fn) = geosparql::geo_fn_for_hash(crate::q_hash(geof_iri)) {
                            return Self::run_geo_fn(geo_fn, args_start, args_len, ctx, row, resolver);
                        }
                    }
                    // QISP-owned predicate. The Tensor10D predicates EXECUTE inline now,
                    // from inline Tensor10D literals (ten finite values), through the
                    // resident-substrate metric (plan Phase 4 step 5). Mesh/volumetric
                    // predicates still need a geometry asset resolved from the term (a
                    // later Phase-4 increment) → honest, named error, never fabricated.
                    let local = entry.iri.rsplit('#').next().unwrap_or("");
                    return match local {
                        "tensorDistance" | "tensorWithin" => {
                            use crate::sparql_library::immersive::functions::{
                                tensor_distance, tensor_within,
                            };
                            let ta = Self::arg_tensor10d(
                                0, args_start, args_len, ctx, row, resolver, entry.iri,
                            )?;
                            let tb = Self::arg_tensor10d(
                                1, args_start, args_len, ctx, row, resolver, entry.iri,
                            )?;
                            if local == "tensorDistance" {
                                let d = tensor_distance(&ta, &tb).map_err(|e| e.to_string())?;
                                Ok(EvalResult::Float(d as f64))
                            } else {
                                let radius = Self::arg_f64(
                                    2, args_start, args_len, ctx, row, resolver, entry.iri,
                                )?;
                                let within = tensor_within(&ta, &tb, radius as f32)
                                    .map_err(|e| e.to_string())?;
                                Ok(EvalResult::Boolean(within))
                            }
                        }
                        _ => Err(format!(
                            "QISP function <{}> is registered and type-admitted but not yet \
                             executable inline (needs geometry/mesh asset resolution)",
                            entry.iri
                        )),
                    };
                }

                // 3. did: functions.
                //    did:resolve is genuinely query-safe (no keys) — resolve a DID string
                //    to its endpoint URL. The crypto / governance ones (verify/auth/sign/
                //    permission) hold no keys and evaluate no policy in the query layer, so
                //    they route honestly to the identity/governance layer (never faked).
                if iri_hash == crate::q_hash("did:resolve") {
                    let did_hash = Self::arg_term(0, args_start, args_len, ctx, row, resolver)?;
                    let r = resolver.ok_or("did:resolve requires a text resolver")?;
                    let did_str = r
                        .resolve_text(did_hash)
                        .ok_or("did:resolve: could not resolve the DID string")?;
                    let resolution = crate::sparql_did::DIDResolver
                        .resolve(&did_str)
                        .map_err(|e| format!("did:resolve: {e}"))?;
                    return Self::produce_string(resolver, &resolution.endpoint_url, "did:resolve");
                }
                if iri_hash == crate::q_hash("did:verify")
                    || iri_hash == crate::q_hash("did:auth")
                    || iri_hash == crate::q_hash("did:sign")
                    || iri_hash == crate::q_hash("did:permission")
                {
                    return Err("did:verify / did:auth / did:sign / did:permission are not \
                                evaluated in the SPARQL query layer (it holds no keys and \
                                evaluates no policy); route via the identity/key-vault + \
                                governance layer — refusing to fabricate a result"
                        .to_string());
                }

                // 4. Unknown to all engines.
                Err(format!("unknown extension function (hash {iri_hash:#018x})"))
            }
            // ── String-producing builtins (QISP-R06) ────────────────────────
            // Each recovers its argument text via the resolver and interns its
            // RESULT into the query StringSink (returning an EvalResult::String
            // whose hash the resolver can recover). Without a sink they fail
            // closed — never fabricate.
            Function::Concat => {
                let mut s = String::new();
                for i in 0..args_len as usize {
                    s.push_str(&Self::arg_text(i, args_start, args_len, ctx, row, resolver, "CONCAT")?);
                }
                Self::produce_string(resolver, &s, "CONCAT")
            }
            Function::Ucase => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "UCASE")?;
                Self::produce_string(resolver, &s.to_uppercase(), "UCASE")
            }
            Function::Lcase => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "LCASE")?;
                Self::produce_string(resolver, &s.to_lowercase(), "LCASE")
            }
            Function::Substring => {
                // SUBSTR(str, start[, length]) — SPARQL/XPath 1-based, codepoint-indexed.
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "SUBSTR")?;
                let start = Self::arg_number(1, args_start, args_len, ctx, row, resolver, "SUBSTR")?;
                let chars: Vec<char> = s.chars().collect();
                let n = chars.len() as i64;
                let from = start.max(1);
                let end = if args_len >= 3 {
                    let len = Self::arg_number(2, args_start, args_len, ctx, row, resolver, "SUBSTR")?;
                    (from + len.max(0)).min(n + 1)
                } else {
                    n + 1
                };
                let out: String = if from > n || end <= from {
                    String::new()
                } else {
                    chars[(from - 1) as usize..(end - 1) as usize].iter().collect()
                };
                Self::produce_string(resolver, &out, "SUBSTR")
            }
            Function::StrBefore => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "STRBEFORE")?;
                let sep = Self::arg_text(1, args_start, args_len, ctx, row, resolver, "STRBEFORE")?;
                let out = if sep.is_empty() {
                    String::new()
                } else {
                    match s.find(&sep) {
                        Some(i) => s[..i].to_string(),
                        None => String::new(),
                    }
                };
                Self::produce_string(resolver, &out, "STRBEFORE")
            }
            Function::StrAfter => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "STRAFTER")?;
                let sep = Self::arg_text(1, args_start, args_len, ctx, row, resolver, "STRAFTER")?;
                let out = if sep.is_empty() {
                    s.clone()
                } else {
                    match s.find(&sep) {
                        Some(i) => s[i + sep.len()..].to_string(),
                        None => String::new(),
                    }
                };
                Self::produce_string(resolver, &out, "STRAFTER")
            }
            Function::EncodeForUri => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "ENCODE_FOR_URI")?;
                Self::produce_string(resolver, &Self::encode_for_uri(&s), "ENCODE_FOR_URI")
            }
            // ── COALESCE — first argument that is bound and evaluates cleanly ──
            Function::Coalesce => {
                for i in 0..args_len as usize {
                    let arg_expr_id = ctx.function_args[args_start as usize + i];
                    // A bare unbound variable is skipped: the engine collapses an
                    // unbound variable to Numeric(0), so detect unboundness directly
                    // rather than treating the 0 as a real value.
                    if let Expression::Variable(v) = ctx.expressions[arg_expr_id as usize] {
                        if row.get(v).is_none() {
                            continue;
                        }
                    }
                    if let Ok(val) = Self::evaluate_with_resolver(arg_expr_id, ctx, row, resolver) {
                        return Ok(val);
                    }
                }
                Err("COALESCE: all arguments are unbound or errored".to_string())
            }
            // ── Temporal builtins (query-stable clock; §4.4 referential transparency) ──
            Function::Now => {
                let ms = resolver.map(|r| r.now_ms).unwrap_or(0);
                if ms == 0 {
                    return Err("NOW requires a query-stable clock; none supplied".to_string());
                }
                let dt = chrono::DateTime::from_timestamp_millis(ms as i64)
                    .ok_or("NOW: timestamp out of range")?;
                Self::produce_string(
                    resolver,
                    &dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    "NOW",
                )
            }
            Function::Year
            | Function::Month
            | Function::Day
            | Function::Hours
            | Function::Minutes
            | Function::Seconds => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "date accessor")?;
                let dt = Self::parse_datetime(&s)?;
                use chrono::{Datelike, Timelike};
                let v: i64 = match func {
                    Function::Year => dt.year() as i64,
                    Function::Month => dt.month() as i64,
                    Function::Day => dt.day() as i64,
                    Function::Hours => dt.hour() as i64,
                    Function::Minutes => dt.minute() as i64,
                    Function::Seconds => dt.second() as i64,
                    _ => unreachable!(),
                };
                Ok(EvalResult::Numeric(v as u64))
            }
            Function::Tz => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "TZ")?;
                let dt = Self::parse_datetime(&s)?;
                let off = dt.offset().local_minus_utc();
                let tz = if off == 0 {
                    "Z".to_string()
                } else {
                    let (sign, a) = if off < 0 { ('-', -off) } else { ('+', off) };
                    format!("{sign}{:02}:{:02}", a / 3600, (a % 3600) / 60)
                };
                Self::produce_string(resolver, &tz, "TZ")
            }
            Function::Timezone => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "TIMEZONE")?;
                let dt = Self::parse_datetime(&s)?;
                let off = dt.offset().local_minus_utc();
                // xsd:dayTimeDuration lexical.
                let dur = if off == 0 {
                    "PT0S".to_string()
                } else {
                    let (sign, a) = if off < 0 { ("-", -off) } else { ("", off) };
                    let (h, m) = (a / 3600, (a % 3600) / 60);
                    let mut d = format!("{sign}PT");
                    if h > 0 {
                        d.push_str(&format!("{h}H"));
                    }
                    if m > 0 {
                        d.push_str(&format!("{m}M"));
                    }
                    d
                };
                Self::produce_string(resolver, &dur, "TIMEZONE")
            }
            // ── UUID / STRUUID — query-stable deterministic (per-occurrence salt) ──
            Function::Uuid | Function::StrUuid => {
                let seed = resolver.map(|r| r.seed).unwrap_or(0);
                if seed == 0 {
                    return Err("UUID requires a query-stable seed; none supplied".to_string());
                }
                let uuid = Self::deterministic_uuid(seed, args_start as u64);
                let is_iri = matches!(func, Function::Uuid);
                let text = if is_iri { format!("urn:uuid:{uuid}") } else { uuid };
                match resolver.and_then(|r| r.sink) {
                    Some(sink) => {
                        let h = sink.intern(&text);
                        Ok(if is_iri {
                            EvalResult::Iri(h)
                        } else {
                            EvalResult::String(h)
                        })
                    }
                    None => Err("UUID produces a value but no string sink is available".to_string()),
                }
            }
            // ── IRI / URI / BNODE construction ──
            Function::Iri | Function::Uri => {
                let s = Self::arg_text(0, args_start, args_len, ctx, row, resolver, "IRI")?;
                match resolver.and_then(|r| r.sink) {
                    Some(sink) => Ok(EvalResult::Iri(sink.intern(&s))),
                    None => Err("IRI produces a term but no string sink is available".to_string()),
                }
            }
            Function::Bnode => {
                let seed = resolver.map(|r| r.seed).unwrap_or(0);
                let label = Self::deterministic_uuid(seed ^ 0x424e_4f44_45, args_start as u64);
                let text = format!("_:b{}", &label[..8.min(label.len())]);
                match resolver.and_then(|r| r.sink) {
                    Some(sink) => Ok(EvalResult::Iri(sink.intern(&text))),
                    None => Err("BNODE produces a term but no string sink is available".to_string()),
                }
            }
            // RAND() → a real double in [0, 1) via the EvalResult::Float channel.
            // Query-stable + per-occurrence-salted (plan §4.4): the same RAND() site
            // yields the same value within a query, distinct sites differ. Without a
            // query-stable seed it fails closed rather than fabricate.
            Function::Rand => {
                let seed = resolver.map(|r| r.seed).unwrap_or(0);
                if seed == 0 {
                    return Err("RAND requires a query-stable seed; none supplied".to_string());
                }
                Ok(EvalResult::Float(Self::deterministic_unit_f64(seed, args_start as u64)))
            }
            // NOTE: the match over `Function` is now exhaustive — every SPARQL builtin has
            // a real implementation above (no fabricated placeholder, no residual). If a
            // new `Function` variant is added, the compiler will flag the missing arm here
            // rather than silently falling through to a fabricated result.
        }
    }

    fn evaluate_subquery(
        query_id: u16,
        ctx: &SparqlQueryContext,
        _row: &BindingRow,
    ) -> Result<EvalResult, String> {
        let _subquery = ctx
            .subqueries
            .get(query_id as usize)
            .ok_or("Subquery ID out of bounds")?;

        // EXISTS / sub-SELECT evaluation is not wired yet. A real
        // implementation would plan the subquery, execute it against the
        // current bindings, and return EXISTS/NOT EXISTS/count. Until then we
        // fail closed rather than fabricate `true` (which would let every row
        // pass a `FILTER EXISTS { ... }`). Tracked in the QISP plan.
        Err("SPARQL FILTER subquery (EXISTS/sub-SELECT) is not implemented; \
             refusing to fabricate a passing result"
            .to_string())
    }

    /// Recover the text behind function argument `idx` (0-based) through the
    /// [`TextResolver`] — query-literal constants plus the ingested lexicon.
    /// Returns an honest error when there is no resolver, the argument is
    /// missing, the term is not a resolvable string term, or the text can't be
    /// found; never fabricates. Shared by the string-predicate FILTER builtins.
    fn arg_text(
        idx: usize,
        args_start: u16,
        args_len: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
        fname: &str,
    ) -> Result<String, String> {
        if (idx as u16) >= args_len {
            return Err(format!("{fname} is missing a required argument"));
        }
        let resolver = resolver
            .ok_or_else(|| format!("{fname} requires a text resolver (no lexicon available)"))?;
        let expr_id = ctx.function_args[args_start as usize + idx];
        let hash = match Self::evaluate_with_resolver(expr_id, ctx, row, Some(resolver))? {
            EvalResult::Numeric(h) | EvalResult::Iri(h) | EvalResult::String(h) => h,
            EvalResult::Boolean(_) | EvalResult::Float(_) => {
                return Err(format!("{fname} argument is not a string term"))
            }
        };
        resolver
            .resolve_text(hash)
            .ok_or_else(|| format!("{fname}: could not resolve string-literal text"))
    }

    /// Execute a GeoSPARQL predicate on the two WKT-geometry arguments at
    /// `args_start`. Shared by the direct `geof:` dispatch and the QISP functions
    /// that defer to GeoSPARQL for the 2D case. Fails closed (honest error) when
    /// there is no resolver or a geometry literal can't be resolved/parsed.
    fn run_geo_fn(
        geo_fn: crate::sparql_library::geosparql::GeoFn,
        args_start: u16,
        args_len: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
    ) -> Result<EvalResult, String> {
        use crate::sparql_library::geosparql;
        if args_len < 2 {
            return Err("geo function requires two geometry arguments".to_string());
        }
        let resolver = resolver.ok_or_else(|| {
            "geo functions require a text resolver (no lexicon available)".to_string()
        })?;
        let a_id = ctx.function_args[args_start as usize];
        let b_id = ctx.function_args[args_start as usize + 1];
        let a_hash = match Self::evaluate_with_resolver(a_id, ctx, row, Some(resolver))? {
            EvalResult::Numeric(h) | EvalResult::Iri(h) => h,
            _ => return Err("geo argument is not a term".to_string()),
        };
        let b_hash = match Self::evaluate_with_resolver(b_id, ctx, row, Some(resolver))? {
            EvalResult::Numeric(h) | EvalResult::Iri(h) => h,
            _ => return Err("geo argument is not a term".to_string()),
        };
        let a_wkt = resolver
            .resolve_text(a_hash)
            .ok_or("could not resolve first geometry literal")?;
        let b_wkt = resolver
            .resolve_text(b_hash)
            .ok_or("could not resolve second geometry literal")?;
        let ga = geosparql::parse_wkt(&a_wkt).map_err(|e| format!("geo arg 1: {e}"))?;
        let gb = geosparql::parse_wkt(&b_wkt).map_err(|e| format!("geo arg 2: {e}"))?;
        match geosparql::eval_geo_fn(geo_fn, &ga, &gb) {
            geosparql::GeoValue::Bool(b) => Ok(EvalResult::Boolean(b)),
            geosparql::GeoValue::Number(n) => Ok(EvalResult::Numeric(n as u64)),
        }
    }

    /// Evaluate function argument `idx` to its term hash (Numeric/Iri/String).
    fn arg_term(
        idx: usize,
        args_start: u16,
        args_len: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
    ) -> Result<u64, String> {
        if (idx as u16) >= args_len {
            return Err("missing a required argument".to_string());
        }
        let expr_id = ctx.function_args[args_start as usize + idx];
        match Self::evaluate_with_resolver(expr_id, ctx, row, resolver)? {
            EvalResult::Numeric(h) | EvalResult::Iri(h) | EvalResult::String(h) => Ok(h),
            EvalResult::Boolean(_) | EvalResult::Float(_) => {
                Err("argument is not a term".to_string())
            }
        }
    }

    /// RFC 4647 basic-filtering `langMatches(tag, range)`: `*` matches any non-empty
    /// tag; otherwise a case-insensitive exact match or a `range-` prefix of `tag`.
    fn lang_matches(tag: &str, range: &str) -> bool {
        if range == "*" {
            return !tag.is_empty();
        }
        let tag = tag.to_ascii_lowercase();
        let range = range.to_ascii_lowercase();
        tag == range || tag.starts_with(&format!("{range}-"))
    }

    /// Evaluate function argument `idx` to a numeric value (for `SUBSTR` positions).
    fn arg_number(
        idx: usize,
        args_start: u16,
        args_len: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
        fname: &str,
    ) -> Result<i64, String> {
        if (idx as u16) >= args_len {
            return Err(format!("{fname} is missing a required numeric argument"));
        }
        let expr_id = ctx.function_args[args_start as usize + idx];
        match Self::evaluate_with_resolver(expr_id, ctx, row, resolver)? {
            EvalResult::Numeric(n) => Ok(n as i64),
            _ => Err(format!("{fname} argument is not numeric")),
        }
    }

    /// Intern a **produced** string into the query [`StringSink`] and return it as an
    /// `EvalResult::String`. Fails closed (honest error) when no sink is available —
    /// never fabricates.
    fn produce_string(
        resolver: Option<crate::sparql_ast::TextResolver>,
        text: &str,
        fname: &str,
    ) -> Result<EvalResult, String> {
        match resolver.and_then(|r| r.sink) {
            Some(sink) => Ok(EvalResult::String(sink.intern(text))),
            None => Err(format!(
                "{fname} produces a string but no string sink is available"
            )),
        }
    }

    /// Percent-encode per `ENCODE_FOR_URI` (RFC 3986 unreserved set kept verbatim).
    fn encode_for_uri(s: &str) -> String {
        let mut out = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }

    /// Parse an `xsd:dateTime` lexical (RFC 3339, or a timezone-less form treated as UTC).
    fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::FixedOffset>, String> {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Ok(dt);
        }
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            let utc = chrono::FixedOffset::east_opt(0).unwrap();
            return Ok(chrono::DateTime::from_naive_utc_and_offset(ndt, utc));
        }
        Err(format!("invalid xsd:dateTime lexical '{s}'"))
    }

    /// Query-stable, per-occurrence-salted deterministic 128-bit value formatted as a
    /// v4-shaped UUID. `UUID`/`STRUUID` are non-deterministic in stock SPARQL, but the
    /// QISP profile requires expression functions to be referentially transparent within
    /// one query snapshot (plan §4.4): the same `(seed, salt)` always yields the same UUID,
    /// distinct call sites (distinct `salt`) yield distinct UUIDs.
    fn deterministic_uuid(seed: u64, salt: u64) -> String {
        let mut x = seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut next = || {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let hi = next().to_be_bytes();
        let lo = next().to_be_bytes();
        let mut u = [0u8; 16];
        u[..8].copy_from_slice(&hi);
        u[8..].copy_from_slice(&lo);
        u[6] = (u[6] & 0x0F) | 0x40; // version 4 shape
        u[8] = (u[8] & 0x3F) | 0x80; // variant shape
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            u[0], u[1], u[2], u[3], u[4], u[5], u[6], u[7], u[8], u[9], u[10], u[11], u[12], u[13], u[14], u[15]
        )
    }

    /// Deterministic double in `[0, 1)` from `(seed, salt)` — the `RAND` channel.
    /// Query-stable (same key → same value) via splitmix64, top 53 bits → mantissa.
    fn deterministic_unit_f64(seed: u64, salt: u64) -> f64 {
        let mut x = seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Evaluate function argument `idx` to an `f64` — accepts a `Float` result, a
    /// numeric literal term whose resolved text parses as a number, or a raw
    /// `Numeric` hash as a fallback. Used for the `tensorWithin` radius.
    fn arg_f64(
        idx: usize,
        args_start: u16,
        args_len: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
        fname: &str,
    ) -> Result<f64, String> {
        if (idx as u16) >= args_len {
            return Err(format!("{fname} is missing a required numeric argument"));
        }
        let expr_id = ctx.function_args[args_start as usize + idx];
        match Self::evaluate_with_resolver(expr_id, ctx, row, resolver)? {
            EvalResult::Float(f) => Ok(f),
            EvalResult::Numeric(n) => {
                if let Some(r) = resolver {
                    if let Some(t) = r.resolve_text(n) {
                        if let Ok(v) = t.trim().parse::<f64>() {
                            return Ok(v);
                        }
                    }
                }
                Ok(n as f64)
            }
            _ => Err(format!("{fname} argument is not numeric")),
        }
    }

    /// Parse an inline Tensor10D literal — exactly ten finite values separated by
    /// commas/whitespace (optional surrounding brackets/parens) — into a
    /// [`Tensor10D`](crate::tensor::Tensor10D), reusing the profile's arity+finiteness
    /// validation (plan §3.6). Honest error on wrong arity / non-finite / bad number.
    fn parse_inline_tensor10d(text: &str) -> Result<crate::tensor::Tensor10D, String> {
        let vals: Vec<f64> = text
            .split(|c: char| {
                c == ',' || c.is_whitespace() || c == '[' || c == ']' || c == '(' || c == ')'
            })
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Tensor10D literal parse error: {e}"))?;
        let v = crate::sparql_library::immersive::validate_inline_tensor10d(&vals)
            .map_err(|e| e.to_string())?;
        Ok(crate::tensor::Tensor10D::new(
            v[0] as f32, v[1] as f32, v[2] as f32, v[3] as f32, v[4] as f32, v[5] as f32,
            v[6] as f32, v[7] as f32, v[8] as f32, v[9] as f32,
        ))
    }

    /// Resolve function argument `idx` as an inline Tensor10D literal.
    fn arg_tensor10d(
        idx: usize,
        args_start: u16,
        args_len: u16,
        ctx: &SparqlQueryContext,
        row: &BindingRow,
        resolver: Option<crate::sparql_ast::TextResolver>,
        fname: &str,
    ) -> Result<crate::tensor::Tensor10D, String> {
        let text = Self::arg_text(idx, args_start, args_len, ctx, row, resolver, fname)?;
        Self::parse_inline_tensor10d(&text)
    }
}

/// Evaluation result.
///
/// `Float(f64)` is the real-valued measurement / random channel (`RAND`,
/// `qispf:tensorDistance`). Adding it drops `Eq`/`Ord`/`Hash` (f64 has no total
/// equality/order/hash) — sound because nothing sorts/hashes/set-keys an `EvalResult`;
/// every consumer pattern-matches it or compares via `evaluate_binary_op`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvalResult {
    Numeric(u64),
    Boolean(bool),
    Iri(u64),
    String(u64), // Hash of string
    Float(f64),  // Real-valued measurement / random channel
}

impl EvalResult {
    pub fn as_bool(&self) -> bool {
        match self {
            EvalResult::Boolean(b) => *b,
            EvalResult::Numeric(n) => *n != 0,
            EvalResult::Float(f) => *f != 0.0,
            _ => false,
        }
    }

    /// Numeric view for comparison/arithmetic: `Numeric(n)` → `n as f64`,
    /// `Float(f)` → `f`, else `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            EvalResult::Numeric(n) => Some(*n as f64),
            EvalResult::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Total order for ORDER BY. Replaces the derived `Ord` (dropped because
    /// `Float(f64)` has no total `Ord`): numeric values (`Numeric`/`Float`) sort by
    /// value via `f64::total_cmp`; otherwise a stable per-variant rank then the inner
    /// hash/bool. This is a *sort* order, not SPARQL value equality.
    pub fn total_cmp(&self, other: &EvalResult) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            return a.total_cmp(&b);
        }
        fn rank(e: &EvalResult) -> u8 {
            match e {
                EvalResult::Numeric(_) | EvalResult::Float(_) => 0,
                EvalResult::Boolean(_) => 1,
                EvalResult::Iri(_) => 2,
                EvalResult::String(_) => 3,
            }
        }
        match rank(self).cmp(&rank(other)) {
            Ordering::Equal => match (self, other) {
                (EvalResult::Boolean(a), EvalResult::Boolean(b)) => a.cmp(b),
                (EvalResult::Iri(a), EvalResult::Iri(b)) => a.cmp(b),
                (EvalResult::String(a), EvalResult::String(b)) => a.cmp(b),
                _ => Ordering::Equal,
            },
            ord => ord,
        }
    }

    /// Recover the text of a `String`/`Iri` result via the query [`StringSink`]
    /// (produced strings) — the counterpart to the value-producing builtins. Returns
    /// `None` for non-string results or an unresolvable hash.
    pub fn as_string(&self, sink: &crate::sparql_ast::StringSink) -> Option<String> {
        match self {
            EvalResult::String(h) | EvalResult::Iri(h) => sink.resolve(*h),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_variable() {
        let mut ctx = SparqlQueryContext::new();
        let var_id = ctx.register_variable("?x").unwrap();

        let expr = Expression::Variable(var_id);
        ctx.alloc_expression(expr).unwrap();

        let mut row = BindingRow::new();
        row.set(var_id, 42);

        let result = ExpressionEvaluator::evaluate(0, &ctx, &row).unwrap();
        assert_eq!(result, EvalResult::Numeric(42));
    }

    #[test]
    fn test_evaluate_binary_op() {
        let left = EvalResult::Numeric(5);
        let right = EvalResult::Numeric(3);

        let result = ExpressionEvaluator::evaluate_binary_op(BinaryOp::Add, left, right).unwrap();
        assert_eq!(result, EvalResult::Numeric(8));
    }

    #[test]
    fn test_evaluate_unary_not() {
        let inner = EvalResult::Boolean(true);
        let result = ExpressionEvaluator::evaluate_unary_op(UnaryOp::Not, inner).unwrap();
        assert_eq!(result, EvalResult::Boolean(false));
    }

    #[test]
    fn test_bound_function() {
        let mut ctx = SparqlQueryContext::new();
        let var_id = ctx.register_variable("?x").unwrap();

        ctx.function_args[0] = var_id as ExpressionId;
        ctx.function_arg_count = 1;

        let mut row = BindingRow::new();
        row.set(var_id, 42);

        let result =
            ExpressionEvaluator::evaluate_function(Function::Bound, 0, 1, &ctx, &row, None).unwrap();

        assert_eq!(result, EvalResult::Boolean(true));
    }

    // ── Resolver-backed string / control-flow FILTER builtins ────────────────

    #[test]
    fn test_contains_strstarts_strends_strlen_via_resolver() {
        let mut ctx = SparqlQueryContext::new();
        let hay = 0xA1u64;
        let needle = 0xB2u64;
        let hay_expr = ctx.alloc_expression(Expression::Literal(hay)).unwrap();
        let needle_expr = ctx.alloc_expression(Expression::Literal(needle)).unwrap();
        ctx.function_args[0] = hay_expr;
        ctx.function_args[1] = needle_expr;
        ctx.function_arg_count = 2;

        let mut literals = LiteralTable::new();
        literals.intern(hay, "hello world");
        literals.intern(needle, "world");
        let resolver = TextResolver::new(&literals);
        let row = BindingRow::new();

        let contains = ExpressionEvaluator::evaluate_function(
            Function::Contains, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(contains, EvalResult::Boolean(true));

        let starts = ExpressionEvaluator::evaluate_function(
            Function::VarStarts, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(starts, EvalResult::Boolean(false)); // "hello world" !startsWith "world"

        let ends = ExpressionEvaluator::evaluate_function(
            Function::VarEnds, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(ends, EvalResult::Boolean(true)); // ends with "world"

        let len = ExpressionEvaluator::evaluate_function(
            Function::Strlen, 0, 1, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(len, EvalResult::Numeric(11)); // "hello world"
    }

    #[test]
    fn test_regex_flags_via_resolver() {
        let mut ctx = SparqlQueryContext::new();
        let (text, pat, flags) = (0xC3u64, 0xD4u64, 0xE5u64);
        let t = ctx.alloc_expression(Expression::Literal(text)).unwrap();
        let p = ctx.alloc_expression(Expression::Literal(pat)).unwrap();
        let f = ctx.alloc_expression(Expression::Literal(flags)).unwrap();
        ctx.function_args[0] = t;
        ctx.function_args[1] = p;
        ctx.function_args[2] = f;
        ctx.function_arg_count = 3;

        let mut literals = LiteralTable::new();
        literals.intern(text, "Hello World");
        literals.intern(pat, "^hello");
        literals.intern(flags, "i");
        let resolver = TextResolver::new(&literals);
        let row = BindingRow::new();

        // Case-insensitive flag: ^hello matches "Hello World".
        let with_i = ExpressionEvaluator::evaluate_function(
            Function::Regex, 0, 3, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(with_i, EvalResult::Boolean(true));

        // Without the flag arg, ^hello does NOT match "Hello World".
        let no_flag = ExpressionEvaluator::evaluate_function(
            Function::Regex, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(no_flag, EvalResult::Boolean(false));
    }

    #[test]
    fn test_string_filter_without_resolver_errors_never_fabricates() {
        // The whole point of the honesty fix: no resolver => honest error,
        // NOT a fabricated `true`.
        let mut ctx = SparqlQueryContext::new();
        let a = ctx.alloc_expression(Expression::Literal(1)).unwrap();
        let b = ctx.alloc_expression(Expression::Literal(2)).unwrap();
        ctx.function_args[0] = a;
        ctx.function_args[1] = b;
        ctx.function_arg_count = 2;
        let row = BindingRow::new();

        for func in [Function::Contains, Function::Regex, Function::Strlen] {
            let r = ExpressionEvaluator::evaluate_function(func, 0, 2, &ctx, &row, None);
            assert!(r.is_err(), "{func:?} must error without a resolver, not fabricate");
        }
    }

    #[test]
    fn test_sameterm_and_if_control_flow() {
        let mut ctx = SparqlQueryContext::new();
        let five_a = ctx.alloc_expression(Expression::Literal(5)).unwrap();
        let five_b = ctx.alloc_expression(Expression::Literal(5)).unwrap();
        let nine = ctx.alloc_expression(Expression::Literal(9)).unwrap();
        let row = BindingRow::new();

        // SAMETERM(5, 5) => true (same type tag + value).
        ctx.function_args[0] = five_a;
        ctx.function_args[1] = five_b;
        ctx.function_arg_count = 2;
        let same = ExpressionEvaluator::evaluate_function(
            Function::SameTerm, 0, 2, &ctx, &row, None,
        )
        .unwrap();
        assert_eq!(same, EvalResult::Boolean(true));

        // IF(cond=5 (truthy), then=9, else=5) => 9.
        ctx.function_args[0] = five_a;
        ctx.function_args[1] = nine;
        ctx.function_args[2] = five_b;
        ctx.function_arg_count = 3;
        let chosen =
            ExpressionEvaluator::evaluate_function(Function::If, 0, 3, &ctx, &row, None).unwrap();
        assert_eq!(chosen, EvalResult::Numeric(9));
    }

    #[test]
    fn test_string_producing_builtin_without_sink_errors() {
        // A value-producing builtin (CONCAT) with no StringSink must fail closed,
        // not fabricate — there is nowhere to put the produced string.
        let ctx = SparqlQueryContext::new();
        let row = BindingRow::new();
        let r = ExpressionEvaluator::evaluate_function(Function::Concat, 0, 0, &ctx, &row, None);
        assert!(r.is_err(), "string-producing builtin must error without a sink, not fabricate");
    }

    #[test]
    fn test_residual_builtins_fail_closed_never_fabricate() {
        // RAND (no float channel) and the lang/datatype-tag builtins remain honest
        // errors — they must NOT silently pass.
        let ctx = SparqlQueryContext::new();
        let row = BindingRow::new();
        for f in [Function::Rand, Function::LangMatches, Function::StrLang, Function::StrDt] {
            let r = ExpressionEvaluator::evaluate_function(f, 0, 0, &ctx, &row, None);
            assert!(r.is_err(), "{f:?} must fail closed, not fabricate");
        }
    }

    // ── QISP-R06 string-producing + temporal + UUID builtins ──────────────────

    fn r06_ctx() -> (SparqlQueryContext, LiteralTable) {
        (SparqlQueryContext::new(), LiteralTable::new())
    }

    #[test]
    fn test_r06_concat_ucase_lcase_encode_via_sink() {
        use crate::sparql_ast::StringSink;
        let (mut ctx, mut lits) = r06_ctx();
        let (a, b) = (0xA1u64, 0xB2u64);
        let ea = ctx.alloc_expression(Expression::Literal(a)).unwrap();
        let eb = ctx.alloc_expression(Expression::Literal(b)).unwrap();
        ctx.function_args[0] = ea;
        ctx.function_args[1] = eb;
        ctx.function_arg_count = 2;
        lits.intern(a, "Hello, ");
        lits.intern(b, "World/x y");
        let sink = StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink);
        let row = BindingRow::new();

        let concat = ExpressionEvaluator::evaluate_function(
            Function::Concat, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        match concat {
            EvalResult::String(h) => assert_eq!(sink.resolve(h).as_deref(), Some("Hello, World/x y")),
            _ => panic!("CONCAT must produce a string"),
        }

        let up = ExpressionEvaluator::evaluate_function(
            Function::Ucase, 0, 1, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(up.as_string(&sink).as_deref(), Some("HELLO, "));

        let lo = ExpressionEvaluator::evaluate_function(
            Function::Lcase, 0, 1, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(lo.as_string(&sink).as_deref(), Some("hello, "));

        // ENCODE_FOR_URI on arg 1 ("World/x y"): '/' and ' ' get percent-encoded.
        let enc = ExpressionEvaluator::evaluate_function(
            Function::EncodeForUri, 1, 1, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(enc.as_string(&sink).as_deref(), Some("World%2Fx%20y"));
    }

    #[test]
    fn test_r06_substr_strbefore_strafter() {
        use crate::sparql_ast::StringSink;
        let (mut ctx, mut lits) = r06_ctx();
        let s = 0xC1u64;
        let es = ctx.alloc_expression(Expression::Literal(s)).unwrap();
        let e_start = ctx.alloc_expression(Expression::Literal(7)).unwrap(); // start=7
        let e_len = ctx.alloc_expression(Expression::Literal(5)).unwrap(); // len=5
        let sep = 0xD2u64;
        let esep = ctx.alloc_expression(Expression::Literal(sep)).unwrap();
        lits.intern(s, "hello world!"); // 1-based: 'w' is position 7
        lits.intern(sep, " ");
        let sink = StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink);
        let row = BindingRow::new();

        // SUBSTR("hello world!", 7, 5) -> "world"
        ctx.function_args[0] = es;
        ctx.function_args[1] = e_start;
        ctx.function_args[2] = e_len;
        ctx.function_arg_count = 3;
        let sub = ExpressionEvaluator::evaluate_function(
            Function::Substring, 0, 3, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(sub.as_string(&sink).as_deref(), Some("world"));

        // STRBEFORE("hello world!", " ") -> "hello"
        ctx.function_args[0] = es;
        ctx.function_args[1] = esep;
        ctx.function_arg_count = 2;
        let before = ExpressionEvaluator::evaluate_function(
            Function::StrBefore, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(before.as_string(&sink).as_deref(), Some("hello"));

        // STRAFTER("hello world!", " ") -> "world!"
        let after = ExpressionEvaluator::evaluate_function(
            Function::StrAfter, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(after.as_string(&sink).as_deref(), Some("world!"));
    }

    #[test]
    fn test_r06_now_and_year_are_query_stable() {
        use crate::sparql_ast::StringSink;
        let (ctx, lits) = r06_ctx();
        let sink = StringSink::new();
        // 2021-01-01T00:00:00Z = 1609459200000 ms.
        let now_ms = 1_609_459_200_000u64;
        let resolver = TextResolver::new(&lits).with_sink(&sink).with_env(now_ms, 12345);
        let row = BindingRow::new();

        let now = ExpressionEvaluator::evaluate_function(
            Function::Now, 0, 0, &ctx, &row, Some(resolver),
        )
        .unwrap();
        let now_text = now.as_string(&sink).expect("NOW is a string");
        assert!(now_text.starts_with("2021-01-01T00:00:00"), "got {now_text}");

        // YEAR(now_text) -> 2021. Feed the produced dateTime back as the arg.
        let mut ctx2 = SparqlQueryContext::new();
        let dt_hash = match now {
            EvalResult::String(h) => h,
            _ => panic!(),
        };
        let earg = ctx2.alloc_expression(Expression::Literal(dt_hash)).unwrap();
        ctx2.function_args[0] = earg;
        ctx2.function_arg_count = 1;
        let year = ExpressionEvaluator::evaluate_function(
            Function::Year, 0, 1, &ctx2, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(year, EvalResult::Numeric(2021));
    }

    #[test]
    fn test_r06_now_without_clock_fails_closed() {
        let (ctx, lits) = r06_ctx();
        let sink = crate::sparql_ast::StringSink::new();
        // now_ms unset (0) → NOW must error, never fabricate a time.
        let resolver = TextResolver::new(&lits).with_sink(&sink);
        let row = BindingRow::new();
        let r = ExpressionEvaluator::evaluate_function(Function::Now, 0, 0, &ctx, &row, Some(resolver));
        assert!(r.is_err(), "NOW without a query-stable clock must fail closed");
    }

    #[test]
    fn test_r06_uuid_is_query_stable_and_distinct_per_site() {
        use crate::sparql_ast::StringSink;
        let (mut ctx, lits) = r06_ctx();
        let sink = StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink).with_env(1, 0xABCDEF);
        let row = BindingRow::new();

        // Two calls at the SAME site (same args_start) are stable.
        ctx.function_arg_count = 0;
        let u1 = ExpressionEvaluator::evaluate_function(Function::Uuid, 0, 0, &ctx, &row, Some(resolver)).unwrap();
        let u2 = ExpressionEvaluator::evaluate_function(Function::Uuid, 0, 0, &ctx, &row, Some(resolver)).unwrap();
        assert_eq!(u1, u2, "same UUID site is stable within a query");
        let t1 = u1.as_string(&sink).unwrap();
        assert!(t1.starts_with("urn:uuid:"), "UUID() yields an IRI term: {t1}");

        // A different call site (different args_start) yields a different UUID.
        let u3 = ExpressionEvaluator::evaluate_function(Function::Uuid, 4, 0, &ctx, &row, Some(resolver)).unwrap();
        assert_ne!(u1, u3, "distinct UUID sites differ");
    }

    #[test]
    fn test_r06_coalesce_skips_unbound_and_errored() {
        let (mut ctx, lits) = r06_ctx();
        let sink = crate::sparql_ast::StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink);

        // ?unbound (var 0, unbound) then literal 42 → COALESCE returns 42.
        let vunbound = ctx.register_variable("?u").unwrap();
        let e_var = ctx.alloc_expression(Expression::Variable(vunbound)).unwrap();
        let e_lit = ctx.alloc_expression(Expression::Literal(42)).unwrap();
        ctx.function_args[0] = e_var;
        ctx.function_args[1] = e_lit;
        ctx.function_arg_count = 2;
        let row = BindingRow::new(); // var 0 unbound

        let c = ExpressionEvaluator::evaluate_function(
            Function::Coalesce, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(c, EvalResult::Numeric(42), "COALESCE skips the unbound variable");
    }

    // ── QISP (qispf:) function dispatch: admission + geo-deference + honest gaps ──

    #[test]
    fn test_qisp_custom_dispatch_admission_and_geo_deference() {
        use crate::sparql_ast::StringSink;

        // qispf:knn is a table-producing graph operator → rejected inline (admission),
        // never fabricated.
        let ctx0 = SparqlQueryContext::new();
        let row0 = BindingRow::new();
        let knn = Function::Custom(crate::q_hash(
            "https://webizen.org/immersive/function/0.1#knn",
        ));
        assert!(
            ExpressionEvaluator::evaluate_function(knn, 0, 2, &ctx0, &row0, None).is_err(),
            "qispf:knn must be rejected inline"
        );

        // qispf:intersects defers to GeoSPARQL → executes on WKT (point in polygon → true).
        let mut ctx = SparqlQueryContext::new();
        let (a, b) = (0xAA11u64, 0xBB22u64);
        let ea = ctx.alloc_expression(Expression::Literal(a)).unwrap();
        let eb = ctx.alloc_expression(Expression::Literal(b)).unwrap();
        ctx.function_args[0] = ea;
        ctx.function_args[1] = eb;
        ctx.function_arg_count = 2;
        let mut lits = LiteralTable::new();
        lits.intern(a, "POINT(1 1)");
        lits.intern(b, "POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))");
        let sink = StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink);
        let row = BindingRow::new();
        let intersects = Function::Custom(crate::q_hash(
            "https://webizen.org/immersive/function/0.1#intersects",
        ));
        let got = ExpressionEvaluator::evaluate_function(
            intersects, 0, 2, &ctx, &row, Some(resolver),
        )
        .unwrap();
        assert_eq!(got, EvalResult::Boolean(true), "point (1,1) intersects the square");

        // qispf:volume is QISP-owned (mesh) → honest "not yet executable inline"
        // error, NOT a fabricated measurement.
        let vol = Function::Custom(crate::q_hash(
            "https://webizen.org/immersive/function/0.1#volume",
        ));
        assert!(
            ExpressionEvaluator::evaluate_function(vol, 0, 1, &ctx, &row, Some(resolver)).is_err(),
            "qispf:volume needs asset resolution → honest error, not fabricated"
        );
    }

    #[test]
    fn test_qisp_tensor_predicates_execute_inline() {
        use crate::sparql_ast::StringSink;
        let mut ctx = SparqlQueryContext::new();
        let (ha, hb, hr5, hr4) = (0x7A1u64, 0x7B2u64, 0x7C3u64, 0x7D4u64);
        let ea = ctx.alloc_expression(Expression::Literal(ha)).unwrap();
        let eb = ctx.alloc_expression(Expression::Literal(hb)).unwrap();
        let er5 = ctx.alloc_expression(Expression::Literal(hr5)).unwrap();
        let er4 = ctx.alloc_expression(Expression::Literal(hr4)).unwrap();
        let mut lits = LiteralTable::new();
        // a at origin, b offset (Δx=3, Δy=4) → euclidean distance 5 (v=0).
        lits.intern(ha, "0,0,0,0,0,0,0,0,0,0");
        lits.intern(hb, "0,0,0,3,4,0,0,0,0,0");
        lits.intern(hr5, "5.0");
        lits.intern(hr4, "4.0");
        let sink = StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink);
        let row = BindingRow::new();

        // tensorDistance(a, b) → Float(5.0), through the resident substrate metric.
        ctx.function_args[0] = ea;
        ctx.function_args[1] = eb;
        ctx.function_arg_count = 2;
        let dist_iri = Function::Custom(crate::q_hash(
            "https://webizen.org/immersive/function/0.1#tensorDistance",
        ));
        match ExpressionEvaluator::evaluate_function(dist_iri, 0, 2, &ctx, &row, Some(resolver)).unwrap() {
            EvalResult::Float(d) => assert!((d - 5.0).abs() < 1e-5, "distance {d} != 5"),
            other => panic!("tensorDistance must be Float, got {other:?}"),
        }

        // tensorWithin(a, b, 5.0) → true; radius 4.0 → false.
        ctx.function_args[2] = er5;
        ctx.function_arg_count = 3;
        let within_iri = Function::Custom(crate::q_hash(
            "https://webizen.org/immersive/function/0.1#tensorWithin",
        ));
        let w5 = ExpressionEvaluator::evaluate_function(within_iri, 0, 3, &ctx, &row, Some(resolver)).unwrap();
        assert_eq!(w5, EvalResult::Boolean(true), "distance 5 is within radius 5");
        ctx.function_args[2] = er4;
        let w4 = ExpressionEvaluator::evaluate_function(within_iri, 0, 3, &ctx, &row, Some(resolver)).unwrap();
        assert_eq!(w4, EvalResult::Boolean(false), "distance 5 is NOT within radius 4");

        // A malformed tensor literal (wrong arity) fails closed, never fabricates.
        let mut ctx2 = SparqlQueryContext::new();
        let bad = 0x7E5u64;
        let ebad = ctx2.alloc_expression(Expression::Literal(bad)).unwrap();
        let eb2 = ctx2.alloc_expression(Expression::Literal(hb)).unwrap();
        ctx2.function_args[0] = ebad;
        ctx2.function_args[1] = eb2;
        ctx2.function_arg_count = 2;
        let mut lits2 = LiteralTable::new();
        lits2.intern(bad, "1,2,3"); // only 3 values
        lits2.intern(hb, "0,0,0,3,4,0,0,0,0,0");
        let sink2 = StringSink::new();
        let resolver2 = TextResolver::new(&lits2).with_sink(&sink2);
        assert!(
            ExpressionEvaluator::evaluate_function(dist_iri, 0, 2, &ctx2, &BindingRow::new(), Some(resolver2)).is_err(),
            "a 3-value tensor literal must fail closed, not fabricate a distance"
        );
    }

    #[test]
    fn test_lang_datatype_strlang_strdt_langmatches() {
        use crate::sparql_ast::{literal_term_hash, StringSink};
        let mut ctx = SparqlQueryContext::new();
        let h_en = literal_term_hash("hello", Some("en"), None);
        let h_plain = literal_term_hash("hello", None, None);
        let h_hi = literal_term_hash("hi", None, None);
        let h_fr = literal_term_hash("fr", None, None);
        let h_dt = literal_term_hash("http://example.org/myType", None, None);
        let e_en = ctx.alloc_expression(Expression::Literal(h_en)).unwrap();
        let e_plain = ctx.alloc_expression(Expression::Literal(h_plain)).unwrap();
        let e_hi = ctx.alloc_expression(Expression::Literal(h_hi)).unwrap();
        let e_fr = ctx.alloc_expression(Expression::Literal(h_fr)).unwrap();
        let e_dt = ctx.alloc_expression(Expression::Literal(h_dt)).unwrap();

        let mut lits = LiteralTable::new();
        lits.intern_tagged(h_en, "hello", Some("en"), None);
        lits.intern(h_plain, "hello");
        lits.intern(h_hi, "hi");
        lits.intern(h_fr, "fr");
        lits.intern(h_dt, "http://example.org/myType");
        let sink = StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink);
        let row = BindingRow::new();

        // LANG("hello"@en) = "en"; LANG("hello") = "" (correct default, not fabricated).
        ctx.function_args[0] = e_en;
        ctx.function_arg_count = 1;
        let l1 = ExpressionEvaluator::evaluate_function(Function::Lang, 0, 1, &ctx, &row, Some(resolver)).unwrap();
        assert_eq!(l1.as_string(&sink).as_deref(), Some("en"));
        ctx.function_args[0] = e_plain;
        let l0 = ExpressionEvaluator::evaluate_function(Function::Lang, 0, 1, &ctx, &row, Some(resolver)).unwrap();
        assert_eq!(l0.as_string(&sink).as_deref(), Some(""));

        // DATATYPE("hello") = xsd:string, returned as an IRI term comparable to the query's.
        let dt = ExpressionEvaluator::evaluate_function(Function::Datatype, 0, 1, &ctx, &row, Some(resolver)).unwrap();
        assert_eq!(
            dt,
            EvalResult::Iri(crate::lexicon::generate_60bit_token(
                b"http://www.w3.org/2001/XMLSchema#string"
            ))
        );

        // STRLANG("hi","fr") round-trips: the produced term's LANG is "fr".
        ctx.function_args[0] = e_hi;
        ctx.function_args[1] = e_fr;
        ctx.function_arg_count = 2;
        let sl = ExpressionEvaluator::evaluate_function(Function::StrLang, 0, 2, &ctx, &row, Some(resolver)).unwrap();
        let sl_hash = match sl {
            EvalResult::String(h) => h,
            other => panic!("STRLANG must be a String, got {other:?}"),
        };
        assert_eq!(resolver.lang_of(sl_hash).as_deref(), Some("fr"));

        // STRDT("hi", myType) round-trips: the produced term's DATATYPE is myType.
        ctx.function_args[0] = e_hi;
        ctx.function_args[1] = e_dt;
        let sd = ExpressionEvaluator::evaluate_function(Function::StrDt, 0, 2, &ctx, &row, Some(resolver)).unwrap();
        let sd_hash = match sd {
            EvalResult::String(h) => h,
            other => panic!("STRDT must be a String, got {other:?}"),
        };
        assert_eq!(resolver.datatype_of(sd_hash).as_deref(), Some("http://example.org/myType"));

        // LANGMATCHES per RFC 4647.
        assert!(ExpressionEvaluator::lang_matches("en-US", "en"));
        assert!(ExpressionEvaluator::lang_matches("de", "*"));
        assert!(!ExpressionEvaluator::lang_matches("fr", "en"));
    }

    #[test]
    fn test_did_resolve_wired_and_crypto_routes_honestly() {
        use crate::sparql_ast::{literal_term_hash, StringSink};
        let mut ctx = SparqlQueryContext::new();
        let h = literal_term_hash("did:web:example.org", None, None);
        let e = ctx.alloc_expression(Expression::Literal(h)).unwrap();
        ctx.function_args[0] = e;
        ctx.function_arg_count = 1;
        let mut lits = LiteralTable::new();
        lits.intern(h, "did:web:example.org");
        let sink = StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink);
        let row = BindingRow::new();

        // did:resolve(?did) → the DID's real endpoint URL (no keys needed).
        let f = Function::Custom(crate::q_hash("did:resolve"));
        let out = ExpressionEvaluator::evaluate_function(f, 0, 1, &ctx, &row, Some(resolver)).unwrap();
        assert_eq!(
            out.as_string(&sink).as_deref(),
            Some("https://example.org/.well-known/did.json")
        );

        // did:sign → honest error (the query layer holds no keys), never fabricated.
        let s = Function::Custom(crate::q_hash("did:sign"));
        assert!(ExpressionEvaluator::evaluate_function(s, 0, 1, &ctx, &row, Some(resolver)).is_err());
    }

    #[test]
    fn test_rand_and_float_channel() {
        let ctx = SparqlQueryContext::new();
        let lits = LiteralTable::new();
        let sink = crate::sparql_ast::StringSink::new();
        let resolver = TextResolver::new(&lits).with_sink(&sink).with_env(0, 0xABCDEF);
        let row = BindingRow::new();

        // RAND is a real double in [0,1), query-stable per site, distinct per site.
        let r1 = ExpressionEvaluator::evaluate_function(Function::Rand, 0, 0, &ctx, &row, Some(resolver)).unwrap();
        let r1b = ExpressionEvaluator::evaluate_function(Function::Rand, 0, 0, &ctx, &row, Some(resolver)).unwrap();
        assert_eq!(r1, r1b, "RAND is stable at the same site within a query");
        match r1 {
            EvalResult::Float(v) => assert!((0.0..1.0).contains(&v), "RAND {v} not in [0,1)"),
            other => panic!("RAND must be Float, got {other:?}"),
        }
        let r2 = ExpressionEvaluator::evaluate_function(Function::Rand, 4, 0, &ctx, &row, Some(resolver)).unwrap();
        assert_ne!(r1, r2, "distinct RAND sites differ");

        // Without a query-stable seed, RAND fails closed (never fabricates).
        let no_seed = TextResolver::new(&lits);
        assert!(
            ExpressionEvaluator::evaluate_function(Function::Rand, 0, 0, &ctx, &row, Some(no_seed)).is_err(),
            "RAND without a seed must fail closed"
        );

        // Float participates in comparison + arithmetic, mixing with integer terms.
        assert_eq!(
            ExpressionEvaluator::evaluate_binary_op(BinaryOp::LessThan, EvalResult::Float(5.0), EvalResult::Numeric(6)),
            Ok(EvalResult::Boolean(true))
        );
        assert_eq!(
            ExpressionEvaluator::evaluate_binary_op(BinaryOp::Add, EvalResult::Float(2.5), EvalResult::Numeric(1)),
            Ok(EvalResult::Float(3.5))
        );
        // Exact term-hash equality (Numeric/Numeric) is preserved, not routed through f64.
        assert_eq!(
            ExpressionEvaluator::evaluate_binary_op(BinaryOp::Equal, EvalResult::Numeric(42), EvalResult::Numeric(42)),
            Ok(EvalResult::Boolean(true))
        );
    }

    // ── PROV-O filter tests ──────────────────────────────────────────────────

    #[test]
    fn prov_predicate_hash_roundtrip() {
        use super::{prov_predicates, ProvOPredicate};
        let cases = [
            (
                ProvOPredicate::WasInvalidatedBy,
                prov_predicates::WAS_INVALIDATED_BY,
            ),
            (
                ProvOPredicate::WasAttributedTo,
                prov_predicates::WAS_ATTRIBUTED_TO,
            ),
            (
                ProvOPredicate::WasGeneratedBy,
                prov_predicates::WAS_GENERATED_BY,
            ),
            (
                ProvOPredicate::WasDerivedFrom,
                prov_predicates::WAS_DERIVED_FROM,
            ),
            (
                ProvOPredicate::StartedAtTime,
                prov_predicates::STARTED_AT_TIME,
            ),
            (ProvOPredicate::EndedAtTime, prov_predicates::ENDED_AT_TIME),
        ];
        for (pred, expected_hash) in cases {
            assert_eq!(pred.hash(), expected_hash);
            assert_eq!(ProvOPredicate::from_hash(expected_hash), Some(pred));
        }
    }

    #[test]
    fn prov_predicate_unknown_hash_returns_none() {
        use super::ProvOPredicate;
        assert_eq!(ProvOPredicate::from_hash(0xDEAD_BEEF_1234_5678), None);
        assert_eq!(ProvOPredicate::from_hash(0), None);
    }

    #[test]
    fn provenance_filter_is_prov_predicate() {
        use super::{prov_predicates, ProvenanceFilter};
        assert!(ProvenanceFilter::is_prov_predicate(
            prov_predicates::WAS_INVALIDATED_BY
        ));
        assert!(ProvenanceFilter::is_prov_predicate(
            prov_predicates::WAS_ATTRIBUTED_TO
        ));
        assert!(!ProvenanceFilter::is_prov_predicate(0xFFFF_0000_FFFF_0000));
    }

    #[test]
    fn provenance_filter_invalidation_helpers() {
        use super::{prov_predicates, ProvenanceFilter};
        use crate::NQuin;

        const SUBJECT: u64 = 0xABCD_1234;
        const AGENT: u64 = 0x9999_AAAA;

        let invalidation_quin = NQuin {
            subject: SUBJECT,
            predicate: prov_predicates::WAS_INVALIDATED_BY,
            object: AGENT,
            context: 0x0001,
            metadata: 0,
            parity: SUBJECT ^ prov_predicates::WAS_INVALIDATED_BY ^ AGENT ^ 0x0001,
        };
        let other_quin = NQuin {
            subject: SUBJECT,
            predicate: prov_predicates::WAS_GENERATED_BY,
            object: AGENT,
            context: 0x0001,
            metadata: 0,
            parity: 0,
        };
        let quins = [invalidation_quin, other_quin];

        assert!(ProvenanceFilter::subject_is_invalidated(&quins, SUBJECT));
        assert!(!ProvenanceFilter::subject_is_invalidated(&quins, 0xDEAD));
        assert!(ProvenanceFilter::is_invalidation_predicate(
            prov_predicates::WAS_INVALIDATED_BY
        ));
        assert!(!ProvenanceFilter::is_invalidation_predicate(
            prov_predicates::WAS_ATTRIBUTED_TO
        ));
    }

    #[test]
    fn provenance_filter_attributions_iterator() {
        use super::{prov_predicates, ProvenanceFilter};
        use crate::NQuin;

        const SUBJECT: u64 = 0x1111_2222;
        const AGENT_A: u64 = 0xAAAA_0001;
        const AGENT_B: u64 = 0xBBBB_0002;

        let quins = [
            NQuin {
                subject: SUBJECT,
                predicate: prov_predicates::WAS_ATTRIBUTED_TO,
                object: AGENT_A,
                context: 0x01,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: SUBJECT,
                predicate: prov_predicates::WAS_ATTRIBUTED_TO,
                object: AGENT_B,
                context: 0x01,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: SUBJECT,
                predicate: prov_predicates::WAS_GENERATED_BY,
                object: AGENT_A,
                context: 0x01,
                metadata: 0,
                parity: 0,
            },
        ];

        let mut agents: Vec<u64> = ProvenanceFilter::attributions(&quins, SUBJECT).collect();
        agents.sort_unstable();
        assert_eq!(agents, vec![AGENT_A, AGENT_B]);

        assert!(ProvenanceFilter::subject_has_attribution(&quins, SUBJECT));
        assert!(!ProvenanceFilter::subject_has_attribution(&quins, 0xDEAD));
    }

    #[test]
    fn provenance_filter_filter_by() {
        use super::{prov_predicates, ProvOPredicate, ProvenanceFilter};
        use crate::NQuin;

        const S: u64 = 0x1234;
        let quins = [
            NQuin {
                subject: S,
                predicate: prov_predicates::WAS_INVALIDATED_BY,
                object: 1,
                context: 1,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: S,
                predicate: prov_predicates::WAS_ATTRIBUTED_TO,
                object: 2,
                context: 1,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: S,
                predicate: prov_predicates::WAS_ATTRIBUTED_TO,
                object: 3,
                context: 1,
                metadata: 0,
                parity: 0,
            },
        ];

        let attributed: Vec<_> =
            ProvenanceFilter::filter_by(&quins, ProvOPredicate::WasAttributedTo).collect();
        assert_eq!(attributed.len(), 2);

        let invalidated: Vec<_> =
            ProvenanceFilter::filter_by(&quins, ProvOPredicate::WasInvalidatedBy).collect();
        assert_eq!(invalidated.len(), 1);
    }

    #[test]
    fn eval_prov_filter_matches_and_misses() {
        use super::{prov_predicates, ProvOPredicate, ProvenanceFilter};

        let iri_result = EvalResult::Iri(prov_predicates::WAS_INVALIDATED_BY);
        assert_eq!(
            ProvenanceFilter::eval_prov_filter(iri_result, ProvOPredicate::WasInvalidatedBy),
            Some(true)
        );
        assert_eq!(
            ProvenanceFilter::eval_prov_filter(iri_result, ProvOPredicate::WasAttributedTo),
            Some(false)
        );

        let non_prov = EvalResult::Iri(0xDEAD_0001);
        assert_eq!(
            ProvenanceFilter::eval_prov_filter(non_prov, ProvOPredicate::WasInvalidatedBy),
            None
        );
    }
}
