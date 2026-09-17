//! Raw biometric capture (E15.1). Ephemeral, modality-specific, local.
//!
//! Raw samples stay with the caller. This record holds only acquisition
//! metadata. It is not a network identifier and is not a template.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::BiometricObjectKind;

/// Distinct acquisition kinds. Do not collapse these into one generic hash.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modality {
    Face = 1,
    Voice = 2,
    Iris = 3,
    Fingerprint = 4,
    Gait = 5,
    Medical = 6,
}

impl Modality {
    pub fn from_wire(value: u8) -> Result<Self, QdnfError> {
        match value {
            1 => Ok(Self::Face),
            2 => Ok(Self::Voice),
            3 => Ok(Self::Iris),
            4 => Ok(Self::Fingerprint),
            5 => Ok(Self::Gait),
            6 => Ok(Self::Medical),
            _ => Err(QdnfError::Malformed),
        }
    }

    #[inline]
    pub const fn to_wire(self) -> u8 {
        self as u8
    }
}

/// Acquisition metadata. Sample bytes are never stored here.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawCapture {
    modality: Modality,
    device: StrongDigest,
    consent: StrongDigest,
    quality: u16,
    timestamp_uncertainty: u16,
}

impl RawCapture {
    pub fn acquire(
        modality: Modality,
        device: StrongDigest,
        consent: StrongDigest,
        quality: u16,
        timestamp_uncertainty: u16,
    ) -> Result<Self, QdnfError> {
        if device.is_zero() || consent.is_zero() {
            return Err(QdnfError::Malformed);
        }
        if quality == 0 {
            return Err(QdnfError::Incomplete);
        }
        Ok(Self {
            modality,
            device,
            consent,
            quality,
            timestamp_uncertainty,
        })
    }

    #[inline]
    pub const fn kind(self) -> BiometricObjectKind {
        BiometricObjectKind::RawCapture
    }

    #[inline]
    pub const fn modality(self) -> Modality {
        self.modality
    }

    #[inline]
    pub const fn device(self) -> StrongDigest {
        self.device
    }

    #[inline]
    pub const fn consent(self) -> StrongDigest {
        self.consent
    }

    #[inline]
    pub const fn quality(self) -> u16 {
        self.quality
    }

    #[inline]
    pub const fn timestamp_uncertainty(self) -> u16 {
        self.timestamp_uncertainty
    }
}

/// A raw capture is not a QDNF routing or session identifier.
pub fn retain_raw_as_network_id(_capture: &RawCapture) -> Result<(), QdnfError> {
    Err(QdnfError::Denied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x.0[47] = 0xC1;
        x
    }

    #[test]
    fn acquire_requires_device_consent_and_quality() {
        assert_eq!(
            RawCapture::acquire(Modality::Face, StrongDigest::ZERO, d(2), 10, 1).unwrap_err(),
            QdnfError::Malformed
        );
        assert_eq!(
            RawCapture::acquire(Modality::Face, d(1), StrongDigest::ZERO, 10, 1).unwrap_err(),
            QdnfError::Malformed
        );
        assert_eq!(
            RawCapture::acquire(Modality::Iris, d(1), d(2), 0, 1).unwrap_err(),
            QdnfError::Incomplete
        );
        let c = RawCapture::acquire(Modality::Voice, d(1), d(2), 12, 3).unwrap();
        assert_eq!(c.kind(), BiometricObjectKind::RawCapture);
        assert_eq!(c.modality(), Modality::Voice);
        assert_eq!(c.quality(), 12);
    }

    #[test]
    fn modalities_are_not_one_generic_hash() {
        assert_ne!(Modality::Face.to_wire(), Modality::Fingerprint.to_wire());
        assert_ne!(Modality::Voice.to_wire(), Modality::Medical.to_wire());
        assert_eq!(Modality::from_wire(0).unwrap_err(), QdnfError::Malformed);
        assert_eq!(Modality::from_wire(4).unwrap(), Modality::Fingerprint);
    }

    #[test]
    fn raw_capture_is_not_a_network_id() {
        let c = RawCapture::acquire(Modality::Gait, d(3), d(4), 8, 0).unwrap();
        assert_eq!(retain_raw_as_network_id(&c).unwrap_err(), QdnfError::Denied);
    }
}
