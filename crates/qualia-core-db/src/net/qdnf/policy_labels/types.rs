//! Bounded information-flow label fields. Unknown is never Public.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Local confidentiality lattice. `Unknown` is a failure state, never C0.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidentiality {
    C0Public = 0,
    C1Private = 1,
    C2Sensitive = 2,
    C3Compartmented = 3,
    Unknown = 255,
}

impl Confidentiality {
    #[inline]
    pub const fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Rank in the C0–C3 lattice. Unknown has no rank and cannot join.
    #[inline]
    pub const fn lattice_rank(self) -> Result<u8, QdnfError> {
        match self {
            Self::C0Public => Ok(0),
            Self::C1Private => Ok(1),
            Self::C2Sensitive => Ok(2),
            Self::C3Compartmented => Ok(3),
            Self::Unknown => Err(QdnfError::Conflict),
        }
    }

    #[inline]
    pub const fn requires_audience(self) -> bool {
        matches!(self, Self::C2Sensitive | Self::C3Compartmented)
    }

    pub fn from_wire(value: u8) -> Result<Self, QdnfError> {
        match value {
            0 => Ok(Self::C0Public),
            1 => Ok(Self::C1Private),
            2 => Ok(Self::C2Sensitive),
            3 => Ok(Self::C3Compartmented),
            255 => Ok(Self::Unknown),
            _ => Err(QdnfError::Malformed),
        }
    }

    #[inline]
    pub const fn to_wire(self) -> u8 {
        self as u8
    }
}

/// Restriction bits. Union (OR) at join; never silently dropped.
pub const NO_REDISTRIBUTE: u16 = 1;
pub const NO_EXTERNAL_AI: u16 = 2;
pub const NO_TRAINING: u16 = 4;
pub const NO_BIOMETRIC_REUSE: u16 = 8;
pub const NO_PUBLIC_INDEX: u16 = 16;

pub const MAX_COMPARTMENTS: usize = 16;
pub const MAX_PURPOSES: usize = 16;
pub const MAX_LABEL_BYTES: usize = 4096;
pub const MAX_JOIN_INPUTS: usize = 64;

/// Untrusted decoded label. Construction does not authorise use.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelFields {
    pub confidentiality: Confidentiality,
    pub compartment_count: u8,
    pub compartments: [StrongDigest; MAX_COMPARTMENTS],
    pub purpose_count: u8,
    pub purposes: [StrongDigest; MAX_PURPOSES],
    pub restriction_bits: u16,
    /// Zero means unrestricted. Zero on C2+ is Conflict at join/egress.
    pub audience: StrongDigest,
    pub issuer: StrongDigest,
}

impl LabelFields {
    /// Blank untrusted fields. Confidentiality is Unknown, never C0 by default.
    pub const fn blank() -> Self {
        Self {
            confidentiality: Confidentiality::Unknown,
            compartment_count: 0,
            compartments: [StrongDigest::ZERO; MAX_COMPARTMENTS],
            purpose_count: 0,
            purposes: [StrongDigest::ZERO; MAX_PURPOSES],
            restriction_bits: 0,
            audience: StrongDigest::ZERO,
            issuer: StrongDigest::ZERO,
        }
    }

    pub fn request(confidentiality: Confidentiality, issuer: StrongDigest) -> Self {
        let mut fields = Self::blank();
        fields.confidentiality = confidentiality;
        fields.issuer = issuer;
        fields
    }

    #[inline]
    pub fn compartments(&self) -> &[StrongDigest] {
        &self.compartments[..self.compartment_count as usize]
    }

    #[inline]
    pub fn purposes(&self) -> &[StrongDigest] {
        &self.purposes[..self.purpose_count as usize]
    }
}

/// Verified handle bound to exact original bytes. Copy is permitted: fields are
/// digests, not secret key material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedLabel {
    fields: LabelFields,
    exact_bytes_digest: StrongDigest,
}

impl VerifiedLabel {
    pub(super) fn from_verified(fields: LabelFields, exact_bytes_digest: StrongDigest) -> Self {
        Self {
            fields,
            exact_bytes_digest,
        }
    }

    #[inline]
    pub const fn fields(&self) -> &LabelFields {
        &self.fields
    }

    #[inline]
    pub const fn exact_bytes_digest(&self) -> StrongDigest {
        self.exact_bytes_digest
    }

    #[inline]
    pub const fn issuer(&self) -> StrongDigest {
        self.fields.issuer
    }

    #[inline]
    pub const fn confidentiality(&self) -> Confidentiality {
        self.fields.confidentiality
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_is_not_public() {
        assert_ne!(Confidentiality::Unknown, Confidentiality::C0Public);
        assert_eq!(LabelFields::blank().confidentiality, Confidentiality::Unknown);
        assert_eq!(
            Confidentiality::Unknown.lattice_rank(),
            Err(QdnfError::Conflict)
        );
        assert_eq!(Confidentiality::C0Public.lattice_rank(), Ok(0));
    }

    #[test]
    fn unknown_wire_value_is_not_c0() {
        assert_eq!(
            Confidentiality::from_wire(255).unwrap(),
            Confidentiality::Unknown
        );
        assert_eq!(Confidentiality::from_wire(0).unwrap(), Confidentiality::C0Public);
        assert_eq!(Confidentiality::from_wire(99), Err(QdnfError::Malformed));
    }
}
