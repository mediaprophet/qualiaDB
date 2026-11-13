//! Installed capability only after local send and RECEIVED peer Finished.
use super::{common::Common, EndpointAuthority, EndpointReservation, Phase, Role, E};
use crate::crypto::network::{
    authority::{AuthorityReceipt, PinAuthority},
    types::KeyPurpose,
};
use crate::net::qdnf::{
    authority::InstalledSessionKeys, crypto::schedule::endpoint::EndpointSecrets,
    session::handshake::SessionBinding, types::StrongDigest,
};
mod protected_channel;
pub use protected_channel::{ChannelBuildError, ProtectedChannel};
pub struct EstablishedSession<'a> {
    authority: &'a EndpointAuthority<'a>,
    reservation: Option<EndpointReservation<'a>>,
    keys: Option<InstalledSessionKeys>,
    receipt: AuthorityReceipt,
    peer_commitment: StrongDigest,
    transcript: StrongDigest,
    role: Role,
    intent: super::Intent,
    route: super::Route,
    mtu: u16,
}
impl EstablishedSession<'_> {
    pub const fn transcript(&self) -> StrongDigest {
        self.transcript
    }
    pub const fn role(&self) -> Role {
        self.role
    }
    /// Synchronous host use under the SAME live authority and receipt. Borrowed
    /// key references cannot escape this closure. Trusted host code CAN copy
    /// exposed key bytes; this is not a confinement boundary against that host.
    /// No async IO/drain transfer is provided.
    pub fn with_current<T>(
        &mut self,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        use_keys: impl FnOnce(&SessionBinding, &InstalledSessionKeys) -> Result<T, E>,
    ) -> Result<T, E> {
        super::common::guarded(self, |s| {
            let keys = s.keys.as_ref().ok_or(E::Closed)?;
            s.authority.after_verified(
                clock,
                |before| {
                    pins.recheck(&s.receipt, &s.peer_commitment.0, before)?;
                    Ok(())
                },
                |permit, now| {
                    pins.recheck(&s.receipt, &s.peer_commitment.0, now)?;
                    let binding = SessionBinding::from_permit(permit, keys, now)?;
                    use_keys(&binding, keys)
                },
            )
        })
    }
}
pub(super) fn install<'a>(
    c: &mut Common<'a>,
    secrets: &EndpointSecrets,
    peer_tag: StrongDigest,
    peer_transcript: StrongDigest,
    pins: &PinAuthority,
    clock: &dyn Fn() -> u64,
) -> Result<EstablishedSession<'a>, E> {
    if c.phase != Phase::Ready {
        return Err(E::Unauthorized);
    }
    let transcript = c.transcript.digest();
    let traffic = secrets.traffic(transcript)?;
    let bytes = traffic.expose(KeyPurpose::QSessionAead)?;
    let i: &[u8; 32] = bytes[..32].try_into().map_err(|_| E::Malformed)?;
    let r: &[u8; 32] = bytes[32..].try_into().map_err(|_| E::Malformed)?;
    let (send, recv) = if c.role.initiator() { (i, r) } else { (r, i) };
    let peer_is_initiator = !c.role.initiator();
    let receipt = c.receipt.as_ref().ok_or(E::Unauthorized)?;
    let keys = c.authority.after_verified(
        clock,
        |before| {
            pins.recheck(receipt, &c.peer_commitment.0, before)?;
            crate::net::qdnf::crypto::finished::verify_finished(
                secrets.finished_key(peer_is_initiator)?,
                &peer_transcript,
                peer_is_initiator,
                &peer_tag,
            )
        },
        |permit, now| {
            pins.recheck(receipt, &c.peer_commitment.0, now)?;
            let keys = InstalledSessionKeys::from_received_finished(
                permit.identity_generation(),
                send,
                recv,
                secrets.finished_key(peer_is_initiator)?,
                &peer_transcript,
                peer_is_initiator,
                &peer_tag,
            )?;
            SessionBinding::from_permit(permit, &keys, now)?;
            Ok(keys)
        },
    )?;
    c.phase = Phase::Consumed;
    Ok(EstablishedSession {
        authority: c.authority,
        reservation: Some(c.reservation.take().ok_or(E::Closed)?),
        keys: Some(keys),
        receipt: c.receipt.take().ok_or(E::Closed)?,
        peer_commitment: c.peer_commitment,
        transcript,
        role: c.role,
        intent: c.intent,
        route: c.route,
        mtu: c.selected,
    })
}

impl super::common::Abort for EstablishedSession<'_> {
    fn abort(&mut self) {
        self.keys.take();
        self.reservation.take();
    }
}
