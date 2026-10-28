//! Host callbacks provide deterministic boundaries; no sleeps or real external services.
use super::registry::{cancel, spawn, status, timeout};
use super::*;
use crate::{
    bind::{Host, LocalHost},
    budget::Budget,
    eval::{Engine, Env},
};

struct DeadlineHost {
    inner: LocalHost,
    scope: TaskScope,
    job: Value,
    calls: usize,
    block: Option<(
        std::sync::mpsc::SyncSender<()>,
        std::sync::mpsc::Receiver<()>,
    )>,
}
impl Host for DeadlineHost {
    fn graph_query(&mut self, args: &[Value], take: u64, span: Span) -> Result<Value, Diagnostic> {
        self.inner.graph_query(args, take, span)
    }
    fn graph_stage(&mut self, term: &Value, span: Span) -> Result<Value, Diagnostic> {
        self.inner.graph_stage(term, span)
    }
    fn graph_commit(&mut self, span: Span) -> Result<Value, Diagnostic> {
        self.inner.graph_commit(span)
    }
    fn aura_validate(
        &mut self,
        node: &Value,
        shape: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.inner.aura_validate(node, shape, span)
    }
    fn pulse_publish(
        &mut self,
        topic: &str,
        payload: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.inner.pulse_publish(topic, payload, span)
    }
    fn quin_seal(
        &mut self,
        s: u64,
        p: u64,
        o: u64,
        c: u64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.inner.quin_seal(s, p, o, c, span)
    }
    fn time_unix(&mut self, span: Span) -> Result<Value, Diagnostic> {
        self.calls += 1;
        if let Some((entered, release)) = self.block.take() {
            entered
                .send(())
                .map_err(|_| super::registry::error(span, "test entry channel closed"))?;
            release
                .recv()
                .map_err(|_| super::registry::error(span, "test release channel closed"))?;
            return Ok(Value::I64(123));
        }
        // The callback represents a non-preemptible admitted native call. Deadline
        // expiry is observed after its attempted effect; no rollback is invented.
        timeout(&self.scope, &self.job, 0, span)?;
        Ok(Value::I64(123))
    }
}

#[test]
fn deadline_after_native_call_withholds_result_without_retrying_the_effect() {
    let mut env = Env::default();
    let job = spawn(
        &env.task_scope,
        "module deadline_task; return time.unix();",
        Span::point(0),
    )
    .unwrap();
    let mut host = DeadlineHost {
        inner: LocalHost::default(),
        scope: env.task_scope.clone(),
        job: job.clone(),
        calls: 0,
        block: None,
    };
    env.vars.insert("job".into(), job.clone());
    let expression = crate::parse_cell("= await job").unwrap();
    let result = Engine::new(&mut host, Budget::default()).eval_expr(&expression, &mut env);
    assert!(result.is_err());
    assert_eq!(host.calls, 1, "admitted host must execute exactly once");
    let Value::Record(record) = status(&env.task_scope, &job, Span::point(0)).unwrap() else {
        unreachable!()
    };
    assert_eq!(
        record.get("status"),
        Some(&Value::String("timed_out".into()))
    );
    assert!(!record.contains_key("result"));
    assert!(Engine::new(&mut host, Budget::default())
        .eval_expr(&expression, &mut env)
        .is_err());
    assert_eq!(
        host.calls, 1,
        "terminal await must not retry an external effect"
    );
}

#[test]
fn concurrent_join_cannot_duplicate_a_native_effect() {
    let mut env = Env::default();
    let job = spawn(
        &env.task_scope,
        "module concurrent_task; return time.unix();",
        Span::point(0),
    )
    .unwrap();
    env.vars.insert("job".into(), job.clone());
    let mut worker_env = env.clone(); // Deliberate host-level sharing of one runtime scope.
    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let mut host = DeadlineHost {
            inner: LocalHost::default(),
            scope: worker_env.task_scope.clone(),
            job,
            calls: 0,
            block: Some((entered_tx, release_rx)),
        };
        let result = Engine::new(&mut host, Budget::default())
            .eval_expr(&crate::parse_cell("= await job").unwrap(), &mut worker_env);
        (result, host.calls)
    });
    let entered = entered_rx.recv_timeout(std::time::Duration::from_secs(10));
    let duplicate = if entered.is_ok() {
        Engine::new(&mut LocalHost::default(), Budget::default())
            .eval_expr(&crate::parse_cell("= await job").unwrap(), &mut env)
    } else {
        Err(super::registry::error(
            Span::point(0),
            "worker did not enter native callback",
        ))
    };
    let _released = release_tx.send(());
    let (completed, calls) = worker.join().unwrap();
    assert!(
        entered.is_ok(),
        "worker callback must be reached: {completed:?}"
    );
    assert!(duplicate.is_err(), "second executor must be refused");
    assert!(completed.is_ok());
    assert_eq!(calls, 1);
}

#[test]
fn inherited_phase_lease_denies_task_syntax_and_explicit_await() {
    use crate::deontic_interrupt::{Phase, PhaseLeaser};
    let mut env = Env::default();
    let job = spawn(&env.task_scope, "= 42", Span::point(0)).unwrap();
    env.vars.insert("job".into(), job);
    let mut host = LocalHost::default();
    let mut leaser = PhaseLeaser::new();
    leaser
        .register_phase(Phase::new("read").allow("graph"))
        .unwrap();
    leaser.enter_phase("read").unwrap();
    let mut engine = Engine::with_phase_leaser(&mut host, Budget::default(), &mut leaser);
    for source in ["= await job", "= task.await(job)"] {
        assert!(engine
            .eval_expr(&crate::parse_cell(source).unwrap(), &mut env)
            .is_err());
    }
}

#[test]
fn blocked_native_requests_are_acknowledged_only_after_executor_returns() {
    for expires in [false, true] {
        let mut env = Env::default();
        let job = spawn(
            &env.task_scope,
            "module blocked_task; return time.unix();",
            Span::point(0),
        )
        .unwrap();
        env.vars.insert("job".into(), job.clone());
        let mut worker_env = env.clone();
        let worker_job = job.clone();
        let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let mut host = DeadlineHost {
                inner: LocalHost::default(),
                scope: worker_env.task_scope.clone(),
                job: worker_job,
                calls: 0,
                block: Some((entered_tx, release_rx)),
            };
            let result = Engine::new(&mut host, Budget::default())
                .eval_expr(&crate::parse_cell("= await job").unwrap(), &mut worker_env);
            (result, host.calls)
        });
        let entered = entered_rx.recv_timeout(std::time::Duration::from_secs(10));
        let request = if expires {
            timeout(&env.task_scope, &job, 0, Span::point(0))
        } else {
            cancel(&env.task_scope, &job, Span::point(0))
        };
        let observed = status(&env.task_scope, &job, Span::point(0));
        // Release before assertions so a failed expectation cannot strand a worker.
        let _released = release_tx.send(());
        let (completed, calls) = worker.join().unwrap();
        assert!(entered.is_ok(), "callback must be reached: {completed:?}");
        assert!(request.is_ok());
        let Value::Record(observed) = observed.unwrap() else {
            unreachable!()
        };
        assert_eq!(
            observed.get("status"),
            Some(&Value::String("running".into()))
        );
        assert_eq!(observed.get("cancel_requested"), Some(&Value::Bool(true)));
        assert_eq!(
            observed.get("cancel_acknowledged"),
            Some(&Value::Bool(false))
        );
        assert!(!observed.contains_key("result"));
        assert!(completed.is_err(), "late native result must be withheld");
        assert_eq!(calls, 1);
        let Value::Record(final_state) = status(&env.task_scope, &job, Span::point(0)).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(
            final_state.get("status"),
            Some(&Value::String(
                if expires { "timed_out" } else { "cancelled" }.into()
            ))
        );
        assert_eq!(
            final_state.get("cancel_acknowledged"),
            Some(&Value::Bool(true))
        );
        assert!(!final_state.contains_key("result"));
    }
}
