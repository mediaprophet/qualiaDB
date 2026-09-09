//! Qualia Peer Runtime facade.
//!
//! This crate is the application replacement for libp2p. It does not import
//! `libp2p`. It enables core-db `qdnf` only (no `libp2p-compat`, `gpu-runtime`,
//! `wgsl-forge`, `privacy-he`, `zk-culling`, or `profile_target_1024`).
//! Default `qualia-core-db` still has those features. Packages remain open.

pub use qualia_core_db::net::peer::cells::{
    extra_identity_multiplies_host_budget, HostAdmission,
};
pub use qualia_core_db::net::peer::host::{
    authorised_ipc_stream_exchange, native_ipc_stream_exchange, ControllerIdentity, NativePeer,
    ServiceId,
};
pub use qualia_core_db::net::peer::runtime::{
    LeaseTable, ReservationHandle, ReservationLedger, ResourceBudget,
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
    ///
    /// `cell_bytes` is charged on the host admission owner before the protected
    /// exchange runs. Extra identities cannot mint a second host budget.
    pub fn exchange(&self, payload: &[u8]) -> Result<usize, QdnfError> {
        let mut host = HostAdmission::new(self.cell_bytes)?;
        let profile = HostAdmission::profile_for_bytes(self.cell_bytes)?;
        let slot = host.admit_cell(profile, self.cell_bytes)?;
        let n = native_ipc_stream_exchange(payload)?;
        host.release_cell(slot)?;
        let _ = extra_identity_multiplies_host_budget();
        Ok(n)
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

    #[test]
    fn allow_enum_does_not_open_session() {
        let _ = PolicyOutcome::Allow;
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, _b) =
            NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let service = ServiceId::from_iri(b"q42:QSync/1").unwrap();
        assert_eq!(a.open_session(service), Err(QdnfError::Unauthorized));
    }
}
