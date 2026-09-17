//! Couple extra NativePeer paths to [`PathCcTable`]. Shared bottleneck by default.

use super::driver::SealedFrame;
use super::NativePeer;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::paths::{mutate_path, PathHandle, PathState};
use crate::net::qdnf::types::ObservedLocator;

impl NativePeer {
    fn path_race_generation(&self) -> u64 {
        let d = self.identity.digest().0;
        let mut seed = [0u8; 8];
        seed.copy_from_slice(&d[..8]);
        let n = u64::from_le_bytes(seed);
        if n == 0 || n == u64::MAX {
            1
        } else {
            n
        }
    }

    /// Default is shared: extra paths do not multiply the bottleneck window.
    #[inline]
    pub fn independent_bottleneck(&self) -> bool {
        self.sessions.path_cc.independent_bottleneck
    }

    pub fn set_independent_bottleneck(&mut self, independent: bool) {
        self.sessions.path_cc.independent_bottleneck = independent;
    }

    pub fn set_path_cc_window(&mut self, window_bytes: u64) {
        self.sessions.path_cc.shared.window_bytes = window_bytes;
    }

    #[inline]
    pub fn path_cc_in_flight(&self) -> u64 {
        self.sessions.path_cc.total_in_flight()
    }

    /// Open an extra path and attach it to this peer's congestion table.
    pub fn open_extra_path(&mut self) -> Result<PathHandle, QdnfError> {
        let id = self
            .sessions
            .paths
            .start_race(self.path_race_generation())?;
        let h = self.sessions.paths.handle_of(id)?;
        self.sessions.paths.attest_reachability(h)?;
        let h = mutate_path(&mut self.sessions.paths, h, PathState::Active)?;
        self.sessions.path_cc.attach(h)?;
        Ok(h)
    }

    pub(super) fn ensure_primary_path(&mut self) -> Result<PathHandle, QdnfError> {
        if let Some(h) = self.sessions.primary_path {
            let _ = self.sessions.path_cc.path_rtt(h)?;
            return Ok(h);
        }
        let h = self.open_extra_path()?;
        self.sessions.primary_path = Some(h);
        Ok(h)
    }

    pub fn send_protected_on_path(
        &mut self,
        path: PathHandle,
        dest: &ObservedLocator,
        payload: &[u8],
    ) -> Result<SealedFrame, QdnfError> {
        let h = self.require_primary()?;
        self.send_protected_with_path(h, path, dest, payload)
    }
}

#[cfg(test)]
mod native_peer {
    use super::*;
    use crate::net::peer::host::session_table::SessionHandle;
    use crate::net::qdnf::authority::{
        binding_for_controllers, AuthorityOwner, ContactState, InstalledSessionKeys,
    };
    use crate::net::qdnf::types::{Generation, ScopeEpoch};

    fn activate(
        peer: &mut NativePeer,
        local: &[u8],
        remote: &[u8],
        tag: u8,
        now: u64,
    ) -> SessionHandle {
        let mut owner = AuthorityOwner::new();
        let binding = binding_for_controllers(local, remote, b"q42:QSync/1", &[b'p', tag]).unwrap();
        let (_cred, _contact, handle) = owner
            .install_grant(binding, now, now.saturating_add(3600), ContactState::Active)
            .unwrap();
        let permit = owner.issue_permit(handle, binding, now, 4096).unwrap();
        let mut send = [tag.saturating_add(1); 32];
        let mut recv = [tag.saturating_add(80); 32];
        send[31] = 1;
        recv[31] = 2;
        let keys = InstalledSessionKeys::new(Generation(1), send, recv, true).unwrap();
        peer.activate_protected(permit, keys, now).unwrap()
    }

    fn pair_ready() -> (NativePeer, NativePeer) {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let now = 1_700_000_000u64;
        a.announce(&b.locator(), now).unwrap();
        let _ = b.accept_announce().unwrap();
        b.announce(&a.locator(), now).unwrap();
        let _ = a.accept_announce().unwrap();
        let _ = activate(&mut a, b"did:q42:a", b"did:q42:b", 1, now);
        (a, b)
    }

    #[test]
    fn send_path_cc_shared_bottleneck_two_extra_paths() {
        let (mut a, b) = pair_ready();
        assert!(!a.independent_bottleneck());
        a.set_path_cc_window(500);
        let p1 = a.open_extra_path().unwrap();
        let p2 = a.open_extra_path().unwrap();
        assert_ne!(p1, p2);
        let dest = b.locator();
        let body = [0x11u8; 500];
        a.send_protected_on_path(p1, &dest, &body).unwrap();
        assert_eq!(a.path_cc_in_flight(), 500);
        assert_eq!(
            a.send_protected_on_path(p2, &dest, &[0x22]).unwrap_err(),
            QdnfError::BudgetExhausted
        );
        assert_eq!(a.path_cc_in_flight(), 500);
        assert_eq!(
            a.send_protected_on_path(p2, &dest, &[0u8; 4097])
                .unwrap_err(),
            QdnfError::Capacity
        );
    }

    #[test]
    fn send_path_cc_shared_bottleneck_independent_windows() {
        let (mut a, b) = pair_ready();
        a.set_path_cc_window(500);
        a.set_independent_bottleneck(true);
        assert!(a.independent_bottleneck());
        let p1 = a.open_extra_path().unwrap();
        let p2 = a.open_extra_path().unwrap();
        let dest = b.locator();
        let body = [0x33u8; 500];
        a.send_protected_on_path(p1, &dest, &body).unwrap();
        a.send_protected_on_path(p2, &dest, &body).unwrap();
        assert_eq!(a.path_cc_in_flight(), 1000);
    }

    #[test]
    fn send_path_cc_shared_bottleneck_unknown_path_id() {
        let (mut a, mut b) = pair_ready();
        a.set_path_cc_window(500);
        let p1 = a.open_extra_path().unwrap();
        let dest = b.locator();
        a.send_protected_on_path(p1, &dest, &[0x44; 40]).unwrap();
        let before = a.path_cc_in_flight();
        let id = a
            .sessions
            .paths
            .start_race(a.path_race_generation())
            .unwrap();
        let unknown = a.sessions.paths.handle_of(id).unwrap();
        assert_eq!(
            a.send_protected_on_path(unknown, &dest, b"x").unwrap_err(),
            QdnfError::Closed
        );
        assert_eq!(a.path_cc_in_flight(), before);
        let foreign = b.open_extra_path().unwrap();
        assert_eq!(
            a.send_protected_on_path(foreign, &dest, b"y").unwrap_err(),
            QdnfError::StaleGeneration
        );
        assert_eq!(a.path_cc_in_flight(), before);
    }
}
