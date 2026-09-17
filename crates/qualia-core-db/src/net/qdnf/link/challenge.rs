//! Discovery/reachability challenges bound to scope, peer, and observed locator.
//!
//! A challenge is not membership, application admission, or a handshake share.
//! Pending challenges occupy a fixed table; hostile beacons cannot grow it.

use crate::crypto::network::kdf::hmac_sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{LinkId, ObservedLocator, ScopeEpoch};

use super::adjacency::Adjacency;
use super::budget::PreAuthBudget;
use super::discovery::Beacon;
use super::neighbor::NeighborTable;

pub const CHALLENGE_TAG_LEN: usize = 16;
pub const MAX_PENDING_CHALLENGES: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryChallenge {
    pub scope: ScopeEpoch,
    pub peer: LinkId,
    pub locator: ObservedLocator,
    pub expiry_unix: u64,
    pub nonce: [u8; 16],
    pub tag: [u8; CHALLENGE_TAG_LEN],
}

pub struct ChallengeTable {
    slots: [Option<DiscoveryChallenge>; MAX_PENDING_CHALLENGES],
}

impl ChallengeTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_PENDING_CHALLENGES],
        }
    }

    /// Issue a challenge HMAC-bound to `(scope, peer, locator)`.
    pub fn issue(
        &mut self,
        scope: ScopeEpoch,
        peer: LinkId,
        locator: ObservedLocator,
        now_unix: u64,
        ttl_secs: u64,
        secret: &[u8],
    ) -> Result<DiscoveryChallenge, QdnfError> {
        let expiry_unix = now_unix.checked_add(ttl_secs).ok_or(QdnfError::Range)?;
        let slot = self.free_slot(now_unix).ok_or(QdnfError::Capacity)?;
        let mut nonce = [0u8; 16];
        nonce[..8].copy_from_slice(&now_unix.to_be_bytes());
        nonce[8..12].copy_from_slice(&(slot as u32).to_be_bytes());
        nonce[12..16].copy_from_slice(&(expiry_unix as u32).to_be_bytes());
        let tag = mint_challenge(scope, peer, &locator, expiry_unix, &nonce, secret)?;
        let challenge = DiscoveryChallenge {
            scope,
            peer,
            locator,
            expiry_unix,
            nonce,
            tag,
        };
        self.slots[slot] = Some(challenge);
        Ok(challenge)
    }

    pub fn verify(
        &self,
        tag: &[u8; CHALLENGE_TAG_LEN],
        scope: ScopeEpoch,
        peer: LinkId,
        locator: &ObservedLocator,
        now_unix: u64,
    ) -> Result<(), QdnfError> {
        let stored = match self.find(tag) {
            Some(c) => c,
            None => return Err(QdnfError::Unauthorized),
        };
        stored.binds(scope, peer, locator, now_unix)
    }

    fn find(&self, tag: &[u8; CHALLENGE_TAG_LEN]) -> Option<&DiscoveryChallenge> {
        self.slots.iter().flatten().find(|c| tag_eq(&c.tag, tag))
    }

    fn free_slot(&self, now_unix: u64) -> Option<usize> {
        let mut expired = None;
        let mut i = 0usize;
        while i < MAX_PENDING_CHALLENGES {
            match self.slots[i] {
                None => return Some(i),
                Some(c) if now_unix >= c.expiry_unix => {
                    if expired.is_none() {
                        expired = Some(i);
                    }
                }
                Some(_) => {}
            }
            i += 1;
        }
        expired
    }
}

impl Default for ChallengeTable {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryChallenge {
    /// Binding check: scope, peer, observed locator, and expiry.
    pub fn binds(
        &self,
        scope: ScopeEpoch,
        peer: LinkId,
        locator: &ObservedLocator,
        now_unix: u64,
    ) -> Result<(), QdnfError> {
        if now_unix >= self.expiry_unix {
            return Err(QdnfError::Expired);
        }
        if self.scope != scope || self.peer != peer || self.locator != *locator {
            return Err(QdnfError::Unauthorized);
        }
        Ok(())
    }
}

/// Charge the pre-auth budget, then reject an expired beacon. No neighbor slot.
pub fn accept_beacon(
    beacon: &Beacon,
    now_unix: u64,
    budget: &mut PreAuthBudget,
) -> Result<(), QdnfError> {
    budget.charge(Beacon::WIRE_LEN as u64, 1)?;
    beacon.accept(now_unix)
}

/// Insert adjacency only after the challenge binds the observed neighbor.
pub fn insert_after_challenge(
    table: &mut NeighborTable,
    adj: Adjacency,
    challenge: &DiscoveryChallenge,
    scope: ScopeEpoch,
    now_unix: u64,
) -> Result<usize, QdnfError> {
    challenge.binds(scope, adj.remote, &adj.observed_peer, now_unix)?;
    table.insert(adj)
}

fn mint_challenge(
    scope: ScopeEpoch,
    peer: LinkId,
    locator: &ObservedLocator,
    expiry_unix: u64,
    nonce: &[u8; 16],
    secret: &[u8],
) -> Result<[u8; CHALLENGE_TAG_LEN], QdnfError> {
    let loc = locator.as_slice();
    let mut info = [0u8; 96];
    let label = b"qdnf-chal-v1";
    let mut off = label.len();
    info[..off].copy_from_slice(label);
    info[off..off + 8].copy_from_slice(&scope.scope.to_be_bytes());
    off += 8;
    info[off..off + 8].copy_from_slice(&scope.epoch.to_be_bytes());
    off += 8;
    info[off..off + 16].copy_from_slice(&peer.0);
    off += 16;
    info[off] = locator.len;
    off += 1;
    info[off..off + loc.len()].copy_from_slice(loc);
    off += loc.len();
    info[off..off + 8].copy_from_slice(&expiry_unix.to_be_bytes());
    off += 8;
    info[off..off + 16].copy_from_slice(nonce);
    off += 16;
    let mut mac = [0u8; 48];
    hmac_sha384(secret, &info[..off], &mut mac).map_err(|_| QdnfError::CryptoFailure)?;
    let mut out = [0u8; CHALLENGE_TAG_LEN];
    out.copy_from_slice(&mac[..CHALLENGE_TAG_LEN]);
    Ok(out)
}

fn tag_eq(a: &[u8; CHALLENGE_TAG_LEN], b: &[u8; CHALLENGE_TAG_LEN]) -> bool {
    let mut diff = 0u8;
    let mut i = 0usize;
    while i < CHALLENGE_TAG_LEN {
        diff |= a[i] ^ b[i];
        i += 1;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::link::adjacency::AdjacencyState;
    use crate::net::qdnf::link::discovery::DiscoveryMode;
    use crate::net::qdnf::link::neighbor::MAX_NEIGHBORS;

    const SECRET: &[u8] = b"qdnf-discovery-challenge-secret";

    fn loc(byte: u8) -> ObservedLocator {
        ObservedLocator::from_slice(&[byte]).unwrap()
    }

    fn scope() -> ScopeEpoch {
        ScopeEpoch { scope: 7, epoch: 3 }
    }

    fn peer(b: u8) -> LinkId {
        let mut id = LinkId::ZERO;
        id.0[0] = b;
        id
    }

    #[test]
    fn challenge_binds_scope_peer_locator() {
        let mut table = ChallengeTable::new();
        let challenge = table
            .issue(scope(), peer(1), loc(2), 1_000, 30, SECRET)
            .unwrap();
        assert_eq!(
            table.verify(&challenge.tag, scope(), peer(1), &loc(2), 1_000),
            Ok(())
        );
        assert_eq!(
            table.verify(&challenge.tag, scope(), peer(9), &loc(2), 1_000),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            table.verify(&challenge.tag, scope(), peer(1), &loc(9), 1_000),
            Err(QdnfError::Unauthorized)
        );
        let other_scope = ScopeEpoch { scope: 8, epoch: 3 };
        assert_eq!(
            table.verify(&challenge.tag, other_scope, peer(1), &loc(2), 1_000),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn expired_beacon_is_expired() {
        let beacon = Beacon {
            mode: DiscoveryMode::PrivatePairwise,
            tag: [1u8; 16],
            link_id: peer(1),
            epoch: 1,
            expiry_unix: 10,
            mtu: 1280,
        };
        let mut budget = PreAuthBudget::new();
        assert_eq!(
            accept_beacon(&beacon, 10, &mut budget),
            Err(QdnfError::Expired)
        );
        assert_eq!(budget.used_bytes(), Beacon::WIRE_LEN as u64);
    }

    #[test]
    fn expired_challenge_is_expired() {
        let mut table = ChallengeTable::new();
        let challenge = table
            .issue(scope(), peer(1), loc(2), 1_000, 10, SECRET)
            .unwrap();
        assert_eq!(
            table.verify(
                &challenge.tag,
                scope(),
                peer(1),
                &loc(2),
                challenge.expiry_unix
            ),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn thirty_third_challenge_is_capacity() {
        let mut table = ChallengeTable::new();
        for i in 0..MAX_PENDING_CHALLENGES {
            table
                .issue(
                    scope(),
                    peer(i as u8 + 1),
                    loc(i as u8 + 1),
                    1_000,
                    30,
                    SECRET,
                )
                .unwrap();
        }
        assert_eq!(
            table.issue(scope(), peer(0xFF), loc(0xFF), 1_000, 30, SECRET),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn neighbor_insert_requires_matching_challenge() {
        let mut neighbors = NeighborTable::new();
        let mut challenges = ChallengeTable::new();
        let locator = loc(4);
        let remote = peer(4);
        let challenge = challenges
            .issue(scope(), remote, locator, 1_000, 30, SECRET)
            .unwrap();
        let adj = Adjacency {
            local: peer(0),
            remote,
            observed_peer: locator,
            state: AdjacencyState::ChallengeReceived,
            generation: 1,
            mtu: 1280,
        };
        assert_eq!(
            insert_after_challenge(&mut neighbors, adj, &challenge, scope(), 1_000),
            Ok(0)
        );
        let wrong = Adjacency {
            observed_peer: loc(9),
            ..adj
        };
        assert_eq!(
            insert_after_challenge(&mut neighbors, wrong, &challenge, scope(), 1_000),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(neighbors.len(), 1);
    }

    #[test]
    fn challenged_insert_still_caps_neighbor_table() {
        let mut neighbors = NeighborTable::new();
        let mut challenges = ChallengeTable::new();
        for i in 0..MAX_NEIGHBORS {
            let remote = peer(i as u8 + 1);
            let locator = loc(i as u8 + 1);
            let challenge = challenges
                .issue(scope(), remote, locator, 1_000, 30, SECRET)
                .unwrap();
            insert_after_challenge(
                &mut neighbors,
                Adjacency {
                    local: peer(0),
                    remote,
                    observed_peer: locator,
                    state: AdjacencyState::Adjacent,
                    generation: 1,
                    mtu: 1280,
                },
                &challenge,
                scope(),
                1_000,
            )
            .unwrap();
        }
        let extra = peer(0xFE);
        let extra_loc = loc(0xFE);
        let mut extra_challenges = ChallengeTable::new();
        let challenge = extra_challenges
            .issue(scope(), extra, extra_loc, 1_000, 30, SECRET)
            .unwrap();
        assert_eq!(
            insert_after_challenge(
                &mut neighbors,
                Adjacency {
                    local: peer(0),
                    remote: extra,
                    observed_peer: extra_loc,
                    state: AdjacencyState::Adjacent,
                    generation: 1,
                    mtu: 1280,
                },
                &challenge,
                scope(),
                1_000,
            ),
            Err(QdnfError::Capacity)
        );
        assert_eq!(neighbors.len(), MAX_NEIGHBORS);
    }
}
