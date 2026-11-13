//! Central managed content-key lifecycle; application disclosure authority is separate.
use super::super::managed_control::{ControlState, ManagedOperationControl};
use super::{
    super::{KeyAlgorithm, KeyManager, KeyOperation, KeyType, SecurityLevel, SigningAccess},
    ManagedObjectSealer, PublicationWorkspace, checks,
};
use crate::crypto::network::{
    digest::sha384,
    entropy::{EphemeralKeyLease, fill_os},
    errors::CryptoError as E,
    kem,
    key_provider::ProviderGrant,
    object_stream::{
        Context, ContextFields, GeometryPage, MAX_OBJECTS, ObjectScope, RecipientRecord,
        WRAP_BYTES, WrapContext, seal_master_raw,
    },
    types::KeyPurpose,
};
use zeroize::{Zeroize, Zeroizing};
/// Trusted local provider provisioning policy. This is NOT application rights.
pub struct PublicationRequest<'a> {
    pub policy_id: &'a str,
    pub policy_key_id: &'a str,
    pub grant: ProviderGrant,
    pub access: SigningAccess<'a>,
    pub generation: u64,
    pub predecessor: crate::net::qdnf::types::StrongDigest,
    pub objects: u32,
    pub metadata_length: u64,
    pub workspace_budget: usize,
}
/// Non-copyable one-attempt owner. Private workspace masters never leave this owner.
pub struct ManagedObjectPublication<'a> {
    pub(super) owner: &'a mut KeyManager,
    pub(super) request: PublicationRequest<'a>,
    pub(super) workspace: &'a mut PublicationWorkspace,
    pub(super) control: &'a ManagedOperationControl,
    pub(super) clock: &'a dyn Fn() -> u64,
    pub(super) fields: ContextFields,
    pub(super) context: Option<Context<'a>>,
    pub(super) poisoned: bool,
    wraps: u8,
}
impl KeyManager {
    pub fn begin_object_publication<'a>(
        &'a mut self,
        request: PublicationRequest<'a>,
        workspace: &'a mut PublicationWorkspace,
        control: &'a ManagedOperationControl,
        clock: &'a dyn Fn() -> u64,
    ) -> Result<ManagedObjectPublication<'a>, E> {
        if request.workspace_budget < PublicationWorkspace::required_bytes()
            || request.objects as usize > MAX_OBJECTS
            || request.policy_id.len() > 128
            || request.policy_key_id.len() > 128
        {
            return Err(E::Capacity);
        }
        let epoch = u32::try_from(request.grant.epoch.epoch).map_err(|_| E::Range)?;
        let mut p = ManagedObjectPublication {
            owner: self,
            fields: ContextFields {
                nonce: [0; 32],
                scope_token: [0; 32],
                generation: request.generation,
                key_epoch: epoch,
                predecessor: request.predecessor,
                objects: request.objects,
                metadata_length: request.metadata_length,
            },
            request,
            workspace,
            control,
            clock,
            context: None,
            poisoned: true,
            wraps: 0,
        };
        p.workspace.clear();
        p.check_now()?;
        fill_os(&mut p.fields.nonce)?;
        fill_os(&mut p.fields.scope_token)?;
        p.fields.validate()?;
        p.poisoned = false;
        Ok(p)
    }
}
impl<'a> ManagedObjectPublication<'a> {
    pub(super) fn attempt<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        if self.poisoned {
            return Err(E::Closed);
        }
        self.poisoned = true;
        struct Guard<'s, 'a> {
            owner: &'s mut ManagedObjectPublication<'a>,
            armed: bool,
        }
        impl Drop for Guard<'_, '_> {
            fn drop(&mut self) {
                if self.armed {
                    self.owner.abort();
                }
            }
        }
        let mut guard = Guard {
            owner: self,
            armed: true,
        };
        let result = action(guard.owner);
        if result.is_ok() {
            guard.owner.poisoned = false;
            guard.armed = false;
        }
        result
    }
    pub fn context_fields(&self) -> ContextFields {
        self.fields
    }
    pub fn bind_context(&mut self, context: Context<'a>) -> Result<(), E> {
        if self.poisoned || self.context.is_some() {
            return Err(E::Closed);
        }
        if context.fields() != self.fields {
            self.abort();
            return Err(E::Unauthorized);
        }
        self.check_now()?;
        self.context = Some(context);
        Ok(())
    }
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }
    pub fn cancel(&mut self) {
        self.abort();
    }
    pub(super) fn check_locked(&self, state: &ControlState, now: u64) -> Result<(), E> {
        checks::grant(self.request.grant, state, KeyPurpose::ObjectContent, now)?;
        checks::policy(
            self.owner,
            self.request.policy_id,
            self.request.policy_key_id,
            KeyOperation::Write,
            self.request.access,
            now,
        )?;
        checks::policy(
            self.owner,
            self.request.policy_id,
            self.request.policy_key_id,
            KeyOperation::Encrypt,
            self.request.access,
            now,
        )
    }
    pub(super) fn check_now(&self) -> Result<(), E> {
        let state = self.control.lock()?;
        self.check_locked(&state, (self.clock)())
    }
    pub(super) fn abort(&mut self) {
        self.poisoned = true;
        self.workspace.clear();
    }
    fn fresh_master(&mut self, slot: usize) -> Result<(), E> {
        let key = self
            .owner
            .key_generator
            .generate_key(
                "object-master".into(),
                KeyType::Symmetric,
                KeyAlgorithm::ChaCha20,
                SecurityLevel::High,
            )
            .map_err(|_| E::CryptoFailure)?;
        let secret = Zeroizing::new(key.key_data);
        let master: &[u8; 32] = secret.as_slice().try_into().map_err(|_| E::CryptoFailure)?;
        if *master == [0; 32] || self.workspace.masters.iter().any(|k| k == master) {
            return Err(E::CryptoFailure);
        }
        self.workspace.masters[slot].copy_from_slice(master);
        Ok(())
    }
    pub fn begin_object<'s>(
        &'s mut self,
        page: GeometryPage<'_>,
        row: usize,
    ) -> Result<ManagedObjectSealer<'s, 'a>, E> {
        if self.poisoned {
            return Err(E::Closed);
        }
        let c = self.context.ok_or(E::Unauthorized)?;
        let reference = c.page_ref(page.page as usize)?;
        if reference.rows as usize != page.count() || sha384(page.encoded()) != reference.digest {
            return Err(E::Unauthorized);
        }
        let g = page.row(row)?;
        let slot = 1 + page.first as usize + row;
        self.begin_slot(slot, g.id, g.length)
    }
    pub fn begin_metadata<'s>(&'s mut self) -> Result<ManagedObjectSealer<'s, 'a>, E> {
        if self.workspace.states[1..1 + self.fields.objects as usize]
            .iter()
            .any(|s| *s != 2)
        {
            return Err(E::Incomplete);
        }
        self.begin_slot(0, 0, self.fields.metadata_length)
    }
    fn begin_slot<'s>(
        &'s mut self,
        slot: usize,
        id: u32,
        length: u64,
    ) -> Result<ManagedObjectSealer<'s, 'a>, E> {
        if self.poisoned || slot > self.fields.objects as usize || self.workspace.states[slot] != 0
        {
            return Err(E::Closed);
        }
        let context = self.context.ok_or(E::Unauthorized)?.id();
        let scope = self.attempt(|p| {
            p.check_now()?;
            let scope =
                ObjectScope::new(context, id, p.fields.generation, p.fields.key_epoch, length)?;
            if length > 0 {
                p.fresh_master(slot)?;
            }
            p.workspace.states[slot] = 1;
            Ok(scope)
        })?;
        Ok(ManagedObjectSealer {
            publication: self,
            slot,
            scope,
            next: 0,
            finished: false,
        })
    }
    /// Only a private metadata serializer may copy a finished entry master into its
    /// admitted PRIVATE metadata buffer. There is no public accessor or export API.
    pub(crate) fn metadata_master_row(&self, ordinal: usize, out: &mut [u8; 32]) -> Result<(), E> {
        if self.poisoned
            || ordinal >= self.fields.objects as usize
            || self.workspace.states[ordinal + 1] != 2
        {
            return Err(E::Unauthorized);
        }
        out.copy_from_slice(&self.workspace.masters[ordinal + 1]);
        Ok(())
    }
    /// Application compositor must first verify recognized recipient proof and the
    /// current whole-source disclosure grant. Provider policy alone is insufficient.
    pub(crate) fn wrap_for(
        &mut self,
        record: RecipientRecord<'_>,
        out: &mut [u8],
    ) -> Result<usize, E> {
        if self.poisoned || self.workspace.states[0] != 2 || self.wraps >= 16 {
            return Err(E::Closed);
        }
        if out.len() < WRAP_BYTES {
            return Err(E::Capacity);
        }
        let c = self.context.ok_or(E::Unauthorized)?;
        if record.fields().context != c.id() {
            return Err(E::Unauthorized);
        }
        let result = self.attempt(|p| {
            p.check_now()?;
            record.valid_at((p.clock)())?;
            p.wraps += 1;
            let (ss, ct) = kem::encapsulate(record.fields().ml_public)?;
            let ss = Zeroizing::new(ss);
            let ephemeral = EphemeralKeyLease::generate(KeyPurpose::ObjectRecipient)?;
            let ep = ephemeral.x25519_public()?;
            let ds = Zeroizing::new(ephemeral.diffie_hellman(record.fields().dh_public)?);
            let wc = WrapContext::new(record, p.fields.metadata_length, &ep, &ct)?;
            let mut encoded = Zeroizing::new([0; WRAP_BYTES]);
            seal_master_raw(wc, &ss, &ds, &p.workspace.masters[0], &mut encoded[..])?;
            let state = p.control.lock()?;
            let now = (p.clock)();
            p.check_locked(&state, now)?;
            record.valid_at(now)?;
            out[..WRAP_BYTES].copy_from_slice(&encoded[..]);
            Ok(WRAP_BYTES)
        });
        if result.is_err() {
            out[..WRAP_BYTES].zeroize();
        }
        result
    }
}
impl Drop for ManagedObjectPublication<'_> {
    fn drop(&mut self) {
        self.workspace.clear();
    }
}
