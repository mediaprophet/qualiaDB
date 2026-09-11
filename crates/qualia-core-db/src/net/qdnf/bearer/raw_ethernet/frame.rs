//! Ethernet-II header codec for QDNF development frames.
//!
//! EtherType `0x4242` is a development value, not an IANA assignment.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::ObservedLocator;

/// Development EtherType for QDNF (not IANA-assigned). Production must use an
/// assigned EtherType or an administered encapsulation.
pub const DEV_ETHERTYPE: u16 = 0x4242;

/// POSIX `IFNAMSIZ` (including the trailing NUL). Interface names occupy at
/// most `IFNAMSIZ - 1` bytes.
pub(crate) const IFNAMSIZ: usize = 16;

pub(crate) fn validate_ifname(interface: &[u8]) -> Result<(), QdnfError> {
    if interface.is_empty() || interface.len() >= IFNAMSIZ {
        Err(QdnfError::Range)
    } else {
        Ok(())
    }
}

pub(crate) fn mac_from_locator(loc: &ObservedLocator) -> Result<[u8; 6], QdnfError> {
    let src = loc.as_slice();
    if src.len() != 6 {
        return Err(QdnfError::Range);
    }
    let mut mac = [0u8; 6];
    mac.copy_from_slice(src);
    Ok(mac)
}

pub(crate) fn locator_from_mac(mac: &[u8; 6]) -> ObservedLocator {
    let mut loc = ObservedLocator::EMPTY;
    loc.bytes[..6].copy_from_slice(mac);
    loc.len = 6;
    loc
}

/// Encode an Ethernet-II header around a QFrame for capture tests and AF_PACKET.
pub fn encapsulate_ethernet(
    dst_mac: &[u8; 6],
    src_mac: &[u8; 6],
    frame: &[u8],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let total = 14usize.checked_add(frame.len()).ok_or(QdnfError::Range)?;
    if out.len() < total {
        return Err(QdnfError::Capacity);
    }
    out[..6].copy_from_slice(dst_mac);
    out[6..12].copy_from_slice(src_mac);
    out[12..14].copy_from_slice(&DEV_ETHERTYPE.to_be_bytes());
    out[14..total].copy_from_slice(frame);
    Ok(total)
}

/// Strip an Ethernet-II header. Observed locators are *not* taken from here;
/// callers copy `src_mac` into `RecvMeta` only when the adapter supplied it.
pub fn decapsulate_ethernet(
    frame: &[u8],
    out: &mut [u8],
) -> Result<(usize, [u8; 6], [u8; 6]), QdnfError> {
    if frame.len() < 14 {
        return Err(QdnfError::Truncated);
    }
    let mut dst_mac = [0u8; 6];
    let mut src_mac = [0u8; 6];
    dst_mac.copy_from_slice(&frame[0..6]);
    src_mac.copy_from_slice(&frame[6..12]);
    let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
    if ethertype != DEV_ETHERTYPE {
        return Err(QdnfError::Unsupported);
    }
    let payload = &frame[14..];
    if out.len() < payload.len() {
        return Err(QdnfError::Capacity);
    }
    out[..payload.len()].copy_from_slice(payload);
    Ok((payload.len(), src_mac, dst_mac))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ethernet_header_uses_dev_ethertype() {
        let mut out = [0u8; 32];
        let n =
            encapsulate_ethernet(&[0xff; 6], &[1, 2, 3, 4, 5, 6], &[0x51, 0x44], &mut out).unwrap();
        assert_eq!(n, 16);
        assert_eq!(&out[12..14], &DEV_ETHERTYPE.to_be_bytes());
    }

    #[test]
    fn decapsulate_rejects_wrong_ethertype() {
        let mut wire = [0u8; 32];
        let n = encapsulate_ethernet(&[0xff; 6], &[1, 2, 3, 4, 5, 6], &[0x51, 0x44], &mut wire)
            .unwrap();
        wire[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        let mut out = [0u8; 32];
        assert_eq!(
            decapsulate_ethernet(&wire[..n], &mut out),
            Err(QdnfError::Unsupported)
        );
    }

    #[test]
    fn decapsulate_truncated_header_is_truncated() {
        let mut out = [0u8; 32];
        assert_eq!(
            decapsulate_ethernet(&[0u8; 13], &mut out),
            Err(QdnfError::Truncated)
        );
    }
}
