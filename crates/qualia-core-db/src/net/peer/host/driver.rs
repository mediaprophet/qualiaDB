//! Protected send/receive/close. Offsets commit only after accepted transfer.

use super::session_table::SessionHandle;
use super::NativePeer;
use crate::net::peer::cells::pass_budget::{PassCharge, PassGuard};
use crate::net::peer::runtime::{CancelEpoch, ResourceBudget};
use crate::net::qdnf::authority::{ExecutionPermit, InstalledSessionKeys};
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{copy_payload, decode_frame, encode_frame, FrameHeader};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::session::{ProtectedAckSession, ProtectedRecv, SessionBinding};
use crate::net::qdnf::types::{LinkId, ObservedLocator};

const SESSION_CHARGE: ResourceBudget = ResourceBudget {
    bytes: 256,
    work: 1,
    io: 1,
};

/// Capture enough sealed bytes that [`require_protected`] can see ciphertext vs plaintext.
const SEALED_CAPTURE: usize = 4096 + 64;

/// Capture helper: last sealed frame bytes for the wire oracle (bounded).
#[derive(Clone, Copy, Debug)]
pub struct SealedFrame {
    pub(crate) bytes: [u8; SEALED_CAPTURE],
    pub(crate) len: u16,
}

impl SealedFrame {
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    fn from_sealed(sealed: &[u8]) -> Self {
        let mut out = Self {
            bytes: [0u8; SEALED_CAPTURE],
            len: 0,
        };
        let copy = sealed.len().min(out.bytes.len());
        out.bytes[..copy].copy_from_slice(&sealed[..copy]);
        out.len = copy as u16;
        out
    }
}

impl NativePeer {
    fn require_not_cancelled(&self) -> Result<(), QdnfError> {
        if self.cancelled {
            Err(QdnfError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn require_primary(&self) -> Result<SessionHandle, QdnfError> {
        self.primary.ok_or(QdnfError::Unauthorized)
    }

    /// Verified neighbor for `dest`. Missing adjacency is [`QdnfError::NoRoute`].
    fn require_verified_dest(&self, dest: &ObservedLocator) -> Result<LinkId, QdnfError> {
        let remote = self.remote_or_reject()?;
        let adj = self
            .neighbors
            .forwarding(remote)
            .ok_or(QdnfError::NoRoute)?;
        if adj.observed_peer.as_slice() != dest.as_slice() {
            return Err(QdnfError::Unauthorized);
        }
        Ok(remote)
    }

    pub fn activate_protected(
        &mut self,
        permit: ExecutionPermit,
        keys: InstalledSessionKeys,
        now_unix: u64,
    ) -> Result<SessionHandle, QdnfError> {
        if self.sessions.is_full() {
            return Err(QdnfError::Capacity);
        }
        let session = SessionBinding::from_permit(&permit, &keys, now_unix)?;
        let protection =
            ProtectedAckSession::from_keys(*keys.send_key(), *keys.recv_key(), keys.generation())?;
        let reservation = self.ledger.reserve(SESSION_CHARGE, true)?;
        let handle = self.sessions.insert(session, protection, reservation)?;
        if self.primary.is_none() {
            self.primary = Some(handle);
        }
        Ok(handle)
    }

    pub fn close_session(&mut self, h: SessionHandle) -> Result<(), QdnfError> {
        self.sessions.close(h, &mut self.ledger)?;
        if self.primary == Some(h) {
            self.primary = self.sessions.first_live();
        }
        Ok(())
    }

    #[inline]
    pub fn live_session_count(&self) -> usize {
        self.sessions.live_count()
    }

    #[inline]
    pub fn remaining_host_bytes(&self) -> u64 {
        self.ledger.host_remaining_bytes()
    }

    #[inline]
    pub fn primary_session(&self) -> Option<SessionHandle> {
        self.primary
    }

    /// Cancel in-flight send/recv. Does not commit application offsets.
    pub fn cancel(&mut self) -> Result<CancelEpoch, QdnfError> {
        self.cancel_epoch = self.cancel_epoch.next()?;
        self.cancelled = true;
        Ok(self.cancel_epoch)
    }

    /// Traffic-secret update. Does not mint a new permit or grant.
    pub fn rekey_protected(
        &mut self,
        table: &mut crate::net::qdnf::session::rekey::RekeyTable,
    ) -> Result<u64, QdnfError> {
        self.require_not_cancelled()?;
        let h = self.require_primary()?;
        let _session = self.sessions.binding(h)?;
        let protection = self.sessions.get_mut(h)?;
        let next = table.rotate()?;
        let (send, recv) = table.keys(next)?;
        protection.install_update(send, recv, crate::net::qdnf::types::Generation(next))?;
        Ok(next)
    }

    /// Charge one Sentinel network pass (accounting only; no 42 MiB map).
    pub fn network_pass(&mut self, charge: PassCharge) -> Result<u64, QdnfError> {
        let _ = self;
        let mut guard = PassGuard::enter(&charge)?;
        let n = guard.charged();
        guard.success();
        Ok(n)
    }

    pub fn send_stream(
        &mut self,
        dest: &ObservedLocator,
        payload: &[u8],
    ) -> Result<usize, QdnfError> {
        let _ = self.send_protected(dest, payload)?;
        Ok(payload.len())
    }

    pub fn recv_stream(&mut self, out: &mut [u8]) -> Result<usize, QdnfError> {
        self.recv_protected(out)
    }

    pub fn send_protected(
        &mut self,
        dest: &ObservedLocator,
        payload: &[u8],
    ) -> Result<SealedFrame, QdnfError> {
        let h = self.require_primary()?;
        self.send_protected_on(h, dest, payload)
    }

    pub fn send_protected_on(
        &mut self,
        h: SessionHandle,
        dest: &ObservedLocator,
        payload: &[u8],
    ) -> Result<SealedFrame, QdnfError> {
        self.require_not_cancelled()?;
        let session = self.sessions.binding(h)?;
        session.admit_application()?;
        let _remote = self.require_verified_dest(dest)?;
        const MAX_PROTECTED_PAYLOAD: usize = 4096;
        const PROTECTED_SEAL_CAP: usize = MAX_PROTECTED_PAYLOAD + 24;
        const PROTECTED_WIRE_CAP: usize = 80 + PROTECTED_SEAL_CAP;
        if payload.len() > MAX_PROTECTED_PAYLOAD {
            return Err(QdnfError::Capacity);
        }
        let mut pass = PassGuard::enter(&PassCharge {
            arena: 0,
            scratch: payload.len() as u64,
            crypto: 24,
            kernel: 80,
            pinned: 0,
        })?;
        let mut pt = [0u8; MAX_PROTECTED_PAYLOAD];
        pt[..payload.len()].copy_from_slice(payload);
        let mut sealed = [0u8; PROTECTED_SEAL_CAP];
        let n = {
            let protection = self.sessions.get_mut(h)?;
            protection.seal_tracked(b"qsession/stream", &mut pt[..payload.len()], &mut sealed)?
        };
        let mut header = FrameHeader::new(FrameType::SessionStream, NextProtocol::QSession);
        header.source_link_id = self.local_link;
        header.payload_len = n as u16;
        let mut wire = [0u8; PROTECTED_WIRE_CAP];
        let wn = encode_frame(&header, &sealed[..n], &mut wire)?;
        match self.bearer.send(dest, &wire[..wn]) {
            Ok(sent) if sent == wn => {
                let len = u16::try_from(payload.len()).map_err(|_| QdnfError::Capacity)?;
                self.sessions.commit_send_offset(h, len)?;
                pass.success();
                Ok(SealedFrame::from_sealed(&sealed[..n]))
            }
            Ok(_) => Err(QdnfError::WouldBlock),
            Err(e) => Err(e),
        }
    }

    pub fn recv_protected(&mut self, out: &mut [u8]) -> Result<usize, QdnfError> {
        let h = self.require_primary()?;
        self.recv_protected_on(h, out)
    }

    pub fn recv_protected_on(
        &mut self,
        h: SessionHandle,
        out: &mut [u8],
    ) -> Result<usize, QdnfError> {
        self.require_not_cancelled()?;
        let session = self.sessions.binding(h)?;
        session.admit_application()?;
        let _ = self.remote_or_reject()?;
        const MAX_PROTECTED_PAYLOAD: usize = 4096;
        const PROTECTED_SEAL_CAP: usize = MAX_PROTECTED_PAYLOAD + 24;
        const PROTECTED_WIRE_CAP: usize = 80 + PROTECTED_SEAL_CAP;
        let mut wire = [0u8; PROTECTED_WIRE_CAP];
        let (got, _meta) = self.bearer.recv(&mut wire)?;
        let (decoded, off, len) = decode_frame(&wire[..got])?;
        if decoded.frame_type != FrameType::SessionStream {
            return Err(QdnfError::Malformed);
        }
        let mut sealed = [0u8; PROTECTED_SEAL_CAP];
        let copied = copy_payload(&wire[..got], off, len, &mut sealed)?;
        let protection = self.sessions.get_mut(h)?;
        match protection.open_tracked(b"qsession/stream", &sealed[..copied], out)? {
            (ProtectedRecv::Delivered, n) => Ok(n),
            (ProtectedRecv::Duplicate, _) => Err(QdnfError::Replay),
        }
    }

    /// Poll until `deadline_unix`. Empty queue before the deadline is WouldBlock;
    /// at/after the deadline with nothing is Expired.
    pub fn poll_recv(
        &mut self,
        now_unix: u64,
        deadline_unix: u64,
        out: &mut [u8],
    ) -> Result<usize, QdnfError> {
        match self.recv_protected(out) {
            Ok(n) => Ok(n),
            Err(QdnfError::WouldBlock) => {
                if now_unix >= deadline_unix {
                    Err(QdnfError::Expired)
                } else {
                    Err(QdnfError::WouldBlock)
                }
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::cells::pass_budget::{PassCharge, SENTINEL_PASS_TOTAL};
    use crate::net::qdnf::types::ScopeEpoch;

    #[test]
    fn network_pass_charges_scratch_crypto_kernel() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, _b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let ok = PassCharge {
            scratch: 16,
            crypto: 16,
            kernel: 16,
            ..PassCharge::ZERO
        };
        assert_eq!(a.network_pass(ok).unwrap(), 48);
        let over = PassCharge {
            scratch: SENTINEL_PASS_TOTAL,
            crypto: 1,
            kernel: 1,
            ..PassCharge::ZERO
        };
        assert_eq!(a.network_pass(over).unwrap_err(), QdnfError::Capacity);
        let dest = crate::net::qdnf::types::ObservedLocator::from_slice(&[0x02]).unwrap();
        assert_eq!(
            a.send_protected(&dest, b"x").unwrap_err(),
            QdnfError::Unauthorized
        );
    }
}
