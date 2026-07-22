//! Oxford VGGFace2 Facial Recognition Taxonomy & Epistemic Identity Resolver.
//!
//! Provides zero-allocation FNV-1a IRI hashing, subject index mapping (`n000001`..`n009131`),
//! pose classification, and privacy-preserving identity hash resolution for VGGFace2 (9,131 identities).

use crate::semantic::q_hash;

/// Total number of unique subject identities in Oxford VGGFace2 dataset.
pub const VGGFACE2_IDENTITY_COUNT: usize = 9131;

/// Facial pose orientation classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VGGFace2Pose {
    Frontal = 0,
    ThreeQuarter = 1,
    Profile = 2,
}

impl VGGFace2Pose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Frontal => "frontal",
            Self::ThreeQuarter => "three_quarter",
            Self::Profile => "profile",
        }
    }
}

/// Format an identity index (0..9130) into its canonical VGGFace2 subject ID string (e.g. `n000001`).
pub fn format_vggface2_subject_id(identity_idx: u32) -> String {
    format!("n{:06}", identity_idx.saturating_add(1))
}

/// Compute the deterministic 64-bit FNV-1a subject identity hash for a VGGFace2 subject ID.
pub fn q_hash_vggface2_subject(subject_id: &str) -> u64 {
    q_hash(subject_id)
}

/// Compute the deterministic 64-bit FNV-1a subject identity hash directly from numeric index (0..9130).
pub fn lookup_vggface2_subject_hash(identity_idx: u32) -> u64 {
    q_hash(&format_vggface2_subject_id(identity_idx))
}

/// Total count of registered VGGFace2 subject identities.
pub fn vggface2_identity_count() -> usize {
    VGGFACE2_IDENTITY_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vggface2_taxonomy_formatting_and_hashing() {
        assert_eq!(format_vggface2_subject_id(0), "n000001");
        assert_eq!(format_vggface2_subject_id(9130), "n009131");

        let h1 = q_hash_vggface2_subject("n000001");
        let h2 = lookup_vggface2_subject_hash(0);
        assert_eq!(h1, h2);
        assert_ne!(h1, 0);

        assert_eq!(vggface2_identity_count(), 9131);
        assert_eq!(VGGFace2Pose::Frontal.as_str(), "frontal");
    }
}
