//! Disclosure and path-selection policy. Applied *before* any probe.

/// Whether the operation may disclose a direct locator to a counterpart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disclosure {
    /// Counterpart may learn an IP locator. Direct ICE gathering is allowed.
    DirectPermitted,
    /// No direct probes, candidate export, LAN broadcast, or public STUN.
    ApprovedRelaysOnly,
    /// Independently operated hops plus a reviewed metadata construction.
    QualifiedMultiHop,
    /// Local authorised routes and sealed store-and-forward only.
    Isolated,
}

/// Connectivity profile selected before sockets open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathPolicy {
    pub disclosure: Disclosure,
    pub ipv6_preferred: bool,
    pub ipv4_prompt_ms: u16,
    pub relay_immediate: bool,
    pub setup_deadline_ms: u32,
    pub first_direct_round_ms: u32,
}

impl PathPolicy {
    pub const ORDINARY: Self = Self {
        disclosure: Disclosure::DirectPermitted,
        ipv6_preferred: true,
        ipv4_prompt_ms: 250,
        relay_immediate: true,
        setup_deadline_ms: 10_000,
        first_direct_round_ms: 3_000,
    };

    pub const RELAY_ONLY: Self = Self {
        disclosure: Disclosure::ApprovedRelaysOnly,
        ipv6_preferred: true,
        ipv4_prompt_ms: 250,
        relay_immediate: true,
        setup_deadline_ms: 10_000,
        first_direct_round_ms: 3_000,
    };

    pub const ISOLATED: Self = Self {
        disclosure: Disclosure::Isolated,
        ipv6_preferred: true,
        ipv4_prompt_ms: 250,
        relay_immediate: false,
        setup_deadline_ms: 10_000,
        first_direct_round_ms: 3_000,
    };

    /// Direct host/srflx gathering is forbidden.
    pub const fn allows_direct_probes(self) -> bool {
        matches!(self.disclosure, Disclosure::DirectPermitted)
    }

    /// Public STUN toward third-party servers is forbidden.
    pub const fn allows_public_stun(self) -> bool {
        self.allows_direct_probes()
    }

    pub const fn allows_relay(self) -> bool {
        !matches!(self.disclosure, Disclosure::Isolated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_only_forbids_direct_and_stun() {
        assert!(!PathPolicy::RELAY_ONLY.allows_direct_probes());
        assert!(!PathPolicy::RELAY_ONLY.allows_public_stun());
        assert!(PathPolicy::RELAY_ONLY.allows_relay());
    }

    #[test]
    fn isolated_forbids_external_discovery() {
        assert!(!PathPolicy::ISOLATED.allows_direct_probes());
        assert!(!PathPolicy::ISOLATED.allows_relay());
    }
}
