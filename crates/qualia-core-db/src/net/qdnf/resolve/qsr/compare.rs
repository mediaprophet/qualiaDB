//! Closed-world QSR vs local XOR-distance comparison (E07.8).
//!
//! Same keys, same work budget, same query. The XOR scan is an in-process
//! nearest-neighbour stub: it is **not** networked Kademlia (no routing table,
//! bootstrap, maintenance, transport, crypto suite, or replication budget).
//! Do not claim QSR is better than Kademlia from this module.

use super::cover::CoverInterval;
use super::key::keys_equal;
use super::outcome::QsrOutcome;
use super::traversal::{lookup_into, QsrSnapshot};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Closed-world slots for the local XOR scan. Not a Kademlia k-bucket.
pub const LOCAL_XOR_CAP: usize = 8;

/// A networked / equivalent-workload Kademlia baseline has not been executed.
pub fn kademlia_comparison_executed() -> bool {
    false
}

/// Unmeasured superiority claims are forbidden. This always returns false.
pub fn unmeasured_better_than_kademlia_claimed() -> bool {
    false
}

/// Recorded closed-world trial. Presence of a record is not a Kademlia result.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosedWorldComparison {
    pub key_count: u8,
    pub budget: u8,
    pub qsr_outcome: QsrOutcome,
    pub qsr_work: u8,
    pub xor_used: u8,
    pub xor_matched_query: bool,
    pub recorded: bool,
}

/// XOR 48-byte keys into `out`. In-process helper, not a DHT hop.
pub fn xor_distance(a: &StrongDigest, b: &StrongDigest, out: &mut [u8; 48]) {
    let mut i = 0usize;
    while i < 48 {
        out[i] = a.0[i] ^ b.0[i];
        i += 1;
    }
}

/// Nearest key in a caller-owned table under a remaining work budget.
///
/// Work is one unit per occupied slot. Exhaustion is BudgetExhausted, not a
/// Kademlia timeout. This is not a networked lookup.
pub fn local_xor_nearest(
    table: &[StrongDigest],
    target: &StrongDigest,
    work_budget: u8,
    out: &mut StrongDigest,
) -> Result<u8, QdnfError> {
    if table.len() > LOCAL_XOR_CAP {
        return Err(QdnfError::Capacity);
    }
    if table.is_empty() {
        return Err(QdnfError::Incomplete);
    }
    let mut best_i = 0usize;
    let mut best = [0xffu8; 48];
    let mut used: u8 = 0;
    let mut i = 0usize;
    while i < table.len() {
        if used == work_budget {
            return Err(QdnfError::BudgetExhausted);
        }
        used = used + 1;
        let mut dist = [0u8; 48];
        xor_distance(target, &table[i], &mut dist);
        if distance_less(&dist, &best) {
            best = dist;
            best_i = i;
        }
        i += 1;
    }
    *out = table[best_i];
    Ok(used)
}

fn distance_less(a: &[u8; 48], b: &[u8; 48]) -> bool {
    let mut i = 0usize;
    while i < 48 {
        if a[i] < b[i] {
            return true;
        }
        if a[i] > b[i] {
            return false;
        }
        i += 1;
    }
    false
}

fn charge_slots(occupied: usize, work_budget: u8) -> Result<u8, QdnfError> {
    if occupied > LOCAL_XOR_CAP {
        return Err(QdnfError::Capacity);
    }
    let mut used: u8 = 0;
    let mut i = 0usize;
    while i < occupied {
        if used == work_budget {
            return Err(QdnfError::BudgetExhausted);
        }
        used = used + 1;
        i += 1;
    }
    Ok(used)
}

/// Run QSR snapshot membership and the local XOR stub on one shared workload.
///
/// `keys` and `values` must be the same length. Both sides spend one work unit
/// per occupied slot from `work_budget`. Results are recorded; this function
/// never sets [`kademlia_comparison_executed`] or
/// [`unmeasured_better_than_kademlia_claimed`].
pub fn run_closed_world_comparison(
    keys: &[StrongDigest],
    values: &[StrongDigest],
    query: &StrongDigest,
    work_budget: u8,
    qsr_out: &mut StrongDigest,
    xor_out: &mut StrongDigest,
) -> Result<ClosedWorldComparison, QdnfError> {
    if keys.len() != values.len() {
        return Err(QdnfError::Malformed);
    }
    if keys.len() > LOCAL_XOR_CAP {
        return Err(QdnfError::Capacity);
    }

    let mut snap = QsrSnapshot::empty(Generation(1));
    let mut i = 0usize;
    while i < keys.len() {
        snap.insert(keys[i], values[i])?;
        i += 1;
    }

    let covers = [CoverInterval { start: 0, end: 15 }];
    let qsr_work = charge_slots(snap.len(), work_budget)?;
    let mut slot = [StrongDigest::ZERO; 1];
    let qsr_outcome = lookup_into(&snap, query, &covers, Generation(1), &mut slot)?;
    *qsr_out = slot[0];

    let xor_used = local_xor_nearest(keys, query, work_budget, xor_out)?;
    let xor_matched_query = keys_equal(xor_out, query);

    Ok(ClosedWorldComparison {
        key_count: keys.len() as u8,
        budget: work_budget,
        qsr_outcome,
        qsr_work,
        xor_used,
        xor_matched_query,
        recorded: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_rest(rest: u8) -> StrongDigest {
        let mut k = StrongDigest::ZERO;
        k.0[0] = 0x10;
        let mut i = 1usize;
        while i < 48 {
            k.0[i] = rest;
            i += 1;
        }
        k
    }

    #[test]
    fn comparison_was_not_executed() {
        assert!(!kademlia_comparison_executed());
    }

    #[test]
    fn unmeasured_better_than_kademlia_is_not_claimed() {
        assert!(!unmeasured_better_than_kademlia_claimed());
    }

    #[test]
    fn local_xor_finds_exact_key_without_being_kademlia() {
        let mut a = StrongDigest::ZERO;
        let mut b = StrongDigest::ZERO;
        a.0[47] = 1;
        b.0[47] = 9;
        let table = [a, b];
        let mut out = StrongDigest::ZERO;
        let used = local_xor_nearest(&table, &a, 8, &mut out).unwrap();
        assert_eq!(used, 2);
        assert_eq!(out, a);
        assert!(!kademlia_comparison_executed());
        assert!(!unmeasured_better_than_kademlia_claimed());
    }

    #[test]
    fn local_xor_budget_exhaustion_is_not_a_dht_timeout() {
        let table = [StrongDigest::ZERO, StrongDigest::ZERO];
        let mut out = StrongDigest::ZERO;
        assert_eq!(
            local_xor_nearest(&table, &StrongDigest::ZERO, 1, &mut out),
            Err(QdnfError::BudgetExhausted)
        );
    }

    #[test]
    fn closed_world_comparison_runs_results_recorded_no_kademlia_claim() {
        let a = key_rest(0x01);
        let b = key_rest(0x02);
        assert_eq!(a.0[0], b.0[0]);
        let mut va = StrongDigest::ZERO;
        let mut vb = StrongDigest::ZERO;
        va.0[47] = 0x11;
        vb.0[47] = 0x22;
        let keys = [a, b];
        let values = [va, vb];
        let mut qsr_out = StrongDigest::ZERO;
        let mut xor_out = StrongDigest::ZERO;
        let record = run_closed_world_comparison(
            &keys,
            &values,
            &a,
            LOCAL_XOR_CAP as u8,
            &mut qsr_out,
            &mut xor_out,
        )
        .unwrap();
        assert!(record.recorded);
        assert_eq!(record.key_count, 2);
        assert_eq!(record.budget, LOCAL_XOR_CAP as u8);
        assert_eq!(record.qsr_work, 2);
        assert_eq!(record.xor_used, 2);
        assert_eq!(record.qsr_outcome, QsrOutcome::Found { count: 1 });
        assert!(record.xor_matched_query);
        assert!(keys_equal(&qsr_out, &va));
        assert!(keys_equal(&xor_out, &a));
        assert!(!kademlia_comparison_executed());
        assert!(!unmeasured_better_than_kademlia_claimed());
    }

    #[test]
    fn empty_in_snapshot_versus_xor_nearest_is_not_a_kademlia_claim() {
        let present = key_rest(0x03);
        let missing = key_rest(0x04);
        let mut value = StrongDigest::ZERO;
        value.0[47] = 0x99;
        let keys = [present];
        let values = [value];
        let mut qsr_out = StrongDigest::ZERO;
        let mut xor_out = StrongDigest::ZERO;
        let record =
            run_closed_world_comparison(&keys, &values, &missing, 8, &mut qsr_out, &mut xor_out)
                .unwrap();
        assert!(record.recorded);
        assert_eq!(record.qsr_outcome, QsrOutcome::EmptyInSnapshot);
        assert_ne!(record.qsr_outcome, QsrOutcome::Found { count: 1 });
        assert!(!record.xor_matched_query);
        assert!(keys_equal(&xor_out, &present));
        assert!(!kademlia_comparison_executed());
        assert!(!unmeasured_better_than_kademlia_claimed());
    }
}
