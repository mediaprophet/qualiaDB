//! Live Task Scheduler & Cooperative Cancellation Registry for VibeScript (`vibe-0.1` / `vibe-0.2`).
//!
//! Connects `ExprKind::Await`, `code.spawn_at`, `task.spawn`, `task.cancel`, `task.nursery`,
//! `task.with_timeout`, and `task.status` to a thread-safe task table with:
//! - Owner-validated 256-bit SHA-256 capability seals (`__task_seal` & `owner_scope`),
//! - Parent `Engine` (`Host`, `Budget`, `PhaseLeaser`, `Env`) preservation on `await`,
//! - Single-mode parsing with strict error propagation (no fallback retry or caller-result trust),
//! - Immutable terminal task states (`Completed`, `Cancelled`, `TimedOut`, `Failed`) and monotonic
//!   runtime deadlines, distinguirhing `cancel_requested` from `cancel_acknowledged`.

use crate::bind::{Host, LocalHost};
use crate::error::{DiagCode, Diagnostic};
use crate::eval::{Engine, Env};
use crate::parse::{parse_cell, parse_program};
use crate::span::Span;
use crate::value::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

/// Lifecycle state of a scheduled task in [`TaskRegistry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskLifecycleState {
    Pending,
    Running,
    Completed,
    Cancelled,
    TimedOut,
    Failed,
}

impl TaskLifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
            Self::Failed => "failed",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::TimedOut | Self::Failed
        )
    }
}

/// Live task record managed by the process-wide [`TaskRegistry`].
#[derive(Debug, Clone)]
pub struct ScheduledTaskEntry {
    pub task_id: u64,
    pub owner_scope: String,
    pub capability_seal: String,
    pub target_node: String,
    pub code_or_hash: String,
    pub args: Value,
    pub state: TaskLifecycleState,
    pub cancel_token: Arc<AtomicBool>,
    pub created_at_inst: Instant,
    pub elapsed_ms: i64,
    pub timeout_ms: Option<i64>,
    pub result: Option<Value>,
    pub error: Option<String>,
}

/// Summary of task counts across lifecycle states in [`TaskRegistry`].
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

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);
static SEAL_NONCE: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);
static REGISTRY: OnceLock<Mutex<BTreeMap<u64, ScheduledTaskEntry>>> = OnceLock::new();

fn registry() -> &'static Mutex<BTreeMap<u64, ScheduledTaskEntry>> {
    REGISTRY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Extract a numeric `task_id` from a task record (`"task_id"`, `"job_id"`, or `"id"`).
pub fn extract_task_id(rec: &BTreeMap<String, Value>) -> Option<u64> {
    for key in ["task_id", "job_id", "id"] {
        match rec.get(key) {
            Some(Value::U64(id)) if *id > 0 => return Some(*id),
            Some(Value::I64(id)) if *id > 0 => return Some(*id as u64),
            _ => {}
        }
    }
    None
}

/// Returns whether a [`Value::Record`] carries task lifecycle fields.
pub fn is_task_record(rec: &BTreeMap<String, Value>) -> bool {
    rec.get("__task") == Some(&Value::Bool(true))
        || rec.get("spawned") == Some(&Value::Bool(true))
        || extract_task_id(rec).is_some()
        || rec.contains_key("status")
        || rec.contains_key("cancelled")
        || rec.contains_key("timed_out")
}

/// Verify that a task record holds the unguessable capability seal and matching session owner.
fn verify_task_ownership(
    entry: &ScheduledTaskEntry,
    rec: &BTreeMap<String, Value>,
    span: Span,
) -> Result<(), Diagnostic> {
    let supplied_seal = rec.get("__task_seal").and_then(|v| v.as_str()).unwrap_or("");
    if supplied_seal != entry.capability_seal {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            format!(
                "task {}: capability seal mismatch (forged or unowned task reference)",
                entry.task_id
            ),
        ));
    }
    if let Some(caller_owner) = rec
        .get("caller_owner")
        .or_else(|| rec.get("owner_scope"))
        .and_then(|v| v.as_str())
    {
        if caller_owner != entry.owner_scope {
            return Err(Diagnostic::new(
                DiagCode::E400,
                span,
                format!(
                    "task {}: owner_scope '{}' is not authorised for task owned by '{}'",
                    entry.task_id, caller_owner, entry.owner_scope
                ),
            ));
        }
    }
    Ok(())
}

/// Register a new live task in the scheduler and return its sealed handle record.
pub fn spawn_scheduled_task(
    target_node: &str,
    code_or_hash: &str,
    args: Value,
    elapsed_ms: i64,
) -> Value {
    let task_id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
    let nonce = SEAL_NONCE
        .fetch_add(0x517C_C1B7_2722_0A95, Ordering::Relaxed)
        .wrapping_mul(task_id);
    let owner_scope = match &args {
        Value::Record(r) => r
            .get("owner_scope")
            .and_then(|v| v.as_str())
            .unwrap_or("session:default")
            .to_string(),
        _ => "session:default".to_string(),
    };
    let seal_material =
        format!("vibe-task-seal:v1:{task_id}:{nonce}:{owner_scope}:{target_node}:{code_or_hash}");
    let capability_seal = crate::crypto::sha256_hex(seal_material.as_bytes());
    let cancel_token = Arc::new(AtomicBool::new(false));
    let entry = ScheduledTaskEntry {
        task_id,
        owner_scope: owner_scope.clone(),
        capability_seal: capability_seal.clone(),
        target_node: target_node.to_string(),
        code_or_hash: code_or_hash.to_string(),
        args: args.clone(),
        state: TaskLifecycleState::Running,
        cancel_token,
        created_at_inst: Instant::now(),
        elapsed_ms,
        timeout_ms: None,
        result: None,
        error: None,
    };
    if let Ok(mut guard) = registry().lock() {
        guard.insert(task_id, entry);
    }

    let mut out = BTreeMap::new();
    out.insert("__task".into(), Value::Bool(true));
    out.insert("__task_seal".into(), Value::String(capability_seal));
    out.insert("owner_scope".into(), Value::String(owner_scope));
    out.insert("task_id".into(), Value::U64(task_id));
    out.insert("spawned".into(), Value::Bool(true));
    out.insert("target_node".into(), Value::Iri(target_node.to_string()));
    out.insert("code_hash".into(), Value::String(code_or_hash.to_string()));
    out.insert("args".into(), args);
    out.insert("status".into(), Value::String("running".into()));
    out.insert("cancelled".into(), Value::Bool(false));
    out.insert("elapsed_ms".into(), Value::I64(elapsed_ms));
    out.insert("scheduler_registered".into(), Value::Bool(true));
    out.insert(
        "execution_qualification".into(),
        Value::String("live_scheduled_task".into()),
    );
    Value::Record(out)
}

/// Contact the live scheduler to cancel a running/pending task, preserving immutable terminal
/// outcomes (`Completed`, `Failed`, `TimedOut`) and verifying task ownership.
pub fn cancel_scheduled_task(
    rec: &BTreeMap<String, Value>,
    span: Span,
) -> Result<Value, Diagnostic> {
    let mut out = rec.clone();
    let task_id_opt = extract_task_id(rec);
    let require_live = rec.get("require_live_job") == Some(&Value::Bool(true));

    if let Some(task_id) = task_id_opt {
        if let Ok(mut guard) = registry().lock() {
            if let Some(entry) = guard.get_mut(&task_id) {
                verify_task_ownership(entry, rec, span)?;
                let prev_status = entry.state.as_str().to_string();
                out.insert("task_id".into(), Value::U64(task_id));
                out.insert("previous_status".into(), Value::String(prev_status));
                out.insert("scheduler_contacted".into(), Value::Bool(true));
                out.insert("cancel_requested".into(), Value::Bool(true));

                if entry.state.is_terminal() {
                    // Terminal outcomes are immutable: do not overwrite state or clear result.
                    let already_cancelled = entry.state == TaskLifecycleState::Cancelled;
                    out.insert("status".into(), Value::String(entry.state.as_str().into()));
                    out.insert("cancelled".into(), Value::Bool(already_cancelled));
                    out.insert("cancel_acknowledged".into(), Value::Bool(already_cancelled));
                    out.insert(
                        "already_committed".into(),
                        Value::Bool(entry.state == TaskLifecycleState::Completed),
                    );
                    out.insert("live_job_cancelled".into(), Value::Bool(false));
                    if let Some(res) = &entry.result {
                        out.insert("result".into(), res.clone());
                    }
                    out.insert(
                        "execution_qualification".into(),
                        Value::String("scheduler_task_already_terminal".into()),
                    );
                    return Ok(Value::Record(out));
                }

                entry.cancel_token.store(true, Ordering::Release);
                entry.state = TaskLifecycleState::Cancelled;
                out.insert("status".into(), Value::String("cancelled".into()));
                out.insert("cancelled".into(), Value::Bool(true));
                out.insert("cancel_acknowledged".into(), Value::Bool(true));
                out.insert("already_committed".into(), Value::Bool(false));
                out.insert("live_job_cancelled".into(), Value::Bool(true));
                out.insert(
                    "execution_qualification".into(),
                    Value::String("live_scheduler_cancellation".into()),
                );
                return Ok(Value::Record(out));
            }
        }
    }

    if require_live {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "task.cancel: no running job found in scheduler for task record",
        ));
    }

    out.insert("status".into(), Value::String("cancelled".into()));
    out.insert("cancelled".into(), Value::Bool(true));
    out.insert("cancel_requested".into(), Value::Bool(true));
    out.insert("cancel_acknowledged".into(), Value::Bool(false));
    out.insert("scheduler_contacted".into(), Value::Bool(true));
    out.insert("live_job_cancelled".into(), Value::Bool(false));
    out.insert(
        "execution_qualification".into(),
        Value::String("detached_record_only_no_running_job".into()),
    );
    Ok(Value::Record(out))
}

/// Apply a monotonic deadline check to a task record and cancel in the scheduler on timeout.
pub fn apply_task_timeout(
    rec: &BTreeMap<String, Value>,
    timeout_ms: i64,
    span: Span,
) -> Result<BTreeMap<String, Value>, Diagnostic> {
    let mut out = rec.clone();
    let supplied_elapsed = rec.get("elapsed_ms").and_then(|v| v.as_i64()).unwrap_or(0);
    let mut effective_elapsed = supplied_elapsed;
    let mut live_cancelled = false;
    let mut resolved_state_str: Option<&'static str> = None;

    if let Some(task_id) = extract_task_id(rec) {
        if let Ok(mut guard) = registry().lock() {
            if let Some(entry) = guard.get_mut(&task_id) {
                verify_task_ownership(entry, rec, span)?;
                let monotonic_ms = entry.created_at_inst.elapsed().as_millis() as i64;
                effective_elapsed = entry.elapsed_ms.max(supplied_elapsed).max(monotonic_ms);
                entry.elapsed_ms = effective_elapsed;
                entry.timeout_ms = Some(timeout_ms);

                if !entry.state.is_terminal() && (effective_elapsed > timeout_ms || timeout_ms <= 0)
                {
                    entry.cancel_token.store(true, Ordering::Release);
                    entry.state = TaskLifecycleState::TimedOut;
                    live_cancelled = true;
                }
                // Never mark an uncomputed Pending/Running task Completed without executing it.
                resolved_state_str = Some(entry.state.as_str());
            }
        }
    }

    let timed_out = effective_elapsed > timeout_ms || timeout_ms <= 0;
    out.insert("timed_out".into(), Value::Bool(timed_out));
    out.insert("timeout_ms".into(), Value::I64(timeout_ms));
    out.insert("elapsed_ms".into(), Value::I64(effective_elapsed));
    out.insert("scheduler_contacted".into(), Value::Bool(true));
    out.insert("live_job_cancelled".into(), Value::Bool(live_cancelled));
    let status_str = if timed_out {
        "timed_out"
    } else if let Some(s) = resolved_state_str {
        s
    } else {
        "completed"
    };
    out.insert("status".into(), Value::String(status_str.into()));
    out.insert(
        "execution_qualification".into(),
        Value::String(
            if live_cancelled {
                "live_scheduler_timeout_cancellation"
            } else if resolved_state_str.is_some() {
                "scheduled_task_deadline_attached"
            } else {
                "detached_record_timeout_check"
            }
            .into(),
        ),
    );
    Ok(out)
}

/// Inspect a task's current status in the live [`TaskRegistry`].
pub fn query_task_status(rec_or_id: &Value, span: Span) -> Result<Value, Diagnostic> {
    let task_id = match rec_or_id {
        Value::U64(id) => Some(*id),
        Value::I64(id) if *id > 0 => Some(*id as u64),
        Value::Record(r) => extract_task_id(r),
        _ => None,
    };
    let Some(id) = task_id else {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "task.status: expected a task Record with task_id or an integer task_id",
        ));
    };
    if let Ok(guard) = registry().lock() {
        if let Some(entry) = guard.get(&id) {
            if let Value::Record(r) = rec_or_id {
                verify_task_ownership(entry, r, span)?;
            }
            let mut out = BTreeMap::new();
            out.insert("task_id".into(), Value::U64(entry.task_id));
            out.insert("found".into(), Value::Bool(true));
            out.insert("owner_scope".into(), Value::String(entry.owner_scope.clone()));
            out.insert("status".into(), Value::String(entry.state.as_str().into()));
            out.insert(
                "cancelled".into(),
                Value::Bool(entry.cancel_token.load(Ordering::Acquire)),
            );
            out.insert("target_node".into(), Value::Iri(entry.target_node.clone()));
            out.insert("code_hash".into(), Value::String(entry.code_or_hash.clone()));
            if let Some(res) = &entry.result {
                out.insert("result".into(), res.clone());
            }
            return Ok(Value::Record(out));
        }
    }
    let mut out = BTreeMap::new();
    out.insert("task_id".into(), Value::U64(id));
    out.insert("found".into(), Value::Bool(false));
    out.insert("status".into(), Value::String("unknown".into()));
    Ok(Value::Record(out))
}

/// Return a snapshot summary of all tasks in [`TaskRegistry`].
pub fn registry_summary() -> TaskRegistrySummary {
    let mut sum = TaskRegistrySummary::default();
    if let Ok(guard) = registry().lock() {
        sum.total = guard.len() as u64;
        for entry in guard.values() {
            match entry.state {
                TaskLifecycleState::Pending => sum.pending += 1,
                TaskLifecycleState::Running => sum.running += 1,
                TaskLifecycleState::Completed => sum.completed += 1,
                TaskLifecycleState::Cancelled => sum.cancelled += 1,
                TaskLifecycleState::TimedOut => sum.timed_out += 1,
                TaskLifecycleState::Failed => sum.failed += 1,
            }
        }
    }
    sum
}

/// Select parsing mode once and evaluate on the admitted parent `Engine` & `Env` (propagating errors).
fn execute_scheduled_payload_on_engine<H: Host>(
    code_or_hash: &str,
    engine: &mut Engine<'_, H>,
    env: &mut Env,
    span: Span,
) -> Result<Value, Diagnostic> {
    let resolved_owned;
    let trimmed = if code_or_hash.trim().starts_with("sha256:") {
        if let Some(src) = crate::stdlib_v2::topology_and_manifold::effects_and_systems::lookup_verified_ast_source(code_or_hash.trim()) {
            resolved_owned = src;
            resolved_owned.trim()
        } else {
            return Err(Diagnostic::new(
                DiagCode::E400,
                span,
                format!("await: content-addressed task payload '{code_or_hash}' could not be verified"),
            ));
        }
    } else {
        code_or_hash.trim()
    };

    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "await: scheduled task has an empty code payload",
        ));
    }

    let is_program_mode = trimmed.starts_with("module ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("pure ")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("const ");

    if is_program_mode {
        let prog = parse_program(trimmed)?;
        engine.eval_program(&prog, env)
    } else {
        let cell_src = if trimmed.starts_with('=') {
            trimmed.to_string()
        } else {
            format!("= {}", trimmed.trim_end_matches(';'))
        };
        let expr = parse_cell(&cell_src)?;
        engine.eval_expr(&expr, env)
    }
}

/// Core scheduler resolution for `await` in the AST interpreter (`Engine::eval_expr`).
pub fn await_scheduled_value<H: Host>(
    engine: &mut Engine<'_, H>,
    val: Value,
    env: &mut Env,
    span: Span,
) -> Result<Value, Diagnostic> {
    match val {
        Value::Record(rec) if is_task_record(&rec) => {
            await_task_record_with_engine(rec, engine, env, span)
        }
        Value::Lambda { params, body } if params.is_empty() => {
            let handle =
                spawn_scheduled_task("did:q42:local_scheduler", "<lambda>", Value::Null, 0);
            let task_id = match &handle {
                Value::Record(r) => extract_task_id(r).unwrap_or(0),
                _ => 0,
            };
            let result = engine.eval_expr(&body, env)?;
            if let Ok(mut guard) = registry().lock() {
                if let Some(entry) = guard.get_mut(&task_id) {
                    if entry.cancel_token.load(Ordering::Acquire) {
                        return Err(Diagnostic::new(
                            DiagCode::E400,
                            span,
                            format!("await: scheduled lambda task {task_id} was cancelled"),
                        ));
                    }
                    entry.state = TaskLifecycleState::Completed;
                    entry.result = Some(result.clone());
                }
            }
            Ok(result)
        }
        other => Ok(other),
    }
}

/// Standalone `await` resolution for bytecode VM (`Op::CallHost("task.await")`) and host dispatch.
pub fn await_value_standalone(val: Value, span: Span) -> Result<Value, Diagnostic> {
    match val {
        Value::Record(rec) if is_task_record(&rec) => {
            let mut host = LocalHost::default();
            let mut env = Env::default();
            let mut engine = Engine::new(&mut host, crate::budget::Budget::default());
            await_task_record_with_engine(rec, &mut engine, &mut env, span)
        }
        other => Ok(other),
    }
}

fn await_task_record_with_engine<H: Host>(
    mut rec: BTreeMap<String, Value>,
    engine: &mut Engine<'_, H>,
    env: &mut Env,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Some(task_id) = extract_task_id(&rec) {
        let live_snapshot = registry()
            .lock()
            .ok()
            .and_then(|guard| guard.get(&task_id).cloned());

        if let Some(entry) = live_snapshot {
            verify_task_ownership(&entry, &rec, span)?;

            if entry.cancel_token.load(Ordering::Acquire)
                || entry.state == TaskLifecycleState::Cancelled
                || rec.get("cancelled") == Some(&Value::Bool(true))
                || rec.get("status").and_then(|v| v.as_str()) == Some("cancelled")
            {
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    span,
                    format!("await: task {task_id} was cancelled in scheduler"),
                ));
            }
            let monotonic_ms = entry.created_at_inst.elapsed().as_millis() as i64;
            let effective_elapsed = entry.elapsed_ms.max(monotonic_ms);
            let deadline_expired = entry
                .timeout_ms
                .map(|t| effective_elapsed > t || t <= 0)
                .unwrap_or(false);
            if entry.state == TaskLifecycleState::TimedOut
                || deadline_expired
                || rec.get("timed_out") == Some(&Value::Bool(true))
                || rec.get("status").and_then(|v| v.as_str()) == Some("timed_out")
            {
                if let Ok(mut guard) = registry().lock() {
                    if let Some(live_mut) = guard.get_mut(&task_id) {
                        live_mut.cancel_token.store(true, Ordering::Release);
                        live_mut.state = TaskLifecycleState::TimedOut;
                    }
                }
                return Err(Diagnostic::new(
                    DiagCode::E400,
                    span,
                    format!("await: task {task_id} timed out in scheduler"),
                ));
            }
            if entry.state == TaskLifecycleState::Failed
                || rec.get("status").and_then(|v| v.as_str()) == Some("failed")
            {
                let msg = entry
                    .error
                    .unwrap_or_else(|| format!("await: task {task_id} failed in scheduler"));
                return Err(Diagnostic::new(DiagCode::E400, span, msg));
            }

            // Never trust a caller-supplied `rec.get("result")`: only use `entry.result`
            // if already committed in the registry, otherwise execute on the admitted Engine.
            let computed = if entry.state == TaskLifecycleState::Completed {
                if let Some(existing) = entry.result {
                    existing
                } else {
                    Value::Null
                }
            } else {
                match execute_scheduled_payload_on_engine(&entry.code_or_hash, engine, env, span) {
                    Ok(v) => v,
                    Err(diag) => {
                        if let Ok(mut guard) = registry().lock() {
                            if let Some(live_mut) = guard.get_mut(&task_id) {
                                live_mut.state = TaskLifecycleState::Failed;
                                live_mut.error = Some(diag.message.clone());
                            }
                        }
                        return Err(diag);
                    }
                }
            };

            if let Ok(mut guard) = registry().lock() {
                if let Some(live_mut) = guard.get_mut(&task_id) {
                    if live_mut.cancel_token.load(Ordering::Acquire) {
                        return Err(Diagnostic::new(
                            DiagCode::E400,
                            span,
                            format!("await: task {task_id} cancelled during execution"),
                        ));
                    }
                    live_mut.state = TaskLifecycleState::Completed;
                    live_mut.result = Some(computed.clone());
                }
            }

            rec.insert("task_id".into(), Value::U64(task_id));
            rec.insert("status".into(), Value::String("completed".into()));
            rec.insert("awaited".into(), Value::Bool(true));
            rec.insert("scheduler_contacted".into(), Value::Bool(true));
            rec.insert("live_job_joined".into(), Value::Bool(true));
            rec.insert("result".into(), computed);
            rec.insert(
                "execution_qualification".into(),
                Value::String("scheduled_task_joined".into()),
            );
            return Ok(Value::Record(rec));
        }
    }

    let status = rec.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if rec.get("cancelled") == Some(&Value::Bool(true)) || status == "cancelled" {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "await: cannot await a cancelled task record",
        ));
    }
    if rec.get("timed_out") == Some(&Value::Bool(true)) || status == "timed_out" {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "await: cannot await a timed-out task record",
        ));
    }
    if status == "failed" || rec.get("ok") == Some(&Value::Bool(false)) {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "await: cannot await a failed task record",
        ));
    }

    rec.insert("status".into(), Value::String("completed".into()));
    rec.insert("awaited".into(), Value::Bool(true));
    rec.insert("scheduler_contacted".into(), Value::Bool(true));
    rec.insert("live_job_joined".into(), Value::Bool(false));
    rec.insert(
        "execution_qualification".into(),
        Value::String("detached_record_immediate_await".into()),
    );
    Ok(Value::Record(rec))
}
