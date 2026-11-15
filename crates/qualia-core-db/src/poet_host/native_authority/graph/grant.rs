// Copyright (c) 2026 Timothy Charles Holborn.
// SPDX-License-Identifier: CC-BY-NC-ND-4.0
use super::super::{
    admission::{live, BoundedBytes},
    ed25519, NativeAuthority, SessionHandle, State,
};
use super::{GraphError, MAX_GRAPH_QUINS};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const GRAPH_GRANT_CONTEXT: &[u8] = b"qualia.native-graph-grant.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphRights {
    pub read: bool,
    pub replace: bool,
    pub subscribe: bool,
}

/// Cold, bounded issuance record. Its fields are not themselves authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphGrant {
    pub version: u32,
    pub project_id: String,
    pub subject: String,
    pub admission_digest: [u8; 32],
    /// Exact complete context lane, including any context-encoded sensitivity.
    pub context: u64,
    pub rights: GraphRights,
    pub max_quins: usize,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedGraphGrant {
    pub grant: GraphGrant,
    pub signature: Vec<u8>,
}

/// Opaque owner/session-bound capability. Never deserialized from evidence.
pub struct GraphScope {
    pub(super) owner: [u8; 32],
    pub(super) session: SessionHandle,
    pub(super) generation: u64,
    pub(super) grant: GraphGrant,
    pub(super) digest: [u8; 32],
    pub(super) signature: [u8; 64],
}
impl GraphScope {
    pub fn context(&self) -> u64 {
        self.grant.context
    }
    pub fn max_quins(&self) -> usize {
        self.grant.max_quins
    }
}

pub fn graph_grant_digest(grant: &GraphGrant) -> Result<[u8; 32], GraphError> {
    if grant.version != 1
        || grant.project_id.is_empty()
        || grant.project_id.len() > 4096
        || grant.subject.is_empty()
        || grant.subject.len() > 4096
        || grant.admission_digest == [0; 32]
        || grant.max_quins == 0
        || grant.max_quins > MAX_GRAPH_QUINS
        || grant.valid_from_unix >= grant.valid_until_unix
        || (!grant.rights.read && !grant.rights.replace)
        || (grant.rights.subscribe && !grant.rights.read)
    {
        return Err(GraphError::InvalidGrant);
    }
    let mut writer = BoundedBytes {
        bytes: Vec::new(),
        limit: 64 * 1024,
    };
    serde_json::to_writer(&mut writer, grant).map_err(|_| GraphError::Capacity)?;
    Ok(Sha256::digest(&writer.bytes).into())
}

impl NativeAuthority {
    /// Issuer signature and live key-possession session are both mandatory.
    /// Admission replacement or session disconnect revokes all derived scopes.
    pub fn bind_graph_scope(
        &self,
        session: &SessionHandle,
        signed: &SignedGraphGrant,
    ) -> Result<GraphScope, GraphError> {
        let digest = graph_grant_digest(&signed.grant)?;
        let signature: [u8; 64] = signed
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| GraphError::Signature)?;
        ed25519::verify_with_context(
            &self.anchor.public_key,
            &digest,
            GRAPH_GRANT_CONTEXT,
            &signature,
        )
        .map_err(|_| GraphError::Signature)?;
        let state = self.state.lock().map_err(|_| GraphError::Poisoned)?;
        let scope = GraphScope {
            owner: self.incarnation,
            session: session.clone(),
            generation: state.generation,
            grant: signed.grant.clone(),
            digest,
            signature,
        };
        self.validate_graph_scope(&state, &scope)?;
        Ok(scope)
    }

    pub(super) fn validate_graph_scope(
        &self,
        state: &State,
        scope: &GraphScope,
    ) -> Result<(), GraphError> {
        if scope.owner != self.incarnation
            || scope.generation != state.generation
            || scope.grant.admission_digest != state.admission_digest
            || scope.grant.project_id != self.anchor.project_id
        {
            return Err(GraphError::StaleScope);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| GraphError::Clock)?
            .as_secs();
        let (_, subject) = self
            .authenticated_lookup(state, &scope.session, now)
            .map_err(|_| GraphError::Authentication)?;
        if subject.subject != scope.grant.subject
            || subject.revoked
            || !live(now, subject.valid_from_unix, subject.valid_until_unix)
            || !live(
                now,
                scope.grant.valid_from_unix,
                scope.grant.valid_until_unix,
            )
        {
            return Err(GraphError::Authentication);
        }
        Ok(())
    }
}
