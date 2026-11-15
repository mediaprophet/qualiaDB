use super::transport::Cursor;
use super::{
    E, EndpointAuthority, EndpointReservation, EndpointWorkspace, Intent, PeerTrust, Phase, Role,
    Route, SigningCommitment,
};
use crate::crypto::network::{
    authority::{AuthorityReceipt, AuthoritySource, PinAuthority},
    cose,
    digest::sha384,
    transcript::Transcript,
};
use crate::specialized_libs::cryptographic_library::ManagedPairedSigning;
pub(super) struct Common<'a> {
    pub intent: Intent,
    pub role: Role,
    pub authority: &'a EndpointAuthority<'a>,
    pub reservation: Option<EndpointReservation<'a>>,
    pub workspace: &'a mut EndpointWorkspace,
    pub route: Route,
    pub peer: PeerTrust,
    pub transcript: Transcript,
    pub receipt: Option<AuthorityReceipt>,
    pub peer_commitment: crate::net::qdnf::types::StrongDigest,
    pub phase: Phase,
    pub cursor: Cursor,
    pub client_nonce: [u8; 32],
    pub server_nonce: [u8; 32],
    pub client_wire: crate::net::qdnf::types::StrongDigest,
    pub selected: u16,
}
impl<'a> Common<'a> {
    pub fn new(
        authority: &'a EndpointAuthority<'a>,
        reservation: EndpointReservation<'a>,
        workspace: &'a mut EndpointWorkspace,
        route: Route,
        peer: PeerTrust,
        phase: Phase,
    ) -> Result<Self, E> {
        workspace.clear();
        workspace.reassembly.reconnect()?;
        Ok(Self {
            intent: authority.intent(),
            role: authority.role(),
            authority,
            reservation: Some(reservation),
            workspace,
            route,
            peer,
            transcript: Transcript::new(),
            receipt: None,
            peer_commitment: crate::net::qdnf::types::StrongDigest::ZERO,
            phase,
            cursor: Cursor::default(),
            client_nonce: [0; 32],
            server_nonce: [0; 32],
            client_wire: crate::net::qdnf::types::StrongDigest::ZERO,
            selected: 1280,
        })
    }
    pub fn commitment(&self) -> SigningCommitment {
        SigningCommitment {
            digest: self.transcript.digest(),
            key_ids: self.intent.keys(self.role),
            epoch: self.intent.epoch(self.role),
            audience: self.intent.peer(self.role),
            role: self.role,
        }
    }
    pub fn check(&self, pins: &PinAuthority, clock: &dyn Fn() -> u64) -> Result<(), E> {
        self.authority.current(clock, |_, now| {
            if let Some(r) = &self.receipt {
                pins.recheck(r, &self.peer_commitment.0, now)?;
            }
            Ok(())
        })
    }
    pub fn sign(
        &mut self,
        body_end: usize,
        capability: ManagedPairedSigning<'_>,
    ) -> Result<usize, E> {
        let commitment = self.commitment();
        let len = cose::sign_pair(
            &commitment.binding(),
            capability,
            &mut self.workspace.cose,
            &mut self.workspace.io[body_end + 2..],
        )?;
        self.workspace.io[body_end..body_end + 2].copy_from_slice(&(len as u16).to_be_bytes());
        Ok(body_end + 2 + len)
    }
    pub fn verify_peer(
        &mut self,
        body_end: usize,
        total: usize,
        tail: usize,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), E> {
        let peer_role = if self.role.initiator() {
            Role::Responder
        } else {
            Role::Initiator
        };
        let digest = self.transcript.digest();
        let commitment = SigningCommitment {
            digest,
            key_ids: self.intent.keys(peer_role),
            epoch: self.intent.epoch(peer_role),
            audience: self.intent.local(self.role),
            role: peer_role,
        };
        let proof = super::wire::envelope(&self.workspace.io[..total], body_end, tail)?;
        let receipt = pins.verify_source(
            AuthoritySource::LocalPin(self.peer.pin),
            proof,
            &commitment.binding(),
            clock(),
            &mut self.workspace.cose,
        )?;
        if receipt.principal() != self.intent.peer(self.role) {
            return Err(E::Unauthorized);
        }
        self.peer_commitment = digest;
        self.receipt = Some(receipt);
        self.check(pins, clock)
    }
    pub fn append_proof(
        &mut self,
        label: &[u8],
        body_end: usize,
        total: usize,
        tail: usize,
    ) -> Result<(), E> {
        let proof = super::wire::envelope(&self.workspace.io[..total], body_end, tail)?;
        self.transcript.append(label, &sha384(proof).0)
    }
    pub fn send<B: crate::net::qdnf::bearer::contract::Bearer>(
        &mut self,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        b: &mut B,
    ) -> Result<bool, E> {
        let receipt = &self.receipt;
        let digest = self.peer_commitment;
        let route = self.route;
        let cursor = &mut self.cursor;
        let w = &mut self.workspace;
        self.authority.after_verified(
            clock,
            |before| {
                if let Some(r) = receipt {
                    pins.recheck(r, &digest.0, before)?;
                }
                Ok(())
            },
            |_, now| {
                if let Some(r) = receipt {
                    pins.recheck(r, &digest.0, now)?;
                }
                cursor.send(w, route, b)
            },
        )
    }
    pub fn clear(&mut self) {
        self.phase = Phase::Failed;
        self.workspace.clear();
        self.reservation.take();
        self.receipt = None;
        self.transcript = Transcript::new();
    }
}
impl Drop for Common<'_> {
    fn drop(&mut self) {
        self.workspace.clear();
    }
}
pub(super) trait Abort {
    fn abort(&mut self);
}
pub(super) fn guarded<T: Abort, R>(
    inner: &mut T,
    call: impl FnOnce(&mut T) -> Result<R, E>,
) -> Result<R, E> {
    struct Guard<'a, T: Abort> {
        inner: &'a mut T,
        armed: bool,
    }
    impl<T: Abort> Drop for Guard<'_, T> {
        fn drop(&mut self) {
            if self.armed {
                self.inner.abort();
            }
        }
    }
    let mut guard = Guard { inner, armed: true };
    let result = call(guard.inner);
    if result.is_ok() || matches!(result, Err(E::WouldBlock)) {
        guard.armed = false;
    }
    result
}
