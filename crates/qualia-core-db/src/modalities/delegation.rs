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

// ─── Attenuation: a delegatee receives ≤ the delegator's authority ────────────────

/// **Attenuation** (ZCAP-LD / Macaroons): a sub-delegation's capability set `child` is valid only
/// if a SUBSET of the delegator's `parent` set — a delegatee never gains MORE authority than the
/// delegator holds. (Empty child trivially attenuates.)
pub fn attenuates(parent: &[u64], child: &[u64]) -> bool {
    child.iter().all(|c| parent.contains(c))
}

// ─── CRL: cascading revocation against a cryptographic revocation list ────────────

/// Is `node` on the cryptographic revocation list `crl`?
#[inline]
pub fn is_revoked(crl: &[u64], node: u64) -> bool {
    crl.contains(&node)
}

/// Does `agent` still hold authority from `root_authority` after excising EVERY node on the
/// revocation list `crl` (a real-time CRL check across the whole chain)? Zero-heap (bounded BFS).
pub fn authority_after_crl(edges: &[NQuin], root_authority: u64, crl: &[u64], agent: u64) -> bool {
    if is_revoked(crl, agent) || is_revoked(crl, root_authority) {
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
            if e.predicate == p
                && e.subject == cur
                && !is_revoked(crl, e.subject)
                && !is_revoked(crl, e.object)
            {
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

// ─── Spatial & temporal bounds on delegated authority ─────────────────────────────

/// Is a delegation temporally in-force at `now`? `[from, until]` Unix-epoch bounds, where
/// `from == 0` means "no start bound" and `until == 0` means "open-ended".
pub fn delegation_in_force(from: u32, until: u32, now: u32) -> bool {
    (from == 0 || now >= from) && (until == 0 || now <= until)
}

/// Is a delegation valid in `location`? Its `scope_region` (`0` = unbounded / global) must equal
/// `location`. Spatial bounding of delegated authority.
pub fn delegation_in_region(scope_region: u64, location: u64) -> bool {
    scope_region == 0 || scope_region == location
}

// ─── Testamentary Succession & Posthumous Delegation ──────────────────────────────

/// Predicate for testamentary delegation edges: `(decedent, q42:testamentarySuccessor, successor)`.
#[inline]
pub fn testamentary_delegates_predicate() -> u64 {
    q_hash("q42:testamentarySuccessor")
}

/// Evaluates whether `agent` holds authority from `root_authority` taking decedent status
/// and testamentary succession into account.
///
/// Rules:
/// 1. If an intermediary is in `deceased_nodes`, ordinary delegation edges from that intermediary
///    lapse unless `metadata & 0x01 != 0` (survivesDeath flag).
/// 2. If `probate_admitted` is true, testamentary successor edges (`q42:testamentarySuccessor`)
///    from deceased nodes are activated.
/// 3. Zero-heap bounded BFS traversal.
pub fn authority_with_succession(
    edges: &[NQuin],
    root_authority: u64,
    agent: u64,
    deceased_nodes: &[u64],
    probate_admitted: bool,
) -> bool {
    if agent == root_authority {
        return true;
    }
    let p_del = delegates_predicate();
    let p_test = testamentary_delegates_predicate();

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

        let cur_deceased = deceased_nodes.contains(&cur);

        for e in edges {
            let is_ordinary = e.predicate == p_del && e.subject == cur;
            let is_testamentary = e.predicate == p_test && e.subject == cur;

            if !is_ordinary && !is_testamentary {
                continue;
            }

            // Ordinary edges from deceased node lapse unless survives_death flag (metadata & 1 != 0) is set
            if is_ordinary && cur_deceased && (e.metadata & 0x01) == 0 {
                continue;
            }

            // Testamentary edges activate only when the delegator is deceased and probate is admitted
            if is_testamentary && (!cur_deceased || !probate_admitted) {
                continue;
            }

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
    false
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
        assert!(
            has_delegated_authority(&edges, root, officer),
            "transitive delegation"
        );
        assert!(has_delegated_authority(&edges, root, ngo));
        // An agent with no chain from root holds nothing.
        assert!(!has_delegated_authority(
            &edges,
            root,
            q_hash("did:stranger")
        ));
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

    #[test]
    fn attenuation_never_broadens_authority() {
        let (read, write, admin) = (q_hash("cap:read"), q_hash("cap:write"), q_hash("cap:admin"));
        assert!(attenuates(&[read, write, admin], &[read, write]));
        assert!(
            !attenuates(&[read], &[read, admin]),
            "cannot grant a capability the delegator lacks"
        );
        assert!(attenuates(&[read], &[]));
    }

    #[test]
    fn crl_excises_every_revoked_node() {
        let root = q_hash("did:state");
        let agency = q_hash("did:agency");
        let officer = q_hash("did:officer");
        let ngo = q_hash("did:ngo");
        let edges = [edge(root, agency), edge(agency, officer), edge(root, ngo)];
        // Empty CRL → behaves like full authority.
        assert!(authority_after_crl(&edges, root, &[], officer));
        // Revoke the agency via the CRL: officer defeated, ngo (independent) survives.
        assert!(!authority_after_crl(&edges, root, &[agency], officer));
        assert!(authority_after_crl(&edges, root, &[agency], ngo));
        // A multi-entry CRL revoking both branches.
        assert!(!authority_after_crl(&edges, root, &[agency, ngo], ngo));
    }

    #[test]
    fn spatial_and_temporal_bounds() {
        // Temporal: in force only within [100, 200].
        assert!(delegation_in_force(100, 200, 150));
        assert!(!delegation_in_force(100, 200, 50), "before start");
        assert!(!delegation_in_force(100, 200, 250), "after end");
        assert!(delegation_in_force(0, 0, 999), "open-ended");
        // Spatial: scoped to a region (0 = global).
        let region = q_hash("region:au");
        assert!(delegation_in_region(region, region));
        assert!(!delegation_in_region(region, q_hash("region:us")));
        assert!(
            delegation_in_region(0, q_hash("region:anywhere")),
            "global scope"
        );
    }

    #[test]
    fn test_authority_with_succession() {
        let root = q_hash("did:community");
        let decedent = q_hash("did:elder_a");
        let proxy_living = q_hash("did:living_assistant");
        let testamentary_heir = q_hash("did:grandchild");

        let mut edge_normal = edge(decedent, proxy_living);
        let mut edge_surviving = edge(decedent, proxy_living);
        edge_surviving.metadata = 0x01; // survivesDeath flag

        let mut edge_testamentary = NQuin {
            subject: decedent,
            predicate: testamentary_delegates_predicate(),
            object: testamentary_heir,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        edge_testamentary.parity = edge_testamentary.subject
            ^ edge_testamentary.predicate
            ^ edge_testamentary.object
            ^ edge_testamentary.context;

        let edges = [edge(root, decedent), edge_normal, edge_testamentary];

        // While elder is alive: normal assistant has authority; testamentary heir does NOT yet
        assert!(authority_with_succession(&edges, root, proxy_living, &[], false));
        assert!(!authority_with_succession(&edges, root, testamentary_heir, &[], false));

        // When elder is deceased but probate not yet admitted:
        // Normal assistant loses authority; testamentary heir has not activated
        assert!(!authority_with_succession(&edges, root, proxy_living, &[decedent], false));
        assert!(!authority_with_succession(&edges, root, testamentary_heir, &[decedent], false));

        // When elder is deceased and probate is admitted:
        // Normal assistant still lacks authority; testamentary heir is active!
        assert!(!authority_with_succession(&edges, root, proxy_living, &[decedent], true));
        assert!(authority_with_succession(&edges, root, testamentary_heir, &[decedent], true));

        // If normal edge had survivesDeath flag, it also survives death
        let surviving_edges = [edge(root, decedent), edge_surviving, edge_testamentary];
        assert!(authority_with_succession(&surviving_edges, root, proxy_living, &[decedent], true));
    }
}

