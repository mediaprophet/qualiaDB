use super::registry::{cancel, owned, record, registry, spawn, status, timeout, State};
use super::*;
use crate::{
    bind::LocalHost,
    budget::Budget,
    eval::{Engine, Env},
    parse_cell, Value,
};

fn span() -> Span {
    Span::point(0)
}
fn run(source: &str, host: &mut LocalHost, env: &mut Env) -> Result<Value, Diagnostic> {
    // Trusted low-level runtime setup: checker tests below exercise admission separately.
    Engine::new(host, Budget::default()).eval_expr(&parse_cell(source)?, env)
}
fn result(value: Value) -> Value {
    let Value::Record(record) = value else {
        panic!("joined task record required")
    };
    record.get("result").expect("committed result").clone()
}

#[test]
fn pure_checker_rejects_registry_operations() {
    for call in [
        "task.spawn(\"= 42\", {})",
        "task.cancel({})",
        "task.status(1u64)",
        "task.await({})",
        "code.hash_ast(\"= 42\")",
    ] {
        let program =
            crate::parse_program(&format!("pure fn main() {{ return {call}; }}")).unwrap();
        assert!(crate::check_program(&program).is_err(), "accepted {call}");
    }
    let dynamic = crate::parse_program("pure fn main(job: Record) { return await job; }").unwrap();
    assert!(crate::check_program(&dynamic).is_err());
    assert!(crate::eval_cell("= await 42", &mut LocalHost::default(), &mut Env::default()).is_ok());
}

#[test]
fn checked_effectful_program_keeps_local_task_functionality() {
    let source = r#"module checked_tasks;
effect async fn main() budget(steps: 400) {
    let job = task.spawn("= 21 + 21", {});
    let joined = await job;
    return joined.result;
}"#;
    let program = crate::load_program(source).unwrap();
    assert_eq!(
        crate::eval_function(
            &program,
            "main",
            vec![],
            &mut LocalHost::default(),
            &mut Env::default()
        )
        .unwrap(),
        Value::I64(42)
    );
}

#[test]
fn cross_scope_copied_records_and_integer_ids_cannot_read_cancel_or_timeout() {
    let owner = TaskScope::default();
    let other = TaskScope::default();
    let task = spawn(&owner, "= 42", span()).unwrap();
    let Value::Record(mut forged) = task.clone() else {
        unreachable!()
    };
    forged.insert(
        "owner_scope".into(),
        Value::String("session:default".into()),
    );
    let forged = Value::Record(forged);
    assert!(status(&other, &forged, span()).is_err());
    assert!(cancel(&other, &forged, span()).is_err());
    assert!(timeout(&other, &forged, 0, span()).is_err());
    let Value::Record(handle) = task else {
        unreachable!()
    };
    assert!(status(&owner, handle.get("task_id").unwrap(), span()).is_err());
}

#[test]
fn both_await_routes_use_the_admitted_environment() {
    let mut env = Env::default();
    let mut host = LocalHost::default();
    env.vars.insert("input".into(), Value::I64(41));
    for expression in ["= await job", "= task.await(job)"] {
        let job = run("= task.spawn(\"= input + 1\", {})", &mut host, &mut env).unwrap();
        env.vars.insert("job".into(), job);
        assert_eq!(
            result(run(expression, &mut host, &mut env).unwrap()),
            Value::I64(42)
        );
    }
}

#[test]
fn completed_outcome_ignores_authored_flags_and_later_timeouts() {
    let mut env = Env::default();
    let mut host = LocalHost::default();
    let job = spawn(&env.task_scope, "= 42", span()).unwrap();
    env.vars.insert("job".into(), job.clone());
    assert_eq!(
        result(run("= await job", &mut host, &mut env).unwrap()),
        Value::I64(42)
    );
    let Value::Record(mut flags) = job.clone() else {
        unreachable!()
    };
    flags.insert("timed_out".into(), Value::Bool(true));
    flags.insert("cancelled".into(), Value::Bool(true));
    flags.insert("result".into(), Value::I64(999));
    env.vars.insert("job".into(), Value::Record(flags));
    assert_eq!(
        result(run("= await job", &mut host, &mut env).unwrap()),
        Value::I64(42)
    );
    assert_eq!(
        result(timeout(&env.task_scope, &job, 0, span()).unwrap()),
        Value::I64(42)
    );
    assert_eq!(
        result(cancel(&env.task_scope, &job, span()).unwrap()),
        Value::I64(42)
    );
}

#[test]
fn zero_deadline_prevents_execution_and_caller_elapsed_is_inert() {
    let mut env = Env::default();
    let mut host = LocalHost::default();
    let job = run(
        "= task.spawn(\"= 42\", { elapsed_ms: 999999, owner_scope: \"forged\" })",
        &mut host,
        &mut env,
    )
    .unwrap();
    let timed = timeout(&env.task_scope, &job, 0, span()).unwrap();
    env.vars.insert("job".into(), timed);
    assert!(run("= await job", &mut host, &mut env).is_err());
}

#[test]
fn executor_claim_prevents_duplicate_execution_and_budget_is_shared() {
    let mut env = Env::default();
    let job = spawn(&env.task_scope, "= 21 + 21", span()).unwrap();
    {
        let mut tasks = registry().lock().unwrap();
        owned(&env.task_scope, &mut tasks, &job, span())
            .unwrap()
            .state = State::Running;
    }
    env.vars.insert("job".into(), job.clone());
    let mut host = LocalHost::default();
    assert!(run("= await job", &mut host, &mut env).is_err());
    {
        let mut tasks = registry().lock().unwrap();
        owned(&env.task_scope, &mut tasks, &job, span())
            .unwrap()
            .state = State::Pending;
    }
    let mut engine = Engine::new(&mut host, Budget::from_steps(3));
    assert!(engine
        .eval_expr(&parse_cell("= await job").unwrap(), &mut env)
        .is_err());
    let mut tasks = registry().lock().unwrap();
    let entry = owned(&env.task_scope, &mut tasks, &job, span()).unwrap();
    assert_eq!(entry.state, State::Failed);
    assert!(entry.result.is_none());
}

#[test]
fn standalone_facades_refuse_and_scope_drop_releases_entries() {
    assert!(
        spawn_scheduled_task("did:q42:local_scheduler", "=42", Value::Null, 0, span()).is_err()
    );
    let scope = TaskScope::default();
    let task = spawn(&scope, "=42", span()).unwrap();
    assert!(await_value_standalone(task.clone(), span()).is_err());
    assert_eq!(
        await_value_standalone(Value::I64(42), span()).unwrap(),
        Value::I64(42)
    );
    let Value::Record(handle) = task else {
        unreachable!()
    };
    let id = extract_task_id(&handle).unwrap();
    assert!(registry().lock().unwrap().contains_key(&id));
    drop(scope);
    assert!(!registry().lock().unwrap().contains_key(&id));
}

#[test]
fn runtime_deadline_checkpoint_rejects_expired_executor() {
    let scope = TaskScope::default();
    let job = spawn(&scope, "=42", span()).unwrap();
    let id;
    {
        let mut tasks = registry().lock().unwrap();
        let entry = owned(&scope, &mut tasks, &job, span()).unwrap();
        entry.state = State::Running;
        id = entry.id;
    }
    timeout(&scope, &job, 0, span()).unwrap();
    assert!(checkpoint(&scope, id, span()).is_err());
    let mut tasks = registry().lock().unwrap();
    let entry = owned(&scope, &mut tasks, &job, span()).unwrap();
    assert_eq!(entry.state, State::TimedOut);
    let Value::Record(output) = record(entry) else {
        unreachable!()
    };
    assert!(!output.contains_key("result"));
}

#[test]
fn per_scope_capacity_is_bounded_and_clones_intentionally_share_it() {
    let scope = TaskScope::default();
    let shared = scope.clone();
    for _ in 0..256 {
        spawn(&scope, "= 42", span()).unwrap();
    }
    assert!(spawn(&shared, "= 42", span()).is_err());
    assert!(spawn(&scope, &"x".repeat(65_537), span()).is_err());
}

#[test]
fn retained_results_are_bounded_before_debug_formatting_or_cloning() {
    use super::retention::retained_bytes;
    let mut nested = Value::Null;
    for _ in 0..40 {
        nested = Value::Ok(Box::new(nested));
    }
    assert!(retained_bytes(&nested, span()).is_err());
    let wide = Value::List(vec![Value::I64(0); 16_385]);
    assert!(retained_bytes(&wide, span()).is_err());
    assert!(retained_bytes(&Value::String("x".repeat(1024 * 1024 + 1)), span()).is_err());
    assert!(retained_bytes(&Value::List(vec![Value::I64(42)]), span()).is_ok());
}

#[test]
fn commented_child_programs_execute_with_inherited_requirements_and_effect_headers() {
    let child = "// child header\nrequires [ capability(\"graph.read\") ];\neffect async fn answer() budget(steps:100) { return 42; }\nreturn answer();";
    let quoted = child
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    let parent_body = format!("effect async fn main() budget(steps:1000) {{ let job=task.spawn(\"{quoted}\", {{}}); let joined=await job; return joined.result; }}");
    let parent = crate::load_program(&format!(
        "requires [ capability(\"graph.read\") ]; {parent_body}"
    ))
    .unwrap();
    assert_eq!(
        crate::eval_function(
            &parent,
            "main",
            vec![],
            &mut LocalHost::default(),
            &mut Env::default()
        )
        .unwrap(),
        Value::I64(42)
    );
    let denied = crate::load_program(&parent_body).unwrap();
    let error = crate::eval_function(
        &denied,
        "main",
        vec![],
        &mut LocalHost::default(),
        &mut Env::default(),
    )
    .unwrap_err();
    assert!(
        error.message.contains("not granted by its caller"),
        "must reach real grant admission, not fail to parse: {error:?}"
    );

    let mut host = LocalHost::default();
    let mut env = Env::default();
    for (source, expected) in [
        ("// comment before effect\neffect fn answer() budget(steps:100) { return 43; } return answer();", Value::I64(43)),
        ("// comment before import\nimport \"vibe:0.1/math\" as childmath; return childmath.sqrt(1764.0);", Value::F64(42.0)),
    ] {
        let job = spawn(&env.task_scope, source, span()).unwrap();
        env.vars.insert("job".into(), job);
        assert_eq!(result(run("= await job", &mut host, &mut env).unwrap()), expected);
        assert!(!env.aliases.contains_key("childmath"), "task imports must restore caller aliases");
    }
}

#[test]
fn nursery_reports_only_actual_sibling_cancellation() {
    let mut host = LocalHost::default();
    let mut env = Env::default();
    let failed = spawn(&env.task_scope, "= 1 / 0", span()).unwrap();
    env.vars.insert("job".into(), failed.clone());
    assert!(run("= await job", &mut host, &mut env).is_err());
    let completed = spawn(&env.task_scope, "= 42", span()).unwrap();
    env.vars.insert("job".into(), completed.clone());
    assert_eq!(
        result(run("= await job", &mut host, &mut env).unwrap()),
        Value::I64(42)
    );
    env.vars
        .insert("group".into(), Value::List(vec![failed.clone(), completed]));
    let Value::Record(group) = run("= task.nursery(group)", &mut host, &mut env).unwrap() else {
        unreachable!()
    };
    assert_eq!(group.get("all_succeeded"), Some(&Value::Bool(false)));
    assert_eq!(group.get("cancelled_siblings"), Some(&Value::Bool(false)));
    assert_eq!(group.get("live_siblings_cancelled"), Some(&Value::U64(0)));
    assert_eq!(
        group.get("cancellation_requested_count"),
        Some(&Value::U64(0))
    );
    let pending = spawn(&env.task_scope, "= 42", span()).unwrap();
    env.vars
        .insert("group".into(), Value::List(vec![failed, pending]));
    let Value::Record(group) = run("= task.nursery(group)", &mut host, &mut env).unwrap() else {
        unreachable!()
    };
    assert_eq!(group.get("cancelled_siblings"), Some(&Value::Bool(true)));
    assert_eq!(group.get("live_siblings_cancelled"), Some(&Value::U64(1)));
    assert_eq!(
        group.get("cancellation_requested_count"),
        Some(&Value::U64(1))
    );
}

#[test]
fn child_capability_declarations_cannot_drop_or_invent_context_constraints() {
    for (parent_cap, child_cap, accepted) in [
        (
            "capability(\"graph.read\", context: \"restricted\")",
            "capability(\"graph.read\")",
            false,
        ),
        (
            "capability(\"graph.read\")",
            "capability(\"graph.read\", context: \"unproved\")",
            false,
        ),
        (
            "capability(\"graph.read\")",
            "capability(\"graph.read\")",
            true,
        ),
    ] {
        let child = format!("requires [ {child_cap} ]; return 42;");
        let quoted = child.replace('\\', "\\\\").replace('"', "\\\"");
        let source = format!("requires [ {parent_cap} ]; effect async fn main() budget(steps:1000) {{ let job=task.spawn(\"{quoted}\", {{}}); let joined=await job; return joined.result; }}");
        let program = crate::load_program(&source).unwrap();
        let output = crate::eval_function(
            &program,
            "main",
            vec![],
            &mut LocalHost::default(),
            &mut Env::default(),
        );
        if accepted {
            assert_eq!(output.unwrap(), Value::I64(42));
        } else {
            assert!(output
                .unwrap_err()
                .message
                .contains("constraints require owning host evaluation"));
        }
    }
}
