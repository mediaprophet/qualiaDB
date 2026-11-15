//! Managed paired signing over the existing key store, with no secret export.
//! Policy and clock inputs are trusted-host context, not wire authentication.
//! This operation is not an AuthorityOwner execution permit or publisher pin.
use super::{AccessPolicy, Key, KeyAlgorithm, KeyManager, KeyType, SigningAccess};
use crate::crypto::network::{
    cose::{Binding, PairedPublicKeys},
    errors::CryptoError,
    key_provider::ProviderGrant,
    types::KeyPurpose,
};
use crate::specialized_libs::cryptographic_library::signing_profile::RawSigningKeys;
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{SerDes, Signer},
};

use super::managed_control::ControlState;
pub use super::managed_control::ManagedOperationControl as SigningControl;

/// Exact locally admitted key/policy identifiers and exact signed binding.
/// ProviderGrant is checked in addition to mandatory key policies; it does not
/// turn caller-claimed identity or methods into trusted evidence.
pub struct PairedSigningRequest<'a> {
    pub ml_key_id: &'a str,
    pub ed_key_id: &'a str,
    pub ml_policy_id: &'a str,
    pub ed_policy_id: &'a str,
    pub binding: Binding<'a>,
    pub grant: ProviderGrant,
    pub access: SigningAccess<'a>,
}

/// Single-use signing capability issued only by the existing KeyManager.
/// Its immutable owner borrow prevents replacing keys/policies while in flight.
/// Decoded provider keys are scoped here and zeroized by their provider owners.
pub struct ManagedPairedSigning<'a> {
    owner: &'a KeyManager,
    request: PairedSigningRequest<'a>,
    control: &'a SigningControl,
    clock: &'a dyn Fn() -> u64,
    ml: ml_dsa_65::PrivateKey,
    ed: SigningKey,
}

fn check_key<'a>(
    owner: &'a KeyManager,
    id: &str,
    algorithm: KeyAlgorithm,
    now: u64,
) -> Result<&'a Key, CryptoError> {
    let key = owner
        .key_storage
        .borrow_managed_key(id)
        .ok_or(CryptoError::Unauthorized)?;
    if key.key_id != id
        || key.metadata.key_id != id
        || key.key_type != KeyType::Private
        || key.metadata.key_type != KeyType::Private
        || key.key_algorithm != algorithm
        || key.metadata.key_algorithm != algorithm
        || now < key.metadata.created_at
        || (key.metadata.expires_at != 0 && now >= key.metadata.expires_at)
    {
        return Err(CryptoError::Unauthorized);
    }
    Ok(key)
}

impl KeyManager {
    /// Local administrative policy installation, not a wire/request operation.
    /// The host must authenticate and authorize administration before calling.
    pub fn install_managed_signing_policy(&mut self, policy: AccessPolicy) {
        self.key_storage.access_control.add_policy(policy);
    }

    /// Admit using trusted host-authenticated context and trusted Unix clock.
    /// No policies means denial, including on a freshly initialized manager.
    /// A caller must also hold any application execution permit separately.
    /// The clock must be side-effect free and must not reenter SigningControl.
    pub fn begin_paired_signing<'a>(
        &'a self,
        request: PairedSigningRequest<'a>,
        control: &'a SigningControl,
        clock: &'a dyn Fn() -> u64,
    ) -> Result<ManagedPairedSigning<'a>, CryptoError> {
        let now = clock();
        {
            let state = control.lock()?;
            validate_request(self, &request, &state, now)?;
        }
        let ml = check_key(self, request.ml_key_id, KeyAlgorithm::MLDSA, now)?;
        let ed = check_key(self, request.ed_key_id, KeyAlgorithm::EdDSA, now)?;
        let ml_bytes: &[u8; 4032] = ml
            .key_data
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::Unauthorized)?;
        let ed_bytes: &[u8; 32] = ed
            .key_data
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::Unauthorized)?;
        let ml = ml_dsa_65::PrivateKey::try_from_bytes(*ml_bytes)
            .map_err(|_| CryptoError::Unauthorized)?;
        let ed = SigningKey::from_bytes(ed_bytes);
        let ml_public = ml.get_public_key().into_bytes();
        let ed_public = ed.verifying_key().to_bytes();
        if request.binding.key_ids
            != (PairedPublicKeys {
                ml_dsa_65: &ml_public,
                ed25519: &ed_public,
            })
            .key_ids()
        {
            return Err(CryptoError::Unauthorized);
        }
        Ok(ManagedPairedSigning {
            owner: self,
            request,
            control,
            clock,
            ml,
            ed,
        })
    }
}

fn validate_request(
    owner: &KeyManager,
    request: &PairedSigningRequest<'_>,
    state: &ControlState,
    now: u64,
) -> Result<(), CryptoError> {
    if state.cancelled {
        return Err(CryptoError::Revoked);
    }
    if request.ml_key_id == request.ed_key_id
        || request.grant.epoch.controller == 0
        || request.grant.cell_epoch == 0
        || request.grant.epoch.epoch == 0
        || request.binding.controller_epoch != request.grant.epoch.epoch
    {
        return Err(CryptoError::Unauthorized);
    }
    request.grant.authorize(
        KeyPurpose::ControllerSign,
        request.grant.epoch.controller,
        now,
        state.generation,
    )?;
    let policy = &owner.key_storage.access_control;
    if !policy.permits_managed_signing(request.ml_policy_id, request.ml_key_id, request.access, now)
        || !policy.permits_managed_signing(
            request.ed_policy_id,
            request.ed_key_id,
            request.access,
            now,
        )
    {
        return Err(CryptoError::Unauthorized);
    }
    check_key(owner, request.ml_key_id, KeyAlgorithm::MLDSA, now)?;
    check_key(owner, request.ed_key_id, KeyAlgorithm::EdDSA, now)?;
    Ok(())
}

impl ManagedPairedSigning<'_> {
    pub(crate) fn keys(&self) -> RawSigningKeys<'_> {
        RawSigningKeys {
            ml_dsa_65: &self.ml,
            ed25519: &self.ed,
        }
    }
    pub(crate) fn check_binding(&self, binding: &Binding<'_>) -> Result<(), CryptoError> {
        if self.request.binding != *binding {
            return Err(CryptoError::Unauthorized);
        }
        let now = (self.clock)();
        let state = self.control.lock()?;
        validate_request(self.owner, &self.request, &state, now)
    }
    /// Serialize the final output publication against cancel/generation updates.
    /// Crypto executes BEFORE this lock; a cancel during signing wins. Consuming
    /// self prevents reuse. The publish closure is an internal bounded encoder.
    pub(crate) fn publish<T>(
        self,
        publish: impl FnOnce() -> Result<T, CryptoError>,
    ) -> Result<T, CryptoError> {
        let state = self.control.lock()?;
        let now = (self.clock)();
        validate_request(self.owner, &self.request, &state, now)?;
        publish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::{cose, types::KeyEpoch};
    use crate::specialized_libs::cryptographic_library::{
        AuthenticationMethod, KeyOperation, SecurityLevel, TimeRestrictions,
    };
    use std::cell::Cell;

    fn policy(id: &str) -> AccessPolicy {
        AccessPolicy {
            policy_id: id.into(),
            key_id: id.into(),
            allowed_operations: vec![KeyOperation::Sign],
            required_auth: vec![AuthenticationMethod::HardwareToken],
            time_restrictions: TimeRestrictions {
                allowed_hours: vec![],
                allowed_days: vec![],
                start_date: None,
                end_date: None,
            },
            ip_restrictions: vec![],
        }
    }
    fn fixture() -> (KeyManager, cose::KeyIds) {
        let mut manager = KeyManager::new();
        manager.initialize().unwrap();
        let (ml, ml_pub) = manager
            .generate_key_pair(
                "ml".into(),
                KeyType::Private,
                KeyAlgorithm::MLDSA,
                SecurityLevel::High,
            )
            .unwrap();
        let (ed, ed_pub) = manager
            .generate_key_pair(
                "ed".into(),
                KeyType::Private,
                KeyAlgorithm::EdDSA,
                SecurityLevel::High,
            )
            .unwrap();
        let ids = PairedPublicKeys {
            ml_dsa_65: ml_pub.key_data.as_slice().try_into().unwrap(),
            ed25519: ed_pub.key_data.as_slice().try_into().unwrap(),
        }
        .key_ids();
        manager.store_key(ml).unwrap();
        manager.store_key(ed).unwrap();
        (manager, ids)
    }
    fn request(ids: cose::KeyIds) -> PairedSigningRequest<'static> {
        PairedSigningRequest {
            ml_key_id: "ml_private",
            ed_key_id: "ed_private",
            ml_policy_id: "ml_private",
            ed_policy_id: "ed_private",
            binding: Binding {
                descriptor_digest: &[7; 48],
                key_ids: ids,
                controller_epoch: 1,
                purpose: b"publication",
                audience: b"local",
            },
            grant: ProviderGrant {
                epoch: KeyEpoch {
                    purpose: KeyPurpose::ControllerSign,
                    epoch: 1,
                    controller: 2,
                },
                cell_epoch: 1,
                authority_generation: 1,
                deadline_unix: u64::MAX,
                revoked: false,
            },
            access: SigningAccess {
                authenticated_methods: &[AuthenticationMethod::HardwareToken],
                ip_address: "local",
            },
        }
    }
    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
    #[test]
    fn managed_requires_policy_algorithms_ids_and_exact_binding() {
        let (mut manager, ids) = fixture();
        let control = SigningControl::new(1).unwrap();
        assert!(
            manager
                .begin_paired_signing(request(ids), &control, &now)
                .is_err()
        );
        manager.install_managed_signing_policy(policy("ml_private"));
        manager.install_managed_signing_policy(policy("ed_private"));
        let mut wrong = request(ids);
        wrong.ed_key_id = "ml_private";
        assert!(manager.begin_paired_signing(wrong, &control, &now).is_err());
        let mut wrong = request(ids);
        wrong.ml_key_id = "missing";
        assert!(manager.begin_paired_signing(wrong, &control, &now).is_err());
        let mut wrong = request(ids);
        wrong.binding.key_ids.ed25519[0] ^= 1;
        assert!(manager.begin_paired_signing(wrong, &control, &now).is_err());
        let cap = manager
            .begin_paired_signing(request(ids), &control, &now)
            .unwrap();
        let mut changed = request(ids).binding;
        changed.audience = b"other";
        assert_eq!(cap.check_binding(&changed), Err(CryptoError::Unauthorized));
        assert_eq!(cap.publish(|| Ok(7)), Ok(7));
    }
    #[test]
    fn cancel_generation_and_late_clock_fence_publication() {
        let (mut manager, ids) = fixture();
        manager.install_managed_signing_policy(policy("ml_private"));
        manager.install_managed_signing_policy(policy("ed_private"));
        let control = SigningControl::new(1).unwrap();
        let cap = manager
            .begin_paired_signing(request(ids), &control, &now)
            .unwrap();
        control.cancel().unwrap();
        let published = Cell::new(false);
        assert_eq!(
            cap.publish(|| {
                published.set(true);
                Ok(())
            }),
            Err(CryptoError::Revoked)
        );
        assert!(!published.get());
        let control = SigningControl::new(1).unwrap();
        let cap = manager
            .begin_paired_signing(request(ids), &control, &now)
            .unwrap();
        control.advance_generation(2).unwrap();
        assert_eq!(cap.publish(|| Ok(())), Err(CryptoError::StaleGeneration));
        assert_eq!(
            control.advance_generation(1),
            Err(CryptoError::StaleGeneration)
        );
        let control = SigningControl::new(1).unwrap();
        let time = Cell::new(now());
        let clock = || time.get();
        let mut req = request(ids);
        req.grant.deadline_unix = time.get() + 10;
        let cap = manager.begin_paired_signing(req, &control, &clock).unwrap();
        time.set(time.get() + 10);
        assert_eq!(cap.publish(|| Ok(())), Err(CryptoError::Expired));
    }

    #[test]
    fn managed_signing_roundtrip_and_late_failure_erases_signatures() {
        let (mut manager, ids) = fixture();
        manager.install_managed_signing_policy(policy("ml_private"));
        manager.install_managed_signing_policy(policy("ed_private"));
        let control = SigningControl::new(1).unwrap();
        let cap = manager
            .begin_paired_signing(request(ids), &control, &now)
            .unwrap();
        let ml_public = cap.keys().ml_dsa_65.get_public_key().into_bytes();
        let ed_public = cap.keys().ed25519.verifying_key().to_bytes();
        let public = PairedPublicKeys {
            ml_dsa_65: &ml_public,
            ed25519: &ed_public,
        };
        let binding = request(ids).binding;
        let mut work = cose::Workspace::new();
        let mut out = [0x5a; cose::MAX_ENVELOPE_BYTES];
        let n = cose::sign_pair(&binding, cap, &mut work, &mut out).unwrap();
        assert!(cose::verify_pair(&out[..n], &binding, &public, &mut work).is_ok());

        let calls = Cell::new(0);
        let start = now();
        // Admission and pre-sign check succeed; the post-sign clock expires.
        let clock = || {
            calls.set(calls.get() + 1);
            if calls.get() >= 3 { start + 10 } else { start }
        };
        let mut req = request(ids);
        req.grant.deadline_unix = start + 10;
        let cap = manager.begin_paired_signing(req, &control, &clock).unwrap();
        out.fill(0x5a);
        assert_eq!(
            cose::sign_pair(&binding, cap, &mut work, &mut out),
            Err(CryptoError::Expired)
        );
        assert!(out.iter().all(|b| *b == 0x5a));
        assert!(work.signatures.ml_dsa_65.iter().all(|b| *b == 0));
        assert!(work.signatures.ed25519.iter().all(|b| *b == 0));

        let mut swapped = request(ids);
        swapped.ml_key_id = "ed_private";
        swapped.ed_key_id = "ml_private";
        swapped.ml_policy_id = "ed_private";
        swapped.ed_policy_id = "ml_private";
        assert!(
            manager
                .begin_paired_signing(swapped, &control, &now)
                .is_err()
        );
    }
}
