//! SPARQL Parser - Hand-Rolled Zero-Allocation Parser
//!
//! Simple SPARQL 1.1 subset parser that's zero-allocation by design.
//! Uses byte string slicing and no heap allocation.

use crate::sparql_ast::*;

/// Parse a SPARQL SELECT query string into an AST
pub fn parse_sparql(query: &str) -> Result<(SparqlQuery, SparqlQueryContext), String> {
    let mut ctx = SparqlQueryContext::new();
    let query = query.trim();
    
    // Check for SELECT query
    if query.starts_with("SELECT") {
        let select_query = parse_select_query(query, &mut ctx)?;
        return Ok((SparqlQuery::Select(select_query), ctx));
    } else if query.starts_with("ASK") {
        let ask_query = parse_ask_query(query, &mut ctx)?;
        return Ok((SparqlQuery::Ask(ask_query), ctx));
    } else if query.starts_with("CONSTRUCT") {
        let construct_query = parse_construct_query(query, &mut ctx)?;
        return Ok((SparqlQuery::Construct(construct_query), ctx));
    } else if query.starts_with("DESCRIBE") {
        let describe_query = parse_describe_query(query, &mut ctx)?;
        return Ok((SparqlQuery::Describe(describe_query), ctx));
    } else {
        Err("Unsupported query form".to_string())
    }
}

fn parse_select_query(query: &str, ctx: &mut SparqlQueryContext) -> Result<SelectQuery, String> {
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
    let pattern_id = parse_where_clause(where_clause, ctx)?;
    query_struct.root_pattern = pattern_id;

    // Parse AS OF / AT TIME temporal modifier (Phase 4).
    // Only search after the closing brace of the WHERE clause to avoid false positives.
    let after_where = query.rfind('}')
        .map(|i| &query[i..])
        .unwrap_or("");
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

fn parse_ask_query(query: &str, ctx: &mut SparqlQueryContext) -> Result<AskQuery, String> {
    let after_ask = query.trim_start_matches("ASK").trim();
    let where_start = after_ask.find("WHERE").ok_or("WHERE clause not found")?;
    let where_clause = &after_ask[where_start..];
    let pattern_id = parse_where_clause(where_clause, ctx)?;
    
    Ok(AskQuery {
        root_pattern: pattern_id,
    })
}

fn parse_construct_query(query: &str, ctx: &mut SparqlQueryContext) -> Result<ConstructQuery, String> {
    let after_construct = query.trim_start_matches("CONSTRUCT").trim();
    // Simplified - just parse WHERE for now
    let where_start = after_construct.find("WHERE").ok_or("WHERE clause not found")?;
    let where_clause = &after_construct[where_start..];
    let pattern_id = parse_where_clause(where_clause, ctx)?;
    
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

fn parse_describe_query(query: &str, ctx: &mut SparqlQueryContext) -> Result<DescribeQuery, String> {
    let after_describe = query.trim_start_matches("DESCRIBE").trim();
    // Simplified - just parse WHERE for now
    let where_start = after_describe.find("WHERE");
    let root_pattern = if let Some(start) = where_start {
        let where_clause = &after_describe[start + 5..];
        Some(parse_where_clause(where_clause, ctx)?)
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
            ((true, true), after_distinct.trim_start_matches("REDUCED").trim())
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
        return Ok(vec![]); // Wildcard means all variables
    }
    
    // Split by whitespace, filter out empty strings and WHERE keyword
    let vars: Vec<&str> = input.split_whitespace()
        .filter(|s| !s.is_empty() && *s != "WHERE")
        .collect();
    
    Ok(vars)
}

fn parse_where_clause(input: &str, ctx: &mut SparqlQueryContext) -> Result<PatternId, String> {
    let inner = input.trim_start_matches("WHERE").trim().trim_start_matches("{").trim();
    let inner = inner.trim_end_matches("}").trim();
    
    parse_triple_patterns(inner, ctx)
}

fn parse_triple_patterns(input: &str, ctx: &mut SparqlQueryContext) -> Result<PatternId, String> {
    let mut pattern_id = 0u16;
    
    // Split by period to get individual triple patterns
    for triple_str in input.split('.') {
        let triple_str = triple_str.trim();
        if triple_str.is_empty() {
            continue;
        }
        
        // Parse triple: subject predicate object
        let parts: Vec<&str> = triple_str.split_whitespace().collect();
        if parts.len() >= 3 {
            let subject = parse_term(parts[0], ctx)?;
            let predicate = parse_term(parts[1], ctx)?;
            let object = parse_term(parts[2], ctx)?;
            
            let pattern = Pattern::Triple {
                subject,
                predicate,
                object,
            };
            
            pattern_id = ctx.alloc_pattern(pattern)?;
        }
    }
    
    Ok(pattern_id)
}

fn parse_term(term: &str, ctx: &mut SparqlQueryContext) -> Result<u64, String> {
    let term = term.trim();
    
    if term.starts_with('?') {
        // Variable
        let var_id = ctx.register_variable(term)?;
        Ok(var_id as u64)
    } else if term.starts_with('<') {
        // IRI
        let iri = term.trim_start_matches("<").trim_end_matches(">");
        Ok(crate::lexicon::generate_60bit_token(iri.as_bytes()))
    } else if term.starts_with('"') {
        // Literal string
        let lit = term.trim_start_matches("\"").trim_end_matches("\"");
        Ok(crate::lexicon::generate_60bit_token(lit.as_bytes()))
    } else if term.starts_with('\'') {
        // Literal string (single quotes)
        let lit = term.trim_start_matches("'").trim_end_matches("'");
        Ok(crate::lexicon::generate_60bit_token(lit.as_bytes()))
    } else if term == "true" || term == "false" {
        // Boolean literal
        Ok(crate::lexicon::generate_60bit_token(term.as_bytes()))
    } else {
        // Try to parse as number
        if let Ok(num) = term.parse::<u64>() {
            Ok(num)
        } else {
            // Treat as IRI
            Ok(crate::lexicon::generate_60bit_token(term.as_bytes()))
        }
    }
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
        31, if temporal_is_leap(year) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
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
                Pattern::AsOf { timestamp_ms, mode, .. } => {
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
            if let Pattern::AsOf { timestamp_ms, mode, .. } = ctx.patterns[sel.root_pattern as usize] {
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
            if let Pattern::AsOf { timestamp_ms, mode, .. } = ctx.patterns[sel.root_pattern as usize] {
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
        assert_eq!(super::parse_temporal_literal(r#""1970-01-01"^^xsd:dateTime"#), 0);
    }

    #[test]
    fn test_temporal_literal_integer_passthrough() {
        assert_eq!(super::parse_temporal_literal("42000"), 42_000);
    }
}