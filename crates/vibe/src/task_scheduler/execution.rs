use super::registry::*;
use crate::{
    bind::Host,
    error::Diagnostic,
    eval::{Engine, Env},
    span::Span,
    value::Value,
};
use std::collections::BTreeMap;

/// An unwind cannot leave a task eligible for a second effectful execution.
struct ExecutionClaim {
    _scope: TaskScope,
    id: u64,
}
impl Drop for ExecutionClaim {
    fn drop(&mut self) {
        if let Ok(mut tasks) = registry().lock() {
            if let Some(entry) = tasks.get_mut(&self.id) {
                executor_checkpoint(entry);
                if entry.state == State::Running {
                    entry.state = State::Failed;
                    entry.failure =
                        Some("task execution unwound; prior effects require reconciliation".into());
                }
            }
        }
    }
}

pub fn await_scheduled_value<H: Host>(
    engine: &mut Engine<'_, H>,
    value: Value,
    env: &mut Env,
    span: Span,
) -> Result<Value, Diagnostic> {
    match value {
        Value::Record(rec) if is_task_record(&rec) => join(engine, env, Value::Record(rec), span),
        Value::Lambda { params, body } if params.is_empty() => engine.eval_expr(&body, env),
        other => Ok(other),
    }
}

fn join<H: Host>(
    engine: &mut Engine<'_, H>,
    env: &mut Env,
    handle: Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    if engine.active_tasks.len() >= 8 {
        return Err(error(span, "nested synchronous task limit exceeded (8)"));
    }
    let scope = env.task_scope.clone();
    let (id, source) = {
        let mut tasks = registry()
            .lock()
            .map_err(|_| error(span, "task registry is poisoned"))?;
        let entry = owned(&scope, &mut tasks, &handle, span)?;
        refresh_observer(entry);
        match entry.state {
            State::Completed => return Ok(record(entry)),
            State::Running => return Err(error(
                span,
                "task already has an executor; synchronous await cannot suspend or run it twice",
            )),
            State::Cancelled | State::TimedOut | State::Failed => {
                return Err(error(
                    span,
                    entry
                        .failure
                        .as_deref()
                        .unwrap_or("task is terminal without a successful result"),
                ))
            }
            State::Pending => {}
        }
        entry.state = State::Running;
        (entry.id, entry.code.clone())
    };
    let claim = ExecutionClaim {
        _scope: scope.clone(),
        id,
    };
    engine.active_tasks.push((scope.clone(), id));
    let output = execute(engine, env, &source, span);
    engine.active_tasks.pop();
    let output = output.and_then(|value| {
        let bytes = retained_bytes(&value, span)?;
        engine.charge_task_source((bytes as u64).saturating_mul(2), span)?;
        Ok((value, bytes))
    });
    let mut tasks = registry()
        .lock()
        .map_err(|_| error(span, "task registry is poisoned"))?;
    let output = output.and_then(|(value, bytes)| {
        admit_bytes(&scope, &tasks, bytes, span)?;
        Ok((value, bytes))
    });
    let entry = owned(&scope, &mut tasks, &handle, span)?;
    executor_checkpoint(entry); // The actual executor has returned from native code.
    if entry.state != State::Running {
        return Err(error(
            span,
            "task cancelled or deadline expired; prior external effects may require reconciliation",
        ));
    }
    match output {
        Ok((value, bytes)) => {
            entry.retained_bytes += bytes;
            entry.result = Some(value);
            entry.state = State::Completed;
        }
        Err(diag) => {
            entry.failure = Some(diag.message.clone());
            entry.state = State::Failed;
            return Err(diag);
        }
    }
    let result = record(entry);
    drop(tasks);
    drop(claim);
    Ok(result)
}

fn execute<H: Host>(
    engine: &mut Engine<'_, H>,
    env: &mut Env,
    source: &str,
    span: Span,
) -> Result<Value, Diagnostic> {
    let resolved;
    let source = if source.trim().starts_with("sha256:") {
        resolved = crate::stdlib_v2::topology_and_manifold::effects_and_systems::lookup_verified_ast_source(source.trim())
            .ok_or_else(|| error(span, "task content-addressed source is unavailable or failed verification"))?;
        resolved.trim()
    } else {
        source.trim()
    };
    if source.len() > 65_536 {
        return Err(error(span, "resolved task source exceeds 65536 bytes"));
    }
    // Select from complete parser results before checking or executing anything.
    // The cell parser handles comments before '='; full programs may start with
    // imports, requirements or any declaration rather than a keyword heuristic.
    let explicit_cell = crate::parse_cell(source);
    if let Ok(expression) = explicit_cell {
        crate::check_cell(&expression)?;
        return engine.eval_expr(&expression, env);
    }
    match crate::parse_program(source) {
        Ok(program) => {
            crate::check_program(&program)?;
            engine.eval_task_program(&program, env, span)
        }
        Err(program_error) => {
            // Legacy bare-expression payloads retain their implicit '=' profile.
            // This is parse-only fallback, never a retry after checking/effects.
            let expression = match crate::parse_cell(&format!("= {source}")) {
                Ok(expression) => expression,
                Err(_) => return Err(program_error),
            };
            crate::check_cell(&expression)?;
            engine.eval_expr(&expression, env)
        }
    }
}

/// Routing occurs only after the Engine phase gate. No fresh host or budget is created.
pub(crate) fn dispatch_on_engine<H: Host>(
    engine: &mut Engine<'_, H>,
    path: &str,
    args: &[Value],
    env: &mut Env,
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    let scope = env.task_scope.clone();
    let first = || {
        args.first()
            .ok_or_else(|| error(span, "task operation requires an argument"))
    };
    let result = match path {
        "task.spawn" => {
            let code = first()?
                .as_str()
                .ok_or_else(|| error(span, "task.spawn expects source text"))?;
            // Legacy args remain inert metadata: they never grant scope, authority or elapsed time.
            engine.charge_task_source(code.len() as u64, span)?;
            spawn(&scope, code, span)?
        }
        "code.spawn_at" => {
            let node = first()?
                .as_str()
                .ok_or_else(|| error(span, "code.spawn_at expects a node identifier"))?;
            if node != "did:q42:local_scheduler" {
                return Err(error(
                    span,
                    "remote/provider task launch is unsupported; no admitted provider is connected",
                ));
            }
            let code = args
                .get(1)
                .and_then(Value::as_str)
                .ok_or_else(|| error(span, "code.spawn_at expects source or a verified digest"))?;
            if code.starts_with("sha256:") && crate::stdlib_v2::topology_and_manifold::effects_and_systems::lookup_verified_ast_source(code).is_none() {
                return Err(error(span, "content-addressed task source is unavailable"));
            }
            engine.charge_task_source(code.len() as u64, span)?;
            spawn(&scope, code, span)?
        }
        "task.await" => await_scheduled_value(engine, first()?.clone(), env, span)?,
        "task.status" => status(&scope, first()?, span)?,
        "task.cancel" => cancel(&scope, first()?, span)?,
        "task.with_timeout" => {
            let ms = args
                .get(1)
                .and_then(Value::as_i64)
                .ok_or_else(|| error(span, "task.with_timeout expects integer milliseconds"))?;
            timeout(&scope, first()?, ms, span)?
        }
        "task.nursery" => nursery(engine, env, first()?, span)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn nursery<H: Host>(
    engine: &mut Engine<'_, H>,
    env: &mut Env,
    input: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let Value::List(tasks) = input else {
        return Err(error(span, "task.nursery expects task handles"));
    };
    if tasks.len() > 256 {
        return Err(error(span, "task nursery capacity exceeds 256"));
    }
    // Validate the complete group before effects. Authored status cannot certify failure or success.
    let mut known_failure = false;
    for task in tasks {
        let Value::Record(current) = status(&env.task_scope, task, span)? else {
            unreachable!()
        };
        known_failure |= matches!(
            current.get("status").and_then(Value::as_str),
            Some("failed" | "cancelled" | "timed_out")
        );
    }
    let mut results = Vec::with_capacity(tasks.len());
    let mut failure = known_failure
        .then(|| "nursery already contains a failed, cancelled or timed-out task".to_string());
    let mut cancelled_count = 0u64;
    let mut requested_count = 0u64;
    for task in tasks {
        if failure.is_some() {
            let cancelled = cancel(&env.task_scope, task, span)?;
            if let Value::Record(record) = &cancelled {
                cancelled_count +=
                    u64::from(record.get("live_job_cancelled") == Some(&Value::Bool(true)));
                requested_count += u64::from(
                    record.get("cancellation_request_accepted") == Some(&Value::Bool(true)),
                );
            }
            results.push(cancelled);
        } else {
            match join(engine, env, task.clone(), span) {
                Ok(result) => results.push(result),
                Err(diag) => {
                    failure = Some(diag.message);
                    results.push(status(&env.task_scope, task, span)?);
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    out.insert("all_succeeded".into(), Value::Bool(failure.is_none()));
    out.insert(
        "cancelled_siblings".into(),
        Value::Bool(cancelled_count > 0),
    );
    out.insert(
        "live_siblings_cancelled".into(),
        Value::U64(cancelled_count),
    );
    out.insert(
        "cancellation_requested".into(),
        Value::Bool(requested_count > 0),
    );
    out.insert(
        "cancellation_requested_count".into(),
        Value::U64(requested_count),
    );
    out.insert("tasks".into(), Value::List(results));
    out.insert(
        "execution_qualification".into(),
        Value::String("sequential_local_nursery_no_async_suspension".into()),
    );
    if let Some(message) = failure {
        out.insert("error".into(), Value::String(message));
    }
    Ok(Value::Record(out))
}
