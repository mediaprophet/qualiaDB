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
    /// Inverse of [`Self::encode`]. Two-process PSK packages carry this layout.
    pub fn decode(src: &[u8]) -> Result<Self, E> {
        let src = src.get(..INTENT_BYTES).ok_or(E::Capacity)?;
        let mut p = 0;
        let digest = |p: &mut usize| -> StrongDigest {
            let mut d = StrongDigest::ZERO;
            d.0.copy_from_slice(&src[*p..*p + 48]);
            *p += 48;
            d
        };
        let initiator = digest(&mut p);
        let responder = digest(&mut p);
        let service = digest(&mut p);
        let purpose = digest(&mut p);
        let instrument = digest(&mut p);
        let scope = digest(&mut p);
        let mut operation = OperationId([0; 16]);
        operation.0.copy_from_slice(&src[p..p + 16]);
        p += 16;
        let key = |p: &mut usize| -> [u8; 48] {
            let mut k = [0u8; 48];
            k.copy_from_slice(&src[*p..*p + 48]);
            *p += 48;
            k
        };
        let initiator_keys = KeyIds {
            ml_dsa_65: key(&mut p),
            ed25519: key(&mut p),
        };
        let responder_keys = KeyIds {
            ml_dsa_65: key(&mut p),
            ed25519: key(&mut p),
        };
        let u64be = |p: &mut usize| -> u64 {
            let mut raw = [0u8; 8];
            raw.copy_from_slice(&src[*p..*p + 8]);
            *p += 8;
            u64::from_be_bytes(raw)
        };
        let intent = Self {
            initiator,
            responder,
            service,
            purpose,
            instrument,
            scope,
            operation,
            initiator_keys,
            responder_keys,
            initiator_epoch: u64be(&mut p),
            responder_epoch: u64be(&mut p),
            expires_unix: u64be(&mut p),
        };
        if p != INTENT_BYTES {
            return Err(E::Capacity);
        }
        intent.validate()?;
        Ok(intent)
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

#[cfg(test)]
mod decode_tests {
    use super::*;
    fn digest(b: u8) -> StrongDigest {
        StrongDigest([b; 48])
    }
    fn keys(a: u8, b: u8) -> KeyIds {
        KeyIds {
            ml_dsa_65: [a; 48],
            ed25519: [b; 48],
        }
    }
    #[test]
    fn encode_decode_roundtrip() {
        let intent = Intent {
            initiator: digest(1),
            responder: digest(2),
            service: digest(3),
            purpose: digest(4),
            instrument: digest(5),
            scope: digest(6),
            operation: OperationId([7; 16]),
            initiator_keys: keys(8, 9),
            responder_keys: keys(10, 11),
            initiator_epoch: 1,
            responder_epoch: 2,
            expires_unix: 90,
        };
        let mut out = [0u8; INTENT_BYTES];
        assert_eq!(intent.encode(&mut out).unwrap(), INTENT_BYTES);
        let got = Intent::decode(&out).unwrap();
        assert_eq!(got, intent);
        match Intent::decode(&out[..INTENT_BYTES - 1]) {
            Err(_) => {}
            Ok(_) => panic!("short intent must refuse"),
        }
    }
}
