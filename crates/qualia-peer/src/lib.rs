//! Qualia Peer Runtime facade.
//!
//! This crate is the application replacement for libp2p. It does not import
//! `libp2p` and does not enable core-db `libp2p-compat` (RT-03.07 partial).
//! GPU/LLM still ride along via `gpu-runtime` until wgpu call sites are gated;
//! `qualia-core-db --no-default-features --features qdnf` does not yet compile.

pub use qualia_core_db::net::peer::host::{
    native_ipc_stream_exchange, ControllerIdentity, NativePeer, ServiceId,
};
pub use qualia_core_db::net::peer::runtime::{
    LeaseTable, ReservationLedger, ResourceBudget,
};
pub use qualia_core_db::net::qdnf::authority::{PolicyOutcome, Plane};
pub use qualia_core_db::net::qdnf::bearer::{ipc_pair, Bearer, IpcEndpoint};
pub use qualia_core_db::net::qdnf::errors::QdnfError;
pub use qualia_core_db::net::qdnf::session::{SessionBinding, SessionState};
pub use qualia_core_db::net::qdnf::types::{ObservedLocator, OperationId, ScopeEpoch};
pub use qualia_core_db::net::qdnf::vertical::two_peer_ipc_exchange;

/// Host builder for a bounded native peer cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerHost {
    pub cell_bytes: u64,
}

impl PeerHost {
    pub const ORDINARY_CELL_CEILING: u64 = 512 * 1024 * 1024;

    pub fn new(cell_bytes: u64) -> Result<Self, QdnfError> {
        if cell_bytes == 0 || cell_bytes > Self::ORDINARY_CELL_CEILING {
            return Err(QdnfError::Capacity);
        }
        Ok(Self { cell_bytes })
    }

    /// Native two-peer exchange. Replaces a libp2p Swarm request-response round trip.
    pub fn exchange(&self, payload: &[u8]) -> Result<usize, QdnfError> {
        native_ipc_stream_exchange(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_cell_cannot_exceed_512_mib() {
        assert_eq!(
            PeerHost::new(PeerHost::ORDINARY_CELL_CEILING + 1),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn native_facade_exchanges_without_ip() {
        let host = PeerHost::new(64 * 1024 * 1024).unwrap();
        assert_eq!(host.exchange(b"hello-qpr").unwrap(), 9);
    }
}
