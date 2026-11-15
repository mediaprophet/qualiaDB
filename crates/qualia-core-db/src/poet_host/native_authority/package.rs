//! Resource/package evidence is separate from a review of editable source.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct PackageWriteIntent {
    pub filename: String,
    pub expected_digest: Option<[u8; 32]>,
}
#[derive(Clone, Debug)]
pub struct PackageWriteReceipt {
    pub digest: [u8; 32],
    pub bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageWriteEvidence {
    pub filename: String,
    pub expected_digest: Option<[u8; 32]>,
    pub result_digest: [u8; 32],
    pub result_bytes: usize,
    pub source: DocumentRevision,
    pub actor_subject: String,
    pub principal_kind: PrincipalKind,
    pub admission_ref: String,
    pub authentication_ref: String,
    pub authority_generation: u64,
    pub occurred_at_unix: u64,
}
impl NativeAuthority {
    /// Retained audit only; these records do not grant package review authority
    /// or constitute a review/acceptance of the complete HMC embodiment.
    pub fn package_write_evidence(
        &self,
        handle: &SessionHandle,
    ) -> Result<Vec<PackageWriteEvidence>, String> {
        let state = self.state.lock().map_err(|_| "native authority poisoned")?;
        self.view(&state, handle, now()?, |_| {
            Ok(state.durable.packages.clone())
        })
    }
}
