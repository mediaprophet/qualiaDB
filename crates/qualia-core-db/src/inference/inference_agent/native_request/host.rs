//! Privileged product-host provisioning. Authentication and policy installation
//! remain separate: opening this capability never creates a policy or subject.
use super::super::governed::{NativePolicyHost, PolicyError, TrustedPolicyIdentity};
use super::{
    NativeGovernedRequest, NativePreparation, NativePreparationSpec, NativePreparedReceipt,
};
use crate::poet_host::native_authority::{
    inference::IdentifierSelection, NativeAuthority, SessionHandle,
};
use ed25519_dalek::SigningKey;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

/// Retained host authority supplied by the native product's authenticated setup.
/// Neither a prompt, agent DID nor identifier can construct the signing key.
/// Keep one capability per policy store so all requests share revocation state.
pub struct NativeHostCapability {
    pub(super) host: Arc<Mutex<NativePolicyHost>>,
    pub(super) installer: SigningKey,
    pub(super) owner: Arc<NativeAuthority>,
}

impl NativeHostCapability {
    /// Reopen a previously explicitly installed policy using configured trust.
    /// Missing policy fails closed; this method never silently installs rules.
    pub fn open_existing(
        root: &Path,
        authority_root: &Path,
        trust: TrustedPolicyIdentity,
        installer: SigningKey,
        owner: Arc<NativeAuthority>,
        budget: usize,
    ) -> Result<Self, PolicyError> {
        if installer.verifying_key().to_bytes() != trust.controller_key() {
            return Err(PolicyError::WrongAuthority);
        }
        let host = NativePolicyHost::open(root, authority_root, trust, budget)?;
        Self::from_installed(Arc::new(Mutex::new(host)), installer, owner)
    }

    /// Retain the actual explicitly installed host. The product may install V3
    /// rules through NativePolicyHost::install_evidence before this handoff.
    pub fn from_installed(
        host: Arc<Mutex<NativePolicyHost>>,
        installer: SigningKey,
        owner: Arc<NativeAuthority>,
    ) -> Result<Self, PolicyError> {
        {
            let guard = host.lock().map_err(|_| PolicyError::Revoked)?;
            if guard.trusted_identity().controller_key() != installer.verifying_key().to_bytes() {
                return Err(PolicyError::WrongAuthority);
            }
            if !guard.status().installed {
                return Err(PolicyError::MissingPolicy);
            }
        }
        Ok(Self {
            host,
            installer,
            owner,
        })
    }

    /// Bind an actual authenticated session and its selected admitted identifier.
    /// A subject may choose any of its admitted identifiers; policy authorization
    /// is still independently checked at admission and each publication boundary.
    pub fn bind_prepared(
        &self,
        session: &SessionHandle,
        identifier: IdentifierSelection<'_>,
        receipt: NativePreparedReceipt,
    ) -> Result<NativeGovernedRequest, PolicyError> {
        let association = self
            .owner
            .bind_inference_association(session, identifier)
            .map_err(|_| PolicyError::WrongAuthority)?;
        self.owner
            .with_current_inference_association(&association, |evidence| {
                if evidence.subject == receipt.actor {
                    Ok(())
                } else {
                    Err(PolicyError::WrongAuthority)
                }
            })
            .map_err(|_| PolicyError::Revoked)??;
        Ok(NativeGovernedRequest {
            preparation: NativePreparation::Prepared(receipt),
            host: Arc::clone(&self.host),
            installer: self.installer.clone(),
            owner: Arc::clone(&self.owner),
            association,
        })
    }

    /// Normal product handoff: bind actual session authority before the worker
    /// owns its thread-local numerical engine. The worker must still prepare all
    /// actual inputs/providers and seal the receipt before any prefill executes.
    pub fn bind_session(
        &self,
        session: &SessionHandle,
        identifier: IdentifierSelection<'_>,
        preparation: NativePreparationSpec,
    ) -> Result<NativeGovernedRequest, PolicyError> {
        let association = self
            .owner
            .bind_inference_association(session, identifier)
            .map_err(|_| PolicyError::WrongAuthority)?;
        self.owner
            .with_current_inference_association(&association, |_| ())
            .map_err(|_| PolicyError::Revoked)?;
        Ok(NativeGovernedRequest {
            preparation: NativePreparation::Pending(preparation),
            host: Arc::clone(&self.host),
            installer: self.installer.clone(),
            owner: Arc::clone(&self.owner),
            association,
        })
    }
}
