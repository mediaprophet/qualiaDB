//! E00.5 evidence-class buckets. Mixing these into one CI “green” is forbidden.

use super::qualification::EvidenceClass;

/// One CI bucket. `qualified` is never inferred from component tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CiBucket {
    pub class: EvidenceClass,
    pub name: &'static str,
    pub qualified: bool,
}

const BUCKETS: [CiBucket; 4] = [
    CiBucket {
        class: EvidenceClass::Component,
        name: "component",
        qualified: false,
    },
    CiBucket {
        class: EvidenceClass::Process,
        name: "process",
        qualified: false,
    },
    CiBucket {
        class: EvidenceClass::PhysicalNetwork,
        name: "physical-network",
        qualified: false,
    },
    CiBucket {
        class: EvidenceClass::Application,
        name: "application",
        qualified: false,
    },
];

/// Separate component / process / physical-network / application results.
pub fn ci_buckets() -> &'static [CiBucket; 4] {
    &BUCKETS
}

/// Always zero. Buckets exist so CI cannot collapse them.
pub fn qualified_bucket_count() -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_buckets_stay_unqualified() {
        assert_eq!(ci_buckets().len(), 4);
        assert_eq!(ci_buckets()[0].class, EvidenceClass::Component);
        assert_eq!(ci_buckets()[1].class, EvidenceClass::Process);
        assert_eq!(ci_buckets()[2].class, EvidenceClass::PhysicalNetwork);
        assert_eq!(ci_buckets()[3].class, EvidenceClass::Application);
        let mut i = 0usize;
        while i < BUCKETS.len() {
            assert!(!BUCKETS[i].qualified);
            i += 1;
        }
        assert_eq!(qualified_bucket_count(), 0);
    }
}
