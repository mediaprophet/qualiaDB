//! Honest Kademlia comparison stub (E07.8).
//!
//! A tiny in-process XOR-distance scan over 48-byte keys is included so a
//! local equivalent-key workload can run. It is **not** networked Kademlia:
//! no routing table, bootstrap, maintenance, transport, crypto suite, or
//! replication budget is compared. Do not claim QSR is better than Kademlia
//! from this module.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
