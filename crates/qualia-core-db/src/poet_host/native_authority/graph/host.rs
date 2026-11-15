// Copyright (c) 2026 Timothy Charles Holborn.
// SPDX-License-Identifier: CC-BY-NC-ND-4.0
//! Explicit Vibe Host binding to an authenticated authority-owned project graph.
//! Cold, bounded authoring only. Other privileged Host operations remain denied.
use super::super::NativeAuthority;
use super::{GraphError, GraphReceipt, GraphScope};
use crate::{
    vibe_host::{
        scan::collect_matches,
        values::{reifier_quin, value_to_quin},
    },
    NQuin,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use vibe::{Budget, DiagCode, Diagnostic, Host, Span, Value};

// Cold host bookkeeping, retained signed scope/receipt strings, and bounded term
// hashing/conversion scratch. Query result cells are charged by Vibe's router.
const FIXED_WORKSPACE: u64 = 256 * 1024;
const MAX_TERM_BYTES: usize = 32 * 1024;

pub struct NativeGraphHost {
    authority: Arc<NativeAuthority>,
    scope: GraphScope,
    cancelled: Arc<AtomicBool>,
    snapshot: Vec<NQuin>,
    staged: Vec<NQuin>,
    base_revision: Option<u64>,
    last_receipt: Option<GraphReceipt>,
}
impl NativeGraphHost {
    /// `scope` must come from the real native owner's issuer-signature and session
    /// verification. The application closes `cancelled` on navigation/worker teardown.
    pub fn new(
        authority: Arc<NativeAuthority>,
        scope: GraphScope,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Self, GraphError> {
        Self::new_with_budget(authority, scope, cancelled, &mut Budget::default())
    }
    /// Reserve this runtime's two quin buffers and bounded cold scratch from the
    /// canonical budget before allocation. Pass the remaining budget to Vibe.
    /// Charges are monotonic, including a failed allocation. The authority's
    /// shared graph/journal storage has its own bounds and is not runtime memory.
    pub fn new_with_budget(
        authority: Arc<NativeAuthority>,
        scope: GraphScope,
        cancelled: Arc<AtomicBool>,
        budget: &mut Budget,
    ) -> Result<Self, GraphError> {
        let state = authority.state.lock().map_err(|_| GraphError::Poisoned)?;
        authority.validate_graph_scope(&state, &scope)?;
        drop(state);
        let capacity = scope.max_quins();
        let bytes = (capacity as u64)
            .checked_mul(2 * std::mem::size_of::<NQuin>() as u64)
            .and_then(|n| n.checked_add(FIXED_WORKSPACE))
            .ok_or(GraphError::Capacity)?;
        budget
            .charge(bytes, Span::point(0))
            .map_err(|_| GraphError::Capacity)?;
        let mut snapshot = Vec::new();
        snapshot
            .try_reserve_exact(capacity)
            .map_err(|_| GraphError::Capacity)?;
        snapshot.resize(capacity, NQuin::default());
        let mut staged = Vec::new();
        staged
            .try_reserve_exact(capacity)
            .map_err(|_| GraphError::Capacity)?;
        Ok(Self {
            authority,
            scope,
            cancelled,
            snapshot,
            staged,
            base_revision: None,
            last_receipt: None,
        })
    }
    /// Last acknowledged native journal receipt. Earlier receipts remain in the
    /// canonical journal; these are not Vibe extension receipts or rollback proof.
    /// A failed attempt leaves the previous successful receipt here; it never
    /// acknowledges that failed attempt. Use the method result for each operation.
    pub fn last_receipt(&self) -> Option<&GraphReceipt> {
        self.last_receipt.as_ref()
    }
    fn current(&self, span: Span) -> Result<(), Diagnostic> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(error(GraphError::Cancelled, span));
        }
        let state = self
            .authority
            .state
            .lock()
            .map_err(|_| error(GraphError::Poisoned, span))?;
        self.authority
            .validate_graph_scope(&state, &self.scope)
            .map_err(|e| error(e, span))
    }
    fn denied(span: Span) -> Diagnostic {
        error(GraphError::Permission, span)
    }
}
fn error(error: GraphError, span: Span) -> Diagnostic {
    Diagnostic::new(DiagCode::E300, span, format!("{error}"))
}

fn leaf_bytes(value: &Value) -> Option<usize> {
    match value {
        Value::String(s) | Value::Iri(s) => Some(s.len()),
        Value::Prefixed(p, l) => p.len().checked_add(l.len())?.checked_add(1),
        Value::U64(_) | Value::I64(_) | Value::QuinRef(_) => Some(0),
        _ => None,
    }
}
fn bounded_terms<'a>(terms: impl IntoIterator<Item = &'a Value>, wildcards: bool) -> bool {
    terms
        .into_iter()
        .try_fold(0usize, |sum, term| {
            let bytes = match term {
                Value::Null if wildcards => Some(0),
                Value::Var(name) if wildcards => Some(name.len()),
                _ => leaf_bytes(term),
            }?;
            sum.checked_add(bytes)
        })
        .is_some_and(|bytes| bytes <= MAX_TERM_BYTES)
}
impl Host for NativeGraphHost {
    fn capability_resolve(&mut self, id: &str, span: Span) -> Result<Value, Diagnostic> {
        self.current(span)?;
        if id.len() > 4096 {
            return Err(error(GraphError::Capacity, span));
        }
        let bound = match id {
            "graph.read" | "graph.query" | "graph.snapshot" => self.scope.grant.rights.read,
            "graph.write" | "graph.stage" | "graph.commit" => {
                self.scope.grant.rights.read && self.scope.grant.rights.replace
            }
            _ => false,
        };
        let mut record = std::collections::BTreeMap::new();
        record.insert("id".into(), Value::String(id.into()));
        record.insert("vibe_bound".into(), Value::Bool(bound));
        record.insert(
            "honesty".into(),
            Value::String("native-authority-project-graph".into()),
        );
        Ok(Value::Record(record))
    }
    fn graph_query(&mut self, args: &[Value], take: u64, span: Span) -> Result<Value, Diagnostic> {
        self.current(span)?;
        if args.len() > 3 || !bounded_terms(args, true) {
            return Err(Self::denied(span));
        }
        let read = self
            .authority
            .read_graph(&self.scope, &mut self.snapshot)
            .map_err(|e| error(e, span))?;
        let take = take.min(self.scope.max_quins() as u64);
        let mut result = Vec::new();
        result
            .try_reserve_exact(take as usize)
            .map_err(|_| error(GraphError::Capacity, span))?;
        collect_matches(
            &self.snapshot[..read.written],
            args.first(),
            args.get(1),
            args.get(2),
            take,
            &mut result,
        );
        // Never return a result after cancellation observed during the scan.
        self.current(span)?;
        Ok(Value::List(result))
    }
    fn graph_begin(&mut self, span: Span) -> Result<(), Diagnostic> {
        self.current(span)?;
        if self.base_revision.is_some() || !self.scope.grant.rights.replace {
            return Err(Self::denied(span));
        }
        let read = self
            .authority
            .read_graph(&self.scope, &mut self.snapshot)
            .map_err(|e| error(e, span))?;
        self.staged.clear();
        self.staged
            .extend_from_slice(&self.snapshot[..read.written]);
        self.base_revision = Some(read.revision);
        Ok(())
    }
    fn graph_stage(&mut self, term: &Value, span: Span) -> Result<Value, Diagnostic> {
        self.current(span)?;
        if self.base_revision.is_none() {
            return Err(Self::denied(span));
        }
        let bounded = match term {
            Value::QuinRef(_) => true,
            Value::Triple(s, p, o) => bounded_terms([s.as_ref(), p.as_ref(), o.as_ref()], false),
            Value::Reified { s, p, o, r } => {
                bounded_terms([s.as_ref(), p.as_ref(), o.as_ref(), r.as_ref()], false)
            }
            _ => false,
        };
        if !bounded {
            return Err(error(GraphError::Capacity, span));
        }
        let primary = value_to_quin(term, self.scope.context());
        let reifier = reifier_quin(term, self.scope.context());
        let count = usize::from(primary.is_some()) + usize::from(reifier.is_some());
        if count == 0
            || self.staged.len() + count > self.scope.max_quins()
            || primary
                .iter()
                .chain(reifier.iter())
                .any(|q| q.context != self.scope.context())
        {
            return Err(error(GraphError::Capacity, span));
        }
        self.staged.extend(primary);
        self.staged.extend(reifier);
        Ok(Value::Null)
    }
    fn graph_abort(&mut self, _: Span) -> Result<(), Diagnostic> {
        self.staged.clear();
        self.base_revision = None;
        Ok(())
    }
    fn graph_commit(&mut self, span: Span) -> Result<Value, Diagnostic> {
        self.current(span)?;
        let revision = self
            .base_revision
            .take()
            .ok_or_else(|| Self::denied(span))?;
        let result =
            self.authority
                .replace_graph(&self.scope, revision, &self.staged, &self.cancelled);
        self.staged.clear();
        self.last_receipt = Some(result.map_err(|e| error(e, span))?);
        // Canonical Vibe Receipt is only returned after the native journal ack.
        Ok(Value::Receipt)
    }
    fn graph_snapshot(&mut self, span: Span) -> Result<Value, Diagnostic> {
        self.current(span)?;
        let read = self
            .authority
            .read_graph(&self.scope, &mut self.snapshot)
            .map_err(|e| error(e, span))?;
        self.current(span)?;
        Ok(Value::U64(read.revision))
    }
    fn aura_validate(&mut self, _: &Value, _: &Value, span: Span) -> Result<Value, Diagnostic> {
        Err(Self::denied(span))
    }
    fn pulse_publish(&mut self, _: &str, _: &Value, span: Span) -> Result<Value, Diagnostic> {
        Err(Self::denied(span))
    }
}
