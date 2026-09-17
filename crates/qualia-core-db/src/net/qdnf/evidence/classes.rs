//! Evidence classes. Every class carries a verified label digest (E19.1).
//!
//! Patient records are never dumped into evidence. Technical classification is
//! not a claim of legal admissibility.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::{require_labelled, Confidentiality, VerifiedLabel};
use crate::net::qdnf::types::StrongDigest;

/// Capture class. Sensitivity and purpose live on the bound label digest.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    EphemeralDiagnostic = 1,
    BoundedOperational = 2,
    SelectedIncident = 3,
    Obligation = 4,
    Preserved = 5,
}

impl EvidenceClass {
    pub fn from_wire(value: u8) -> Result<Self, QdnfError> {
        match value {
            1 => Ok(Self::EphemeralDiagnostic),
            2 => Ok(Self::BoundedOperational),
            3 => Ok(Self::SelectedIncident),
            4 => Ok(Self::Obligation),
            5 => Ok(Self::Preserved),
            _ => Err(QdnfError::Malformed),
        }
    }

    /// Contrary/contextual material is required for these selections.
    #[inline]
    pub const fn requires_context(self) -> bool {
        matches!(
            self,
            Self::SelectedIncident | Self::Obligation | Self::Preserved
        )
    }
}

/// Classified selection bound to exact label bytes via SHA-384 digest.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassifiedEvidence {
    pub class: EvidenceClass,
    pub label_digest: StrongDigest,
    pub purpose: StrongDigest,
    pub confidentiality: Confidentiality,
}

/// Bind class to a verified label. Missing/unknown labels fail closed.
pub fn classify(
    class: EvidenceClass,
    label: &VerifiedLabel,
) -> Result<ClassifiedEvidence, QdnfError> {
    require_labelled(label.fields())?;
    if label.exact_bytes_digest().is_zero() {
        return Err(QdnfError::Malformed);
    }
    let purpose = if label.fields().purpose_count == 0 {
        StrongDigest::ZERO
    } else {
        label.fields().purposes[0]
    };
    Ok(ClassifiedEvidence {
        class,
        label_digest: label.exact_bytes_digest(),
        purpose,
        confidentiality: label.confidentiality(),
    })
}

/// Patient records are not an unrestricted evidence dump.
pub fn unrestricted_patient_export() -> bool {
    false
}

/// Technical evidence support is not a guarantee of admissibility or prosecution.
pub fn claims_legal_admissibility() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::policy_labels::{encode_label_into, verify_label, LabelFields};

    fn labelled(conf: Confidentiality, purpose: &[u8]) -> VerifiedLabel {
        let issuer = sha384(b"e19-issuer");
        let mut fields = LabelFields::request(conf, issuer);
        if conf.requires_audience() {
            fields.audience = issuer;
        }
        if !purpose.is_empty() {
            fields.purpose_count = 1;
            fields.purposes[0] = sha384(purpose);
        }
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        verify_label(fields, &buf[..n]).unwrap()
    }

    #[test]
    fn every_class_carries_label_digest_and_purpose() {
        let classes = [
            EvidenceClass::EphemeralDiagnostic,
            EvidenceClass::BoundedOperational,
            EvidenceClass::SelectedIncident,
            EvidenceClass::Obligation,
            EvidenceClass::Preserved,
        ];
        let mut i = 0usize;
        while i < classes.len() {
            let label = labelled(Confidentiality::C1Private, b"incident-purpose");
            let c = classify(classes[i], &label).unwrap();
            assert_eq!(c.class, classes[i]);
            assert_eq!(c.label_digest, label.exact_bytes_digest());
            assert!(!c.label_digest.is_zero());
            assert_eq!(c.purpose, sha384(b"incident-purpose"));
            i += 1;
        }
    }

    #[test]
    fn patient_records_are_not_dumped() {
        assert!(!unrestricted_patient_export());
        assert!(!claims_legal_admissibility());
        let clinical = labelled(Confidentiality::C3Compartmented, b"clinical");
        let c = classify(EvidenceClass::SelectedIncident, &clinical).unwrap();
        assert_eq!(c.confidentiality, Confidentiality::C3Compartmented);
        assert!(!unrestricted_patient_export());
    }

    #[test]
    fn unknown_wire_class_is_malformed() {
        assert_eq!(EvidenceClass::from_wire(0), Err(QdnfError::Malformed));
        assert_eq!(
            EvidenceClass::from_wire(1).unwrap(),
            EvidenceClass::EphemeralDiagnostic
        );
    }

    #[test]
    fn selected_classes_require_context() {
        assert!(!EvidenceClass::EphemeralDiagnostic.requires_context());
        assert!(!EvidenceClass::BoundedOperational.requires_context());
        assert!(EvidenceClass::SelectedIncident.requires_context());
        assert!(EvidenceClass::Obligation.requires_context());
        assert!(EvidenceClass::Preserved.requires_context());
    }
}
