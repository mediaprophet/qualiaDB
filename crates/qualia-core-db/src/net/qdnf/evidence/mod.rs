//! Selective electronic evidence (E19). Technical custody only.
//!
//! This library does not claim legal admissibility, prosecution fitness, or
//! courtroom authenticity. Hashes are not examinable content.

pub mod classes;
pub mod export;
pub mod holds;
pub mod promote;
pub mod seal;

pub use classes::{
    claims_legal_admissibility, classify, unrestricted_patient_export, ClassifiedEvidence,
    EvidenceClass,
};
pub use export::{
    export_examiner, renew_wrap, renewal_replaces_original, verify_offline, CompletenessReport,
    ExportReceipt,
};
pub use holds::{gc_expired, live_hold_count, place_hold, release_hold};
pub use promote::{
    hash_preserves_content, promote, AuthorityAtTime, CustodyBind, EvidenceRecord, EvidenceStore,
    MAX_EVIDENCE, MAX_HOLDS, MAX_ORIGINAL,
};
pub use seal::{custodian_plaintext, examinable, seal, EvidenceView};

#[cfg(test)]
mod tests {
    use super::claims_legal_admissibility;
    use super::classes::unrestricted_patient_export;
    use super::promote::hash_preserves_content;
    use super::seal::{examinable, EvidenceView};

    #[test]
    fn technical_support_is_not_admissibility() {
        assert!(!claims_legal_admissibility());
        assert!(!unrestricted_patient_export());
        assert!(!hash_preserves_content());
        assert!(!examinable(EvidenceView::HashOnly));
    }
}
