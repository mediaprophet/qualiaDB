//! AF_XDP acceleration is not admitted on this Linux userspace (E10.6).
//!
//! Opening returns [`QdnfError::PlatformUnsupported`]. There is no CAP path,
//! no UMEM registration, and no silent fallback to a fake zero-copy NIC
//! (IPC / in-process loop). Portable bearer semantics must pass first.

use crate::net::qdnf::errors::QdnfError;

/// POSIX `IFNAMSIZ` (including the trailing NUL).
const IFNAMSIZ: usize = 16;

fn validate_ifname(interface: &[u8]) -> Result<(), QdnfError> {
    if interface.is_empty() || interface.len() >= IFNAMSIZ {
        Err(QdnfError::Range)
    } else {
        Ok(())
    }
}

/// AF_XDP endpoint. Construction is fail-closed; no live socket is held.
#[derive(Debug)]
pub struct AfXdp {
    _private: (),
}

impl AfXdp {
    /// Open an AF_XDP socket. Always [`QdnfError::PlatformUnsupported`] after a
    /// valid interface name. Never returns Ok on this host.
    pub fn open(interface: &[u8]) -> Result<Self, QdnfError> {
        validate_ifname(interface)?;
        Err(QdnfError::PlatformUnsupported)
    }
}

/// Module-level open. Same fail-closed gate as [`AfXdp::open`].
pub fn open_af_xdp(interface: &[u8]) -> Result<AfXdp, QdnfError> {
    AfXdp::open(interface)
}

/// AF_XDP never falls back to IPC, loopback, or a synthetic zero-copy NIC.
#[inline]
pub const fn silent_zero_copy_fallback() -> bool {
    false
}

/// UMEM is never registered by this module.
#[inline]
pub const fn umem_registered() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::errors::QdnfError;

    #[test]
    fn open_fails_closed_platform_unsupported() {
        assert_eq!(
            AfXdp::open(b"eth0").unwrap_err(),
            QdnfError::PlatformUnsupported
        );
        assert_eq!(
            open_af_xdp(b"lo").unwrap_err(),
            QdnfError::PlatformUnsupported
        );
        assert!(!silent_zero_copy_fallback());
        assert!(!umem_registered());
    }

    #[test]
    fn open_never_ok_and_never_fake_zero_copy() {
        let names: [&[u8]; 3] = [b"eth0", b"enp0s1", b"wlan0"];
        let mut i = 0usize;
        while i < names.len() {
            match AfXdp::open(names[i]) {
                Err(QdnfError::PlatformUnsupported) => {}
                other => panic!("AF_XDP must fail closed, got {other:?}"),
            }
            i += 1;
        }
        assert!(!silent_zero_copy_fallback());
        assert!(!umem_registered());
    }

    #[test]
    fn invalid_ifname_is_range_not_success() {
        assert_eq!(AfXdp::open(b"").unwrap_err(), QdnfError::Range);
        assert_eq!(
            AfXdp::open(&[b'x'; 16]).unwrap_err(),
            QdnfError::Range
        );
        assert!(AfXdp::open(b"eth0").is_err());
    }
}
