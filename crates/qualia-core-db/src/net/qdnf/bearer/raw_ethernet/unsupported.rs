//! Windows, WASM, and other non-Linux hosts: explicit unsupported gate.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::super::contract::{Bearer, RecvMeta};
use super::frame::validate_ifname;
use super::EthernetEvidence;

#[derive(Debug)]
pub struct RawEthernet {
    _private: (),
}

impl RawEthernet {
    pub fn open(interface: &[u8]) -> Result<Self, QdnfError> {
        validate_ifname(interface)?;
        Err(QdnfError::PlatformUnsupported)
    }

    pub fn ethernet_evidence_level(&self) -> EthernetEvidence {
        EthernetEvidence::AfPacketAttempt
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
