//! SPARQL 1.1 Update parser.
//!
//! Parses `INSERT DATA`, `DELETE DATA`, `DELETE … INSERT … WHERE`, `CLEAR`,
//! `CREATE`, `DROP`, and `LOAD` into the `UpdateOperation` the existing
//! `UpdateExecutor` runs. Parsing is non-destructive; the caller decides whether
//! and how to apply the operation (the mutation path is governed — signed WAL,
//! scope-respecting — see the daemon's update handler).
//!
//! `INSERT DATA`/`DELETE DATA` carry concrete ground triples (no variables),
//! packed into the operation's fixed `[NQuin; 64]` array. `DELETE/INSERT WHERE`
//! carry `PatternId`s built via the group-pattern parser.

use std::collections::HashMap;

use crate::sparql_ast::SparqlQueryContext;
use crate::sparql_library::sparql_grammar::pattern::parse_where_group;
use crate::sparql_library::sparql_grammar::tokenizer::{tokenize, Token};
use crate::sparql_library::sparql_update::UpdateOperation;
use crate::NQuin;

/// Detect whether a query string is a SPARQL Update request (vs a read query).
pub fn is_update(query: &str) -> bool {
    let up = query.trim_start().to_ascii_uppercase();
    up.starts_with("INSERT")
        || up.starts_with("DELETE")
        || up.starts_with("CLEAR")
        || up.starts_with("CREATE")
        || up.starts_with("DROP")
        || up.starts_with("LOAD")
}

/// Parse a single SPARQL Update operation. (Multi-operation requests separated
/// by `;` are not handled here — the caller may split on top-level `;`.)
pub fn parse_update(
    input: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<UpdateOperation, String> {
    let trimmed = input.trim();
    let up = trimmed.to_ascii_uppercase();

    if up.starts_with("INSERT DATA") {
        let body = brace_body(trimmed, "INSERT DATA")?;
        let (quins, quin_count) = parse_ground_triples(&body, prefixes)?;
        Ok(UpdateOperation::InsertData { quins, quin_count })
    } else if up.starts_with("DELETE DATA") {
        let body = brace_body(trimmed, "DELETE DATA")?;
        let (quins, quin_count) = parse_ground_triples(&body, prefixes)?;
        Ok(UpdateOperation::DeleteData { quins, quin_count })
    } else if up.starts_with("DELETE") && up.contains("WHERE") {
        parse_delete_insert(trimmed, ctx, prefixes)
    } else if up.starts_with("INSERT") && up.contains("WHERE") {
        parse_delete_insert(trimmed, ctx, prefixes)
    } else if up.starts_with("CLEAR") {
        Ok(UpdateOperation::Clear {
            graph: parse_graph_ref(trimmed, "CLEAR", prefixes),
        })
    } else if up.starts_with("CREATE") {
        Ok(UpdateOperation::Create {
            graph: parse_graph_ref(trimmed, "CREATE", prefixes),
        })
    } else if up.starts_with("DROP") {
        Ok(UpdateOperation::Drop {
            graph: parse_graph_ref(trimmed, "DROP", prefixes),
        })
    } else if up.starts_with("LOAD") {
        parse_load(trimmed, prefixes)
    } else {
        Err("unrecognised SPARQL Update operation".to_string())
    }
}

/// Extract the `{ … }` body after a keyword prefix (matching balanced braces).
fn brace_body(input: &str, keyword: &str) -> Result<String, String> {
    let after = input[keyword.len()..].trim_start();
    let open = after
        .find('{')
        .ok_or_else(|| format!("expected '{{' after {keyword}"))?;
    let bytes = after.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    let mut end = None;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let end = end.ok_or_else(|| format!("unbalanced braces after {keyword}"))?;
    Ok(after[open + 1..end].to_string())
}

/// Parse concrete ground triples (`s p o .` …) into a fixed `[NQuin; 64]`.
/// Variables are rejected — DATA blocks must be ground.
fn parse_ground_triples(
    body: &str,
    prefixes: &HashMap<String, String>,
) -> Result<([NQuin; 64], u8), String> {
    let tokens = tokenize(body)?;
    let mut quins = [NQuin::default(); 64];
    let mut count = 0usize;
    let mut i = 0usize;

    // Read terms three at a time, skipping `.` separators.
    let mut terms: Vec<u64> = Vec::new();
    while i < tokens.len() {
        match &tokens[i] {
            Token::Punct('.') => {
                i += 1;
            }
            Token::Var(_) => {
                return Err("INSERT/DELETE DATA must be ground (no variables)".to_string());
            }
            tok => {
                terms.push(term_hash(tok, prefixes)?);
                i += 1;
                if terms.len() == 3 {
                    if count >= 64 {
                        return Err("too many triples in a DATA block (max 64)".to_string());
                    }
                    quins[count] = make_quin(terms[0], terms[1], terms[2]);
                    count += 1;
                    terms.clear();
                }
            }
        }
    }
    if !terms.is_empty() {
        return Err("trailing incomplete triple in DATA block".to_string());
    }
    if count == 0 {
        return Err("DATA block contains no triples".to_string());
    }
    Ok((quins, count as u8))
}

fn parse_delete_insert(
    input: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<UpdateOperation, String> {
    // Extract optional DELETE { … }, optional INSERT { … }, and WHERE { … }.
    let delete_pattern = if let Some(pos) = ci_find(input, "DELETE") {
        // Only the DELETE template (before INSERT/WHERE).
        let body = brace_body(&input[pos..], "DELETE")?;
        Some(parse_group_text(&body, ctx, prefixes)?)
    } else {
        None
    };
    let insert_pattern = if let Some(pos) = ci_find(input, "INSERT") {
        let body = brace_body(&input[pos..], "INSERT")?;
        Some(parse_group_text(&body, ctx, prefixes)?)
    } else {
        None
    };
    let where_pos = ci_find(input, "WHERE").ok_or("DELETE/INSERT requires a WHERE clause")?;
    let where_body = brace_body(&input[where_pos..], "WHERE")?;
    let where_pattern = parse_group_text(&where_body, ctx, prefixes)?;

    Ok(UpdateOperation::DeleteInsert {
        delete_pattern: delete_pattern.unwrap_or(where_pattern),
        insert_pattern: insert_pattern.unwrap_or(where_pattern),
        where_pattern,
    })
}

/// Parse a `{ … }`-less group body by wrapping it back in braces for the group
/// parser.
fn parse_group_text(
    body: &str,
    ctx: &mut SparqlQueryContext,
    prefixes: &HashMap<String, String>,
) -> Result<crate::sparql_ast::PatternId, String> {
    let wrapped = format!("{{ {body} }}");
    parse_where_group(&wrapped, ctx, prefixes)
}

fn parse_graph_ref(input: &str, keyword: &str, prefixes: &HashMap<String, String>) -> u64 {
    // `CLEAR [SILENT] [GRAPH] <iri>` / `CLEAR DEFAULT|ALL`. Default graph → 0.
    let rest = input[keyword.len()..].trim();
    let rest_up = rest.to_ascii_uppercase();
    if rest_up.starts_with("DEFAULT") || rest_up.starts_with("ALL") || rest.is_empty() {
        return 0;
    }
    if let Ok(tokens) = tokenize(rest) {
        for t in &tokens {
            if let Token::Iri(_) | Token::Prefixed(_, _) = t {
                if let Ok(h) = term_hash(t, prefixes) {
                    return h;
                }
            }
        }
    }
    0
}

fn parse_load(input: &str, prefixes: &HashMap<String, String>) -> Result<UpdateOperation, String> {
    let tokens = tokenize(&input[4..])?;
    let mut uri = 0u64;
    let mut graph = 0u64;
    let mut seen_into = false;
    for t in &tokens {
        match t {
            Token::Word(w) if w.eq_ignore_ascii_case("INTO") => seen_into = true,
            Token::Iri(_) | Token::Prefixed(_, _) => {
                let h = term_hash(t, prefixes)?;
                if seen_into {
                    graph = h;
                } else {
                    uri = h;
                }
            }
            _ => {}
        }
    }
    Ok(UpdateOperation::Load { uri, graph })
}

/// Case-insensitive substring search returning the byte index of the keyword.
fn ci_find(haystack: &str, needle: &str) -> Option<usize> {
    let h = haystack.to_ascii_uppercase();
    h.find(&needle.to_ascii_uppercase())
}

fn term_hash(tok: &Token, prefixes: &HashMap<String, String>) -> Result<u64, String> {
    match tok {
        Token::Iri(iri) => Ok(crate::lexicon::generate_60bit_token(iri.as_bytes())),
        Token::Prefixed(prefix, local) => {
            let expanded = match prefixes.get(prefix) {
                Some(base) => format!("{base}{local}"),
                None => format!("{prefix}:{local}"),
            };
            Ok(crate::lexicon::generate_60bit_token(expanded.as_bytes()))
        }
        Token::Str { value, .. } => Ok(crate::lexicon::generate_60bit_token(value.as_bytes())),
        Token::Num(text) => Ok(text
            .parse::<u64>()
            .unwrap_or_else(|_| crate::lexicon::generate_60bit_token(text.as_bytes()))),
        Token::Bool(b) => Ok(crate::lexicon::generate_60bit_token(if *b {
            b"true"
        } else {
            b"false"
        })),
        Token::Word(w) if w == "a" => Ok(crate::lexicon::generate_60bit_token(
            b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
        )),
        Token::Word(w) => Ok(crate::lexicon::generate_60bit_token(w.as_bytes())),
        other => Err(format!("invalid term in update: {other:?}")),
    }
}

fn make_quin(subject: u64, predicate: u64, object: u64) -> NQuin {
    let mut q = NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    q
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> SparqlQueryContext {
        SparqlQueryContext::new()
    }

    #[test]
    fn detects_update_vs_query() {
        assert!(is_update("INSERT DATA { <a> <b> <c> }"));
        assert!(is_update("DELETE WHERE { ?s ?p ?o }"));
        assert!(!is_update("SELECT ?s WHERE { ?s ?p ?o }"));
    }

    #[test]
    fn parses_insert_data() {
        let mut c = ctx();
        let op = parse_update(
            "INSERT DATA { <http://a> <http://b> <http://c> . <http://d> <http://e> <http://f> }",
            &mut c,
            &HashMap::new(),
        )
        .unwrap();
        match op {
            UpdateOperation::InsertData { quin_count, quins } => {
                assert_eq!(quin_count, 2);
                assert_ne!(quins[0].subject, 0);
                assert_eq!(
                    quins[0].parity,
                    quins[0].subject ^ quins[0].predicate ^ quins[0].object
                );
            }
            other => panic!("expected InsertData, got {other:?}"),
        }
    }

    #[test]
    fn parses_delete_data() {
        let mut c = ctx();
        let op = parse_update(
            "DELETE DATA { <http://a> <http://b> <http://c> }",
            &mut c,
            &HashMap::new(),
        )
        .unwrap();
        assert!(matches!(
            op,
            UpdateOperation::DeleteData { quin_count: 1, .. }
        ));
    }

    #[test]
    fn insert_data_rejects_variables() {
        let mut c = ctx();
        let err = parse_update("INSERT DATA { ?s <b> <c> }", &mut c, &HashMap::new()).unwrap_err();
        assert!(err.contains("ground"), "got {err}");
    }

    #[test]
    fn parses_delete_insert_where() {
        let mut c = ctx();
        let op = parse_update(
            "DELETE { ?s <http://old> ?o } INSERT { ?s <http://new> ?o } WHERE { ?s <http://old> ?o }",
            &mut c,
            &HashMap::new(),
        )
        .unwrap();
        assert!(matches!(op, UpdateOperation::DeleteInsert { .. }));
    }

    #[test]
    fn parses_clear_and_drop() {
        let mut c = ctx();
        assert!(matches!(
            parse_update("CLEAR GRAPH <http://g>", &mut c, &HashMap::new()).unwrap(),
            UpdateOperation::Clear { .. }
        ));
        assert!(matches!(
            parse_update("DROP GRAPH <http://g>", &mut c, &HashMap::new()).unwrap(),
            UpdateOperation::Drop { .. }
        ));
        assert!(matches!(
            parse_update("CLEAR DEFAULT", &mut c, &HashMap::new()).unwrap(),
            UpdateOperation::Clear { graph: 0 }
        ));
    }
}
