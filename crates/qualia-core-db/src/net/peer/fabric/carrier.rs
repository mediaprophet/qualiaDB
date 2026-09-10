//! Path classes and carrier kinds. Adding a carrier must not enlarge disclosure.

use crate::net::peer::connectivity::policy::Disclosure;

/// How packets would reach the counterpart.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathClass {
    /// Access-network IP would be observable to the counterpart.
    DirectV6 = 1,
    DirectV4 = 2,
    /// Bound/relayed forwarding. The access relay still sees the client source.
    Relayed = 3,
    /// Browser WebTransport / WSS to an approved gateway.
    BrowserGateway = 4,
    /// Sealed store-and-forward. No live Internet path.
    Offline = 5,
}

/// Preferred or compatibility engine. Selection is policy, not a socket grant.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierKind {
    NativeQdnf = 1,
    QuicMasque = 2,
    Http2Capsule = 3,
    WireGuardIceWss = 4,
    Browser = 5,
    OfflineDurable = 6,
}

impl PathClass {
    pub const fn discloses_peer_ip(self) -> bool {
        matches!(self, Self::DirectV6 | Self::DirectV4)
    }

    pub const fn is_live_network(self) -> bool {
        !matches!(self, Self::Offline)
    }
}

/// First filter: a prohibited class never enters scoring.
pub const fn prohibited(disclosure: Disclosure, class: PathClass) -> bool {
    match disclosure {
        Disclosure::DirectPermitted => false,
        Disclosure::ApprovedRelaysOnly | Disclosure::QualifiedMultiHop => class.discloses_peer_ip(),
        Disclosure::Isolated => class.is_live_network(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_only_forbids_direct_classes() {
        assert!(prohibited(
            Disclosure::ApprovedRelaysOnly,
            PathClass::DirectV6
        ));
        assert!(prohibited(
            Disclosure::ApprovedRelaysOnly,
            PathClass::DirectV4
        ));
        assert!(!prohibited(
            Disclosure::ApprovedRelaysOnly,
            PathClass::Relayed
        ));
    }

    #[test]
    fn isolated_forbids_live_paths() {
        assert!(prohibited(Disclosure::Isolated, PathClass::Relayed));
        assert!(!prohibited(Disclosure::Isolated, PathClass::Offline));
    }
}
