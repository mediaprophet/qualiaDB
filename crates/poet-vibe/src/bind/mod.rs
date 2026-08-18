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
    /// in Pure cells. Default returns 0 (WASM / hosts without a clock); native
    /// hosts override with `SystemTime::now`. Replay uses the receipt clock,
    /// not this binding.
    fn time_unix(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::I64(0))
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
#[derive(Default)]
pub struct MockHost {
    pub staged: usize,
    pub committed: usize,
    pub published: Vec<String>,
    pub query_rows: usize,
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

    /// Deterministic epoch for unit tests so assertions don't depend on wall time.
    fn time_unix(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::I64(0))
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
