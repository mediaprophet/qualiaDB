//! Bounded candidate and pair tables. No unbounded Cartesian product.

use super::policy::PathPolicy;

pub const MAX_LOCAL: usize = 32;
pub const MAX_REMOTE: usize = 32;
pub const MAX_PAIRS: usize = 64;
pub const MAX_ATTEMPTS: usize = 8;
pub const MAX_ACTIVE_PATHS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Host = 126,
    PeerReflexive = 110,
    ServerReflexive = 100,
    Relayed = 0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrFamily {
    V6,
    V4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub kind: CandidateKind,
    pub family: AddrFamily,
    pub foundation: u32,
    pub component: u8,
    pub priority: u32,
}

impl Candidate {
    pub fn host(family: AddrFamily, foundation: u32) -> Self {
        Self::new(CandidateKind::Host, family, foundation)
    }

    pub fn relayed(family: AddrFamily, foundation: u32) -> Self {
        Self::new(CandidateKind::Relayed, family, foundation)
    }

    pub fn srflx(family: AddrFamily, foundation: u32) -> Self {
        Self::new(CandidateKind::ServerReflexive, family, foundation)
    }

    fn new(kind: CandidateKind, family: AddrFamily, foundation: u32) -> Self {
        let type_pref = kind as u32;
        let local_pref = match family {
            AddrFamily::V6 => 65_535,
            AddrFamily::V4 => 65_534,
        };
        let priority = (type_pref << 24) + (local_pref << 8) + (256 - 1);
        Self {
            kind,
            family,
            foundation,
            component: 1,
            priority,
        }
    }
}

/// RFC 8445 pair priority. `g` is the controlling agent's candidate priority.
pub fn pair_priority(g: u32, d: u32) -> u64 {
    let min = u64::from(g.min(d));
    let max = u64::from(g.max(d));
    (1u64 << 32) * min + 2 * max + u64::from(g > d)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairState {
    Waiting,
    InProgress,
    Succeeded,
    Failed,
    Frozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidatePair {
    pub local_idx: u8,
    pub remote_idx: u8,
    pub priority: u64,
    pub state: PairState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateTables {
    pub local: [Option<Candidate>; MAX_LOCAL],
    pub local_len: usize,
    pub remote: [Option<Candidate>; MAX_REMOTE],
    pub remote_len: usize,
    pub pairs: [Option<CandidatePair>; MAX_PAIRS],
    pub pair_len: usize,
}

impl CandidateTables {
    pub const fn empty() -> Self {
        Self {
            local: [None; MAX_LOCAL],
            local_len: 0,
            remote: [None; MAX_REMOTE],
            remote_len: 0,
            pairs: [None; MAX_PAIRS],
            pair_len: 0,
        }
    }

    pub fn push_local(&mut self, c: Candidate) -> Result<usize, ()> {
        if self.local_len >= MAX_LOCAL {
            return Err(());
        }
        let i = self.local_len;
        self.local[i] = Some(c);
        self.local_len += 1;
        Ok(i)
    }

    pub fn push_remote(&mut self, c: Candidate) -> Result<usize, ()> {
        if self.remote_len >= MAX_REMOTE {
            return Err(());
        }
        let i = self.remote_len;
        self.remote[i] = Some(c);
        self.remote_len += 1;
        Ok(i)
    }

    /// Diversity-aware pairing. Stops at [`MAX_PAIRS`].
    pub fn form_pairs(&mut self, controlling: bool) -> usize {
        self.pair_len = 0;
        self.pairs = [None; MAX_PAIRS];
        let mut i = 0usize;
        while i < self.local_len && self.pair_len < MAX_PAIRS {
            let Some(l) = self.local[i] else {
                i += 1;
                continue;
            };
            let mut j = 0usize;
            while j < self.remote_len && self.pair_len < MAX_PAIRS {
                let Some(r) = self.remote[j] else {
                    j += 1;
                    continue;
                };
                if l.family != r.family || l.component != r.component {
                    j += 1;
                    continue;
                }
                let (g, d) = if controlling {
                    (l.priority, r.priority)
                } else {
                    (r.priority, l.priority)
                };
                self.pairs[self.pair_len] = Some(CandidatePair {
                    local_idx: i as u8,
                    remote_idx: j as u8,
                    priority: pair_priority(g, d),
                    state: PairState::Waiting,
                });
                self.pair_len += 1;
                j += 1;
            }
            i += 1;
        }
        self.sort_pairs();
        self.pair_len
    }

    fn sort_pairs(&mut self) {
        let n = self.pair_len;
        let mut a = 0usize;
        while a + 1 < n {
            let mut b = 0usize;
            while b + 1 < n - a {
                let pa = self.pairs[b].unwrap().priority;
                let pb = self.pairs[b + 1].unwrap().priority;
                if pa < pb {
                    self.pairs.swap(b, b + 1);
                }
                b += 1;
            }
            a += 1;
        }
    }
}

/// Gather the candidates this policy permits. No sockets.
pub fn gather_permitted(policy: PathPolicy, has_v6: bool, has_v4: bool) -> CandidateTables {
    let mut t = CandidateTables::empty();
    if policy.allows_relay() {
        let _ = t.push_local(Candidate::relayed(AddrFamily::V6, 1));
        if has_v4 {
            let _ = t.push_local(Candidate::relayed(AddrFamily::V4, 2));
        }
    }
    if policy.allows_direct_probes() {
        if has_v6 {
            let _ = t.push_local(Candidate::host(AddrFamily::V6, 3));
        }
        if has_v4 {
            let _ = t.push_local(Candidate::host(AddrFamily::V4, 4));
        }
        if policy.allows_public_stun() {
            if has_v6 {
                let _ = t.push_local(Candidate::srflx(AddrFamily::V6, 5));
            }
            if has_v4 {
                let _ = t.push_local(Candidate::srflx(AddrFamily::V4, 6));
            }
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_only_has_no_host_or_srflx() {
        let t = gather_permitted(PathPolicy::RELAY_ONLY, true, true);
        let mut i = 0;
        while i < t.local_len {
            let k = t.local[i].unwrap().kind;
            assert_eq!(k, CandidateKind::Relayed);
            i += 1;
        }
    }

    #[test]
    fn pair_cap_is_64() {
        assert_eq!(MAX_PAIRS, 64);
        assert_eq!(MAX_ATTEMPTS, 8);
        assert_eq!(MAX_ACTIVE_PATHS, 3);
    }
}
