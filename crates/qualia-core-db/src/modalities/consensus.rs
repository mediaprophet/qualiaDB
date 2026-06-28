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

// ─── Byzantine Fault Tolerant quorum (PBFT / HotStuff safety arithmetic) ──────────

/// The maximum Byzantine faults `f` a network of `n` nodes tolerates: `f = ⌊(n-1)/3⌋`
/// (BFT safety needs `n ≥ 3f+1`).
#[inline]
pub fn bft_max_faults(n: usize) -> usize {
    if n == 0 {
        0
    } else {
        (n - 1) / 3
    }
}

/// The BFT quorum size — the supermajority `2f+1` that makes a commit safe despite `f` Byzantine
/// nodes (PBFT prepare/commit certificate, HotStuff QC).
#[inline]
pub fn bft_quorum(n: usize) -> usize {
    2 * bft_max_faults(n) + 1
}

/// Has a safe BFT quorum been reached (`votes ≥ bft_quorum(n)`)?
#[inline]
pub fn bft_committed(n: usize, votes: usize) -> bool {
    n > 0 && votes >= bft_quorum(n)
}

// ─── Lamport & vector clocks (causal order / partition healing) ───────────────────

/// Lamport clock tick on a local event.
#[inline]
pub fn lamport_tick(clock: u64) -> u64 {
    clock.saturating_add(1)
}

/// Lamport clock on receiving a message stamped `msg`: `max(local, msg) + 1`.
#[inline]
pub fn lamport_recv(local: u64, msg: u64) -> u64 {
    local.max(msg).saturating_add(1)
}

/// Vector-clock **happens-before** `a → b`: `a[i] ≤ b[i]` for all i AND `a ≠ b`.
pub fn vc_happens_before(a: &[u64], b: &[u64]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut strictly_less = false;
    for i in 0..a.len() {
        if a[i] > b[i] {
            return false;
        }
        if a[i] < b[i] {
            strictly_less = true;
        }
    }
    strictly_less
}

/// Two vector clocks are **concurrent** iff neither happens-before the other.
pub fn vc_concurrent(a: &[u64], b: &[u64]) -> bool {
    a.len() == b.len() && a != b && !vc_happens_before(a, b) && !vc_happens_before(b, a)
}

/// Merge vector clocks (componentwise max) into `out` — partition-healing reconciliation.
/// Zero-heap (caller-supplied `out`).
pub fn vc_merge(a: &[u64], b: &[u64], out: &mut [u64]) -> bool {
    if a.len() != b.len() || out.len() < a.len() {
        return false;
    }
    for i in 0..a.len() {
        out[i] = a[i].max(b[i]);
    }
    true
}

// ─── Dynamic validator set rotation ───────────────────────────────────────────────

/// **Dynamic validator rotation**: is validator `validator_idx` in the active committee for
/// `epoch`? A window of `active_size` validators rotates by one position per epoch over a set of
/// `set_size` (round-robin), so committee membership churns deterministically. Indices and the
/// window wrap modulo `set_size`.
pub fn is_active_validator(
    validator_idx: usize,
    epoch: u64,
    set_size: usize,
    active_size: usize,
) -> bool {
    if set_size == 0 || active_size == 0 || validator_idx >= set_size {
        return false;
    }
    let active = active_size.min(set_size);
    let start = (epoch as usize) % set_size;
    // Is validator_idx within [start, start+active) modulo set_size?
    let offset = (validator_idx + set_size - start) % set_size;
    offset < active
}

// ─── Validator equivocation → slashing ────────────────────────────────────────────

/// **Equivocation** (a slashable Byzantine fault): a validator signed TWO DIFFERENT values at the
/// same height/round. `(height_a, value_a)` vs `(height_b, value_b)` from the SAME validator.
#[inline]
pub fn is_equivocation(height_a: u64, value_a: u64, height_b: u64, value_b: u64) -> bool {
    height_a == height_b && value_a != value_b
}

// ─── ZK light-client verification ─────────────────────────────────────────────────

/// A light client accepts consensus state only if a succinct zk proof of the quorum verifies (it
/// does NOT re-execute the chain): accept iff `proof_verified` AND a BFT quorum was claimed.
#[inline]
pub fn light_client_accepts(proof_verified: bool, n: usize, claimed_votes: usize) -> bool {
    proof_verified && bft_committed(n, claimed_votes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bft_quorum_tolerates_a_third_faulty() {
        // n=4 → f=1, quorum=3 (classic PBFT minimum).
        assert_eq!(bft_max_faults(4), 1);
        assert_eq!(bft_quorum(4), 3);
        assert!(bft_committed(4, 3));
        assert!(!bft_committed(4, 2), "2 votes is below the quorum");
        // n=7 → f=2, quorum=5.
        assert_eq!(bft_max_faults(7), 2);
        assert_eq!(bft_quorum(7), 5);
    }

    #[test]
    fn lamport_and_vector_clocks() {
        assert_eq!(lamport_tick(4), 5);
        assert_eq!(lamport_recv(4, 9), 10); // max(4,9)+1
                                            // a → b (a precedes b causally).
        assert!(vc_happens_before(&[1, 0, 0], &[1, 1, 0]));
        assert!(!vc_happens_before(&[1, 1, 0], &[1, 0, 0]));
        // Concurrent: neither precedes the other.
        assert!(vc_concurrent(&[1, 0], &[0, 1]));
        assert!(!vc_concurrent(&[1, 0], &[1, 1]));
        // Merge = componentwise max (partition healing).
        let mut out = [0u64; 3];
        assert!(vc_merge(&[1, 3, 0], &[2, 1, 5], &mut out));
        assert_eq!(out, [2, 3, 5]);
    }

    #[test]
    fn equivocation_and_zk_light_client() {
        // Two different values at the same height → slashable equivocation.
        assert!(is_equivocation(10, 0xAA, 10, 0xBB));
        assert!(
            !is_equivocation(10, 0xAA, 11, 0xBB),
            "different heights → not equivocation"
        );
        assert!(
            !is_equivocation(10, 0xAA, 10, 0xAA),
            "same value → just a re-vote"
        );
        // Light client accepts only a quorum-backed, proof-verified state.
        assert!(light_client_accepts(true, 4, 3));
        assert!(!light_client_accepts(false, 4, 3), "no proof → reject");
        assert!(!light_client_accepts(true, 4, 2), "below quorum → reject");
    }

    #[test]
    fn validator_set_rotates_per_epoch() {
        // 5 validators, active committee of 3, rotating one position per epoch.
        assert!(is_active_validator(0, 0, 5, 3)); // epoch 0: {0,1,2}
        assert!(is_active_validator(2, 0, 5, 3));
        assert!(!is_active_validator(3, 0, 5, 3));
        // Epoch 1 shifts the window to {1,2,3}.
        assert!(is_active_validator(3, 1, 5, 3));
        assert!(!is_active_validator(0, 1, 5, 3));
        // Out-of-range / degenerate inputs.
        assert!(!is_active_validator(9, 0, 5, 3));
        assert!(!is_active_validator(0, 0, 0, 3));
    }

    #[test]
    fn multi_party_commits_only_on_full_consensus() {
        assert_eq!(transaction_status(3, 3), TxStatus::Committed);
        assert_eq!(transaction_status(2, 3), TxStatus::Suspended);
        assert_eq!(transaction_status(0, 0), TxStatus::Suspended); // no parties → cannot commit
    }

    #[test]
    fn local_validity_is_not_global() {
        assert!(
            !is_globally_valid(true, false),
            "valid locally but unsynced → not global"
        );
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
