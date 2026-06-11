//! Shared N-Triples pattern execution for HTTP `/query` and WS `/qualia-bridge`.

use crate::webizen_bytecode::{self, ExecutionStats};
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

/// Compile and run a single N-Triples pattern against `graph`.
/// Returns execution stats and matched quins (for HTTP serialisation).
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
    let stats = webizen_bytecode::execute_program_with_stats(&program, graph, &mut out_buffer)
        .map_err(|e| match e {
            webizen_bytecode::VmError::OutputBufferFull => QueryExecError::OutputBufferFull,
            webizen_bytecode::VmError::InvalidProgram => QueryExecError::InvalidProgram,
        })?;

    let results = out_buffer[..stats.match_count].to_vec();
    for quin in &results {
        if quin.get_sensitivity_byte() == NQuin::SENSITIVITY_CLASSIFIED {
            return Err(QueryExecError::ClassifiedEgress);
        }
    }

    Ok((stats, results))
}

/// Metrics-only path for WebSocket benchmarks (no result serialisation).
pub fn execute_ntriples_metrics(
    query: &str,
    graph: &[NQuin],
) -> Result<ExecutionStats, QueryExecError> {
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
    let stats = webizen_bytecode::execute_program_with_stats(&program, graph, &mut out_buffer)
        .map_err(|e| match e {
            webizen_bytecode::VmError::OutputBufferFull => QueryExecError::OutputBufferFull,
            webizen_bytecode::VmError::InvalidProgram => QueryExecError::InvalidProgram,
        })?;

    for quin in &out_buffer[..stats.match_count] {
        if quin.get_sensitivity_byte() == NQuin::SENSITIVITY_CLASSIFIED {
            return Err(QueryExecError::ClassifiedEgress);
        }
    }

    Ok(stats)
}
