//! In-process Vibe host.

use super::Host;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::{Instant, Value};

/// In-process host: graph/pulse/time in memory, catalog kernels for Animation/etc.
pub struct LocalHost {
    pub staged: usize,
    pub committed: usize,
    pub published: Vec<String>,
    pub query_rows: usize,
    pub clock_time: Option<i64>,
    pub monotonic_time: u64,
    #[doc(hidden)]
    pub tx_checkpoints: Vec<(usize, usize)>,
}

impl Default for LocalHost {
    fn default() -> Self {
        Self {
            staged: 0,
            committed: 0,
            published: Vec::new(),
            query_rows: 0,
            clock_time: Some(1_000_000_000),
            monotonic_time: 0,
            tx_checkpoints: Vec::new(),
        }
    }
}

impl Host for LocalHost {
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

    fn graph_begin(&mut self, _span: Span) -> Result<(), Diagnostic> {
        self.tx_checkpoints.push((self.staged, self.committed));
        Ok(())
    }

    fn graph_abort(&mut self, _span: Span) -> Result<(), Diagnostic> {
        if let Some((saved_staged, _saved_committed)) = self.tx_checkpoints.pop() {
            self.staged = saved_staged;
        }
        Ok(())
    }

    fn graph_end(&mut self, span: Span) -> Result<(), Diagnostic> {
        if let Some((saved_staged, saved_committed)) = self.tx_checkpoints.pop() {
            if self.committed == saved_committed && self.staged > saved_staged {
                self.graph_commit(span)?;
            }
        }
        Ok(())
    }

    fn aura_validate(
        &mut self,
        node: &Value,
        shape: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        if matches!(node, Value::Null) || matches!(shape, Value::Null) {
            return Ok(Value::Bool(false));
        }
        if let Value::Record(shape_rec) = shape {
            if let Some(Value::List(required)) = shape_rec.get("required") {
                let Value::Record(node_rec) = node else {
                    return Ok(Value::Bool(required.is_empty()));
                };
                for key in required {
                    let Some(key_name) = key.as_str() else {
                        return Ok(Value::Bool(false));
                    };
                    match node_rec.get(key_name) {
                        None | Some(Value::Null) => return Ok(Value::Bool(false)),
                        Some(_) => {}
                    }
                }
            }
            if let Some(Value::Record(expected_fields)) = shape_rec.get("fields") {
                let Value::Record(node_rec) = node else {
                    return Ok(Value::Bool(expected_fields.is_empty()));
                };
                for key in expected_fields.keys() {
                    match node_rec.get(key) {
                        None | Some(Value::Null) => return Ok(Value::Bool(false)),
                        Some(_) => {}
                    }
                }
            }
        }
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
                "no clock available on this local host",
            )),
        }
    }

    /// Returns an `Instant` from the mock clock (X6).
    fn time_now(&mut self, span: Span) -> Result<Value, Diagnostic> {
        match self.clock_time {
            Some(secs) => Ok(Value::Instant(Instant::unix(secs, 500_000_000))),
            None => Err(Diagnostic::new(
                DiagCode::E702,
                span,
                "no clock available on this local host",
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
                "no clock available on this local host",
            )),
        }
    }

    fn time_monotonic_nanos(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::U64(self.monotonic_time))
    }

    fn capability_invoke(
        &mut self,
        id: &str,
        args: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        crate::catalog::invoke_local(id, args, span)
    }

    fn available_acceleration(&self) -> crate::AccelerationTier {
        crate::detect_available_tier()
    }
}
