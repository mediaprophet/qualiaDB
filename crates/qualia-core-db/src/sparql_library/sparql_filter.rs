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
            BinaryOp::Equal => Ok(EvalResult::Boolean(left == right)),
            BinaryOp::NotEqual => Ok(EvalResult::Boolean(left != right)),
            BinaryOp::LessThan => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => Ok(EvalResult::Boolean(l < r)),
                _ => Err("LESS THAN operator requires numeric operands".to_string()),
            },
            BinaryOp::LessThanOrEqual => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => Ok(EvalResult::Boolean(l <= r)),
                _ => Err("LESS THAN OR EQUAL operator requires numeric operands".to_string()),
            },
            BinaryOp::GreaterThan => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => Ok(EvalResult::Boolean(l > r)),
                _ => Err("GREATER THAN operator requires numeric operands".to_string()),
            },
            BinaryOp::GreaterThanOrEqual => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => Ok(EvalResult::Boolean(l >= r)),
                _ => Err("GREATER THAN OR EQUAL operator requires numeric operands".to_string()),
            },
            BinaryOp::Add => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                    Ok(EvalResult::Numeric(l.wrapping_add(r)))
                }
                _ => Err("ADD operator requires numeric operands".to_string()),
            },
            BinaryOp::Subtract => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                    Ok(EvalResult::Numeric(l.wrapping_sub(r)))
                }
                _ => Err("SUBTRACT operator requires numeric operands".to_string()),
            },
            BinaryOp::Multiply => match (left, right) {
                (EvalResult::Numeric(l), EvalResult::Numeric(r)) => {
                    Ok(EvalResult::Numeric(l.wrapping_mul(r)))
                }
                _ => Err("MULTIPLY operator requires numeric operands".to_string()),
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
                // STR(expr) - convert to string (simplified)
                if args_len >= 1 {
                    let expr_id = ctx.function_args[args_start as usize];
                    let result = Self::evaluate(expr_id, ctx, row)?;
                    Ok(result)
                } else {
                    Err("STR requires at least one argument".to_string())
                }
            }
            Function::Lang => {
                // LANG(expr) - get language tag (simplified)
                if args_len >= 1 {
                    Ok(EvalResult::String(0)) // Placeholder
                } else {
                    Err("LANG requires at least one argument".to_string())
                }
            }
            Function::Datatype => {
                // DATATYPE(expr) - get datatype IRI (simplified)
                if args_len >= 1 {
                    Ok(EvalResult::Iri(0)) // Placeholder
                } else {
                    Err("DATATYPE requires at least one argument".to_string())
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
            Function::Custom(iri_hash) => {
                // Extension functions dispatched by function-IRI hash. Currently
                // the GeoSPARQL predicates (geof:distance / sfContains / sfWithin
                // / sfIntersects / sfTouches). Each needs its two geometry
                // arguments as WKT *text*, recovered from term hashes via the
                // resolver (query constants + ingested lexicon). Without a
                // resolver, or if a literal can't be resolved, we return an
                // honest error rather than a fabricated result.
                use crate::sparql_library::geosparql;
                match geosparql::geo_fn_for_hash(iri_hash) {
                    Some(geo_fn) => {
                        if args_len < 2 {
                            return Err("geo function requires two geometry arguments".to_string());
                        }
                        let resolver = resolver.ok_or_else(|| {
                            "geo functions require a text resolver (no lexicon available)"
                                .to_string()
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
                    None => Err(format!("unknown extension function (hash {iri_hash:#018x})")),
                }
            }
            // Any builtin without a real handler above MUST NOT silently
            // evaluate to `true` — a FILTER that fabricates a pass returns
            // wrong rows (e.g. `FILTER(REGEX(?n,"x"))` would match everything).
            // Fail closed with an honest, named error instead. Implementing
            // these (REGEX/CONTAINS/STR* predicates via the resolver, plus the
            // date/UUID/constructor builtins) is tracked in
            // docs/plans/immersive-sparql-hypermedia-profile.md.
            other => Err(format!(
                "SPARQL FILTER function {other:?} is not implemented; \
                 refusing to fabricate a passing result"
            )),
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
}

/// Evaluation result
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EvalResult {
    Numeric(u64),
    Boolean(bool),
    Iri(u64),
    String(u64), // Hash of string
}

impl EvalResult {
    pub fn as_bool(&self) -> bool {
        match self {
            EvalResult::Boolean(b) => *b,
            EvalResult::Numeric(n) => *n != 0,
            _ => false,
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
