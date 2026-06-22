//! Delegation & credential-chain logic (§21, legal_logic.md) — the trust fabric.
//!
//! Legal power and identity flow through chains of authorization. This module governs how
//! authority propagates along a delegation DAG and how **revocation of an upstream node
//! cascades** to defeat every downstream dependent (ZCAP-LD / capability chains; Open Badges
//! `EndorsementCredential`). Authority delegation edges are `(delegator, q42:delegatesTo,
//! delegatee)`. Bounded BFS, zero-heap.

use crate::{q_hash, NQuin};

/// Bound on distinct nodes in one delegation query.
pub const MAX_DELEGATION_NODES: usize = 256;

const NO_REVOCATION: u64 = u64::MAX;

/// The delegation-edge predicate `(delegator, q42:delegatesTo, delegatee)`.
#[inline]
pub fn delegates_predicate() -> u64 {
    q_hash("q42:delegatesTo")
}

/// Internal: is `agent` reachable from `root_authority` along delegation edges, with `revoked`
/// excised (revocation cuts the chain there and below)? Bounded, zero-heap.
fn reaches(edges: &[NQuin], root_authority: u64, agent: u64, revoked: u64) -> bool {
    if agent == revoked || root_authority == revoked {
        return false;
    }
    if agent == root_authority {
        return true;
    }
    let p = delegates_predicate();
    let mut frontier = [0u64; MAX_DELEGATION_NODES];
    let mut visited = [0u64; MAX_DELEGATION_NODES];
    let mut fl = 1usize;
    let mut vl = 0usize;
    frontier[0] = root_authority;
    while fl > 0 {
        fl -= 1;
        let cur = frontier[fl];
        if visited[..vl].contains(&cur) {
            continue;
        }
        if vl < MAX_DELEGATION_NODES {
            visited[vl] = cur;
            vl += 1;
        } else {
            break;
        }
        for e in edges {
            if e.predicate == p && e.subject == cur && e.subject != revoked && e.object != revoked {
                let nxt = e.object;
                if nxt == agent {
                    return true;
                }
                if fl < MAX_DELEGATION_NODES && !visited[..vl].contains(&nxt) {
                    frontier[fl] = nxt;
                    fl += 1;
                }
            }
        }
    }
    false
}

/// Does `agent` hold authority delegated (transitively) from `root_authority`?
/// `Auth(α,p) ∧ Deleg*(α,…,β) → Auth(β,p)`.
#[inline]
pub fn has_delegated_authority(edges: &[NQuin], root_authority: u64, agent: u64) -> bool {
    reaches(edges, root_authority, agent, NO_REVOCATION)
}

/// **Revocation cascade**: after `revoked` is revoked, does `agent` still hold authority from
/// `root_authority`? An agent whose only chain ran through `revoked` is now **defeated**.
#[inline]
pub fn authority_after_revocation(
    edges: &[NQuin],
    root_authority: u64,
    revoked: u64,
    agent: u64,
) -> bool {
    reaches(edges, root_authority, agent, revoked)
}

/// Collect, into `out`, the `candidates` **defeated** by revoking `revoked` — held authority
/// before, lost it after. Returns the count. Zero-heap.
pub fn revoked_descendants(
    edges: &[NQuin],
    root_authority: u64,
    revoked: u64,
    candidates: &[u64],
    out: &mut [u64],
) -> usize {
    let mut n = 0usize;
    for &c in candidates {
        if has_delegated_authority(edges, root_authority, c)
            && !authority_after_revocation(edges, root_authority, revoked, c)
        {
            if n >= out.len() {
                break;
            }
            out[n] = c;
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(delegator: u64, delegatee: u64) -> NQuin {
        let mut q = NQuin {
            subject: delegator,
            predicate: delegates_predicate(),
            object: delegatee,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn authority_flows_along_the_chain() {
        // root → agency → officer ; root → ngo
        let root = q_hash("did:state");
        let agency = q_hash("did:agency");
        let officer = q_hash("did:officer");
        let ngo = q_hash("did:ngo");
        let edges = [edge(root, agency), edge(agency, officer), edge(root, ngo)];
        assert!(has_delegated_authority(&edges, root, officer), "transitive delegation");
        assert!(has_delegated_authority(&edges, root, ngo));
        // An agent with no chain from root holds nothing.
        assert!(!has_delegated_authority(&edges, root, q_hash("did:stranger")));
    }

    #[test]
    fn revocation_cascades_to_descendants() {
        let root = q_hash("did:state");
        let agency = q_hash("did:agency");
        let officer = q_hash("did:officer");
        let ngo = q_hash("did:ngo");
        let edges = [edge(root, agency), edge(agency, officer), edge(root, ngo)];
        // Revoke the agency: the officer (downstream) is defeated; the NGO (independent) is not.
        assert!(!authority_after_revocation(&edges, root, agency, officer));
        assert!(authority_after_revocation(&edges, root, agency, ngo));
        let mut out = [0u64; 8];
        let n = revoked_descendants(&edges, root, agency, &[agency, officer, ngo], &mut out);
        // Both the agency itself and the officer lose authority; the NGO keeps it.
        assert!(out[..n].contains(&officer));
        assert!(out[..n].contains(&agency));
        assert!(!out[..n].contains(&ngo));
    }
}
