//! QLink discovery beacons. Private beacons carry no stable person identifier.

use crate::crypto::network::kdf::hmac_sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::LinkId;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryMode {
    PrivatePairwise = 1,
    ClosedGroup = 2,
    PublicService = 3,
    Manual = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Beacon {
    pub mode: DiscoveryMode,
    pub tag: [u8; 16],
    pub link_id: LinkId,
    pub epoch: u64,
    pub expiry_unix: u64,
    pub mtu: u16,
}

impl Beacon {
    pub const WIRE_LEN: usize = 1 + 16 + 16 + 8 + 8 + 2;

    pub fn encode(&self, out: &mut [u8]) -> Result<usize, QdnfError> {
        if out.len() < Self::WIRE_LEN {
            return Err(QdnfError::Capacity);
        }
        out[0] = self.mode as u8;
        out[1..17].copy_from_slice(&self.tag);
        out[17..33].copy_from_slice(&self.link_id.0);
        out[33..41].copy_from_slice(&self.epoch.to_be_bytes());
        out[41..49].copy_from_slice(&self.expiry_unix.to_be_bytes());
        out[49..51].copy_from_slice(&self.mtu.to_be_bytes());
        Ok(Self::WIRE_LEN)
    }

    pub fn decode(src: &[u8]) -> Result<Self, QdnfError> {
        if src.len() < Self::WIRE_LEN {
            return Err(QdnfError::Truncated);
        }
        let mode = match src[0] {
            1 => DiscoveryMode::PrivatePairwise,
            2 => DiscoveryMode::ClosedGroup,
            3 => DiscoveryMode::PublicService,
            4 => DiscoveryMode::Manual,
            _ => return Err(QdnfError::Unsupported),
        };
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&src[1..17]);
        let mut link = LinkId::ZERO;
        link.0.copy_from_slice(&src[17..33]);
        Ok(Self {
            mode,
            tag,
            link_id: link,
            epoch: u64::from_be_bytes(src[33..41].try_into().unwrap()),
            expiry_unix: u64::from_be_bytes(src[41..49].try_into().unwrap()),
            mtu: u16::from_be_bytes([src[49], src[50]]),
        })
    }
}

/// Rotating private rendezvous tag from a relationship secret.
pub fn rotating_tag(secret: &[u8], epoch: u64, out: &mut [u8; 16]) -> Result<(), QdnfError> {
    let mut mac = [0u8; 48];
    let mut info = [0u8; 16];
    info[..8].copy_from_slice(b"qdnf-tag");
    info[8..].copy_from_slice(&epoch.to_be_bytes());
    hmac_sha384(secret, &info, &mut mac)?;
    out.copy_from_slice(&mac[..16]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_beacon_has_no_did_field() {
        let beacon = Beacon {
            mode: DiscoveryMode::PrivatePairwise,
            tag: [1u8; 16],
            link_id: LinkId([2u8; 16]),
            epoch: 3,
            expiry_unix: 10,
            mtu: 1280,
        };
        let mut buf = [0u8; 64];
        let n = beacon.encode(&mut buf).unwrap();
        let decoded = Beacon::decode(&buf[..n]).unwrap();
        assert_eq!(decoded.mode, DiscoveryMode::PrivatePairwise);
        assert_eq!(decoded.mtu, 1280);
    }
}
