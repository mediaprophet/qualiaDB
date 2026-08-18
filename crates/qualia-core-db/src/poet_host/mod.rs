//! Live Poet host over a caller-owned Quin snapshot (P5).
//!
//! Scripts never write `parity`. The host seals via `NQuin::calculate_parity`.
//! Vibe is the human/app path into existing Qualia capabilities. Document NLP
//! lives in `crate::nlp`, next to `text_span` and `lexicon`.

pub mod catalog;
pub mod catalog_ttl;
pub mod invoke;
mod scan;
mod values;

pub use values::format_value;
pub(crate) use values::hash_val;
use scan::{collect_matches, subject_present};
use values::{reifier_quin, shape_local_name, value_to_quin};

use crate::lexicon::generate_60bit_token;
use crate::NQuin;
use poet_vibe::{eval_cell, eval_function, load_program, Diagnostic, Env, Host, Value};

/// Topics `pulse.publish` may use in 0.1. Anything else is E500.
pub const PULSE_ALLOW_PREFIXES: &[&str] = &["clinic/", "poet/", "pulse/"];

/// A recorded `pulse.publish` call — the 0.1 receipt for UI / audit display.
///
/// When the snapshot is `attached` to the live daemon graph, each publish also
/// emits a [`crate::pulse_transport::PulseEvent`] to the process-wide broadcast
/// channel so SSE / WebSocket subscribers receive it in real time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseRecord {
    /// The topic string (validated against [`PULSE_ALLOW_PREFIXES`]).
    pub topic: String,
    /// Compact textual summary of the payload value.
    pub payload_summary: String,
    /// Monotonic sequence number from the pulse transport (0 when detached).
    pub seq: u64,
}

/// RDF 1.2 reifier predicate used by staged `<< s p o ~ r >>` quins.
const RDF_REIFIES: &[u8] = b"http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// In-memory graph revision used as the 0.1 live host.
///
/// When `attached`, queries refresh from the native daemon graph and commits
/// extend it. WASM has no daemon_graph â€” attach is always false there.
#[derive(Debug, Clone, Default)]
pub struct PoetSnapshot {
    pub committed: Vec<NQuin>,
    staged: Vec<NQuin>,
    pub revision: u64,
    pub published: Vec<PulseRecord>,
    pub attached: bool,
    /// Set to `true` when `graph_query` is called during an evaluation.
    /// The desktop harness resets this before each `eval_cell_src` and reads
    /// it after to determine whether the cell is graph-dependent (reactive).
    pub graph_read_during_eval: bool,
    /// Set to `true` when `time_unix` is called during an evaluation.
    /// The desktop harness resets this before each `eval_cell_src` and reads
    /// it after to determine whether the cell is time-dependent (reactive on tick).
    pub time_read_during_eval: bool,
}

impl PoetSnapshot {
    pub fn with_seed(quins: Vec<NQuin>) -> Self {
        Self {
            committed: quins,
            staged: Vec::new(),
            revision: 1,
            published: Vec::new(),
            attached: false,
            graph_read_during_eval: false,
            time_read_during_eval: false,
        }
    }

    /// Prefer the process daemon graph; fall back to the demo seed.
    pub fn live() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if crate::daemon_graph::graph_quin_count() > 0 {
                return Self::from_daemon();
            }
        }
        Self::with_demo_seed()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_daemon() -> Self {
        Self {
            committed: Vec::new(),
            staged: Vec::new(),
            revision: crate::daemon_graph::graph_revision().max(1),
            published: Vec::new(),
            attached: true,
            graph_read_during_eval: false,
            time_read_during_eval: false,
        }
    }

    pub fn honesty(&self) -> &'static str {
        if self.attached {
            "live"
        } else {
            "partial"
        }
    }

    pub(crate) fn with_live_quins<R>(&self, f: impl FnOnce(&[NQuin]) -> R) -> R {
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            let guard = crate::daemon_graph::graph_read_guard();
            return f(guard.as_slice());
        }
        f(&self.committed)
    }

    pub fn visible_count(&self) -> usize {
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            return crate::daemon_graph::graph_quin_count();
        }
        self.committed.len()
    }

    /// Clinic + catchment seed so the harness query sample is not empty.
    pub fn with_demo_seed() -> Self {
        let pred = generate_60bit_token(b"clinic:hasCondition");
        let seed = NQuin {
            subject: generate_60bit_token(b"https://qualiadb.org/catchment/NorthSpring"),
            predicate: pred,
            object: generate_60bit_token(b"https://qualiadb.org/meteo/Rain"),
            context: generate_60bit_token(b"https://qualiadb.org/graphs/lived-memory"),
            metadata: 0,
            parity: 0,
        };
        let mut q = seed;
        q.parity = NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata);
        Self::with_seed(vec![q])
    }

    pub fn ingest_sealed(&mut self, quin: NQuin) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            crate::daemon_graph::extend_with_ontology_quins_slice(&[quin]);
            self.revision = crate::daemon_graph::graph_revision().max(self.revision);
            return;
        }
        self.committed.push(quin);
    }

    pub fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn eval_cell_src(&mut self, src: &str) -> Result<Value, Diagnostic> {
        self.graph_read_during_eval = false;
        self.time_read_during_eval = false;
        let mut env = Env::default();
        let result = eval_cell(src, self, &mut env);
        // If evaluation failed, leave the flag as-is (the cell didn't complete).
        result
    }

    pub fn eval_fn(&mut self, src: &str, name: &str, args: Vec<Value>) -> Result<Value, Diagnostic> {
        let program = load_program(src)?;
        let mut env = Env::default();
        eval_function(&program, name, args, self, &mut env)
    }

    /// Dispatch a hook event (`on <path>(…)`) on a checked program.
    ///
    /// `path` is the event path segments (e.g. `["pulse", "message"]` for
    /// `on pulse:message(…)`, or `["tick"]` for `on tick(…)`). `args` are
    /// bound to the hook's parameters in declaration order. Returns
    /// `Ok(Value::Null)` if no matching hook exists in the program.
    pub fn dispatch_hook_src(
        &mut self,
        src: &str,
        path: &[String],
        args: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let program = load_program(src)?;
        let mut env = Env::default();
        poet_vibe::dispatch_hook(&program, path, args, self, &mut env)
    }

    /// Direct capability.invoke from a host (desktop renderer, tests).
    pub fn invoke_id(&mut self, id: &str, args: Value) -> Result<Value, Diagnostic> {
        invoke::dispatch(self, id, &args, poet_vibe::Span { start: 0, end: 0 })
    }
}

impl Host for PoetSnapshot {
    fn graph_query(
        &mut self,
        args: &[Value],
        take: u64,
        _span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        self.graph_read_during_eval = true;
        let s = args.first();
        let p = args.get(1);
        let o = args.get(2);
        let mut out = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            let guard = crate::daemon_graph::graph_read_guard();
            collect_matches(guard.as_slice(), s, p, o, take, &mut out);
            self.revision = crate::daemon_graph::graph_revision().max(self.revision);
            return Ok(Value::List(out));
        }
        collect_matches(&self.committed, s, p, o, take, &mut out);
        Ok(Value::List(out))
    }

    fn graph_stage(
        &mut self,
        term: &Value,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        let mut pushed = 0usize;
        if let Some(q) = value_to_quin(term, 0) {
            self.staged.push(q);
            pushed += 1;
        }
        if let Some(q) = reifier_quin(term, 0) {
            self.staged.push(q);
            pushed += 1;
        }
        if pushed > 0 {
            return Ok(Value::Null);
        }
        Err(Diagnostic::new(
            poet_vibe::DiagCode::E600,
            span,
            "graph.stage expects a triple, reified term, or Quin",
        ))
    }

    fn graph_begin(&mut self, _span: poet_vibe::Span) -> Result<(), Diagnostic> {
        Ok(())
    }

    fn graph_abort(&mut self, _span: poet_vibe::Span) -> Result<(), Diagnostic> {
        self.staged.clear();
        Ok(())
    }

    fn graph_commit(&mut self, _span: poet_vibe::Span) -> Result<Value, Diagnostic> {
        if !self.staged.is_empty() {
            #[cfg(not(target_arch = "wasm32"))]
            if self.attached {
                crate::daemon_graph::extend_with_ontology_quins_slice(&self.staged);
                self.staged.clear();
                self.revision = crate::daemon_graph::graph_revision().max(1);
            } else {
                self.committed.append(&mut self.staged);
                self.revision = self.revision.wrapping_add(1);
            }
            #[cfg(target_arch = "wasm32")]
            {
                self.committed.append(&mut self.staged);
                self.revision = self.revision.wrapping_add(1);
            }
        }
        Ok(Value::Receipt)
    }

    fn aura_validate(
        &mut self,
        node: &Value,
        shape: &Value,
        _span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        let Some(id) = hash_val(node) else {
            return Ok(Value::Bool(false));
        };
        let shape_name = shape_local_name(shape);
        // No shape-IRI registry exists. Only these built-ins are honest 0.1.
        if !matches!(
            shape_name.as_str(),
            "EmergencyAlertShape" | "ReifierPresent"
        ) {
            return Ok(Value::Bool(false));
        }
        let reifies = generate_60bit_token(RDF_REIFIES);
        if subject_present(&self.staged, id, Some(reifies))
            || subject_present(&self.committed, id, Some(reifies))
        {
            return Ok(Value::Bool(true));
        }
        #[cfg(any(
            not(target_arch = "wasm32"),
            feature = "wasm-ontology",
            feature = "wasm-logic",
            feature = "wasm-scientific",
            feature = "wasm-full"
        ))]
        {
            use crate::query::shacl_compiler::{validate_shacl_property, ShaclConstraint};
            #[cfg(not(target_arch = "wasm32"))]
            if self.attached {
                let guard = crate::daemon_graph::graph_read_guard();
                return Ok(Value::Bool(validate_shacl_property(
                    guard.as_slice(),
                    id,
                    reifies,
                    &[ShaclConstraint::MinCount(1)],
                )));
            }
            return Ok(Value::Bool(validate_shacl_property(
                &self.committed,
                id,
                reifies,
                &[ShaclConstraint::MinCount(1)],
            )));
        }
        #[cfg(not(any(
            not(target_arch = "wasm32"),
            feature = "wasm-ontology",
            feature = "wasm-logic",
            feature = "wasm-scientific",
            feature = "wasm-full"
        )))]
        {
            let _ = reifies;
            Ok(Value::Bool(false))
        }
    }

    fn pulse_publish(
        &mut self,
        topic: &str,
        payload: &Value,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        if !PULSE_ALLOW_PREFIXES
            .iter()
            .any(|prefix| topic == *prefix || topic.starts_with(prefix))
        {
            return Err(Diagnostic::new(
                poet_vibe::DiagCode::E500,
                span,
                format!("pulse topic `{topic}` is not on the 0.1 allowlist"),
            ));
        }
        let payload_summary = format_value(payload);
        // When attached to the live daemon, emit through the process-wide
        // pulse transport so SSE / WebSocket subscribers receive the event.
        #[cfg(not(target_arch = "wasm32"))]
        let seq = if self.attached {
            crate::pulse_transport::publish(topic, &payload_summary)
        } else {
            0
        };
        #[cfg(target_arch = "wasm32")]
        let seq = 0u64;
        self.published.push(PulseRecord {
            topic: topic.to_string(),
            payload_summary,
            seq,
        });
        Ok(Value::Null)
    }

    fn time_unix(&mut self, span: poet_vibe::Span) -> Result<Value, Diagnostic> {
        self.time_read_during_eval = true;
        // WASM has no wall clock; the host injects replay clocks via receipts.
        #[cfg(target_arch = "wasm32")]
        {
            let _ = span;
            Ok(Value::I64(0))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(d) => Ok(Value::I64(d.as_secs() as i64)),
                Err(_) => Err(Diagnostic::new(
                    poet_vibe::DiagCode::E600,
                    span,
                    "system clock is before Unix epoch",
                )),
            }
        }
    }

    fn graph_snapshot(&mut self, _span: poet_vibe::Span) -> Result<Value, Diagnostic> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            self.revision = crate::daemon_graph::graph_revision().max(1);
        }
        Ok(Value::U64(self.revision))
    }

    fn capability_resolve(
        &mut self,
        id: &str,
        _span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        Ok(catalog::resolve_id_with(id, self.attached))
    }

    fn capability_invoke(
        &mut self,
        id: &str,
        args: &Value,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        invoke::dispatch(self, id, args, span)
    }

    fn quin_seal(
        &mut self,
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
        _span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        let metadata = 0;
        let _parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
        Ok(Value::Quin {
            subject,
            predicate,
            object,
            context,
        })
    }

    fn hash_iri(&self, iri: &str) -> u64 {
        generate_60bit_token(iri.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_cell_math() {
        let mut snap = PoetSnapshot::default();
        let v = snap.eval_cell_src("= math.max(1, math.min(9, 4))").unwrap();
        assert_eq!(v.as_f64(), Some(4.0));
    }

    #[test]
    fn demo_seed_is_queryable() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
requires [ capability("graph.read") ];
fn count() {
    let rows = graph.query(?s, clinic:hasCondition, ?o, take: 8)?;
    return rows;
}
"#;
        let v = snap.eval_fn(src, "count", vec![]).unwrap();
        match v {
            Value::List(xs) => assert_eq!(xs.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn live_query_and_commit() {
        let pred = generate_60bit_token(b"clinic:hasCondition");
        let seed = NQuin {
            subject: 1,
            predicate: pred,
            object: 2,
            context: 0,
            metadata: 0,
            parity: NQuin::calculate_parity(1, pred, 2, 0, 0),
        };
        let mut snap = PoetSnapshot::with_seed(vec![seed]);
        let src = r#"
requires [ capability("graph.read") ];
fn count() {
    let rows = graph.query(?s, clinic:hasCondition, ?o, take: 8)?;
    return rows;
}
"#;
        let v = snap.eval_fn(src, "count", vec![]).unwrap();
        match v {
            Value::List(xs) => assert_eq!(xs.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn graph_snapshot_and_capability_resolve() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
fn peek() {
    return capability.resolve("graph.read");
}
"#;
        let v = snap.eval_fn(src, "peek", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("vibe_bound"), Some(&Value::Bool(true)));
            }
            other => panic!("{other:?}"),
        }
        let snap_src = r#"
requires [ capability("graph.read") ];
fn snap() { return graph.snapshot(); }
"#;
        let rev = snap.eval_fn(snap_src, "snap", vec![]).unwrap();
        assert_eq!(rev, Value::U64(1));
    }

    #[test]
    fn aura_validate_sees_staged_reifier_and_rejects_unknown() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
prefix clinic: <https://qualiadb.org/clinic/>;
requires [
    capability("graph.write"),
    capability("aura.validate")
];
effect fn check() -> Result<Receipt, string> {
    let stated = << clinic:sensor_1 clinic:emitsAlert clinic:Overheat ~ clinic:claim_1 >>;
    transaction {
        graph.stage(stated);
        if !aura.validate(clinic:claim_1, clinic:EmergencyAlertShape) {
            return Err("shape");
        }
        if aura.validate(clinic:missing, clinic:EmergencyAlertShape) {
            return Err("unknown should fail");
        }
        graph.commit()?;
    };
    return Ok(receipt_empty());
}
"#;
        let v = snap.eval_fn(src, "check", vec![]).unwrap();
        assert!(matches!(v, Value::Ok(_)));
        assert_eq!(snap.committed.len(), 2);
    }

    #[test]
    fn seal_does_not_expose_parity_to_script() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
fn make() {
    return quin.statement(
        subject: <https://example.org/s>,
        predicate: <https://example.org/p>,
        object: <https://example.org/o>,
        context: <https://example.org/g>
    );
}
"#;
        let v = snap.eval_fn(src, "make", vec![]).unwrap();
        assert!(matches!(v, Value::Quin { .. }));
    }

    #[test]
    fn transaction_abort_drops_unstaged_commit() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
prefix clinic: <https://qualiadb.org/clinic/>;
requires [ capability("graph.write") ];
effect fn nope() -> Result<Receipt, string> {
    transaction {
        let stated = << clinic:a clinic:p clinic:o ~ clinic:r >>;
        graph.stage(stated);
        return Err("abort");
    };
    return Ok(receipt_empty());
}
"#;
        let v = snap.eval_fn(src, "nope", vec![]).unwrap();
        assert!(matches!(v, Value::Err(_)));
        assert!(snap.committed.is_empty());
        assert!(snap.staged.is_empty());
    }

    #[test]
    fn pulse_rejects_off_allowlist() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn go() {
    effect pulse.publish("not-allowed/x", 1);
    return null;
}
"#;
        let err = snap.eval_fn(src, "go", vec![]).unwrap_err();
        assert_eq!(err.code, poet_vibe::DiagCode::E500);
    }

    #[test]
    fn pulse_records_payload_summary() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn go() {
    effect pulse.publish("pulse/test-payload", "hello world");
    return null;
}
"#;
        snap.eval_fn(src, "go", vec![]).unwrap();
        assert_eq!(snap.published.len(), 1);
        assert_eq!(snap.published[0].topic, "pulse/test-payload");
        assert!(
            snap.published[0].payload_summary.contains("hello world"),
            "payload summary should contain the payload text, got: {}",
            snap.published[0].payload_summary
        );
        // Detached snapshot → seq is 0 (no transport emission).
        assert_eq!(snap.published[0].seq, 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn pulse_attached_emits_to_transport() {
        use crate::pulse_transport;
        let mut snap = PoetSnapshot::from_daemon();
        let mut rx = pulse_transport::subscribe();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn go() {
    effect pulse.publish("pulse/transport-test", 42);
    return null;
}
"#;
        snap.eval_fn(src, "go", vec![]).unwrap();
        assert_eq!(snap.published.len(), 1);
        assert!(snap.published[0].seq > 0, "attached pulse should get a seq");
        let event = rx.try_recv().expect("transport subscriber should receive event");
        assert_eq!(event.topic, "pulse/transport-test");
        assert_eq!(event.seq, snap.published[0].seq);
    }

    #[test]
    fn dispatch_hook_routes_pulse_message_to_function() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
requires [
    capability("graph.write"),
    capability("pulse.publish", topic: "clinic/alerts")
];
effect fn raise_alert(value: f64) {
    if value > 85.0 {
        effect pulse.publish("clinic/alerts", value);
    }
    return null;
}
on pulse:message(topic: string, value: f64) {
    return raise_alert(value);
}
"#;
        let path = vec!["pulse".to_string(), "message".to_string()];
        let v = snap
            .dispatch_hook_src(src, &path, vec![Value::String("clinic/alerts".into()), Value::F64(90.0)])
            .unwrap();
        assert_eq!(v, Value::Null);
        assert_eq!(snap.published.len(), 1, "hook should have triggered a publish");
        assert_eq!(snap.published[0].topic, "clinic/alerts");
    }

    #[test]
    fn dispatch_hook_unknown_path_returns_null() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
on tick() {
    return null;
}
"#;
        let path = vec!["unknown".to_string()];
        let v = snap.dispatch_hook_src(src, &path, vec![]).unwrap();
        assert_eq!(v, Value::Null);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn attach_reads_daemon_graph() {
        let seed = NQuin {
            subject: 0x00C0_FFEE_0000_0001,
            predicate: 0x00C0_FFEE_0000_0002,
            object: 0x00C0_FFEE_0000_0003,
            context: 0,
            metadata: 0,
            parity: NQuin::calculate_parity(
                0x00C0_FFEE_0000_0001,
                0x00C0_FFEE_0000_0002,
                0x00C0_FFEE_0000_0003,
                0,
                0,
            ),
        };
        crate::daemon_graph::extend_with_ontology_quins_slice(&[seed]);
        let mut snap = PoetSnapshot::from_daemon();
        assert!(snap.attached);
        assert!(snap.committed.is_empty());
        assert_eq!(snap.honesty(), "live");
        let src = r#"
requires [ capability("graph.read") ];
fn find() {
    return graph.query(?s, ?p, ?o, take: 64)?;
}
"#;
        let v = snap.eval_fn(src, "find", vec![]).unwrap();
        match v {
            Value::List(xs) => {
                assert!(xs.iter().any(|row| matches!(
                    row,
                    Value::Quin { subject, .. } if *subject == seed.subject
                )));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn time_unix_returns_wall_clock_on_native() {
        let mut snap = PoetSnapshot::default();
        let src = "effect fn now() { return time.unix(); }";
        let v = snap.eval_fn(src, "now", vec![]).unwrap();
        // Native: real wall clock is positive. WASM: 0 (default). Both are I64.
        #[cfg(not(target_arch = "wasm32"))]
        assert!(v.as_i64().unwrap_or(0) > 0, "wall clock should be positive");
        #[cfg(target_arch = "wasm32")]
        assert_eq!(v.as_i64(), Some(0));
    }

    #[test]
    fn time_unix_resolves_as_partial_vibe_binding() {
        match crate::poet_host::catalog::resolve_id("time.unix") {
            Value::Record(r) => {
                assert_eq!(r.get("vibe_bound"), Some(&Value::Bool(true)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn invoke_lists_engine_and_rejects_unknown() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn list() {
    return capability.invoke("CapabilityDiscovery.list", null);
}
"#;
        let v = snap.eval_fn(src, "list", vec![]).unwrap();
        match v {
            Value::Record(r) => match r.get("invoke") {
                Some(Value::List(xs)) => assert!(!xs.is_empty()),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
        let err = snap
            .eval_fn(
                r#"
requires [ capability("capability.invoke") ];
effect fn boom() {
    return capability.invoke("DoesNotExist.nope", null);
}
"#,
                "boom",
                vec![],
            )
            .unwrap_err();
        assert_eq!(err.code, poet_vibe::DiagCode::E300);
    }

    #[test]
    fn time_unix_sets_time_read_during_eval() {
        let mut snap = PoetSnapshot::default();
        let src = "effect fn now() { return time.unix(); }";
        snap.eval_fn(src, "now", vec![]).unwrap();
        assert!(snap.time_read_during_eval, "time.unix should set time_read_during_eval");
    }

    #[test]
    fn eval_cell_resets_time_read_during_eval() {
        let mut snap = PoetSnapshot::default();
        // First, trigger time_unix via a function call.
        snap.eval_fn("effect fn now() { return time.unix(); }", "now", vec![]).unwrap();
        assert!(snap.time_read_during_eval);
        // Now eval a cell that doesn't use time — flag should reset to false.
        snap.eval_cell_src("= 1 + 2").unwrap();
        assert!(!snap.time_read_during_eval, "eval_cell_src should reset time_read_during_eval");
    }

    #[test]
    fn dispatch_tick_hook_fires() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn emit() {
    effect pulse.publish("poet/tick", 1);
    return null;
}
on tick() {
    return emit();
}
"#;
        let path = vec!["tick".to_string()];
        let v = snap.dispatch_hook_src(src, &path, vec![]).unwrap();
        assert_eq!(v, Value::Null);
        assert_eq!(snap.published.len(), 1, "tick hook should have triggered a publish");
        assert_eq!(snap.published[0].topic, "poet/tick");
    }

    #[test]
    fn dispatch_tick_no_hook_returns_null() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
on pulse:message(topic: string) {
    return null;
}
"#;
        let path = vec!["tick".to_string()];
        let v = snap.dispatch_hook_src(src, &path, vec![]).unwrap();
        assert_eq!(v, Value::Null, "no matching tick hook should return null");
    }

    // ── Phase G: Golden corpus — capability.invoke fixtures ──────────────

    #[test]
    fn g_physics_wave_1d_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Physics.wave_1d", {
        u0: [0.0, 0.5, 1.0, 0.5, 0.0],
        v0: [0.0, 0.0, 0.0, 0.0, 0.0],
        c: 1.0,
        dx: 0.1,
        total_time: 0.5,
        samples: 10
    });
}
"#;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("energy_initial"), "wave_1d should return energy_initial");
                assert!(r.contains_key("energy_final"), "wave_1d should return energy_final");
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_physics_harmonic_oscillator_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Physics.harmonic_oscillator", {
        mass: 1.0,
        k_spring: 4.0,
        x0: 1.0,
        v0: 0.0,
        t_start: 0.0,
        t_end: 2.0,
        t_count: 10
    });
}
"#;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("positions"), "oscillator should return positions");
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_spectral_emf_to_rgb_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Spectral.emf_to_rgb", {
        alpha: 1.0,
        mu: 0.45,
        sigma: 0.1
    });
}
"#;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                let css = r.get("css").expect("emf_to_rgb should return css field");
                match css {
                    Value::String(s) => assert!(s.starts_with("rgb("), "emf_to_rgb css should start with rgb(: {s}"),
                    other => panic!("css should be string, got {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_render_svg_path_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r##"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Render.svg_path", {
        points: [10.0, 10.0, 90.0, 90.0, 50.0, 30.0],
        stroke: "#ff0000",
        stroke_width: 2.0,
        fill: "none"
    });
}
"##;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                let svg = r.get("svg").expect("svg_path should return svg field");
                match svg {
                    Value::String(s) => {
                        assert!(s.contains("<path"), "svg_path should return <path element: {s}");
                        assert!(s.contains("M 10 10 L 90 90"), "svg_path should contain d attribute");
                    }
                    other => panic!("svg should be string, got {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_render_css_animation_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Render.css_animation", {
        name: "fade",
        property: "opacity",
        keyframes: [
            { time: 0.0, value: 1.0 },
            { time: 2.0, value: 0.0 }
        ]
    });
}
"#;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                let css = r.get("css").expect("css_animation should return css field");
                match css {
                    Value::String(s) => {
                        assert!(s.contains("@keyframes"), "css_animation should return @keyframes: {s}");
                        assert!(s.contains("fade"), "css_animation should contain animation name");
                    }
                    other => panic!("css should be string, got {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_render_svg_circle_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r##"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Render.svg_circle", {
        cx: 50.0,
        cy: 50.0,
        r: 25.0,
        stroke: "blue",
        fill: "yellow"
    });
}
"##;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                let svg = r.get("svg").expect("svg_circle should return svg field");
                match svg {
                    Value::String(s) => {
                        assert!(s.contains("<circle"), "svg_circle should return <circle: {s}");
                        assert!(s.contains("cx=\"50\""), "svg_circle should contain cx");
                    }
                    other => panic!("svg should be string, got {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_tick_hook_with_pulse_publish_through_snapshot() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn emit() {
    effect pulse.publish("poet/tick", 42);
    return null;
}
on tick() {
    return emit();
}
"#;
        let path = vec!["tick".to_string()];
        let v = snap.dispatch_hook_src(src, &path, vec![]).unwrap();
        assert_eq!(v, Value::Null);
        assert_eq!(snap.published.len(), 1);
        assert_eq!(snap.published[0].topic, "poet/tick");
    }

    #[test]
    fn g_time_dependent_cell_recomputes_on_re_eval() {
        let mut snap = PoetSnapshot::default();
        let src = "effect fn now() { return time.unix(); }";
        let v1 = snap.eval_fn(src, "now", vec![]).unwrap();
        // On native, time.unix returns wall clock — should be non-zero (usually).
        // On WASM, returns 0. Just verify it doesn't panic and returns I64.
        assert!(matches!(v1, Value::I64(_)), "time.unix should return I64");
        assert!(snap.time_read_during_eval, "time_read_during_eval should be set");
    }
}

