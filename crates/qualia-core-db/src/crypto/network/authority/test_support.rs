use super::*;
use crate::crypto::network::types::{KeyEpoch, KeyPurpose, ML_DSA_65_PK_LEN};
use crate::crypto::network::{cose, key_provider::ProviderGrant};
use crate::net::qdnf::types::StrongDigest;
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{KeyGen, SerDes},
};

pub(super) const DIGEST: [u8; 48] = [42; 48];
pub(super) struct Fixture {
    ml_sk: ml_dsa_65::PrivateKey,
    ed_sk: SigningKey,
    pub ml_pk: [u8; ML_DSA_65_PK_LEN],
    pub ed_pk: [u8; 32],
    pub epoch: u64,
}
impl Fixture {
    pub fn new(seed: u8, epoch: u64) -> Self {
        let (ml_pk, ml_sk) = ml_dsa_65::KG::keygen_from_seed(&[seed; 32]);
        let ed_sk = SigningKey::from_bytes(&[seed + 1; 32]);
        let ed_pk = ed_sk.verifying_key().to_bytes();
        Self {
            ml_sk,
            ed_sk,
            ml_pk: ml_pk.into_bytes(),
            ed_pk,
            epoch,
        }
    }
    pub fn keys(&self) -> cose::PairedPublicKeys<'_> {
        cose::PairedPublicKeys {
            ml_dsa_65: &self.ml_pk,
            ed25519: &self.ed_pk,
        }
    }
    pub fn config(&self) -> PinConfig<'_> {
        PinConfig {
            principal: StrongDigest([31; 48]),
            provider_epoch: KeyEpoch {
                purpose: KeyPurpose::ControllerSign,
                epoch: self.epoch,
                controller: 9,
            },
            keys: self.keys(),
            purpose: b"publication",
            audience: b"reader",
            not_before_unix: 10,
            expires_unix: 100,
        }
    }
    pub fn binding(&self) -> cose::Binding<'static> {
        cose::Binding {
            descriptor_digest: &DIGEST,
            key_ids: self.keys().key_ids(),
            controller_epoch: self.epoch,
            purpose: b"publication",
            audience: b"reader",
        }
    }
    pub fn signed(&self) -> ([u8; cose::MAX_ENVELOPE_BYTES], usize) {
        let mut out = [0; cose::MAX_ENVELOPE_BYTES];
        let keys = cose::SigningKeys {
            ml_dsa_65: &self.ml_sk,
            ed25519: &self.ed_sk,
        };
        let n = cose::sign_pair_raw(
            &self.binding(),
            &keys,
            &mut cose::Workspace::new(),
            &mut out,
        )
        .unwrap();
        (out, n)
    }
    pub fn grant(&self) -> ProviderGrant {
        ProviderGrant {
            epoch: self.config().provider_epoch,
            cell_epoch: 5,
            authority_generation: 7,
            deadline_unix: 90,
            revoked: false,
        }
    }
    pub fn installed(&self) -> (PinAuthority, LocalBootstrap, PinHandle, AuthorityReceipt) {
        let (mut owner, admin) = PinAuthority::new_local_bootstrap().unwrap();
        let handle = owner.install_pin(&admin, self.config(), 10).unwrap();
        let (wire, n) = self.signed();
        let receipt = owner
            .verify_source(
                AuthoritySource::LocalPin(handle),
                &wire[..n],
                &self.binding(),
                10,
                &mut cose::Workspace::new(),
            )
            .unwrap();
        (owner, admin, handle, receipt)
    }
}
pub(super) fn request(grant: &ProviderGrant) -> KeyRequest<'_> {
    KeyRequest {
        grant,
        current_provider_generation: 7,
        cell_epoch: 5,
        deadline_unix: 80,
    }
}
