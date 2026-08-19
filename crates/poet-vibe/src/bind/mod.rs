//! 0.1 host bindings.

mod math;
mod quin;
mod rdf;

pub use math::call_math;
pub use quin::call_quin;
pub use rdf::call_rdf;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Host supplied by Qualia / tests.
pub trait Host {
    fn graph_query(
        &mut self,
        args: &[Value],
        take: u64,
        span: Span,
    ) -> Result<Value, Diagnostic>;

    fn graph_stage(&mut self, term: &Value, span: Span) -> Result<Value, Diagnostic>;

    fn graph_commit(&mut self, span: Span) -> Result<Value, Diagnostic>;

    /// Open a transaction. Default is a no-op.
    fn graph_begin(&mut self, _span: Span) -> Result<(), Diagnostic> {
        Ok(())
    }

    /// Drop uncommitted staged work. Default is a no-op.
    fn graph_abort(&mut self, _span: Span) -> Result<(), Diagnostic> {
        Ok(())
    }

    fn aura_validate(&mut self, node: &Value, shape: &Value, span: Span)
        -> Result<Value, Diagnostic>;

    fn pulse_publish(&mut self, topic: &str, payload: &Value, span: Span)
        -> Result<Value, Diagnostic>;

    /// Wall clock as seconds since Unix epoch. External (core §11): forbidden
    /// in Pure cells. Default fails closed with E702 (WASM / hosts without a clock);
    /// native hosts override with `SystemTime::now`. Replay uses the receipt clock,
    /// not this binding.
    fn time_unix(&mut self, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "no clock available on this host",
        ))
    }

    /// Structured Unix time: `{ secs: I64, nanos: U64 }` (T19).
    /// Default calls `time_unix` and wraps it into a Record.
    fn time_unix_nanos(&mut self, span: Span) -> Result<Value, Diagnostic> {
        let secs_val = self.time_unix(span)?;
        let secs = secs_val.as_i64().unwrap_or(0);
        let mut rec = std::collections::BTreeMap::new();
        rec.insert("secs".into(), Value::I64(secs));
        rec.insert("nanos".into(), Value::U64(0));
        Ok(Value::Record(rec))
    }

    /// Monotonic nanos for frame timing and physics dt (T20).
    /// Default returns 0 (WASM has no monotonic clock until W12 replay Instant).
    fn time_monotonic_nanos(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::U64(0))
    }

    fn quin_seal(
        &mut self,
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let _ = span;
        Ok(Value::Quin {
            subject,
            predicate,
            object,
            context,
        })
    }

    fn hash_iri(&self, iri: &str) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        for b in iri.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn graph_snapshot(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::U64(0))
    }

    fn capability_resolve(&mut self, id: &str, _span: Span) -> Result<Value, Diagnostic> {
        let mut rec = std::collections::BTreeMap::new();
        rec.insert("id".into(), Value::String(id.into()));
        rec.insert("vibe_bound".into(), Value::Bool(true));
        rec.insert("honesty".into(), Value::String("mock".into()));
        Ok(Value::Record(rec))
    }

    /// Reach an engine capability by id. Default fails closed.
    fn capability_invoke(
        &mut self,
        id: &str,
        _args: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E300,
            span,
            format!("capability.invoke not bound on this host: {id}"),
        ))
    }
}

/// In-memory host for unit tests.
pub struct MockHost {
    pub staged: usize,
    pub committed: usize,
    pub published: Vec<String>,
    pub query_rows: usize,
    pub clock_time: Option<i64>,
    pub monotonic_time: u64,
}

impl Default for MockHost {
    fn default() -> Self {
        Self {
            staged: 0,
            committed: 0,
            published: Vec::new(),
            query_rows: 0,
            clock_time: Some(1_000_000_000),
            monotonic_time: 0,
        }
    }
}

impl Host for MockHost {
    fn graph_query(
        &mut self,
        _args: &[Value],
        take: u64,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        let n = self.query_rows.min(take as usize);
        Ok(Value::List(vec![Value::I64(1); n]))
    }

    fn graph_stage(&mut self, _term: &Value, _span: Span) -> Result<Value, Diagnostic> {
        self.staged += 1;
        Ok(Value::Null)
    }

    fn graph_commit(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        self.committed += 1;
        Ok(Value::Receipt)
    }

    fn aura_validate(
        &mut self,
        _node: &Value,
        _shape: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Bool(true))
    }

    fn pulse_publish(
        &mut self,
        topic: &str,
        _payload: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        self.published.push(topic.to_string());
        Ok(Value::Null)
    }

    /// Deterministic epoch for unit tests.
    fn time_unix(&mut self, span: Span) -> Result<Value, Diagnostic> {
        match self.clock_time {
            Some(t) => Ok(Value::I64(t)),
            None => Err(Diagnostic::new(
                DiagCode::E702,
                span,
                "no clock available on this mock host",
            )),
        }
    }

    fn time_unix_nanos(&mut self, span: Span) -> Result<Value, Diagnostic> {
        match self.clock_time {
            Some(t) => {
                let mut rec = std::collections::BTreeMap::new();
                rec.insert("secs".into(), Value::I64(t));
                rec.insert("nanos".into(), Value::U64(500_000));
                Ok(Value::Record(rec))
            }
            None => Err(Diagnostic::new(
                DiagCode::E702,
                span,
                "no clock available on this mock host",
            )),
        }
    }

    fn time_monotonic_nanos(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::U64(self.monotonic_time))
    }
}

pub fn dispatch<H: Host>(
    host: &mut H,
    path: &str,
    args: &[Value],
    named: &[(String, Value)],
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Some(v) = call_math(path, args, span)? {
        return Ok(v);
    }
    if let Some(v) = call_rdf(path, args, span)? {
        return Ok(v);
    }
    if let Some(v) = call_quin(host, path, named, span)? {
        return Ok(v);
    }
    match path {
        "receipt_empty" => Ok(Value::Receipt),
        "graph.query" => {
            let take = named
                .iter()
                .find(|(k, _)| k == "take")
                .and_then(|(_, v)| v.as_i64())
                .unwrap_or(0) as u64;
            host.graph_query(args, take, span)
        }
        "graph.stage" => {
            let term = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "graph.stage needs a term")
            })?;
            host.graph_stage(term, span)
        }
        "graph.commit" => host.graph_commit(span),
        "aura.validate" => {
            let n = args.first().unwrap_or(&Value::Null);
            let s = args.get(1).unwrap_or(&Value::Null);
            host.aura_validate(n, s, span)
        }
        "pulse.publish" => {
            let topic = match args.first() {
                Some(Value::String(t)) => t.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "pulse.publish topic must be a string",
                    ))
                }
            };
            let payload = args.get(1).cloned().unwrap_or(Value::Null);
            host.pulse_publish(&topic, &payload, span)
        }
        "graph.snapshot" => host.graph_snapshot(span),
        "time.unix" => host.time_unix(span),
        "time.unix_nanos" => host.time_unix_nanos(span),
        "time.monotonic_nanos" => host.time_monotonic_nanos(span),
        "capability.resolve" => {
            let id = match args.first() {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Iri(s)) => s.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "capability.resolve needs a string id",
                    ))
                }
            };
            host.capability_resolve(&id, span)
        }
        "capability.invoke" => {
            let id = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "capability.invoke needs a string id",
                    ))
                }
            };
            let payload = args.get(1).cloned().unwrap_or(Value::Null);
            host.capability_invoke(&id, &payload, span)
        }
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("unknown binding {path}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoClockHost;

    impl Host for NoClockHost {
        fn graph_query(
            &mut self,
            _args: &[Value],
            _take: u64,
            _span: Span,
        ) -> Result<Value, Diagnostic> {
            Ok(Value::Null)
        }
        fn graph_stage(&mut self, _term: &Value, _span: Span) -> Result<Value, Diagnostic> {
            Ok(Value::Null)
        }
        fn graph_commit(&mut self, _span: Span) -> Result<Value, Diagnostic> {
            Ok(Value::Null)
        }
        fn aura_validate(&mut self, _node: &Value, _shape: &Value, _span: Span) -> Result<Value, Diagnostic> {
            Ok(Value::Bool(true))
        }
        fn pulse_publish(&mut self, _topic: &str, _payload: &Value, _span: Span) -> Result<Value, Diagnostic> {
            Ok(Value::Null)
        }
    }

    #[test]
    fn mock_host_time_returns_some() {
        let mut host = MockHost::default();
        let res = host.time_unix(Span::point(0));
        assert_eq!(res.unwrap(), Value::I64(1_000_000_000));
    }

    #[test]
    fn no_clock_host_time_returns_diagnostic() {
        let mut host = NoClockHost;
        let res = host.time_unix(Span::point(0));
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn time_unix_nanos_none_when_no_clock() {
        let mut host = NoClockHost;
        let res = host.time_unix_nanos(Span::point(0));
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn time_unix_nanos_returns_structured() {
        let mut host = MockHost::default();
        let val = host.time_unix_nanos(Span::point(0)).unwrap();
        match val {
            Value::Record(r) => {
                assert_eq!(r.get("secs"), Some(&Value::I64(1_000_000_000)));
                assert_eq!(r.get("nanos"), Some(&Value::U64(500_000)));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn time_monotonic_nanos_default_zero() {
        let mut host = MockHost::default();
        assert_eq!(host.time_monotonic_nanos(Span::point(0)).unwrap(), Value::U64(0));
    }

    #[test]
    fn time_monotonic_nanos_custom() {
        let mut host = MockHost {
            monotonic_time: 42_000_000,
            ..Default::default()
        };
        assert_eq!(
            host.time_monotonic_nanos(Span::point(0)).unwrap(),
            Value::U64(42_000_000)
        );
    }
}
