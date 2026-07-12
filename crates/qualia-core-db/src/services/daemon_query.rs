//! Shared query execution for HTTP `/query`, MCP `query_sparql`, and WS `/qualia-bridge`.
//!
//! Routes full SPARQL (SELECT/ASK/PREFIX/RDF-Star) through `sparql_library`;
//! simple N-Triples patterns continue to use the bytecode fast path.

use crate::lexicon::generate_embedded_triple_id;

use crate::sparql_ast::{BindingRow, SparqlQuery};
use crate::sparql_executor::QueryExecutor;
use crate::sparql_parser;
use crate::sparql_planner::QueryPlanner;
use crate::webizen_bytecode::ExecutionStats;
use crate::NQuin;

pub const QUERY_OUT_SLOTS: usize = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryExecError {
    EmptyQuery,
    ParseError(String),
    OutputBufferFull,
    InvalidProgram,
    ClassifiedEgress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryEngine {
    NTriplesPattern,
    SparqlLibrary,
}

#[derive(Debug, Clone)]
pub struct SparqlQueryStats {
    pub binding_count: usize,
    pub ask_result: Option<bool>,
}

/// Detect whether a query should use the SPARQL library instead of the bytecode VM.
pub fn detect_query_engine(query: &str) -> QueryEngine {
    let trimmed = query.trim();
    let upper = trimmed.to_ascii_uppercase();
    if upper.starts_with("SELECT")
        || upper.starts_with("ASK")
        || upper.starts_with("CONSTRUCT")
        || upper.starts_with("DESCRIBE")
        || upper.starts_with("PREFIX")
        || trimmed.contains("<<")
    {
        QueryEngine::SparqlLibrary
    } else {
        QueryEngine::NTriplesPattern
    }
}

/// Unified graph query entry point.
pub fn execute_query_on_graph(
    query: &str,
    graph: &[NQuin],
) -> Result<(ExecutionStats, Vec<NQuin>), QueryExecError> {
    match detect_query_engine(query) {
        QueryEngine::NTriplesPattern => execute_ntriples_pattern_on_graph(query, graph),
        QueryEngine::SparqlLibrary => execute_sparql_on_graph(query, graph),
    }
}

/// Compile and run a single N-Triples pattern against `graph`.
pub fn execute_ntriples_pattern_on_graph(
    query: &str,
    graph: &[NQuin],
) -> Result<(ExecutionStats, Vec<NQuin>), QueryExecError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(QueryExecError::EmptyQuery);
    }

    let mut program = [0u8; 1024];
    if let Err(parse_err) =
        crate::mini_parser::compile_ntriples_to_bytecode(trimmed.as_bytes(), &mut program)
    {
        return Err(QueryExecError::ParseError(format!("{parse_err:?}")));
    }

    let mut out_buffer = vec![NQuin::default(); QUERY_OUT_SLOTS];
    let stats =
        crate::webizen_bytecode::execute_program_with_stats(&program, graph, &mut out_buffer, None)
            .map_err(|e| match e {
                crate::webizen_bytecode::VmError::OutputBufferFull => {
                    QueryExecError::OutputBufferFull
                }
                crate::webizen_bytecode::VmError::InvalidProgram => QueryExecError::InvalidProgram,
                crate::webizen_bytecode::VmError::HaltViolation => QueryExecError::InvalidProgram,
            })?;

    let results = out_buffer[..stats.match_count].to_vec();
    filter_classified(results, stats)
}

/// Execute SPARQL SELECT/ASK via the library planner + executor.
pub fn execute_sparql_on_graph(
    query: &str,
    graph: &[NQuin],
) -> Result<(ExecutionStats, Vec<NQuin>), QueryExecError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(QueryExecError::EmptyQuery);
    }

    let (sparql_query, ctx) =
        sparql_parser::parse_sparql(trimmed).map_err(|e| QueryExecError::ParseError(e))?;
    let plan =
        QueryPlanner::plan(&sparql_query, &ctx).map_err(|e| QueryExecError::ParseError(e))?;
    let executor = QueryExecutor::new(graph);

    let stats = ExecutionStats {
        match_count: 0,
        vm_cycles: 0,
        direct_jump_ops: 0,
        lexicon_lookup_ops: 0,
    };

    match &sparql_query {
        SparqlQuery::Ask(_) => {
            let ok = executor
                .execute_ask(&plan, &ctx)
                .map_err(|e| QueryExecError::ParseError(e))?;
            let mut results = Vec::new();
            if ok {
                results.push(synthetic_binding_quin(1, 0, 0));
            }
            let mut out_stats = stats;
            out_stats.match_count = results.len();
            filter_classified(results, out_stats)
        }
        _ => {
            let bindings = executor
                .execute(&plan, &ctx)
                .map_err(|e| QueryExecError::ParseError(e))?;
            let results = bindings_to_quins(&bindings);
            let mut out_stats = stats;
            out_stats.match_count = results.len();
            filter_classified(results, out_stats)
        }
    }
}

/// Metrics-only path for WebSocket benchmarks (no result serialisation).
pub fn execute_ntriples_metrics(
    query: &str,
    graph: &[NQuin],
) -> Result<ExecutionStats, QueryExecError> {
    let (stats, _) = execute_query_on_graph(query, graph)?;
    Ok(stats)
}

fn bindings_to_quins(bindings: &[BindingRow]) -> Vec<NQuin> {
    bindings
        .iter()
        .map(|row| {
            synthetic_binding_quin(
                row.slots[0].unwrap_or(0),
                row.slots[1].unwrap_or(0),
                row.slots[2].unwrap_or(0),
            )
        })
        .collect()
}

fn synthetic_binding_quin(subject: u64, predicate: u64, object: u64) -> NQuin {
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

fn filter_classified(
    results: Vec<NQuin>,
    stats: ExecutionStats,
) -> Result<(ExecutionStats, Vec<NQuin>), QueryExecError> {
    for quin in &results {
        if quin.get_sensitivity_byte() == NQuin::SENSITIVITY_CLASSIFIED {
            return Err(QueryExecError::ClassifiedEgress);
        }
    }
    Ok((stats, results))
}

/// Build RDF-Star annotation quins for tests: `<<s p o>> ann pred val`.
pub fn make_star_annotation(
    subject: u64,
    predicate: u64,
    object: u64,
    ann_predicate: u64,
    ann_object: u64,
) -> [NQuin; 2] {
    let base = synthetic_binding_quin(subject, predicate, object);
    let virtual_id = generate_embedded_triple_id(subject, predicate, object);
    let annotation = synthetic_binding_quin(virtual_id, ann_predicate, ann_object);
    [base, annotation]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn detects_sparql_select_engine() {
        assert_eq!(
            detect_query_engine("SELECT ?s WHERE { ?s ?p ?o }"),
            QueryEngine::SparqlLibrary
        );
    }

    #[test]
    fn detects_ntriples_pattern_engine() {
        assert_eq!(
            detect_query_engine("<s> <p> <o> ."),
            QueryEngine::NTriplesPattern
        );
    }

    #[test]
    fn sparql_select_returns_bindings() {
        let s = q_hash("alice");
        let p = q_hash("knows");
        let o = q_hash("bob");
        let graph = vec![synthetic_binding_quin(s, p, o)];
        let query = "SELECT ?x WHERE { ?x ?p ?o }";
        let (_, results) = execute_sparql_on_graph(query, &graph).expect("sparql");
        assert!(!results.is_empty());
        assert_eq!(results[0].subject, s);
    }

    #[test]
    fn sparql_union_combines_both_branches() {
        // End-to-end: `{ … } UNION { … }` now parses (grammar slice 2) and the
        // executor's Union operator returns rows from both branches.
        let s_alice = q_hash("alice");
        let s_bob = q_hash("bob");
        let p_knows = crate::lexicon::generate_60bit_token(b"knows");
        let p_likes = crate::lexicon::generate_60bit_token(b"likes");
        let o = q_hash("carol");
        let graph = vec![
            synthetic_binding_quin(s_alice, p_knows, o),
            synthetic_binding_quin(s_bob, p_likes, o),
        ];
        let query =
            "SELECT ?s WHERE { { ?s <knows> ?o } UNION { ?s <likes> ?o } }";
        let (_, results) = execute_sparql_on_graph(query, &graph).expect("union query");
        assert_eq!(results.len(), 2, "UNION should return rows from both branches");
    }

    #[test]
    fn sparql_filter_numeric_prunes_rows() {
        // End-to-end: a real FILTER clause is now parsed (grammar slice 1) and
        // evaluated by the existing executor. alice(age 20) passes `?o >= 18`,
        // bob(age 10) is pruned.
        let s_alice = q_hash("alice");
        let s_bob = q_hash("bob");
        let pred = crate::lexicon::generate_60bit_token(b"age");
        let graph = vec![
            synthetic_binding_quin(s_alice, pred, 20),
            synthetic_binding_quin(s_bob, pred, 10),
        ];
        let query = "SELECT ?s WHERE { ?s <age> ?o . FILTER(?o >= 18) }";
        let (_, results) = execute_sparql_on_graph(query, &graph).expect("filter query");
        assert_eq!(results.len(), 1, "FILTER should prune bob (age 10)");
    }

    #[test]
    fn sparql_star_annotation_query() {
        let s = q_hash("alice");
        let p = q_hash("knows");
        let o = q_hash("bob");
        let ann_p = q_hash("certainty");
        let ann_o = 95;
        let quins = make_star_annotation(s, p, o, ann_p, ann_o);
        let query = "SELECT ?innerS ?val WHERE { << ?innerS ?innerP ?innerO >> ?annP ?val }";
        let (_, results) = execute_sparql_on_graph(query, &quins).expect("star query");
        assert!(!results.is_empty());
        assert_eq!(results[0].subject, s);
        assert_eq!(results[0].predicate, ann_o);
    }
}
