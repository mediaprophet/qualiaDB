//! Helper functions, lookups, and unit tests for the audio capability registry.

use super::*;

/// Look up a capability by id.
pub fn by_id(id: &str) -> Option<&'static AudioCapability> {
    CAPABILITIES.iter().find(|c| c.id == id)
}

/// Count rows with a given status.
pub fn count_by_status(status: CapabilityStatus) -> usize {
    CAPABILITIES.iter().filter(|c| c.status == status).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        for (i, a) in CAPABILITIES.iter().enumerate() {
            for b in &CAPABILITIES[i + 1..] {
                assert_ne!(a.id, b.id, "duplicate capability id: {}", a.id);
            }
        }
    }

    #[test]
    fn present_rows_are_real() {
        // A Present row must not describe itself as a placeholder/fake.
        for c in CAPABILITIES.iter().filter(|c| c.status == Present) {
            let n = c.note.to_ascii_lowercase();
            assert!(
                !n.contains("fake") && !n.contains("placeholder") && !n.contains("not "),
                "Present row {} has a non-real note: {}",
                c.id,
                c.note
            );
        }
    }

    #[test]
    fn lookup_works() {
        assert_eq!(by_id("vocoder").map(|c| c.status), Some(NeedsWeights));
        assert_eq!(by_id("energy_rms").map(|c| c.status), Some(Present));
        assert_eq!(by_id("mfcc").map(|c| c.status), Some(Present));
        assert!(by_id("nonexistent").is_none());
    }

    #[test]
    fn progress_and_remaining_are_both_honest() {
        // Waves 0–3 have landed real algorithms; later waves + gated learned heads remain.
        assert!(
            count_by_status(Present) >= 50,
            "expected substantial Present coverage"
        );
        assert!(
            count_by_status(NeedsWeights) >= 5,
            "principal-gated learned heads (AED/speech/CREPE/vocoder/MOS) must read as NeedsWeights"
        );
    }
}
