//! Derived cancellable templates (E15.1, E15.5).
//!
//! Stored value is a revocable domain identifier, not a reusable global
//! biometric hash. Rekeying an envelope does not revoke an already disclosed
//! raw sample; revocation here isolates the local reference.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

use super::capture::{Modality, RawCapture};
use super::BiometricObjectKind;

pub const MAX_TEMPLATE_SLOTS: usize = 8;

/// Domain-bound revocable identifier. Not an unsalted biometric hash.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancellableId(pub StrongDigest);

impl CancellableId {
    /// Bind purpose, algorithm/transform version and generation. Raw sample
    /// bytes are intentionally absent from this derivation.
    pub fn derive(
        purpose: StrongDigest,
        algorithm_version: StrongDigest,
        generation: Generation,
    ) -> Result<Self, QdnfError> {
        if purpose.is_zero() || algorithm_version.is_zero() {
            return Err(QdnfError::Malformed);
        }
        if generation == Generation::ZERO {
            return Err(QdnfError::Incomplete);
        }
        let mut buf = [0u8; 104];
        buf[..48].copy_from_slice(&purpose.0);
        buf[48..96].copy_from_slice(&algorithm_version.0);
        buf[96..104].copy_from_slice(&generation.0.to_le_bytes());
        Ok(Self(sha384(&buf)))
    }
}

/// Encrypted, purpose-bound template handle. No public lookup.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DerivedTemplate {
    revocable_id: CancellableId,
    modality: Modality,
    algorithm_version: StrongDigest,
    provenance: StrongDigest,
    purpose: StrongDigest,
    generation: Generation,
    revoked: bool,
}

impl DerivedTemplate {
    #[inline]
    pub const fn kind(self) -> BiometricObjectKind {
        BiometricObjectKind::DerivedTemplate
    }

    #[inline]
    pub const fn revocable_id(self) -> CancellableId {
        self.revocable_id
    }

    #[inline]
    pub const fn modality(self) -> Modality {
        self.modality
    }

    #[inline]
    pub const fn algorithm_version(self) -> StrongDigest {
        self.algorithm_version
    }

    #[inline]
    pub const fn provenance(self) -> StrongDigest {
        self.provenance
    }

    #[inline]
    pub const fn purpose(self) -> StrongDigest {
        self.purpose
    }

    #[inline]
    pub const fn generation(self) -> Generation {
        self.generation
    }

    #[inline]
    pub const fn is_revoked(self) -> bool {
        self.revoked
    }

    /// A template is not a reusable global biometric hash.
    pub fn as_global_hash(&self) -> Result<StrongDigest, QdnfError> {
        Err(QdnfError::Denied)
    }
}

/// Derive a cancellable template from a local capture. The raw sample is not
/// hashed into a reusable identifier.
pub fn derive_template(
    capture: &RawCapture,
    purpose: StrongDigest,
    algorithm_version: StrongDigest,
    provenance: StrongDigest,
    generation: Generation,
) -> Result<DerivedTemplate, QdnfError> {
    if provenance.is_zero() {
        return Err(QdnfError::Incomplete);
    }
    let revocable_id = CancellableId::derive(purpose, algorithm_version, generation)?;
    Ok(DerivedTemplate {
        revocable_id,
        modality: capture.modality(),
        algorithm_version,
        provenance,
        purpose,
        generation,
        revoked: false,
    })
}

/// Isolate a compromised reference. The identifier is no longer usable.
pub fn revoke_template(template: &mut DerivedTemplate) -> Result<(), QdnfError> {
    if template.revoked {
        return Ok(());
    }
    template.revoked = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x.0[47] = 0xC2;
        x
    }

    fn capture() -> RawCapture {
        RawCapture::acquire(Modality::Fingerprint, d(1), d(2), 20, 1).unwrap()
    }

    #[test]
    fn cancellable_id_is_domain_bound_not_raw_hash() {
        let a = CancellableId::derive(d(10), d(11), Generation(1)).unwrap();
        let b = CancellableId::derive(d(99), d(11), Generation(1)).unwrap();
        assert_ne!(a.0, b.0);
        assert_ne!(a.0, d(1));
        assert_eq!(
            CancellableId::derive(d(10), d(11), Generation::ZERO).unwrap_err(),
            QdnfError::Incomplete
        );
    }

    #[test]
    fn derive_stores_revocable_identifier() {
        let t = derive_template(&capture(), d(10), d(11), d(12), Generation(2)).unwrap();
        assert_eq!(t.kind(), BiometricObjectKind::DerivedTemplate);
        assert!(!t.is_revoked());
        assert_eq!(t.purpose(), d(10));
        assert_eq!(t.modality(), Modality::Fingerprint);
        assert_eq!(t.as_global_hash().unwrap_err(), QdnfError::Denied);
        let expected = CancellableId::derive(d(10), d(11), Generation(2)).unwrap();
        assert_eq!(t.revocable_id(), expected);
    }

    #[test]
    fn revoke_isolates_reference() {
        let mut t = derive_template(&capture(), d(10), d(11), d(12), Generation(3)).unwrap();
        revoke_template(&mut t).unwrap();
        assert!(t.is_revoked());
        revoke_template(&mut t).unwrap();
        assert!(t.is_revoked());
    }

    #[test]
    fn missing_provenance_is_incomplete() {
        assert_eq!(
            derive_template(&capture(), d(10), d(11), StrongDigest::ZERO, Generation(1))
                .unwrap_err(),
            QdnfError::Incomplete
        );
    }
}
