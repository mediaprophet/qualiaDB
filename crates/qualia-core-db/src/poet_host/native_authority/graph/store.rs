// Copyright (c) 2026 Timothy Charles Holborn.
// SPDX-License-Identifier: CC-BY-NC-ND-4.0
use super::super::{ed25519, NativeAuthority, State, TrustedProject};
use super::{
    graph_grant_digest, GraphError, GraphScope, SignedGraphGrant, GRAPH_GRANT_CONTEXT,
    MAX_GRAPH_CONTEXTS, MAX_GRAPH_QUINS, MAX_TOTAL_GRAPH_QUINS,
};
use crate::NQuin;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphReceipt {
    pub context: u64,
    pub previous_revision: u64,
    pub revision: u64,
    pub subject: String,
    pub grant_digest: [u8; 32],
    pub admission_digest: [u8; 32],
    pub content_digest: [u8; 32],
    pub authorization: SignedGraphGrant,
    pub authority_generation: u64,
    pub authentication_ref: String,
    pub occurred_at_unix: u64,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct StoredGraph {
    context: u64,
    quins: Vec<NQuin>,
    receipt: GraphReceipt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphRead {
    pub context: u64,
    pub revision: u64,
    pub written: usize,
}

/// Polling/coalescing subscription; retains only a scope and last delivered
/// revision. It never subscribes to or exposes global daemon graph revisions.
pub struct GraphSubscription {
    scope: GraphScope,
    seen: Option<u64>,
    closed: bool,
}
impl GraphSubscription {
    pub fn close(&mut self) {
        self.closed = true;
    }
    pub fn max_quins(&self) -> usize {
        self.scope.max_quins()
    }
}

fn digest(quins: &[NQuin]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"qualia:native-context-quins:v1\0");
    hash.update((quins.len() as u64).to_le_bytes());
    for q in quins {
        for lane in [
            q.subject,
            q.predicate,
            q.object,
            q.context,
            q.metadata,
            q.parity,
        ] {
            hash.update(lane.to_le_bytes());
        }
    }
    hash.finalize().into()
}
pub(in super::super) fn validate_graphs(
    graphs: &[StoredGraph],
    anchor: &TrustedProject,
) -> Result<(), String> {
    if graphs.len() > MAX_GRAPH_CONTEXTS {
        return Err("native graph context capacity".into());
    }
    let mut total = 0usize;
    for (index, graph) in graphs.iter().enumerate() {
        let grant = &graph.receipt.authorization.grant;
        let signature: [u8; 64] = graph
            .receipt
            .authorization
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| "invalid retained graph signature length")?;
        let grant_digest = graph_grant_digest(grant).map_err(|e| e.to_string())?;
        ed25519::verify_with_context(
            &anchor.public_key,
            &grant_digest,
            GRAPH_GRANT_CONTEXT,
            &signature,
        )
        .map_err(|_| "retained native graph grant signature verification failed")?;
        if grant.project_id != anchor.project_id
            || grant.context != graph.context
            || grant.subject != graph.receipt.subject
            || !grant.rights.replace
            || grant_digest != graph.receipt.grant_digest
            || grant.admission_digest != graph.receipt.admission_digest
            || graph.quins.len() > grant.max_quins
            || graph.receipt.authority_generation == 0
            || graph.receipt.authentication_ref.is_empty()
            || graph.receipt.authentication_ref.len() > 4096
            || graph.receipt.occurred_at_unix < grant.valid_from_unix
            || graph.receipt.occurred_at_unix >= grant.valid_until_unix
        {
            return Err("retained native graph grant/receipt mismatch".into());
        }
        total = total
            .checked_add(graph.quins.len())
            .ok_or("native graph size overflow")?;
        if total > MAX_TOTAL_GRAPH_QUINS
            || graph.quins.len() > MAX_GRAPH_QUINS
            || graph
                .quins
                .iter()
                .any(|q| q.context != graph.context || !q.verify_ecc_parity())
            || graphs[..index].iter().any(|g| g.context == graph.context)
            || graph.receipt.context != graph.context
            || graph.receipt.revision == 0
            || graph.receipt.previous_revision.checked_add(1) != Some(graph.receipt.revision)
            || graph.receipt.subject.is_empty()
            || graph.receipt.subject.len() > 4096
            || graph.receipt.content_digest != digest(&graph.quins)
        {
            return Err("invalid native graph ledger".into());
        }
    }
    Ok(())
}
/// A native journal operation can change one context once. Source/admission
/// operations retain prior graph receipts verbatim, including historical grants.
pub(in super::super) fn validate_graph_transition(
    previous: &[StoredGraph],
    next: &[StoredGraph],
    admission_digest: [u8; 32],
) -> Result<(), String> {
    if previous
        .iter()
        .any(|old| !next.iter().any(|new| new.context == old.context))
    {
        return Err("native journal removed a graph context".into());
    }
    let mut changed = 0;
    for graph in next {
        let old = previous.iter().find(|old| old.context == graph.context);
        if old == Some(graph) {
            continue;
        }
        changed += 1;
        let previous_revision = old.map_or(0, |old| old.receipt.revision);
        if changed > 1
            || graph.receipt.previous_revision != previous_revision
            || previous_revision.checked_add(1) != Some(graph.receipt.revision)
            || graph.receipt.admission_digest != admission_digest
        {
            return Err("native graph journal revision/admission transition mismatch".into());
        }
    }
    Ok(())
}

fn read(state: &State, scope: &GraphScope, output: &mut [NQuin]) -> Result<GraphRead, GraphError> {
    if !scope.grant.rights.read {
        return Err(GraphError::Permission);
    }
    let graph = state
        .durable
        .graphs
        .iter()
        .find(|g| g.context == scope.context());
    let quins = graph.map_or(&[][..], |g| g.quins.as_slice());
    if quins.len() > scope.max_quins() || quins.len() > output.len() {
        return Err(GraphError::Capacity);
    }
    output[..quins.len()].copy_from_slice(quins);
    Ok(GraphRead {
        context: scope.context(),
        revision: graph.map_or(0, |g| g.receipt.revision),
        written: quins.len(),
    })
}
impl NativeAuthority {
    /// Read exact-context canonical lanes without allocations. Errors leave output
    /// untouched. Returned data is legitimately released at this locked boundary.
    pub fn read_graph(
        &self,
        scope: &GraphScope,
        output: &mut [NQuin],
    ) -> Result<GraphRead, GraphError> {
        let state = self.state.try_lock().map_err(|error| match error {
            std::sync::TryLockError::WouldBlock => GraphError::Busy,
            std::sync::TryLockError::Poisoned(_) => GraphError::Poisoned,
        })?;
        self.validate_graph_scope(&state, scope)?;
        read(&state, scope, output)
    }
    /// Replace one bounded context with optimistic revision checking. This is cold
    /// authoring, not a hot query kernel. All six lanes are preserved verbatim.
    /// Cancellation before journal I/O has no effect; once append begins, success
    /// returns its committed receipt even if cancellation arrives during I/O.
    pub fn replace_graph(
        &self,
        scope: &GraphScope,
        expected_revision: u64,
        quins: &[NQuin],
        cancelled: &AtomicBool,
    ) -> Result<GraphReceipt, GraphError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(GraphError::Cancelled);
        }
        if !scope.grant.rights.replace {
            return Err(GraphError::Permission);
        }
        if quins.len() > scope.max_quins() || quins.iter().any(|q| q.context != scope.context()) {
            return Err(GraphError::Capacity);
        }
        if quins.iter().any(|q| !q.verify_ecc_parity()) {
            return Err(GraphError::InvalidQuin);
        }
        let mut state = self.state.lock().map_err(|_| GraphError::Poisoned)?;
        self.validate_graph_scope(&state, scope)?;
        let index = state
            .durable
            .graphs
            .iter()
            .position(|g| g.context == scope.context());
        let current = index.map_or(0, |i| state.durable.graphs[i].receipt.revision);
        if expected_revision != current {
            return Err(GraphError::RevisionConflict);
        }
        let total: usize = state
            .durable
            .graphs
            .iter()
            .filter(|g| g.context != scope.context())
            .map(|g| g.quins.len())
            .sum();
        if total + quins.len() > MAX_TOTAL_GRAPH_QUINS
            || (index.is_none() && state.durable.graphs.len() >= MAX_GRAPH_CONTEXTS)
        {
            return Err(GraphError::Capacity);
        }
        let occurred_at_unix = super::super::now().map_err(|_| GraphError::Clock)?;
        let (session, _) = self
            .authenticated_lookup(&state, &scope.session, occurred_at_unix)
            .map_err(|_| GraphError::Authentication)?;
        let receipt = GraphReceipt {
            context: scope.context(),
            previous_revision: current,
            revision: current.checked_add(1).ok_or(GraphError::Capacity)?,
            subject: scope.grant.subject.clone(),
            grant_digest: scope.digest,
            admission_digest: state.admission_digest,
            content_digest: digest(quins),
            authorization: SignedGraphGrant {
                grant: scope.grant.clone(),
                signature: scope.signature.to_vec(),
            },
            authority_generation: state.generation,
            authentication_ref: session.authentication_ref.clone(),
            occurred_at_unix,
        };
        let mut next = state.durable.clone();
        let graph = StoredGraph {
            context: scope.context(),
            quins: quins.to_vec(),
            receipt: receipt.clone(),
        };
        if let Some(index) = index {
            next.graphs[index] = graph;
        } else {
            next.graphs.push(graph);
        }
        // Recheck expiry after bounded cloning/hashing, immediately before the
        // irreversible journal boundary. No callback or grant unlock occurs.
        self.validate_graph_scope(&state, scope)?;
        if cancelled.load(Ordering::Acquire) {
            return Err(GraphError::Cancelled);
        }
        if let Err(detail) = state.journal.append(&next) {
            state.reconciliation = true;
            return Err(GraphError::Journal(detail));
        }
        state.durable = next;
        Ok(receipt)
    }
    pub fn subscribe_graph(&self, scope: GraphScope) -> Result<GraphSubscription, GraphError> {
        let state = self.state.try_lock().map_err(|error| match error {
            std::sync::TryLockError::WouldBlock => GraphError::Busy,
            std::sync::TryLockError::Poisoned(_) => GraphError::Poisoned,
        })?;
        self.validate_graph_scope(&state, &scope)?;
        if !scope.grant.rights.subscribe {
            return Err(GraphError::Permission);
        }
        Ok(GraphSubscription {
            scope,
            seen: None,
            closed: false,
        })
    }
    /// Full snapshots reconcile skipped revisions. No queue grows on slow readers.
    /// A revoked/expired scope returns error even when its graph has not changed.
    pub fn poll_graph(
        &self,
        subscription: &mut GraphSubscription,
        output: &mut [NQuin],
    ) -> Result<Option<GraphRead>, GraphError> {
        if subscription.closed {
            return Err(GraphError::Cancelled);
        }
        let state = self.state.try_lock().map_err(|error| match error {
            std::sync::TryLockError::WouldBlock => GraphError::Busy,
            std::sync::TryLockError::Poisoned(_) => GraphError::Poisoned,
        })?;
        self.validate_graph_scope(&state, &subscription.scope)?;
        let revision = state
            .durable
            .graphs
            .iter()
            .find(|g| g.context == subscription.scope.context())
            .map_or(0, |g| g.receipt.revision);
        if subscription.seen == Some(revision) {
            return Ok(None);
        }
        let result = read(&state, &subscription.scope, output)?;
        subscription.seen = Some(revision);
        Ok(Some(result))
    }
}
