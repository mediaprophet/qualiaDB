//! Bearer contracts. Observed locators come from the adapter, never the payload.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BearerCapabilities {
    pub profile: BearerProfile,
    pub mtu: u16,
    pub group_delivery: bool,
    pub ordered: bool,
    pub may_duplicate: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecvMeta {
    pub observed_source: ObservedLocator,
    pub scope: ScopeEpoch,
    pub mtu: u16,
}

pub trait Bearer {
    /// Bearer profile.
    fn profile(&self) -> BearerProfile;
    /// MTU.
    fn mtu(&self) -> u16;
    /// Scope epoch.
    fn scope(&self) -> ScopeEpoch;
    /// Sends a frame.
    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError>;
    /// Receives into `out`.
    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError>;
    /// Shuts down.
    fn shutdown(&mut self) -> Result<(), QdnfError>;
}

/// `Ok(())` when the frame fits the MTU.
pub fn check_frame_mtu(frame_len: usize, mtu: u16) -> Result<(), QdnfError> {
    if frame_len > mtu as usize {
        Err(QdnfError::Capacity)
    } else {
        Ok(())
    }
}
