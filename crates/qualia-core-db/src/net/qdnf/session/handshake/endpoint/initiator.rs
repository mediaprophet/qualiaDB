use super::*;
use super::{
    common::{Abort, Common, guarded},
    types::fresh_nonce,
    wire,
};
use crate::crypto::network::{
    authority::PinAuthority, entropy::EphemeralKeyLease, kem::MlKem768Secret,
    pq_handshake::encode_initiator_share, share_encoding::decode_responder_share,
    types::KeyPurpose,
};
use crate::net::qdnf::{
    bearer::contract::Bearer,
    crypto::{
        finished::{finished_mac, verify_finished},
        handshake::{
            ResponderShare,
            shares::leased::{initiator_complete_leased, initiator_share_leased},
        },
        schedule::endpoint::EndpointSecrets,
    },
    types::StrongDigest,
};
use crate::specialized_libs::cryptographic_library::ManagedPairedSigning;
pub struct Initiator<'a> {
    pub(super) common: Common<'a>,
    ephemeral: Option<EphemeralKeyLease>,
    kem: Option<MlKem768Secret>,
    secrets: Option<EndpointSecrets>,
    peer_tag: StrongDigest,
    peer_transcript: StrongDigest,
}
impl<'a> Initiator<'a> {
    pub fn new(
        authority: &'a EndpointAuthority<'a>,
        reservation: EndpointReservation<'a>,
        workspace: &'a mut EndpointWorkspace,
        route: Route,
        peer: PeerTrust,
        clock: &dyn Fn() -> u64,
    ) -> Result<Self, E> {
        if authority.role() != Role::Initiator {
            return Err(E::Unauthorized);
        }
        authority.current(clock, |_, _| Ok(()))?;
        let ephemeral = EphemeralKeyLease::generate(KeyPurpose::QSessionEphemeral)?;
        let (kem, pk) = MlKem768Secret::generate()?;
        let share = initiator_share_leased(&ephemeral, &pk)?;
        let mut common = Common::new(
            authority,
            reservation,
            workspace,
            route,
            peer,
            Phase::ClientProof,
        )?;
        common.client_nonce = fresh_nonce()?;
        wire::start(&mut common.workspace.io, 1)?;
        let mut p = wire::START;
        common.intent.encode(&mut common.workspace.io[p..])?;
        p += intent::INTENT_BYTES;
        common.workspace.io[p..p + 32].copy_from_slice(&common.client_nonce);
        p += 32;
        let offer = wire::offer(route.mtu);
        common.workspace.io[p..p + 8].copy_from_slice(&offer);
        p += 8;
        encode_initiator_share(&share, &mut common.workspace.io[p..p + 1216])?;
        common.transcript = transcript::client(
            common.intent,
            &common.client_nonce,
            &offer,
            &common.workspace.io[p..p + 1216],
        )?;
        Ok(Self {
            common,
            ephemeral: Some(ephemeral),
            kem: Some(kem),
            secrets: None,
            peer_tag: StrongDigest::ZERO,
            peer_transcript: StrongDigest::ZERO,
        })
    }
    pub fn phase(&self) -> Phase {
        self.common.phase
    }
    pub fn signing_commitment(&self) -> Result<SigningCommitment, E> {
        if self.phase() != Phase::ClientProof {
            return Err(E::Unauthorized);
        }
        Ok(self.common.commitment())
    }
    pub fn sign_client(
        &mut self,
        capability: ManagedPairedSigning<'_>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), E> {
        guarded(self, |s| {
            if s.phase() != Phase::ClientProof {
                return Err(E::Replay);
            }
            s.common.check(pins, clock)?;
            let n = s.common.sign(wire::CLIENT_END, capability)?;
            s.common.check(pins, clock)?;
            s.common.client_wire =
                crate::crypto::network::digest::sha384(&s.common.workspace.io[..n]);
            s.common
                .append_proof(b"client-proof", wire::CLIENT_END, n, 0)?;
            s.common.cursor.queue(s.common.workspace, n, 1280, 1)?;
            s.common.phase = Phase::ClientSending;
            Ok(())
        })
    }
    /// One actual frame. A successful final send is bearer acceptance, NOT an
    /// acknowledgement that the remote endpoint installed its session keys.
    pub fn send_step<B: Bearer>(
        &mut self,
        b: &mut B,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<bool, E> {
        guarded(self, |s| {
            let phase = s.phase();
            if !matches!(
                phase,
                Phase::ClientSending | Phase::InitiatorFinishedSending
            ) {
                return Err(E::Unauthorized);
            }
            let done = s.common.send(pins, clock, b)?;
            if done {
                s.common.phase = if phase == Phase::ClientSending {
                    Phase::AwaitingServer
                } else {
                    Phase::Ready
                };
            }
            Ok(done)
        })
    }
    pub fn receive_step<B: Bearer>(
        &mut self,
        b: &mut B,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<bool, E> {
        guarded(self, |s| {
            if s.phase() != Phase::AwaitingServer {
                return Err(E::Unauthorized);
            }
            s.common.check(pins, clock)?;
            // Server flight uses floor MTU too: selection is authenticated only after reassembly.
            let Some(n) =
                s.common
                    .cursor
                    .receive(s.common.workspace, s.common.route, b, 2, 1280)?
            else {
                return Ok(false);
            };
            s.accept_server(n, pins, clock)?;
            Ok(true)
        })
    }
    fn accept_server(
        &mut self,
        n: usize,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), E> {
        let c = &mut self.common;
        wire::check(&c.workspace.io[..n], 2)?;
        wire::envelope(&c.workspace.io[..n], wire::SERVER_END, 48)?;
        let mut p = wire::START;
        if c.workspace.io[p..p + 48] != c.client_wire.0 {
            return Err(E::Unauthorized);
        }
        p += 48;
        if c.workspace.io[p..p + 32] != c.client_nonce {
            return Err(E::Unauthorized);
        }
        p += 32;
        c.server_nonce.copy_from_slice(&c.workspace.io[p..p + 32]);
        if c.server_nonce == [0; 32] || c.server_nonce == c.client_nonce {
            return Err(E::Unauthorized);
        }
        p += 32;
        let selection: [u8; 10] = c.workspace.io[p..p + 10].try_into().unwrap();
        p += 10;
        c.selected = wire::read_selection(&selection, c.route.mtu)?;
        c.transcript.append(b"responder-nonce", &c.server_nonce)?;
        c.transcript.append(b"server-selection", &selection)?;
        c.transcript
            .append(b"responder-share", &c.workspace.io[p..p + 1120])?;
        let (ct, pk) = decode_responder_share(&c.workspace.io[p..p + 1120])?;
        c.verify_peer(wire::SERVER_END, n, 48, pins, clock)?;
        c.append_proof(b"server-proof", wire::SERVER_END, n, 48)?;
        let shared = initiator_complete_leased(
            self.kem.as_ref().ok_or(E::Closed)?,
            self.ephemeral.as_ref().ok_or(E::Closed)?,
            &ResponderShare {
                ml_kem_ct: ct,
                x25519_pk: pk,
            },
        )?;
        let secrets = EndpointSecrets::derive(&shared, &c.transcript)?;
        self.kem.take();
        self.ephemeral.take();
        self.peer_transcript = c.transcript.digest();
        self.peer_tag = StrongDigest::from_bytes(&c.workspace.io[n - 48..n])?;
        verify_finished(
            secrets.finished_key(false)?,
            &self.peer_transcript,
            false,
            &self.peer_tag,
        )?;
        c.transcript
            .append(b"responder-finished", &self.peer_tag.0)?;
        let t1 = c.transcript.digest();
        let tag = finished_mac(secrets.finished_key(true)?, &t1, true)?;
        c.transcript.append(b"initiator-finished", &tag.0)?;
        c.check(pins, clock)?;
        wire::start(&mut c.workspace.io, 3)?;
        let mut p = wire::START;
        c.workspace.io[p..p + 32].copy_from_slice(&c.client_nonce);
        p += 32;
        c.workspace.io[p..p + 32].copy_from_slice(&c.server_nonce);
        p += 32;
        c.workspace.io[p..p + 48].copy_from_slice(&t1.0);
        p += 48;
        c.workspace.io[p..p + 48].copy_from_slice(&tag.0);
        c.cursor.queue(c.workspace, wire::FIN_LEN, c.selected, 3)?;
        self.secrets = Some(secrets);
        c.phase = Phase::InitiatorFinishedSending;
        Ok(())
    }
    pub fn install(
        &mut self,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<EstablishedSession<'a>, E> {
        guarded(self, |s| {
            let result = established::install(
                &mut s.common,
                s.secrets.as_ref().ok_or(E::Closed)?,
                s.peer_tag,
                s.peer_transcript,
                pins,
                clock,
            )?;
            s.secrets.take();
            Ok(result)
        })
    }
    pub fn cancel(&mut self) {
        self.abort();
    }
}
impl Abort for Initiator<'_> {
    fn abort(&mut self) {
        self.ephemeral.take();
        self.kem.take();
        self.secrets.take();
        self.common.clear();
    }
}
