//! Raw Ethernet backend. Linux AF_PACKET is privilege-gated; others fail closed.
//!
//! In-process loopback is a codec/queue test double (`InProcessLoop`). A
//! successful AF_PACKET `open` is `AfPacketAttempt`. `PhysicalLink` is reserved
//! for two-host harnesses and is never returned from these constructors.

mod frame;
mod loopback;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod unsupported;

pub use frame::{decapsulate_ethernet, encapsulate_ethernet, DEV_ETHERTYPE};
pub use loopback::{ethernet_loop_pair, EthernetLoop};

#[cfg(target_os = "linux")]
pub use linux::RawEthernet;
#[cfg(not(target_os = "linux"))]
pub use unsupported::RawEthernet;

/// Honesty labels for Ethernet demonstrations. Unit tests never emit
/// `PhysicalLink`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthernetEvidence {
    InProcessLoop = 0,
    AfPacketAttempt = 1,
    PhysicalLink = 2,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::errors::QdnfError;

    #[test]
    fn open_is_explicitly_unsupported_without_raw_privilege() {
        assert_eq!(
            RawEthernet::open(b"eth0").unwrap_err(),
            QdnfError::PlatformUnsupported
        );
    }

    #[test]
    fn open_empty_name_fails_closed() {
        let err = RawEthernet::open(b"").unwrap_err();
        assert!(err == QdnfError::Range || err == QdnfError::Malformed);
    }

    #[test]
    fn open_overlong_name_fails_closed() {
        assert_eq!(
            RawEthernet::open(&[b'x'; 16]).unwrap_err(),
            QdnfError::Range
        );
    }
}
