//! Bounded MTU negotiation for QDNF bearers.
//!
//! Negotiated MTU is `min(local, remote)` clamped to
//! `[MIN_QDNF_MTU, MAX_QDNF_MTU]`. Jumbo advertisements are reduced; too-small
//! advertisements fail closed. Oversize payload without fragmentation is
//! [`QdnfError::Capacity`].

use crate::net::qdnf::errors::QdnfError;

/// Conservative floor (IPv6 minimum). Below this, PQ handshake fragments
/// cannot be negotiated honestly.
pub const MIN_QDNF_MTU: u16 = 1280;
/// Ethernet-II payload ceiling without jumbo frames.
pub const MAX_QDNF_MTU: u16 = 1500;
/// Default advertised MTU when the adapter has not reported one.
pub const DEFAULT_QDNF_MTU: u16 = MIN_QDNF_MTU;

pub fn negotiate_mtu(local: u16, remote: u16) -> Result<u16, QdnfError> {
    if local == 0 || remote == 0 {
        return Err(QdnfError::Range);
    }
    let n = local.min(remote);
    if n < MIN_QDNF_MTU {
        return Err(QdnfError::Range);
    }
    Ok(n.min(MAX_QDNF_MTU))
}

/// Oversize application/handshake bytes on a single unfragmented frame.
pub fn unfragmented_fit(payload_len: usize, mtu: u16) -> Result<(), QdnfError> {
    if mtu == 0 {
        return Err(QdnfError::Range);
    }
    if payload_len > mtu as usize {
        Err(QdnfError::Capacity)
    } else {
        Ok(())
    }
}

/// Interface loss: closed adapter vs platform that never had a bearer.
pub fn interface_loss_error(platform_unsupported: bool) -> QdnfError {
    if platform_unsupported {
        QdnfError::PlatformUnsupported
    } else {
        QdnfError::Closed
    }
}

/// Reconnect after loss never reuses a prior fragment admission.
pub const fn reconnect_requires_new_admission() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiate_takes_min_within_bounds() {
        assert_eq!(negotiate_mtu(1500, 1280).unwrap(), 1280);
        assert_eq!(negotiate_mtu(9000, 9000).unwrap(), MAX_QDNF_MTU);
        assert_eq!(negotiate_mtu(0, 1500), Err(QdnfError::Range));
        assert_eq!(negotiate_mtu(512, 512), Err(QdnfError::Range));
    }

    #[test]
    fn oversize_without_fragment_is_capacity() {
        assert_eq!(unfragmented_fit(1501, 1500), Err(QdnfError::Capacity));
        assert_eq!(unfragmented_fit(1280, 1280), Ok(()));
    }

    #[test]
    fn interface_loss_is_closed_or_unsupported() {
        assert_eq!(interface_loss_error(false), QdnfError::Closed);
        assert_eq!(interface_loss_error(true), QdnfError::PlatformUnsupported);
        assert!(reconnect_requires_new_admission());
    }
}
