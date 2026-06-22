//! Causal & counterfactual logic (§16, legal_logic.md) — liability & dependency.
//!
//! Standard implication (`p → q`) is insufficient for legal liability. Adjudicating a
//! human-rights violation or a structural failure needs **but-for causation** (was the cause
//! *necessary* for the harm?), **root-node dependency** (removing a foundational support
//! voids everything that depends on it — the "deepest absence"), and **overdetermination**
//! (several independent sufficient causes → joint liability, where no single one is but-for).
//!
//! Causation is a DAG of `(cause, q42:causeOf, effect)` edges over a set of `roots` (the
//! base facts that actually occurred). All evaluation is bounded BFS reachability — zero-heap
//! (fixed frontier/visited arrays), the same shape as `dl::check_subsumption_quin`.

use crate::{q_hash, NQuin};

/// Bound on distinct nodes in one causal query.
pub const MAX_CAUSAL_NODES: usize = 256;

/// Sentinel meaning "remove nothing" — `q_hash` is 60-bit, so `u64::MAX` is never a real node.
const NO_REMOVAL: u64 = u64::MAX;

/// The causal-edge predicate `(cause, q42:causeOf, effect)`.
#[inline]
pub fn cause_predicate() -> u64 {
    q_hash("q42:causeOf")
}

/// Internal: is `target` reachable from any `root` along `causeOf` edges, with `removed`
/// node excised from the graph (both as a root and as any edge endpoint)? Bounded, zero-heap.
fn caused_internal(edges: &[NQuin], roots: &[u64], target: u64, removed: u64) -> bool {
    if target == removed {
        return false;
    }
    let p = cause_predicate();
    let mut frontier = [0u64; MAX_CAUSAL_NODES];
    let mut visited = [0u64; MAX_CAUSAL_NODES];
    let mut fl = 0usize;
    let mut vl = 0usize;
    for &r in roots {
        if r == removed {
            continue;
        }
        if r == target {
            return true;
        }
        if fl < MAX_CAUSAL_NODES {
            frontier[fl] = r;
            fl += 1;
        }
    }
    while fl > 0 {
        fl -= 1;
        let cur = frontier[fl];
        if visited[..vl].contains(&cur) {
            continue;
        }
        if vl < MAX_CAUSAL_NODES {
            visited[vl] = cur;
            vl += 1;
        } else {
            break; // closure exceeds the bound — refuse rather than mis-answer
        }
        for e in edges {
            if e.predicate == p && e.subject == cur && e.subject != removed && e.object != removed {
                let nxt = e.object;
                if nxt == target {
                    return true;
                }
                if fl < MAX_CAUSAL_NODES && !visited[..vl].contains(&nxt) {
                    frontier[fl] = nxt;
                    fl += 1;
                }
            }
        }
    }
    false
}

/// Did `effect` occur — i.e. is it reachable from the occurred `roots` along causeOf edges?
#[inline]
pub fn caused(edges: &[NQuin], roots: &[u64], effect: u64) -> bool {
    caused_internal(edges, roots, effect, NO_REMOVAL)
}

/// **But-for causation**: `effect` occurred, and *but for* `cause` it would NOT have — i.e.
/// `cause` is a *necessary* condition (removing it makes `effect` unreachable). This is the
/// legal "but-for" / sine-qua-non test.
pub fn but_for_cause(edges: &[NQuin], roots: &[u64], cause: u64, effect: u64) -> bool {
    caused(edges, roots, effect) && !caused_internal(edges, roots, effect, cause)
}

/// **Root-node dependency**: is `node` voided by removing the foundational support `removed`?
/// True iff `node` occurs normally but becomes unreachable once `removed` is gone — "if food/
/// shelter is removed, all dependent rights and capacities are voided" (the deepest-absence rule).
pub fn is_voided_by(edges: &[NQuin], roots: &[u64], removed: u64, node: u64) -> bool {
    caused(edges, roots, node) && !caused_internal(edges, roots, node, removed)
}

/// Collect, into `out`, the `candidates` that are voided by removing `removed`. Returns the
/// count written. Zero-heap (caller-supplied `out`).
pub fn dependents_voided(
    edges: &[NQuin],
    roots: &[u64],
    removed: u64,
    candidates: &[u64],
    out: &mut [u64],
) -> usize {
    let mut n = 0usize;
    for &c in candidates {
        if is_voided_by(edges, roots, removed, c) {
            if n >= out.len() {
                break;
            }
            out[n] = c;
            n += 1;
        }
    }
    n
}

/// **Causal overdetermination** (joint liability): `effect` occurred, there are ≥2 candidate
/// `causes`, and **no single one is but-for** — removing any one alone still yields the effect
/// (another sufficient cause remains). Liability is then shared across all of them.
pub fn is_overdetermined(edges: &[NQuin], roots: &[u64], causes: &[u64], effect: u64) -> bool {
    if causes.len() < 2 || !caused(edges, roots, effect) {
        return false;
    }
    // No cause is necessary: for each, the effect still occurs without it.
    causes
        .iter()
        .all(|&c| caused_internal(edges, roots, effect, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(cause: u64, effect: u64) -> NQuin {
        let mut q = NQuin {
            subject: cause,
            predicate: cause_predicate(),
            object: effect,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn but_for_along_a_chain() {
        // missing-funding → no-staff → service-failure (the harm). Roots: missing-funding occurred.
        let fund = q_hash("cause:missingFunding");
        let staff = q_hash("cause:noStaff");
        let harm = q_hash("harm:serviceFailure");
        let edges = [edge(fund, staff), edge(staff, harm)];
        let roots = [fund];
        assert!(caused(&edges, &roots, harm));
        // Every node on the only path is but-for necessary.
        assert!(but_for_cause(&edges, &roots, fund, harm));
        assert!(but_for_cause(&edges, &roots, staff, harm));
        // An unrelated node is not a but-for cause.
        assert!(!but_for_cause(&edges, &roots, q_hash("cause:weather"), harm));
    }

    #[test]
    fn overdetermination_is_joint_not_but_for() {
        // Two independent sufficient causes of the same harm.
        let c1 = q_hash("cause:fireA");
        let c2 = q_hash("cause:fireB");
        let harm = q_hash("harm:houseDestroyed");
        let edges = [edge(c1, harm), edge(c2, harm)];
        let roots = [c1, c2];
        assert!(caused(&edges, &roots, harm));
        // Neither alone is but-for (the other still destroys the house).
        assert!(!but_for_cause(&edges, &roots, c1, harm));
        assert!(!but_for_cause(&edges, &roots, c2, harm));
        // → overdetermined → joint liability.
        assert!(is_overdetermined(&edges, &roots, &[c1, c2], harm));
    }

    #[test]
    fn root_removal_voids_dependents() {
        // food → health → work ; shelter → health (diamond on health).
        let food = q_hash("support:food");
        let shelter = q_hash("support:shelter");
        let health = q_hash("capacity:health");
        let work = q_hash("capacity:work");
        let edges = [edge(food, health), edge(shelter, health), edge(health, work)];
        let roots = [food, shelter];
        // Removing food alone does NOT void health/work (shelter still supports health).
        assert!(!is_voided_by(&edges, &roots, food, work));
        // But a single-support chain: education → literacy. Remove education → literacy voided.
        let edu = q_hash("support:education");
        let lit = q_hash("capacity:literacy");
        let edges2 = [edge(edu, lit)];
        let roots2 = [edu];
        let mut out = [0u64; 4];
        let n = dependents_voided(&edges2, &roots2, edu, &[lit], &mut out);
        assert_eq!(n, 1);
        assert_eq!(out[0], lit);
    }
}
