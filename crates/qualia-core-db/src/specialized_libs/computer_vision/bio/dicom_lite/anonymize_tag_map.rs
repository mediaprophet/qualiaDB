//! Redact known PHI tag keys in a simple DICOM attribute map.
//!
//! Operates on [`DicomTagMap`] only — not on raw file bytes. Safe for
//! pre-export scrub of tags produced by [`parse_dicom_tags_basic`].

use super::parse_dicom_tags_basic::{
    DicomTagMap, TagKey, TAG_PATIENT_BIRTH, TAG_PATIENT_ID, TAG_PATIENT_NAME,
};

/// Report of anonymization actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnonymizeReport {
    pub redacted: usize,
    pub retained: usize,
}

/// PHI / identity tags zeroed or replaced with a fixed placeholder.
///
/// Includes common patient-module keys; extend cautiously (prefer fail-closed
/// allowlists at export boundaries rather than ever-growing deny lists alone).
pub const PHI_TAG_KEYS: &[TagKey] = &[
    TAG_PATIENT_NAME,       // (0010,0010)
    TAG_PATIENT_ID,         // (0010,0020)
    TAG_PATIENT_BIRTH,      // (0010,0030)
    (0x0010, 0x0040),       // PatientSex
    (0x0010, 0x1000),       // OtherPatientIDs
    (0x0010, 0x1001),       // OtherPatientNames
    (0x0010, 0x1040),       // PatientAddress
    (0x0010, 0x2154),       // PatientTelephoneNumbers
    (0x0008, 0x0080),       // InstitutionName
    (0x0008, 0x0090),       // ReferringPhysicianName
    (0x0008, 0x1050),       // PerformingPhysicianName
    (0x0008, 0x1070),       // OperatorsName
    (0x0010, 0x1010),       // PatientAge
    (0x0032, 0x1032),       // RequestingPhysician
];

/// Placeholder written over redacted string values.
pub const REDACTED_PLACEHOLDER: &str = "REDACTED";

/// Zero/redact known PHI keys in `map` (in place).
///
/// Non-PHI keys are left unchanged. Returns counts of redacted vs retained.
pub fn anonymize_tag_map(map: &mut DicomTagMap) -> AnonymizeReport {
    let mut redacted = 0usize;
    let mut retained = 0usize;

    // Collect keys first to avoid borrow issues.
    let keys: Vec<TagKey> = map.keys().copied().collect();
    for key in keys {
        if is_phi_tag(key) {
            if let Some(v) = map.get_mut(&key) {
                *v = REDACTED_PLACEHOLDER.to_string();
                redacted += 1;
            }
        } else {
            retained += 1;
        }
    }

    AnonymizeReport { redacted, retained }
}

/// True if `key` is in the PHI table.
#[inline]
pub fn is_phi_tag(key: TagKey) -> bool {
    PHI_TAG_KEYS.iter().any(|&k| k == key)
}

/// Scrub helper: return PatientID redacted if present (convenience for call sites).
pub fn scrub_patient_id(map: &mut DicomTagMap) -> bool {
    if map.contains_key(&TAG_PATIENT_ID) {
        map.insert(TAG_PATIENT_ID, REDACTED_PLACEHOLDER.to_string());
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::parse_dicom_tags_basic::{TAG_COLUMNS, TAG_MODALITY, TAG_ROWS};

    #[test]
    fn redacts_patient_keeps_geometry() {
        let mut map = DicomTagMap::new();
        map.insert(TAG_PATIENT_ID, "SECRET-99".into());
        map.insert(TAG_PATIENT_NAME, "Doe^Jane".into());
        map.insert(TAG_ROWS, "512".into());
        map.insert(TAG_COLUMNS, "512".into());
        map.insert(TAG_MODALITY, "CT".into());

        let r = anonymize_tag_map(&mut map);
        assert_eq!(r.redacted, 2);
        assert_eq!(r.retained, 3);
        assert_eq!(map.get(&TAG_PATIENT_ID).map(String::as_str), Some(REDACTED_PLACEHOLDER));
        assert_eq!(map.get(&TAG_PATIENT_NAME).map(String::as_str), Some(REDACTED_PLACEHOLDER));
        assert_eq!(map.get(&TAG_ROWS).map(String::as_str), Some("512"));
        assert_eq!(map.get(&TAG_MODALITY).map(String::as_str), Some("CT"));
    }

    #[test]
    fn empty_map() {
        let mut map = DicomTagMap::new();
        let r = anonymize_tag_map(&mut map);
        assert_eq!(r, AnonymizeReport { redacted: 0, retained: 0 });
    }

    #[test]
    fn scrub_patient_id_only() {
        let mut map = DicomTagMap::new();
        map.insert(TAG_PATIENT_ID, "X".into());
        assert!(scrub_patient_id(&mut map));
        assert_eq!(map[&TAG_PATIENT_ID], REDACTED_PLACEHOLDER);
    }

    #[test]
    fn phi_table_includes_name_and_id() {
        assert!(is_phi_tag(TAG_PATIENT_NAME));
        assert!(is_phi_tag(TAG_PATIENT_ID));
        assert!(!is_phi_tag(TAG_ROWS));
    }
}
