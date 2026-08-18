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
    pub published: Vec<String>,
    pub attached: bool,
}

impl PoetSnapshot {
    pub fn with_seed(quins: Vec<NQuin>) -> Self {
        Self {
            committed: quins,
            staged: Vec::new(),
            revision: 1,
            published: Vec::new(),
            attached: false,
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
        let mut env = Env::default();
        eval_cell(src, self, &mut env)
    }

    pub fn eval_fn(&mut self, src: &str, name: &str, args: Vec<Value>) -> Result<Value, Diagnostic> {
        let program = load_program(src)?;
        let mut env = Env::default();
        eval_function(&program, name, args, self, &mut env)
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
        _payload: &Value,
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
        self.published.push(topic.to_string());
        Ok(Value::Null)
    }

    fn time_unix(&mut self, span: poet_vibe::Span) -> Result<Value, Diagnostic> {
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
        Ok(catalog::resolve_id(id))
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
}

