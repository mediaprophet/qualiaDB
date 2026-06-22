//! Distributed state & consensus (§28, legal_logic.md) — multi-agent sync.
//!
//! Human-centric obligations span many sovereign vaults; no single node holds the complete
//! global state. A multi-party obligation must therefore be **suspended until consensus**, a
//! norm valid *locally* does not bind the network until *synchronised*, and a network
//! **partition** must not silently break standing obligations (those established before the
//! split survive; new joint obligations pause until healing). Complements the engine's CRDT
//! (LWW) layer with the deontic transaction semantics. Zero-heap, total predicates.

/// The state of a multi-party obligation/transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStatus {
    /// Awaiting cryptographic consensus from all parties.
    Suspended,
    /// All parties have assented — committed.
    Committed,
}

/// A multi-party obligation is `Committed` only when **every** party has assented; otherwise it
/// is `Suspended` (Pending). An obligation with no parties cannot commit.
pub fn transaction_status(parties_assented: usize, total_parties: usize) -> TxStatus {
    if total_parties > 0 && parties_assented >= total_parties {
        TxStatus::Committed
    } else {
        TxStatus::Suspended
    }
}

/// **Local validity ≠ global validity**: a norm an agent holds in its local cell does not bind
/// the network until it has been synchronised. Both must hold.
#[inline]
pub fn is_globally_valid(local_valid: bool, synced: bool) -> bool {
    local_valid && synced
}

/// **Partition tolerance**: an obligation established *before* a partition remains active across
/// the split (standing duties are not silently broken by a network event).
#[inline]
pub fn survives_partition(established_before_partition: bool) -> bool {
    established_before_partition
}

/// During a partition, a NEW joint (multi-party) obligation may not be formed — it must wait
/// for the network to heal so all parties can reach consensus.
#[inline]
pub fn can_form_joint_during_partition(partitioned: bool) -> bool {
    !partitioned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_party_commits_only_on_full_consensus() {
        assert_eq!(transaction_status(3, 3), TxStatus::Committed);
        assert_eq!(transaction_status(2, 3), TxStatus::Suspended);
        assert_eq!(transaction_status(0, 0), TxStatus::Suspended); // no parties → cannot commit
    }

    #[test]
    fn local_validity_is_not_global() {
        assert!(!is_globally_valid(true, false), "valid locally but unsynced → not global");
        assert!(is_globally_valid(true, true));
        assert!(!is_globally_valid(false, true));
    }

    #[test]
    fn partition_preserves_standing_duties_but_pauses_new_ones() {
        // Pre-existing obligation survives the split.
        assert!(survives_partition(true));
        assert!(!survives_partition(false));
        // New joint obligations pause during a partition, resume on healing.
        assert!(!can_form_joint_during_partition(true));
        assert!(can_form_joint_during_partition(false));
    }
}
