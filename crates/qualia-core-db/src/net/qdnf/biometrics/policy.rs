//! Template publication, reuse, identity assertion and recovery (E15.2, E15.4).
//!
//! Public templates, unsalted reusable hashes, cross-purpose linking and
//! silent identification/surveillance reuse are Denied. Identity assertion
//! requires an independent authorised instrument. A non-biometric PIN/key-ref
//! recovery path remains available.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::profiles::{load_key_ref, KeyRef, LocalVault};
use crate::net::qdnf::types::{Generation, StrongDigest};

use super::capture::RawCapture;
use super::match_local::MatchResult;
use super::template::DerivedTemplate;
use super::BiometricObjectKind;

/// Independent credential/identity decision. Never implicit sameAs from a match.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityAssertion {
    issuer: StrongDigest,
    instrument: StrongDigest,
    purpose: StrongDigest,
    generation: Generation,
}

impl IdentityAssertion {
    pub fn from_independent_instrument(
        issuer: StrongDigest,
        instrument: StrongDigest,
        purpose: StrongDigest,
        generation: Generation,
    ) -> Result<Self, QdnfError> {
        if issuer.is_zero() || instrument.is_zero() || purpose.is_zero() {
            return Err(QdnfError::Unauthorized);
        }
        if generation == Generation::ZERO {
            return Err(QdnfError::Unauthorized);
        }
        Ok(Self {
            issuer,
            instrument,
            purpose,
            generation,
        })
    }

    #[inline]
    pub const fn kind(self) -> BiometricObjectKind {
        BiometricObjectKind::IdentityAssertion
    }

    #[inline]
    pub const fn issuer(self) -> StrongDigest {
        self.issuer
    }

    #[inline]
    pub const fn instrument(self) -> StrongDigest {
        self.instrument
    }

    #[inline]
    pub const fn purpose(self) -> StrongDigest {
        self.purpose
    }

    #[inline]
    pub const fn generation(self) -> Generation {
        self.generation
    }
}

/// A match result is not an identity assertion and cannot mint one.
pub fn assert_identity_from_match(_result: &MatchResult) -> Result<IdentityAssertion, QdnfError> {
    Err(QdnfError::Denied)
}

/// Public template indexes are not permitted.
pub fn publish_template(_template: &DerivedTemplate) -> Result<(), QdnfError> {
    Err(QdnfError::Denied)
}

/// Unsalted reusable biometric hashes are never allowed.
#[inline]
pub const fn unsalted_reusable_hash_allowed() -> bool {
    false
}

pub fn unsalted_reusable_hash(_capture: &RawCapture) -> Result<StrongDigest, QdnfError> {
    Err(QdnfError::Denied)
}

/// Cross-purpose / cross-domain template linking is Denied (S19).
pub fn cross_purpose_link(a: &DerivedTemplate, b: &DerivedTemplate) -> Result<(), QdnfError> {
    let _ = (a, b);
    Err(QdnfError::Denied)
}

/// Requested reuse of a stored template.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReuseIntent {
    AuthorisedLocalMatch = 1,
    Identification = 2,
    Surveillance = 3,
}

/// Silent identification and surveillance reuse are Denied. Authorised local
/// match is allowed only for the recorded purpose on a live template.
pub fn reuse_template(
    template: &DerivedTemplate,
    intent: ReuseIntent,
    requested_purpose: StrongDigest,
) -> Result<(), QdnfError> {
    match intent {
        ReuseIntent::Identification | ReuseIntent::Surveillance => Err(QdnfError::Denied),
        ReuseIntent::AuthorisedLocalMatch => {
            if template.is_revoked() {
                return Err(QdnfError::Revoked);
            }
            if requested_purpose.is_zero() || requested_purpose != template.purpose() {
                return Err(QdnfError::Denied);
            }
            Ok(())
        }
    }
}

/// Covert population identification from a template is Denied.
pub fn silent_identify(_template: &DerivedTemplate) -> Result<(), QdnfError> {
    Err(QdnfError::Denied)
}

/// Non-biometric recovery/access alternative remains available (S20).
#[inline]
pub const fn recovery_without_biometric() -> bool {
    true
}

/// PIN knowledge check. `pin_digest` is not a biometric hash.
pub fn recover_with_pin(
    pin_digest: StrongDigest,
    stored_pin_digest: StrongDigest,
) -> Result<(), QdnfError> {
    if pin_digest.is_zero() || stored_pin_digest.is_zero() {
        return Err(QdnfError::Malformed);
    }
    if pin_digest != stored_pin_digest {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

/// Hardware/vault key-ref recovery. Possession of a slot handle, not a sample.
pub fn recover_with_key_ref(
    vault: &mut LocalVault,
    key_ref: KeyRef,
    now_unix: u64,
) -> Result<StrongDigest, QdnfError> {
    load_key_ref(vault, key_ref, now_unix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::biometrics::capture::{Modality, RawCapture};
    use crate::net::qdnf::biometrics::match_local::match_local;
    use crate::net::qdnf::biometrics::template::{derive_template, revoke_template};
    use crate::net::qdnf::profiles::store_ref;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x.0[47] = 0xC5;
        x
    }

    fn capture() -> RawCapture {
        RawCapture::acquire(Modality::Face, d(1), d(2), 30, 1).unwrap()
    }

    fn template_for(purpose: u8) -> DerivedTemplate {
        derive_template(&capture(), d(purpose), d(11), d(12), Generation(5)).unwrap()
    }

    #[test]
    fn identity_assertion_is_independent_of_match() {
        let probe = capture();
        let reference = template_for(10);
        let matched = match_local(&probe, &reference, true, 1, 50).unwrap();
        assert_eq!(
            assert_identity_from_match(&matched).unwrap_err(),
            QdnfError::Denied
        );
        let assertion =
            IdentityAssertion::from_independent_instrument(d(20), d(21), d(22), Generation(1))
                .unwrap();
        assert_eq!(assertion.kind(), BiometricObjectKind::IdentityAssertion);
        assert_ne!(assertion.kind(), matched.kind());
    }

    #[test]
    fn public_templates_and_unsalted_hashes_denied() {
        let t = template_for(10);
        assert_eq!(publish_template(&t).unwrap_err(), QdnfError::Denied);
        assert!(!unsalted_reusable_hash_allowed());
        assert_eq!(
            unsalted_reusable_hash(&capture()).unwrap_err(),
            QdnfError::Denied
        );
    }

    #[test]
    fn cross_purpose_linking_denied() {
        let a = template_for(10);
        let b = template_for(11);
        assert_ne!(a.purpose(), b.purpose());
        assert_eq!(cross_purpose_link(&a, &b).unwrap_err(), QdnfError::Denied);
        assert_eq!(cross_purpose_link(&a, &a).unwrap_err(), QdnfError::Denied);
    }

    #[test]
    fn silent_identification_and_surveillance_denied() {
        let t = template_for(10);
        assert_eq!(silent_identify(&t).unwrap_err(), QdnfError::Denied);
        assert_eq!(
            reuse_template(&t, ReuseIntent::Identification, d(10)).unwrap_err(),
            QdnfError::Denied
        );
        assert_eq!(
            reuse_template(&t, ReuseIntent::Surveillance, d(10)).unwrap_err(),
            QdnfError::Denied
        );
        reuse_template(&t, ReuseIntent::AuthorisedLocalMatch, d(10)).unwrap();
        assert_eq!(
            reuse_template(&t, ReuseIntent::AuthorisedLocalMatch, d(99)).unwrap_err(),
            QdnfError::Denied
        );
    }

    #[test]
    fn revoked_template_cannot_be_reused_locally() {
        let mut t = template_for(10);
        revoke_template(&mut t).unwrap();
        assert_eq!(
            reuse_template(&t, ReuseIntent::AuthorisedLocalMatch, d(10)).unwrap_err(),
            QdnfError::Revoked
        );
    }

    #[test]
    fn pin_and_key_ref_recovery_are_not_biometric_hashes() {
        assert!(recovery_without_biometric());
        recover_with_pin(d(7), d(7)).unwrap();
        assert_eq!(recover_with_pin(d(7), d(8)).unwrap_err(), QdnfError::Denied);
        assert_eq!(
            recover_with_pin(StrongDigest::ZERO, d(7)).unwrap_err(),
            QdnfError::Malformed
        );

        let mut vault = LocalVault::new();
        let slot = store_ref(&mut vault, d(40), 1, 100).unwrap();
        let key_ref = vault.issued_ref(slot).unwrap();
        let loaded = recover_with_key_ref(&mut vault, key_ref, 2).unwrap();
        assert_eq!(loaded, d(40));
        assert_ne!(loaded, capture().device());
    }
}
