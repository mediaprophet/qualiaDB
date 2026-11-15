//! Sensitivity egress filter for Solid leave/migrate.
//!
//! Fiduciary rule: classified context never leaves the local node. Restricted
//! and sanctuary-tier quins leave only when the principal explicitly opts in.
//!
//! Local sanctuary predicate (mirrors `q42_volume::quin_requires_sanctuary`) so
//! this module compiles on `wasm32` / `wasm-webcivics` where `q42_volume` is
//! native-only.

use crate::{NQuin, PermissiveRoutingLane};

/// What may cross into a Solid-portable RDF bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolidEgressPolicy {
    /// Include `SENSITIVITY_RESTRICTED` / sanctuary-tier quins (owner leave path).
    /// Classified is **never** included regardless of this flag.
    pub include_restricted: bool,
}

impl Default for SolidEgressPolicy {
    fn default() -> Self {
        Self {
            include_restricted: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EgressStats {
    pub scanned: usize,
    pub exported: usize,
    pub redacted_classified: usize,
    pub redacted_restricted: usize,
    pub skipped_parity: usize,
}

fn requires_sanctuary(quin: &NQuin) -> bool {
    match quin.get_sensitivity_byte() {
        NQuin::SENSITIVITY_RESTRICTED | NQuin::SENSITIVITY_CLASSIFIED => return true,
        _ => {}
    }
    match quin.get_sensitivity_tier() {
        NQuin::SENSITIVITY_TIER_LEGAL
        | NQuin::SENSITIVITY_TIER_MEDICAL
        | NQuin::SENSITIVITY_TIER_FIDUCIARY => return true,
        _ => {}
    }
    quin.identify_routing_lane() == PermissiveRoutingLane::EnforceBilateralMicroCommons
}

/// Decide whether a quin may enter the Solid RDF export set.
pub fn may_egress(quin: &NQuin, policy: SolidEgressPolicy) -> bool {
    if quin.get_sensitivity_byte() == NQuin::SENSITIVITY_CLASSIFIED {
        return false;
    }
    if requires_sanctuary(quin) && !policy.include_restricted {
        return false;
    }
    true
}

/// Partition `quins` into exportable vs redacted counts (caller buffers output).
pub fn filter_for_egress(
    quins: &[NQuin],
    policy: SolidEgressPolicy,
    out: &mut Vec<NQuin>,
) -> EgressStats {
    let mut stats = EgressStats::default();
    out.clear();
    for quin in quins {
        stats.scanned += 1;
        let mut q = *quin;
        // Wire / JS often sends parity=0; recompute so leave path matches ingest.
        if q.parity == 0 {
            q.recalculate_parity();
        }
        if !q.verify_ecc_parity() {
            stats.skipped_parity += 1;
            continue;
        }
        if q.get_sensitivity_byte() == NQuin::SENSITIVITY_CLASSIFIED {
            stats.redacted_classified += 1;
            continue;
        }
        if requires_sanctuary(&q) && !policy.include_restricted {
            stats.redacted_restricted += 1;
            continue;
        }
        out.push(q);
        stats.exported += 1;
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    fn base_quin() -> NQuin {
        let mut q = NQuin {
            subject: q_hash("Alice"),
            predicate: q_hash("knows"),
            object: q_hash("Bob"),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.recalculate_parity();
        q
    }

    #[test]
    fn public_quin_egresses_by_default() {
        let q = base_quin();
        assert!(may_egress(&q, SolidEgressPolicy::default()));
    }

    #[test]
    fn classified_never_egresses() {
        let mut q = base_quin();
        q.set_sensitivity_byte(NQuin::SENSITIVITY_CLASSIFIED);
        q.recalculate_parity();
        assert!(!may_egress(
            &q,
            SolidEgressPolicy {
                include_restricted: true,
            }
        ));
    }

    #[test]
    fn restricted_needs_opt_in() {
        let mut q = base_quin();
        q.set_sensitivity_byte(NQuin::SENSITIVITY_RESTRICTED);
        q.recalculate_parity();
        assert!(!may_egress(&q, SolidEgressPolicy::default()));
        assert!(may_egress(
            &q,
            SolidEgressPolicy {
                include_restricted: true
            }
        ));
    }
}
