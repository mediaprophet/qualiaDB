//! Central facade for managed COSE profile signing; no second key store.
use super::CryptographicLibrary;
use crate::crypto::network::errors::CryptoError;
use crate::specialized_libs::cryptographic_library::{
    AccessPolicy, CryptographicError, Key, KeyAlgorithm, KeyType, ManagedPairedSigning,
    PairedSigningRequest, SecurityLevel, SigningControl,
};

impl CryptographicLibrary {
    /// Explicit local admin provisioning through the EXISTING algorithm generator.
    /// Returns public keys only; private keys move directly into the existing store.
    /// This is not an identity enrollment, trusted pin, or paired epoch rotation.
    pub fn generate_managed_signing_pair(
        &mut self,
        ml_id: &str,
        ed_id: &str,
        security: SecurityLevel,
    ) -> Result<(Key, Key), CryptographicError> {
        if ml_id.is_empty() || ed_id.is_empty() || ml_id == ed_id {
            return Err(CryptographicError::AccessDenied(
                "distinct key IDs required".into(),
            ));
        }
        let ids = self.key_manager.list_keys();
        if [format!("{ml_id}_private"), format!("{ed_id}_private")]
            .iter()
            .any(|id| ids.contains(id))
        {
            return Err(CryptographicError::AccessDenied(
                "key replacement requires rotation admission".into(),
            ));
        }
        let (ml, ml_public) = self.key_manager.generate_key_pair(
            ml_id.into(),
            KeyType::Private,
            KeyAlgorithm::MLDSA,
            security.clone(),
        )?;
        let (ed, ed_public) = self.key_manager.generate_key_pair(
            ed_id.into(),
            KeyType::Private,
            KeyAlgorithm::EdDSA,
            security,
        )?;
        self.key_manager.store_key(ml)?;
        self.key_manager.store_key(ed)?;
        Ok((ml_public, ed_public))
    }

    /// Trusted local administrative boundary; never install a wire-claimed policy.
    pub fn install_managed_signing_policy(&mut self, policy: AccessPolicy) {
        self.key_manager.install_managed_signing_policy(policy);
    }

    /// Uses host-authenticated access context and a trusted Unix clock.
    /// Requires policies for BOTH keys. The returned capability cannot export keys.
    pub fn begin_paired_signing<'a>(
        &'a self,
        request: PairedSigningRequest<'a>,
        control: &'a SigningControl,
        clock: &'a dyn Fn() -> u64,
    ) -> Result<ManagedPairedSigning<'a>, CryptoError> {
        self.key_manager
            .begin_paired_signing(request, control, clock)
    }
}
