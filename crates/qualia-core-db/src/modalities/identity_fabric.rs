//! Resilient relational identity (§27, legal_logic.md) — fabric resolution.
//!
//! The strict axiom: an **identifier is not an identity**. A key, DID, or name is a pointer;
//! identity is the dynamically-computed result of a *fabric* of anchors (identifiers,
//! attestations, relations). So losing a primary key must NOT collapse identity — it is
//! re-computed from the surviving fabric (the refugee / device-loss / theft resilience case),
//! provided a quorum of anchors survives (k-of-n social/relational recovery).
//!
//! See [[principle-identifiers-not-identity]]. This complements the foundational
//! `modal_kind`/`resolve` layer with the *resilience* primitive. Zero-heap.

/// Identity survives the loss of anchors iff a **quorum** of the fabric remains. `quorum` must
/// be ≥1 (an identity anchored by nothing is not an identity). k-of-n recovery.
#[inline]
pub fn identity_survives_loss(total_anchors: usize, lost_anchors: usize, quorum: usize) -> bool {
    quorum > 0 && total_anchors.saturating_sub(lost_anchors) >= quorum
}

/// The surviving anchor count after a loss (saturating).
#[inline]
pub fn surviving_anchors(total_anchors: usize, lost_anchors: usize) -> usize {
    total_anchors.saturating_sub(lost_anchors)
}

/// Re-compute the active anchor set from `all_anchors`, excluding any in `lost`, into `out`.
/// Returns the count — the surviving fabric an identity is reconstructed from. Zero-heap.
pub fn recompute_fabric(all_anchors: &[u64], lost: &[u64], out: &mut [u64]) -> usize {
    let mut n = 0usize;
    for &a in all_anchors {
        if !lost.contains(&a) {
            if n >= out.len() {
                break;
            }
            out[n] = a;
            n += 1;
        }
    }
    n
}

/// The axiom, made explicit: an identifier is never, by itself, the identity.
#[inline]
pub const fn identifier_is_not_identity() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn identity_survives_key_loss_with_quorum() {
        // 5 anchors, lose the primary key (1), quorum of 3 → survives.
        assert!(identity_survives_loss(5, 1, 3));
        // Lose 3 of 5, quorum 3 → exactly meets → survives.
        assert!(identity_survives_loss(5, 2, 3));
        // Lose too many → identity cannot be reconstructed.
        assert!(!identity_survives_loss(5, 3, 3));
        // Quorum 0 is invalid (nothing anchors nothing).
        assert!(!identity_survives_loss(5, 0, 0));
    }

    #[test]
    fn fabric_recomputes_from_survivors() {
        let key = q_hash("anchor:primaryKey");
        let social = q_hash("anchor:socialAttestation");
        let bio = q_hash("anchor:biometric");
        let device = q_hash("anchor:device");
        let all = [key, social, bio, device];
        let lost = [key, device]; // stolen phone + its key
        let mut out = [0u64; 8];
        let n = recompute_fabric(&all, &lost, &mut out);
        assert_eq!(n, 2, "identity re-computes from the surviving relational fabric");
        assert!(out[..n].contains(&social) && out[..n].contains(&bio));
        assert!(!out[..n].contains(&key));
        assert!(identifier_is_not_identity());
    }
}
