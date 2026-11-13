use super::{E, Role};
use crate::crypto::network::cose::KeyIds;
use crate::net::qdnf::{
    authority::AuthorityBinding,
    types::{OperationId, StrongDigest},
};
pub const INTENT_BYTES: usize = 520;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intent {
    pub initiator: StrongDigest,
    pub responder: StrongDigest,
    pub service: StrongDigest,
    pub purpose: StrongDigest,
    pub instrument: StrongDigest,
    pub scope: StrongDigest,
    pub operation: OperationId,
    pub initiator_keys: KeyIds,
    pub responder_keys: KeyIds,
    pub initiator_epoch: u64,
    pub responder_epoch: u64,
    pub expires_unix: u64,
}
impl Intent {
    pub fn validate(&self) -> Result<(), E> {
        if [
            self.initiator,
            self.responder,
            self.service,
            self.purpose,
            self.instrument,
            self.scope,
        ]
        .iter()
        .any(|d| d.is_zero())
            || self.initiator == self.responder
            || self.operation.is_zero()
            || self.initiator_epoch == 0
            || self.responder_epoch == 0
            || self.expires_unix == 0
        {
            return Err(E::Unauthorized);
        }
        for k in [
            &self.initiator_keys.ml_dsa_65,
            &self.initiator_keys.ed25519,
            &self.responder_keys.ml_dsa_65,
            &self.responder_keys.ed25519,
        ] {
            if *k == [0; 48] {
                return Err(E::Unauthorized);
            }
        }
        Ok(())
    }
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, E> {
        self.validate()?;
        let out = out.get_mut(..INTENT_BYTES).ok_or(E::Capacity)?;
        let mut p = 0;
        for d in [
            self.initiator,
            self.responder,
            self.service,
            self.purpose,
            self.instrument,
            self.scope,
        ] {
            out[p..p + 48].copy_from_slice(&d.0);
            p += 48;
        }
        out[p..p + 16].copy_from_slice(&self.operation.0);
        p += 16;
        for k in [
            self.initiator_keys.ml_dsa_65,
            self.initiator_keys.ed25519,
            self.responder_keys.ml_dsa_65,
            self.responder_keys.ed25519,
        ] {
            out[p..p + 48].copy_from_slice(&k);
            p += 48;
        }
        for n in [
            self.initiator_epoch,
            self.responder_epoch,
            self.expires_unix,
        ] {
            out[p..p + 8].copy_from_slice(&n.to_be_bytes());
            p += 8;
        }
        Ok(p)
    }
    pub(super) fn authority(&self, role: Role) -> AuthorityBinding {
        AuthorityBinding {
            principal: self.local(role),
            recipient: self.peer(role),
            purpose: self.purpose,
            instrument: self.instrument,
            scope: self.scope,
            operation: self.operation,
        }
    }
    pub(super) fn local(&self, r: Role) -> StrongDigest {
        if r.initiator() {
            self.initiator
        } else {
            self.responder
        }
    }
    pub(super) fn peer(&self, r: Role) -> StrongDigest {
        if r.initiator() {
            self.responder
        } else {
            self.initiator
        }
    }
    pub(super) fn keys(&self, r: Role) -> KeyIds {
        if r.initiator() {
            self.initiator_keys
        } else {
            self.responder_keys
        }
    }
    pub(super) fn epoch(&self, r: Role) -> u64 {
        if r.initiator() {
            self.initiator_epoch
        } else {
            self.responder_epoch
        }
    }
}
