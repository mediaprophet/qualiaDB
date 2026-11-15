//! Retained authenticated product route. Setup supplies real session authority
//! and explicit source/input budgets; each turn receives a fresh single-use bind.
use super::super::governed::PolicyError;
use super::{NativeGovernedRequest, NativeHostCapability, NativePreparationSpec};
use crate::poet_host::native_authority::{inference::IdentifierSelection, SessionHandle};
use std::sync::Arc;

impl NativePreparationSpec {
    /// Open one retained, writer-excluded file with an independently selected
    /// full SHA-256. The caller accounts aggregate primary/shard budgets and
    /// retains the returned lease; no source snapshot or pathname fallback.
    pub fn open_model_lease(
        path: &std::path::Path,
        expected_sha256: [u8; 32],
        limits: [u64; 3],
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Arc<etalon_model_io::lease::ModelLease>, String> {
        etalon_model_io::lease::ModelLease::open(
            path, expected_sha256,
            etalon_model_io::lease::LeaseLimits {
                max_source_bytes: limits[0], max_mapped_bytes: limits[1], max_total_bytes: limits[2],
            }, cancelled,
        ).map(Arc::new).map_err(|e| format!("Native model lease: {e:?}"))
    }
}

pub struct NativeProductSession {
    pub(super) capability: Arc<NativeHostCapability>,
    pub(super) session: SessionHandle,
    pub(super) identifier: [String; 5],
    preparation: NativePreparationSpec,
}

impl NativeProductSession {
    /// Only a privileged host that already owns an authenticated session can
    /// establish a route. No policy, key or admission is created here.
    pub fn new(
        capability: Arc<NativeHostCapability>,
        session: SessionHandle,
        selected: IdentifierSelection<'_>,
        preparation: NativePreparationSpec,
    ) -> Result<Self, PolicyError> {
        if let NativePreparationSpec::Leased { shards, .. } = &preparation {
            if shards.len() > 64 {
                return Err(PolicyError::Capacity);
            }
        }
        // Validate before retaining strings; identifier size is owner checked.
        capability.bind_session(&session, selected, preparation.clone())?;
        Ok(Self {
            capability,
            session,
            identifier: [
                selected.value,
                selected.scheme,
                selected.issuer,
                selected.project_id,
                selected.evidence_ref,
            ]
            .map(str::to_owned),
            preparation,
        })
    }

    pub fn bind(&self) -> Result<NativeGovernedRequest, PolicyError> {
        let [value, scheme, issuer, project_id, evidence_ref] = &self.identifier;
        self.capability.bind_session(
            &self.session,
            IdentifierSelection {
                value,
                scheme,
                issuer,
                project_id,
                evidence_ref,
            },
            self.preparation.clone(),
        )
    }
}
