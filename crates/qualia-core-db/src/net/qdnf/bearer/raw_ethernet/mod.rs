//! Raw Ethernet backend. Linux AF_PACKET is privilege-gated; others fail closed.
//!
//! In-process loopback is a codec/queue test double (`InProcessLoop`). A
//! successful AF_PACKET `open` is `AfPacketAttempt`. `PhysicalLink` is reserved
//! for two-host harnesses and is never returned from these constructors.
//!
//! EtherType `0x4242` (`DEV_ETHERTYPE`) is a **development** value, not an IANA
//! assignment. Production must use an assigned EtherType or an administered
//! encapsulation. There is no silent Internet, DNS, or IP fallback
//! (`silent_ip_fallback() == false`). Windows, WASM, and other non-Linux hosts
//! remain [`QdnfError::PlatformUnsupported`] via `unsupported.rs`.

use crate::net::qdnf::errors::QdnfError;

mod frame;
mod loopback;
mod two_host;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod unsupported;

pub use frame::{DEV_ETHERTYPE, decapsulate_ethernet, encapsulate_ethernet};
pub use loopback::{EthernetLoop, ethernet_loop_pair};
pub use two_host::{TwoHostProbe, TwoHostProbeReason, probe_two_host};

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

impl EthernetEvidence {
    /// Unit-test constructors never claim two-host physical delivery.
    pub const fn is_physical_link(self) -> bool {
        matches!(self, Self::PhysicalLink)
    }
}

/// Two-host physical Ethernet has not been demonstrated in this process.
pub const fn physical_two_host_qualified() -> bool {
    false
}

/// veth + network-namespace two-process delivery has not been qualified.
/// A CAP_NET_ADMIN probe in this environment returns EPERM.
pub const fn veth_namespace_qualified() -> bool {
    false
}

/// Native Ethernet never falls back to IP, DNS, or Internet sockets.
pub const fn silent_ip_fallback() -> bool {
    false
}

/// Capability probe for veth + netns qualification. Does not change this
/// process's namespace. EPERM/ENOSYS/`clone` failure is
/// [`QdnfError::PlatformUnsupported`]. A successful child `unshare` is still
/// not two-process Ethernet evidence ([`veth_namespace_qualified`] stays false).
pub fn probe_veth_namespace() -> Result<(), QdnfError> {
    #[cfg(target_os = "linux")]
    {
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(QdnfError::PlatformUnsupported);
        }
        if pid == 0 {
            let rc = unsafe { libc::unshare(libc::CLONE_NEWNET) };
            let code = if rc == 0 {
                0
            } else {
                std::io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or(libc::EPERM)
            };
            unsafe { libc::_exit(code) };
        }
        let mut status = 0;
        if unsafe { libc::waitpid(pid, &mut status, 0) } < 0 {
            return Err(QdnfError::PlatformUnsupported);
        }
        if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0 {
            return Err(QdnfError::Incomplete);
        }
        Err(QdnfError::PlatformUnsupported)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(QdnfError::PlatformUnsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::errors::QdnfError;
    use crate::net::qdnf::types::ScopeEpoch;

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

    #[test]
    fn silent_ip_fallback_is_false() {
        assert!(!silent_ip_fallback());
    }

    #[test]
    fn unit_tests_never_claim_physical_link() {
        assert!(!physical_two_host_qualified());
        assert!(!veth_namespace_qualified());
        assert!(!EthernetEvidence::InProcessLoop.is_physical_link());
        assert!(!EthernetEvidence::AfPacketAttempt.is_physical_link());
        let (a, _b) = ethernet_loop_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
        assert_eq!(a.ethernet_evidence_level(), EthernetEvidence::InProcessLoop);
        assert_ne!(a.ethernet_evidence_level(), EthernetEvidence::PhysicalLink);
        assert_eq!(
            RawEthernet::open(b"eth0").unwrap_err(),
            QdnfError::PlatformUnsupported
        );
        let probe = probe_veth_namespace();
        assert!(
            probe == Err(QdnfError::PlatformUnsupported) || probe == Err(QdnfError::Incomplete)
        );
        assert!(!veth_namespace_qualified());
        assert!(!physical_two_host_qualified());
    }

    #[test]
    fn development_ethertype_is_0x4242() {
        assert_eq!(DEV_ETHERTYPE, 0x4242);
    }
}
