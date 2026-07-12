//! SPARQL Parser - Hand-Rolled Zero-Allocation Parser
//!
//! Simple SPARQL 1.1 subset parser that's zero-allocation by design.
//! Uses byte string slicing and no heap allocation.

use crate::sparql_ast::*;
use std::collections::HashMap;

/// Parse a SPARQL query into an AST (discarding the collected literal table).
pub fn parse_sparql(query: &str) -> Result<(SparqlQuery, SparqlQueryContext), String> {
    let (q, ctx, _lits) = parse_sparql_full(query)?;
    Ok((q, ctx))
}

/// Parse a SPARQL query and also return the [`LiteralTable`] of string/geometry
/// constants collected during the parse — needed by the executor to resolve
/// literal text for `geof:*`/text extension functions.
pub fn parse_sparql_full(
    query: &str,
) -> Result<(SparqlQuery, SparqlQueryContext, LiteralTable), String> {
    crate::sparql_library::sparql_grammar::expr::reset_parse_literals();
    let mut ctx = SparqlQueryContext::new();
    let (query, prefixes) = strip_prefix_declarations(query.trim());
    let query = query.as_str();

    let parsed = if query.starts_with("SELECT") {
        parse_select_query(query, &mut ctx, &prefixes).map(SparqlQuery::Select)
    } else if query.starts_with("ASK") {
        parse_ask_query(query, &mut ctx, &prefixes).map(SparqlQuery::Ask)
    } else if query.starts_with("CONSTRUCT") {
        parse_construct_query(query, &mut ctx, &prefixes).map(SparqlQuery::Construct)
    } else if query.starts_with("DESCRIBE") {
        parse_describe_query(query, &mut ctx, &prefixes).map(SparqlQuery::Describe)
    } else {
        Err("Unsupported query form".to_string())
    }?;

    let literals = crate::sparql_library::sparql_grammar::expr::take_parse_literals();
    Ok((parsed, ctx, literals))
}

fn strip_prefix_declarations(query: &str) -> (String, HashMap<String, String>) {
    let mut prefixes = HashMap::new();
    let mut body = String::new();
    for line in query.lines() {
        let trimmed = line.trim();
        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("PREFIX") {
            if let Some((prefix, iri)) = parse_prefix_line(trimmed) {
                prefixes.insert(prefix, iri);
            }
            continue;
        }
        if !body.is_empty() {
            body.push(' ');
        }
        body.push_str(trimmed);
    }
    if body.is_empty() {
        (query.to_string(), prefixes)
    } else {
        (body, prefixes)
    }
}

fn parse_prefix_line(line: &str) -> Option<(String, String)> {
    let rest = line
        .trim_start_matches("PREFIX")
        .trim_start_matches("prefix")
        .trim();
    let colon = rest.find(':')?;
    let prefix = rest[..colon]
        .trim()
        .trim_start_matches("PREFIX")
        .to_string();
    let after = rest[colon + 1..].trim();
    let iri = if after.starts_with('<') {
        after
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_string()
    } else {
        after.trim_matches('"').to_string()
    };
    Some((prefix, iri))
}

fn parse_select_query(
    query: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<SelectQuery, String> {
    let mut query_struct = SelectQuery::default();

    // Parse SELECT clause
    let after_select = query.trim_start_matches("SELECT").trim();
    let (distinct_reduced, after_distinct) = parse_distinct(after_select);
    query_struct.distinct = distinct_reduced.0;
    query_struct.reduced = distinct_reduced.1;

    // Parse variables
    let variables = parse_variables(after_distinct)?;
    for var in variables {
        let var_id = ctx.register_variable(var)?;
        if query_struct.var_count < MAX_VARIABLES as u8 {
            query_struct.variables[query_struct.var_count as usize] = var_id;
            query_struct.var_count += 1;
        }
    }

    // Parse WHERE clause - find WHERE in the original query
    let where_start = query.find("WHERE").ok_or("WHERE clause not found")?;
    let where_clause = &query[where_start..];
    let pattern_id = parse_where_clause(where_clause, ctx, prefixes)?;
    query_struct.root_pattern = pattern_id;

    // Parse AS OF / AT TIME temporal modifier (Phase 4).
    // Only search after the closing brace of the WHERE clause to avoid false positives.
    let after_where = query.rfind('}').map(|i| &query[i..]).unwrap_or("");
    if let Some(pos) = after_where.find("AS OF") {
        let ts_ms = parse_temporal_literal(after_where[pos + 5..].trim_start());
        let as_of_pat = Pattern::AsOf {
            inner: query_struct.root_pattern,
            timestamp_ms: ts_ms,
            mode: TemporalMode::AsOf,
        };
        query_struct.root_pattern = ctx.alloc_pattern(as_of_pat)?;
    } else if let Some(pos) = after_where.find("AT TIME") {
        let ts_ms = parse_temporal_literal(after_where[pos + 7..].trim_start());
        let at_time_pat = Pattern::AsOf {
            inner: query_struct.root_pattern,
            timestamp_ms: ts_ms,
            mode: TemporalMode::AtTime,
        };
        query_struct.root_pattern = ctx.alloc_pattern(at_time_pat)?;
    }

    // Parse LIMIT/OFFSET if present
    let limit_start = where_clause.find("LIMIT");
    if let Some(start) = limit_start {
        let limit_str = &where_clause[start + 5..];
        query_struct.limit = parse_integer(limit_str);
    }

    let offset_start = where_clause.find("OFFSET");
    if let Some(start) = offset_start {
        let offset_str = &where_clause[start + 6..];
        query_struct.offset = parse_integer(offset_str).unwrap_or(0);
    }

    Ok(query_struct)
}

fn parse_ask_query(
    query: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<AskQuery, String> {
    let after_ask = query.trim_start_matches("ASK").trim();
    let where_start = after_ask.find("WHERE").ok_or("WHERE clause not found")?;
    let where_clause = &after_ask[where_start..];
    let pattern_id = parse_where_clause(where_clause, ctx, prefixes)?;

    Ok(AskQuery {
        root_pattern: pattern_id,
    })
}

fn parse_construct_query(
    query: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<ConstructQuery, String> {
    let after_construct = query.trim_start_matches("CONSTRUCT").trim();
    // Simplified - just parse WHERE for now
    let where_start = after_construct
        .find("WHERE")
        .ok_or("WHERE clause not found")?;
    let where_clause = &after_construct[where_start..];
    let pattern_id = parse_where_clause(where_clause, ctx, prefixes)?;

    Ok(ConstructQuery {
        template_pattern: 0, // TODO: Parse template
        root_pattern: pattern_id,
        group_by: [0; MAX_VARIABLES],
        group_by_count: 0,
        having: None,
        order_by: [OrderCondition::default(); MAX_ORDER_CONDITIONS],
        order_by_count: 0,
        limit: None,
        offset: 0,
    })
}

fn parse_describe_query(
    query: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<DescribeQuery, String> {
    let after_describe = query.trim_start_matches("DESCRIBE").trim();
    // Simplified - just parse WHERE for now
    let where_start = after_describe.find("WHERE");
    let root_pattern = if let Some(start) = where_start {
        let where_clause = &after_describe[start + 5..];
        Some(parse_where_clause(where_clause, ctx, prefixes)?)
    } else {
        None
    };

    Ok(DescribeQuery {
        vars_or_ids: [0; MAX_VARIABLES],
        var_count: 0,
        root_pattern,
    })
}

fn parse_distinct(input: &str) -> ((bool, bool), &str) {
    let input = input.trim();
    if input.starts_with("DISTINCT") {
        let after_distinct = input.trim_start_matches("DISTINCT").trim();
        if after_distinct.starts_with("REDUCED") {
            (
                (true, true),
                after_distinct.trim_start_matches("REDUCED").trim(),
            )
        } else {
            ((true, false), after_distinct)
        }
    } else {
        ((false, false), input)
    }
}

fn parse_variables(input: &str) -> Result<Vec<&str>, String> {
    let input = input.trim();
    if input == "*" {
        return Ok(vec![]);
    }
    let select_clause = if let Some(pos) = input.to_ascii_uppercase().find("WHERE") {
        &input[..pos]
    } else {
        input
    };
    let vars: Vec<&str> = select_clause
        .split_whitespace()
        .filter(|s| !s.is_empty() && s.starts_with('?'))
        .collect();
    Ok(vars)
}

fn parse_where_clause(
    input: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<PatternId, String> {
    // Delegate to the recursive-descent group-graph-pattern parser in
    // `sparql_grammar`: FILTER / OPTIONAL / UNION / MINUS / nested groups /
    // quoted triples, producing the `Pattern` arena the planner + executor
    // already run. (`input` begins with `WHERE { … }`; the grammar consumes the
    // balanced braces and ignores any trailing solution modifiers, which the
    // caller parses separately.)
    crate::sparql_library::sparql_grammar::parse_where_group(input, ctx, prefixes)
}

fn parse_integer(input: &str) -> Option<u64> {
    input.trim().parse().ok()
}

/// Parse a temporal literal (Phase 4 AS OF / AT TIME).
///
/// Accepts two forms:
/// - Integer milliseconds since Unix epoch: `1717286400000`
/// - Typed ISO 8601 date literal: `"2024-06-01"^^xsd:dateTime`
///
/// Falls back to `0` if parsing fails.
fn parse_temporal_literal(input: &str) -> u64 {
    let input = input.trim();
    // Strip typed literal wrapper: "YYYY-MM-DD"^^xsd:dateTime
    let bare = if let Some(inner) = input.strip_prefix('"') {
        inner.split("\"^^").next().unwrap_or(inner)
    } else {
        input
    };
    // Try integer milliseconds first.
    if let Ok(ms) = bare.parse::<u64>() {
        return ms;
    }
    // Minimal ISO 8601 date: YYYY-MM-DD → milliseconds at midnight UTC.
    if bare.len() >= 10 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            bare[0..4].parse::<u64>(),
            bare[5..7].parse::<u64>(),
            bare[8..10].parse::<u64>(),
        ) {
            let days = temporal_days_since_epoch(y, m, d);
            return days * 86_400_000;
        }
    }
    0
}

fn temporal_days_since_epoch(year: u64, month: u64, day: u64) -> u64 {
    let mut days = 0u64;
    for y in 1970..year {
        days += if temporal_is_leap(y) { 366 } else { 365 };
    }
    let month_days: [u64; 12] = [
        31,
        if temporal_is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for m in 1..month {
        days += month_days[(m - 1) as usize];
    }
    days + day - 1
}

fn temporal_is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: dots *inside* a `<…>` IRI or a `"…"` literal must not be
    /// treated as triple terminators. The tokenizer handles this structurally
    /// (whole-IRI / whole-string tokens), so these now parse end-to-end.
    #[test]
    fn dotted_iris_and_literals_do_not_break_bgp() {
        let two = "SELECT ?a WHERE { ?a <https://ns.webcivics.net/values/partOf> <https://ns.webcivics.net/values/x#Instrument> . \
                   ?a <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ns.webcivics.net/values/Undertaking> }";
        assert!(parse_sparql(two).is_ok(), "two dotted-IRI triples must parse");

        let lit = "SELECT ?a WHERE { ?a <https://ns.webcivics.net/values/originalText> \"Art. 3 applies.\" }";
        assert!(
            parse_sparql(lit).is_ok(),
            "a dotted literal must not break the BGP"
        );
    }

    /// End-to-end parse of a typed BGP with explicit dotted IRIs — must produce a
    /// triple pattern, not error out.
    #[test]
    fn typed_iri_bgp_parses() {
        let q = "SELECT ?a WHERE { ?a <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                 <https://ns.webcivics.net/values/Undertaking> }";
        let (_, ctx) = parse_sparql(q).expect("typed IRI BGP must parse");
        assert!(
            ctx.pattern_count > 0,
            "the typed triple pattern must be allocated"
        );
    }

    #[test]
    fn test_parse_simple_select() {
        let query = "SELECT ?s WHERE { ?s knows Bob }";
        let result = parse_sparql(query);
        assert!(result.is_ok());

        let (sparql_query, ctx) = result.unwrap();
        if let SparqlQuery::Select(select) = sparql_query {
            assert!(select.var_count > 0);
            assert!(ctx.pattern_count > 0);
        } else {
            panic!("Expected SELECT query");
        }
    }

    #[test]
    fn test_parse_distinct() {
        let query = "SELECT DISTINCT ?s WHERE { ?s knows Bob }";
        let result = parse_sparql(query);
        assert!(result.is_ok());

        let (sparql_query, _) = result.unwrap();
        if let SparqlQuery::Select(select) = sparql_query {
            assert!(select.distinct);
        } else {
            panic!("Expected SELECT query");
        }
    }

    #[test]
    fn test_parse_limit() {
        let query = "SELECT ?s WHERE { ?s knows Bob } LIMIT 10";
        let result = parse_sparql(query);
        assert!(result.is_ok());

        let (sparql_query, _) = result.unwrap();
        if let SparqlQuery::Select(select) = sparql_query {
            assert_eq!(select.limit, Some(10));
        } else {
            panic!("Expected SELECT query");
        }
    }

    #[test]
    fn test_parse_ask() {
        let query = "ASK WHERE { ?s knows Bob }";
        let result = parse_sparql(query);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_as_of_integer() {
        let query = "SELECT ?s WHERE { ?s knows Bob } AS OF 1717286400000";
        let (q, ctx) = parse_sparql(query).expect("parse failed");
        if let SparqlQuery::Select(sel) = q {
            let root = &ctx.patterns[sel.root_pattern as usize];
            match root {
                Pattern::AsOf {
                    timestamp_ms, mode, ..
                } => {
                    assert_eq!(*timestamp_ms, 1_717_286_400_000);
                    assert_eq!(*mode, TemporalMode::AsOf);
                }
                other => panic!("expected AsOf, got {:?}", other),
            }
        } else {
            panic!("expected SELECT");
        }
    }

    #[test]
    fn test_parse_as_of_iso_date() {
        // 2024-06-01 = days since epoch × 86_400_000
        let query = r#"SELECT ?s WHERE { ?s knows Bob } AS OF "2024-06-01"^^xsd:dateTime"#;
        let (q, ctx) = parse_sparql(query).expect("parse failed");
        if let SparqlQuery::Select(sel) = q {
            if let Pattern::AsOf {
                timestamp_ms, mode, ..
            } = ctx.patterns[sel.root_pattern as usize]
            {
                assert!(timestamp_ms > 0, "timestamp should be > 0");
                assert_eq!(mode, TemporalMode::AsOf);
            } else {
                panic!("expected AsOf pattern");
            }
        }
    }

    #[test]
    fn test_parse_at_time() {
        let query = "SELECT ?s WHERE { ?s knows Bob } AT TIME 9999999";
        let (q, ctx) = parse_sparql(query).expect("parse failed");
        if let SparqlQuery::Select(sel) = q {
            if let Pattern::AsOf {
                timestamp_ms, mode, ..
            } = ctx.patterns[sel.root_pattern as usize]
            {
                assert_eq!(timestamp_ms, 9_999_999);
                assert_eq!(mode, TemporalMode::AtTime);
            } else {
                panic!("expected AsOf pattern");
            }
        }
    }

    #[test]
    fn test_temporal_literal_epoch() {
        // 1970-01-01 = day 0 = ms 0
        assert_eq!(
            super::parse_temporal_literal(r#""1970-01-01"^^xsd:dateTime"#),
            0
        );
    }

    #[test]
    fn test_temporal_literal_integer_passthrough() {
        assert_eq!(super::parse_temporal_literal("42000"), 42_000);
    }

    #[test]
    fn test_parse_prefix_and_star() {
        let query = r#"
            PREFIX ex: <http://example.org/>
            SELECT ?s WHERE { << ?s ex:knows ex:bob >> ex:certainty ?c }
        "#;
        let (q, ctx) = parse_sparql(query).expect("parse");
        assert!(matches!(q, SparqlQuery::Select(_)));
        assert!(ctx.pattern_count > 0);
    }
}
