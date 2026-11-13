//! Public-only provisioning and scoped operations through the existing library owner.
use super::CryptographicLibrary;
use crate::crypto::network::errors::CryptoError;
use crate::specialized_libs::cryptographic_library::{
    CryptographicError, Key, KeyAlgorithm, KeyType, SecurityLevel,
};
use crate::specialized_libs::cryptographic_library::{
    ManagedObjectPublication, ManagedOperationControl, ManagedRecipientUnwrap, PublicationRequest,
    PublicationWorkspace, RecipientUnwrapRequest,
};
impl CryptographicLibrary {
    /// Trusted local provisioning boundary. Returns PUBLIC keys only; private keys
    /// move directly into the existing store. Does not enroll or recognize an identity.
    pub fn generate_managed_recipient_pair(
        &mut self,
        ml_id: &str,
        dh_id: &str,
        security: SecurityLevel,
    ) -> Result<(Key, Key), CryptographicError> {
        if ml_id.is_empty()
            || dh_id.is_empty()
            || ml_id == dh_id
            || ml_id.len() > 128
            || dh_id.len() > 128
        {
            return Err(CryptographicError::AccessDenied(
                "distinct bounded recipient IDs required".into(),
            ));
        }
        let ids = self.key_manager.list_keys();
        if [format!("{ml_id}_private"), format!("{dh_id}_private")]
            .iter()
            .any(|id| ids.contains(id))
        {
            return Err(CryptographicError::AccessDenied(
                "replacement requires rotation admission".into(),
            ));
        }
        let (ml, ml_public) = self.key_manager.generate_key_pair(
            ml_id.into(),
            KeyType::Private,
            KeyAlgorithm::Kyber,
            security.clone(),
        )?;
        let (dh, dh_public) = self.key_manager.generate_key_pair(
            dh_id.into(),
            KeyType::Private,
            KeyAlgorithm::X25519,
            security,
        )?;
        self.key_manager.store_key(ml)?;
        self.key_manager.store_key(dh)?;
        Ok((ml_public, dh_public))
    }
    pub fn begin_recipient_unwrap<'a>(
        &'a self,
        request: RecipientUnwrapRequest<'a>,
        control: &'a ManagedOperationControl,
        clock: &'a dyn Fn() -> u64,
    ) -> Result<ManagedRecipientUnwrap<'a>, CryptoError> {
        self.key_manager
            .begin_recipient_unwrap(request, control, clock)
    }
    pub fn begin_object_publication<'a>(
        &'a mut self,
        request: PublicationRequest<'a>,
        workspace: &'a mut PublicationWorkspace,
        control: &'a ManagedOperationControl,
        clock: &'a dyn Fn() -> u64,
    ) -> Result<ManagedObjectPublication<'a>, CryptoError> {
        self.key_manager
            .begin_object_publication(request, workspace, control, clock)
    }
}
