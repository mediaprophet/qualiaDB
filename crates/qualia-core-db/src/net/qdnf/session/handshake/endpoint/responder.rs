use super::*;
use super::{
    common::{Abort, Common, guarded},
    types::fresh_nonce,
    wire,
};
use crate::crypto::network::{
    authority::PinAuthority, entropy::EphemeralKeyLease, pq_handshake::decode_initiator_share,
    secret_lease::SecretLease, share_encoding::encode_responder_share, types::KeyPurpose,
};
use crate::net::qdnf::{
    bearer::contract::Bearer,
    crypto::{
        finished::{finished_mac, verify_finished},
        handshake::shares::leased::responder_complete_leased,
        schedule::endpoint::EndpointSecrets,
    },
    types::StrongDigest,
};
use crate::specialized_libs::cryptographic_library::ManagedPairedSigning;
pub struct Responder<'a> {
    pub(super) common: Common<'a>,
    ephemeral: Option<EphemeralKeyLease>,
    shared: Option<SecretLease>,
    secrets: Option<EndpointSecrets>,
    peer_tag: StrongDigest,
    peer_transcript: StrongDigest,
}
impl<'a> Responder<'a> {
    pub fn new(
        authority: &'a EndpointAuthority<'a>,
        reservation: EndpointReservation<'a>,
        workspace: &'a mut EndpointWorkspace,
        route: Route,
        peer: PeerTrust,
        clock: &dyn Fn() -> u64,
    ) -> Result<Self, E> {
        if authority.role() != Role::Responder {
            return Err(E::Unauthorized);
        }
        authority.current(clock, |_, _| Ok(()))?;
        Ok(Self {
            common: Common::new(
                authority,
                reservation,
                workspace,
                route,
                peer,
                Phase::AwaitingClient,
            )?,
            ephemeral: None,
            shared: None,
            secrets: None,
            peer_tag: StrongDigest::ZERO,
            peer_transcript: StrongDigest::ZERO,
        })
    }
    pub fn phase(&self) -> Phase {
        self.common.phase
    }
    pub fn signing_commitment(&self) -> Result<SigningCommitment, E> {
        if self.phase() != Phase::ServerProof {
            return Err(E::Unauthorized);
        }
        Ok(self.common.commitment())
    }
    pub fn receive_step<B: Bearer>(
        &mut self,
        b: &mut B,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<bool, E> {
        guarded(self, |s| {
            let phase = s.phase();
            let (kind, mtu) = match phase {
                Phase::AwaitingClient => (1, 1280),
                Phase::AwaitingFinished => (3, s.common.selected),
                _ => return Err(E::Unauthorized),
            };
            s.common.check(pins, clock)?;
            let Some(n) =
                s.common
                    .cursor
                    .receive(s.common.workspace, s.common.route, b, kind, mtu)?
            else {
                return Ok(false);
            };
            if phase == Phase::AwaitingClient {
                s.accept_client(n, pins, clock)?;
            } else {
                s.accept_finished(n, pins, clock)?;
            }
            Ok(true)
        })
    }
    fn accept_client(
        &mut self,
        n: usize,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), E> {
        let c = &mut self.common;
        wire::check(&c.workspace.io[..n], 1)?;
        wire::envelope(&c.workspace.io[..n], wire::CLIENT_END, 0)?;
        let mut p = wire::START;
        let mut expected = [0; intent::INTENT_BYTES];
        c.intent.encode(&mut expected)?;
        if c.workspace.io[p..p + intent::INTENT_BYTES] != expected {
            return Err(E::Unauthorized);
        }
        p += intent::INTENT_BYTES;
        c.client_nonce.copy_from_slice(&c.workspace.io[p..p + 32]);
        p += 32;
        if c.client_nonce == [0; 32] {
            return Err(E::Unauthorized);
        }
        let offer: [u8; 8] = c.workspace.io[p..p + 8].try_into().unwrap();
        p += 8;
        let remote = wire::read_offer(&offer)?;
        let share = decode_initiator_share(&c.workspace.io[p..p + 1216])?;
        c.transcript = transcript::client(
            c.intent,
            &c.client_nonce,
            &offer,
            &c.workspace.io[p..p + 1216],
        )?;
        c.verify_peer(wire::CLIENT_END, n, 0, pins, clock)?;
        c.append_proof(b"client-proof", wire::CLIENT_END, n, 0)?;
        c.client_wire = crate::crypto::network::digest::sha384(&c.workspace.io[..n]);
        c.selected = crate::net::qdnf::bearer::mtu::negotiate_mtu(remote, c.route.mtu)?;
        c.server_nonce = fresh_nonce()?;
        if c.server_nonce == c.client_nonce {
            return Err(E::EntropyFailure);
        }
        let ephemeral = EphemeralKeyLease::generate(KeyPurpose::QSessionEphemeral)?;
        let (reply, shared) = responder_complete_leased(&share, &ephemeral)?;
        wire::start(&mut c.workspace.io, 2)?;
        let mut p = wire::START;
        c.workspace.io[p..p + 48].copy_from_slice(&c.client_wire.0);
        p += 48;
        c.workspace.io[p..p + 32].copy_from_slice(&c.client_nonce);
        p += 32;
        c.workspace.io[p..p + 32].copy_from_slice(&c.server_nonce);
        p += 32;
        let selection = wire::selection(c.route.mtu, c.selected);
        c.workspace.io[p..p + 10].copy_from_slice(&selection);
        p += 10;
        encode_responder_share(
            &reply.ml_kem_ct,
            &reply.x25519_pk,
            &mut c.workspace.io[p..p + 1120],
        )?;
        c.transcript.append(b"responder-nonce", &c.server_nonce)?;
        c.transcript.append(b"server-selection", &selection)?;
        c.transcript
            .append(b"responder-share", &c.workspace.io[p..p + 1120])?;
        c.check(pins, clock)?;
        self.ephemeral = Some(ephemeral);
        self.shared = Some(shared);
        c.phase = Phase::ServerProof;
        Ok(())
    }
    pub fn sign_server(
        &mut self,
        capability: ManagedPairedSigning<'_>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), E> {
        guarded(self, |s| {
            if s.phase() != Phase::ServerProof {
                return Err(E::Replay);
            }
            let c = &mut s.common;
            c.check(pins, clock)?;
            let n = c.sign(wire::SERVER_END, capability)?;
            c.append_proof(b"server-proof", wire::SERVER_END, n, 0)?;
            let secrets =
                EndpointSecrets::derive(s.shared.as_ref().ok_or(E::Closed)?, &c.transcript)?;
            s.shared.take();
            s.ephemeral.take();
            let tag = finished_mac(secrets.finished_key(false)?, &c.transcript.digest(), false)?;
            c.workspace.io[n..n + 48].copy_from_slice(&tag.0);
            c.transcript.append(b"responder-finished", &tag.0)?;
            s.peer_transcript = c.transcript.digest();
            c.check(pins, clock)?;
            c.cursor.queue(c.workspace, n + 48, 1280, 2)?;
            s.secrets = Some(secrets);
            c.phase = Phase::ServerSending;
            Ok(())
        })
    }
    pub fn send_step<B: Bearer>(
        &mut self,
        b: &mut B,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<bool, E> {
        guarded(self, |s| {
            if s.phase() != Phase::ServerSending {
                return Err(E::Unauthorized);
            }
            let done = s.common.send(pins, clock, b)?;
            if done {
                s.common.phase = Phase::AwaitingFinished;
            }
            Ok(done)
        })
    }
    fn accept_finished(
        &mut self,
        n: usize,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), E> {
        let c = &mut self.common;
        if n != wire::FIN_LEN {
            return Err(E::Malformed);
        }
        wire::check(&c.workspace.io[..n], 3)?;
        let mut p = wire::START;
        if c.workspace.io[p..p + 32] != c.client_nonce {
            return Err(E::Unauthorized);
        }
        p += 32;
        if c.workspace.io[p..p + 32] != c.server_nonce {
            return Err(E::Unauthorized);
        }
        p += 32;
        if c.workspace.io[p..p + 48] != self.peer_transcript.0 {
            return Err(E::Unauthorized);
        }
        p += 48;
        self.peer_tag = StrongDigest::from_bytes(&c.workspace.io[p..p + 48])?;
        verify_finished(
            self.secrets.as_ref().ok_or(E::Closed)?.finished_key(true)?,
            &self.peer_transcript,
            true,
            &self.peer_tag,
        )?;
        c.transcript
            .append(b"initiator-finished", &self.peer_tag.0)?;
        c.check(pins, clock)?;
        c.phase = Phase::Ready;
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
impl Abort for Responder<'_> {
    fn abort(&mut self) {
        self.ephemeral.take();
        self.shared.take();
        self.secrets.take();
        self.common.clear();
    }
}
