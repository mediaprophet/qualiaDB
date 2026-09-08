//! Raw Ethernet backend. Linux AF_PACKET is privilege-gated; others fail closed.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::contract::{Bearer, RecvMeta};

/// Development EtherType for QDNF (not IANA-assigned). Production must use an
/// assigned EtherType or an administered encapsulation.
pub const DEV_ETHERTYPE: u16 = 0x4242;

#[derive(Debug)]
pub struct RawEthernet {
    _private: (),
}

impl RawEthernet {
    pub fn open(_interface: &[u8]) -> Result<Self, QdnfError> {
        Err(QdnfError::PlatformUnsupported)
    }
}

impl Bearer for RawEthernet {
    fn profile(&self) -> BearerProfile {
        BearerProfile::RawEthernetV1
    }

    fn mtu(&self) -> u16 {
        1500
    }

    fn scope(&self) -> ScopeEpoch {
        ScopeEpoch { scope: 0, epoch: 0 }
    }

    fn send(&mut self, _dest: &ObservedLocator, _frame: &[u8]) -> Result<usize, QdnfError> {
        Err(QdnfError::PlatformUnsupported)
    }

    fn recv(&mut self, _out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        Err(QdnfError::PlatformUnsupported)
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        Ok(())
    }
}

/// Encode a Ethernet-II header around a QFrame for capture tests.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_is_explicitly_unsupported_without_raw_privilege() {
        assert_eq!(
            RawEthernet::open(b"eth0").unwrap_err(),
            QdnfError::PlatformUnsupported
        );
    }

    #[test]
    fn ethernet_header_uses_dev_ethertype() {
        let mut out = [0u8; 32];
        let n = encapsulate_ethernet(&[0xff; 6], &[1, 2, 3, 4, 5, 6], &[0x51, 0x44], &mut out)
            .unwrap();
        assert_eq!(n, 16);
        assert_eq!(&out[12..14], &DEV_ETHERTYPE.to_be_bytes());
    }
}
