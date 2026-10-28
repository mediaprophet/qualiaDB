//! Bounded local deferred tasks. Execution is synchronous on the admitted AST engine.
//!
//! No remote execution, continuation suspension or human delegation is implied.
//! TaskScope is runtime state; record fields never confer ownership. Env clones share
//! the scope (including committed effects); independently defaulted environments do not.
#[path = "task_scheduler/execution.rs"]
mod execution;
#[cfg(test)]
#[path = "task_scheduler/host_tests.rs"]
mod host_tests;
#[path = "task_scheduler/registry.rs"]
mod registry;
#[path = "task_scheduler/retention.rs"]
mod retention;
#[cfg(test)]
#[path = "task_scheduler/tests.rs"]
mod tests;

pub use execution::await_scheduled_value;
pub(crate) use execution::dispatch_on_engine;
pub(crate) use registry::checkpoint;
pub use registry::{
    extract_task_id, is_task_record, registry_summary, TaskRegistrySummary, TaskScope,
};

use crate::{
    error::{DiagCode, Diagnostic},
    span::Span,
    value::Value,
};
use std::collections::BTreeMap;

fn unavailable(span: Span) -> Diagnostic {
    Diagnostic::new(DiagCode::E400, span,
        "task operation requires an admitted AST Engine and runtime TaskScope; standalone/bytecode execution is unsupported")
}

/// Legacy facade: callers without an admitted runtime scope fail before execution.
pub fn spawn_scheduled_task(
    _: &str,
    _: &str,
    _: Value,
    _: i64,
    span: Span,
) -> Result<Value, Diagnostic> {
    Err(unavailable(span))
}
pub fn cancel_scheduled_task(_: &BTreeMap<String, Value>, span: Span) -> Result<Value, Diagnostic> {
    Err(unavailable(span))
}
pub fn apply_task_timeout(
    _: &BTreeMap<String, Value>,
    _: i64,
    span: Span,
) -> Result<BTreeMap<String, Value>, Diagnostic> {
    Err(unavailable(span))
}
pub fn query_task_status(_: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(unavailable(span))
}
pub fn await_value_standalone(value: Value, span: Span) -> Result<Value, Diagnostic> {
    match value {
        Value::Record(ref record) if is_task_record(record) => Err(unavailable(span)),
        Value::Lambda { .. } => Err(unavailable(span)),
        other => Ok(other),
    }
}
