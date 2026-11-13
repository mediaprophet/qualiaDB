//! Single-use actual managed ML-KEM/X25519 custody. No public master output.
use super::super::managed_control::ManagedOperationControl;
use super::{
    super::{KeyAlgorithm, KeyManager, KeyOperation, SigningAccess},
    checks,
};
use crate::crypto::network::{
    errors::CryptoError as E,
    kem::MlKem768Secret,
    key_provider::ProviderGrant,
    object_stream::{
        Context, RecipientRecord, Wrap, WrapContext, open_master_raw, recipient_key_ids,
    },
    secret_lease::SecretLease,
    types::KeyPurpose,
    x25519::X25519Secret,
};
use fips203::{ml_kem_768, traits::SerDes};
use zeroize::Zeroizing;
pub struct RecipientUnwrapRequest<'a> {
    pub ml_key_id: &'a str,
    pub dh_key_id: &'a str,
    pub ml_policy_id: &'a str,
    pub dh_policy_id: &'a str,
    pub record: RecipientRecord<'a>,
    pub context: Context<'a>,
    pub grant: ProviderGrant,
    pub access: SigningAccess<'a>,
}
/// Opaque bounded operation. Application authority must be composed separately.
pub struct ManagedRecipientUnwrap<'a> {
    pub(super) owner: &'a KeyManager,
    pub(super) request: RecipientUnwrapRequest<'a>,
    pub(super) control: &'a ManagedOperationControl,
    pub(super) clock: &'a dyn Fn() -> u64,
}
impl KeyManager {
    pub fn begin_recipient_unwrap<'a>(
        &'a self,
        request: RecipientUnwrapRequest<'a>,
        control: &'a ManagedOperationControl,
        clock: &'a dyn Fn() -> u64,
    ) -> Result<ManagedRecipientUnwrap<'a>, E> {
        let operation = ManagedRecipientUnwrap {
            owner: self,
            request,
            control,
            clock,
        };
        operation.check((clock)())?;
        Ok(operation)
    }
}
impl<'a> ManagedRecipientUnwrap<'a> {
    pub(super) fn check_locked(
        &self,
        state: &super::super::managed_control::ControlState,
        now: u64,
    ) -> Result<(), E> {
        let r = &self.request;
        checks::grant(r.grant, state, KeyPurpose::ObjectRecipient, now)?;
        r.record.valid_at(now)?;
        if r.ml_key_id == r.dh_key_id
            || r.record.fields().context != r.context.id()
            || r.record.fields().key_epoch != r.grant.epoch.epoch
        {
            return Err(E::Unauthorized);
        }
        checks::policy(
            self.owner,
            r.ml_policy_id,
            r.ml_key_id,
            KeyOperation::Decrypt,
            r.access,
            now,
        )?;
        checks::policy(
            self.owner,
            r.dh_policy_id,
            r.dh_key_id,
            KeyOperation::Derive,
            r.access,
            now,
        )?;
        checks::private(self.owner, r.ml_key_id, KeyAlgorithm::Kyber, now)?;
        checks::private(self.owner, r.dh_key_id, KeyAlgorithm::X25519, now)?;
        Ok(())
    }
    pub(super) fn check(&self, now: u64) -> Result<(), E> {
        let state = self.control.lock()?;
        self.check_locked(&state, now)
    }
    /// Called only by the private authority/current-use compositor. This method
    /// performs provider checks, not recognized recipient proof or disclosure admission.
    pub(crate) fn unwrap(self, wrap: Wrap<'_>) -> Result<super::open::UnwrappedMetadata<'a>, E> {
        let now = (self.clock)();
        self.check(now)?;
        let r = &self.request;
        let ml = checks::private(self.owner, r.ml_key_id, KeyAlgorithm::Kyber, now)?;
        let dh = checks::private(self.owner, r.dh_key_id, KeyAlgorithm::X25519, now)?;
        let ml_bytes: &[u8; 2400] = ml
            .key_data
            .as_slice()
            .try_into()
            .map_err(|_| E::Unauthorized)?;
        let dh_bytes: &[u8; 32] = dh
            .key_data
            .as_slice()
            .try_into()
            .map_err(|_| E::Unauthorized)?;
        // Existing provider validates the serialized secret; never clone stored Key/Vec.
        let validated =
            ml_kem_768::DecapsKey::try_from_bytes(*ml_bytes).map_err(|_| E::Unauthorized)?;
        drop(validated);
        let ml_public: &[u8; 1184] = ml_bytes[1152..2336].try_into().unwrap();
        let dh = X25519Secret::from_bytes(*dh_bytes);
        let ids = recipient_key_ids(ml_public, &dh.public());
        if ids != r.record.key_ids() || ids != wrap.key_ids {
            return Err(E::Unauthorized);
        }
        let c = WrapContext::new(
            r.record,
            r.context.fields().metadata_length,
            wrap.ephemeral,
            wrap.ciphertext,
        )?;
        c.matches(wrap)?;
        let kem = MlKem768Secret::from_bytes(*ml_bytes);
        let ss = Zeroizing::new(kem.decapsulate(wrap.ciphertext)?);
        let ds = Zeroizing::new(dh.diffie_hellman(wrap.ephemeral)?);
        let mut master = Zeroizing::new([0; 32]);
        open_master_raw(c, wrap, &ss, &ds, &mut master)?;
        let state = self.control.lock()?;
        let now = (self.clock)();
        self.check_locked(&state, now)?;
        let scope = crate::crypto::network::object_stream::ObjectScope::new(
            r.context.id(),
            0,
            r.context.fields().generation,
            r.context.fields().key_epoch,
            r.context.fields().metadata_length,
        )?;
        let master = SecretLease::new(KeyPurpose::ObjectContent, &master[..])?;
        drop(state);
        Ok(super::open::UnwrappedMetadata {
            master,
            scope,
            operation: self,
            next: 0,
            poisoned: false,
        })
    }
}
