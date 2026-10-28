pub(super) use super::retention::retained_bytes;
use crate::{
    error::{DiagCode, Diagnostic},
    span::Span,
    value::Value,
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

const MAX_SCOPE_TASKS: usize = 256;
const MAX_PROCESS_TASKS: usize = 4096;
const MAX_SCOPE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PROCESS_BYTES: usize = 64 * 1024 * 1024;
static NEXT_SCOPE: AtomicU64 = AtomicU64::new(1);
static NEXT_TASK: AtomicU64 = AtomicU64::new(1);
static REGISTRY: OnceLock<Mutex<BTreeMap<u64, Entry>>> = OnceLock::new();
pub(super) fn registry() -> &'static Mutex<BTreeMap<u64, Entry>> {
    REGISTRY.get_or_init(Default::default)
}
pub(super) fn error(span: Span, message: &str) -> Diagnostic {
    Diagnostic::new(DiagCode::E400, span, message)
}

/// Opaque runtime ownership, independent of authored owner fields and human authority.
#[derive(Clone, Debug)]
pub struct TaskScope(Arc<ScopeOwner>);
#[derive(Debug)]
struct ScopeOwner {
    id: u64,
}
impl Default for TaskScope {
    fn default() -> Self {
        Self(Arc::new(ScopeOwner {
            id: NEXT_SCOPE.fetch_add(1, Ordering::Relaxed),
        }))
    }
}
impl Drop for ScopeOwner {
    fn drop(&mut self) {
        if let Ok(mut tasks) = registry().lock() {
            tasks.retain(|_, entry| entry.owner != self.id);
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    Pending,
    Running,
    Completed,
    Cancelled,
    TimedOut,
    Failed,
}
impl State {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Failed => "failed",
        }
    }
    pub(super) fn terminal(self) -> bool {
        !matches!(self, Self::Pending | Self::Running)
    }
}
#[derive(Clone)]
pub(super) struct Entry {
    pub(super) id: u64,
    owner: u64,
    pub(super) code: String,
    pub(super) state: State,
    created: Instant,
    deadline: Option<Instant>,
    pub(super) cancel_requested: bool,
    timeout_requested: bool,
    pub(super) result: Option<Value>,
    pub(super) failure: Option<String>,
    pub(super) retained_bytes: usize,
}
pub fn extract_task_id(rec: &BTreeMap<String, Value>) -> Option<u64> {
    match rec.get("task_id") {
        Some(Value::U64(id)) if *id > 0 => Some(*id),
        Some(Value::I64(id)) if *id > 0 => Some(*id as u64),
        _ => None,
    }
}
pub fn is_task_record(rec: &BTreeMap<String, Value>) -> bool {
    rec.get("__task") == Some(&Value::Bool(true))
        || extract_task_id(rec).is_some()
        || rec.contains_key("status")
}
pub(super) fn record(entry: &Entry) -> Value {
    let mut out = BTreeMap::new();
    out.insert("__task".into(), Value::Bool(true));
    out.insert("task_id".into(), Value::U64(entry.id));
    out.insert("status".into(), Value::String(entry.state.label().into()));
    out.insert(
        "cancelled".into(),
        Value::Bool(entry.state == State::Cancelled),
    );
    out.insert(
        "timed_out".into(),
        Value::Bool(entry.state == State::TimedOut),
    );
    out.insert(
        "cancel_requested".into(),
        Value::Bool(entry.cancel_requested),
    );
    out.insert(
        "cancel_acknowledged".into(),
        Value::Bool(matches!(entry.state, State::Cancelled | State::TimedOut)),
    );
    out.insert(
        "deadline_expired".into(),
        Value::Bool(entry.timeout_requested),
    );
    out.insert("scheduler_registered".into(), Value::Bool(true));
    out.insert("scheduler_contacted".into(), Value::Bool(true));
    out.insert(
        "live_job_joined".into(),
        Value::Bool(entry.state == State::Completed),
    );
    out.insert(
        "execution_qualification".into(),
        Value::String("local_deferred_synchronous_ast_task".into()),
    );
    if let Some(result) = &entry.result {
        out.insert("result".into(), result.clone());
    }
    if let Some(failure) = &entry.failure {
        out.insert("error".into(), Value::String(failure.clone()));
    }
    Value::Record(out)
}
pub(super) fn owned<'a>(
    scope: &TaskScope,
    tasks: &'a mut BTreeMap<u64, Entry>,
    value: &Value,
    span: Span,
) -> Result<&'a mut Entry, Diagnostic> {
    let Value::Record(rec) = value else {
        return Err(error(
            span,
            "task operation requires a scoped task record; integer IDs are not handles",
        ));
    };
    let id = extract_task_id(rec).ok_or_else(|| error(span, "task record has no task_id"))?;
    let entry = tasks
        .get_mut(&id)
        .ok_or_else(|| error(span, "task is unknown or its runtime scope was disposed"))?;
    if entry.owner != scope.0.id {
        return Err(error(span, "task belongs to a different runtime scope"));
    }
    Ok(entry)
}
pub(super) fn refresh_observer(entry: &mut Entry) {
    if entry.state.terminal() {
        return;
    }
    if entry
        .deadline
        .is_some_and(|deadline| Instant::now() >= deadline)
    {
        entry.timeout_requested = true;
        entry.cancel_requested = true;
    }
    // Observers cannot acknowledge a running native callback has stopped.
    if entry.state == State::Pending {
        acknowledge(entry);
    }
}
pub(super) fn acknowledge(entry: &mut Entry) {
    if entry.state.terminal() {
        return;
    }
    if entry.timeout_requested {
        entry.state = State::TimedOut;
    } else if entry.cancel_requested {
        entry.state = State::Cancelled;
    }
}
pub(super) fn executor_checkpoint(entry: &mut Entry) {
    refresh_observer(entry);
    if entry.state == State::Running {
        acknowledge(entry);
    }
}
pub(super) fn spawn(scope: &TaskScope, code: &str, span: Span) -> Result<Value, Diagnostic> {
    if code.trim().is_empty() || code.len() > 65_536 {
        return Err(error(span, "task source must contain 1..65536 UTF-8 bytes"));
    }
    let mut tasks = registry()
        .lock()
        .map_err(|_| error(span, "task registry is poisoned"))?;
    if tasks.len() >= MAX_PROCESS_TASKS
        || tasks
            .values()
            .filter(|entry| entry.owner == scope.0.id)
            .count()
            >= MAX_SCOPE_TASKS
    {
        return Err(error(
            span,
            "task registry capacity exhausted; dispose the owning environment to release its tasks",
        ));
    }
    admit_bytes(scope, &tasks, code.len(), span)?;
    let id = NEXT_TASK.fetch_add(1, Ordering::Relaxed);
    let entry = Entry {
        id,
        owner: scope.0.id,
        code: code.into(),
        state: State::Pending,
        created: Instant::now(),
        deadline: None,
        cancel_requested: false,
        timeout_requested: false,
        result: None,
        failure: None,
        retained_bytes: code.len(),
    };
    let output = record(&entry);
    tasks.insert(id, entry);
    Ok(output)
}
pub(super) fn admit_bytes(
    scope: &TaskScope,
    tasks: &BTreeMap<u64, Entry>,
    bytes: usize,
    span: Span,
) -> Result<(), Diagnostic> {
    let scope_bytes: usize = tasks
        .values()
        .filter(|entry| entry.owner == scope.0.id)
        .map(|entry| entry.retained_bytes)
        .sum();
    let process_bytes: usize = tasks.values().map(|entry| entry.retained_bytes).sum();
    if bytes > MAX_SCOPE_BYTES.saturating_sub(scope_bytes)
        || bytes > MAX_PROCESS_BYTES.saturating_sub(process_bytes)
    {
        return Err(error(
            span,
            "task retention budget exhausted (8 MiB per scope / 64 MiB per process)",
        ));
    }
    Ok(())
}
pub(super) fn status(scope: &TaskScope, value: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut tasks = registry()
        .lock()
        .map_err(|_| error(span, "task registry is poisoned"))?;
    let entry = owned(scope, &mut tasks, value, span)?;
    refresh_observer(entry);
    Ok(record(entry))
}
pub(super) fn cancel(scope: &TaskScope, value: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut tasks = registry()
        .lock()
        .map_err(|_| error(span, "task registry is poisoned"))?;
    let entry = owned(scope, &mut tasks, value, span)?;
    refresh_observer(entry);
    let terminal = entry.state.terminal();
    if !terminal {
        entry.cancel_requested = true;
        if entry.state == State::Pending {
            entry.state = State::Cancelled;
        }
    }
    let Value::Record(mut out) = record(entry) else {
        unreachable!()
    };
    out.insert(
        "already_committed".into(),
        Value::Bool(entry.state == State::Completed),
    );
    out.insert("cancel_requested".into(), Value::Bool(true));
    out.insert(
        "cancellation_request_accepted".into(),
        Value::Bool(!terminal),
    );
    out.insert(
        "live_job_cancelled".into(),
        Value::Bool(!terminal && entry.state == State::Cancelled),
    );
    Ok(Value::Record(out))
}
pub(super) fn timeout(
    scope: &TaskScope,
    value: &Value,
    ms: i64,
    span: Span,
) -> Result<Value, Diagnostic> {
    let mut tasks = registry()
        .lock()
        .map_err(|_| error(span, "task registry is poisoned"))?;
    let entry = owned(scope, &mut tasks, value, span)?;
    if !entry.state.terminal() {
        let deadline = entry
            .created
            .checked_add(Duration::from_millis(ms.max(0) as u64))
            .ok_or_else(|| error(span, "task timeout exceeds the monotonic clock range"))?;
        entry.deadline = Some(entry.deadline.map_or(deadline, |old| old.min(deadline)));
        refresh_observer(entry);
    }
    Ok(record(entry))
}
pub(crate) fn checkpoint(scope: &TaskScope, id: u64, span: Span) -> Result<(), Diagnostic> {
    let mut tasks = registry()
        .lock()
        .map_err(|_| error(span, "task registry is poisoned"))?;
    let entry = tasks
        .get_mut(&id)
        .filter(|entry| entry.owner == scope.0.id)
        .ok_or_else(|| error(span, "active task lost its runtime scope"))?;
    executor_checkpoint(entry);
    if entry.state != State::Running {
        return Err(error(
            span,
            "task cancelled or deadline expired; prior external effects may require reconciliation",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskRegistrySummary {
    pub total: u64,
    pub pending: u64,
    pub running: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub timed_out: u64,
    pub failed: u64,
}
pub fn registry_summary() -> TaskRegistrySummary {
    let mut summary = TaskRegistrySummary::default();
    if let Ok(tasks) = registry().lock() {
        summary.total = tasks.len() as u64;
        for entry in tasks.values() {
            match entry.state {
                State::Pending => summary.pending += 1,
                State::Running => summary.running += 1,
                State::Completed => summary.completed += 1,
                State::Cancelled => summary.cancelled += 1,
                State::TimedOut => summary.timed_out += 1,
                State::Failed => summary.failed += 1,
            }
        }
    }
    summary
}
