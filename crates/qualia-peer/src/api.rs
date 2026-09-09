//! Application QPR host. Replaces the 64-byte two-peer demo as the public API.

use crate::{
    extra_identity_multiplies_host_budget, authorised_ipc_stream_exchange_in, pair_ipc_cells,
    HostAdmission, NativePeer, QdnfError, ScopeEpoch,
};

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

    /// Two separately constructed peers sized to this host’s `cell_bytes`.
    pub fn pair(
        &self,
        a: &[u8],
        b: &[u8],
        scope: ScopeEpoch,
        mtu: u16,
    ) -> Result<(NativePeer, NativePeer), QdnfError> {
        pair_ipc_cells(a, b, scope, mtu, self.cell_bytes)
    }

    /// Authorised protected exchange. Payloads up to min(4096, MTU-related cap).
    ///
    /// `cell_bytes` is charged on the host admission owner before the protected
    /// handshake+send/recv path runs. Extra identities cannot mint a second
    /// host budget. Denied, stale, and wrong-recipient traffic never reaches
    /// the application.
    pub fn exchange_protected(&self, payload: &[u8]) -> Result<usize, QdnfError> {
        let mut host = HostAdmission::new(self.cell_bytes)?;
        let profile = HostAdmission::profile_for_bytes(self.cell_bytes)?;
        let slot = host.admit_cell(profile, self.cell_bytes)?;
        let n = authorised_ipc_stream_exchange_in(payload, self.cell_bytes, 1280)?;
        host.release_cell(slot)?;
        let _ = extra_identity_multiplies_host_budget();
        Ok(n)
    }

    /// Thin wrapper over [`Self::exchange_protected`]. Keeps `b"hello-qpr"` (9).
    pub fn exchange(&self, payload: &[u8]) -> Result<usize, QdnfError> {
        self.exchange_protected(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        two_peer_ipc_exchange, PeerBuilder, PolicyOutcome, ServiceId, AUTHORISED_ROUNDTRIP_CAP,
        LinkId,
    };

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
    fn exchange_protected_roundtrip_cap() {
        let host = PeerHost::new(64 * 1024 * 1024).unwrap();
        let mut app = [0u8; AUTHORISED_ROUNDTRIP_CAP];
        let mut i = 0usize;
        while i < app.len() {
            app[i] = (i as u8).wrapping_add(0x3C);
            i = i.saturating_add(1);
        }
        assert_eq!(
            host.exchange_protected(&app).unwrap(),
            AUTHORISED_ROUNDTRIP_CAP
        );
    }

    #[test]
    fn vertical_64_demo_still_works() {
        assert_eq!(two_peer_ipc_exchange(b"qsync-hello").unwrap(), 11);
        assert_eq!(
            two_peer_ipc_exchange(&[0u8; 65]).unwrap_err(),
            QdnfError::Capacity
        );
    }

    #[test]
    fn allow_enum_does_not_open_session() {
        let _ = PolicyOutcome::Allow;
        let host = PeerHost::new(64 * 1024 * 1024).unwrap();
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, _b) = host.pair(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let service = ServiceId::from_iri(b"q42:QSync/1").unwrap();
        assert_eq!(a.open_session(service), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn send_without_announce_is_unauthorized() {
        let host = PeerHost::new(64 * 1024 * 1024).unwrap();
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, b) = host.pair(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        assert_eq!(
            a.send_stream(&b.locator(), b"x"),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn pair_uses_host_cell_bytes_and_builder_rejects_zero() {
        assert_eq!(PeerHost::new(0), Err(QdnfError::Capacity));
        assert_eq!(
            PeerBuilder::new(b"did:q42:a", LinkId::ZERO, 0).unwrap_err(),
            QdnfError::Capacity
        );
        let host = PeerHost::new(4096).unwrap();
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (a, _b) = host.pair(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        assert_eq!(a.remaining_host_bytes(), 4096);
    }
}
