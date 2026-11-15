use super::super::super::resource::RetainedEndpointReservation;
use super::super::super::{EndpointAuthority, Intent, Role, Route};
use super::super::EstablishedSession;
use super::{CHUNK_BYTES, E, FRAME_BYTES};
use crate::crypto::network::authority::{AuthorityReceipt, PinAuthority};
use crate::net::qdnf::{
    authority::CurrentAuthorityGuard, bearer::contract::Bearer,
    session::packet_protection::PacketProtection, types::StrongDigest,
};
use zeroize::{Zeroize, Zeroizing};

/// Consumed endpoint plus actual bearer. No Clone, key accessor or reinstallation.
/// Drop closes the bearer/erases local buffers but deliberately retains the
/// canonical charge. Use try_close after actual I/O has drained to release it.
pub struct ProtectedChannel<'a, B: Bearer> {
    pub(super) core: ChannelCore<'a>,
    pub(super) bearer: B,
    pub(super) reservation: RetainedEndpointReservation<'a>,
    transport_closed: bool,
    closed: bool,
}
pub(super) struct ChannelCore<'a> {
    pub authority: &'a EndpointAuthority<'a>,
    pub protection: Option<PacketProtection>,
    pub receipt: AuthorityReceipt,
    pub peer_commitment: StrongDigest,
    pub transcript: StrongDigest,
    pub role: Role,
    pub intent: Intent,
    pub route: Route,
    pub mtu: u16,
    pub pending: usize,
    pub wire: Zeroizing<[u8; FRAME_BYTES]>,
}
/// Refusal returns BOTH original actual owners; no fresh keys/charge are minted.
pub struct ChannelBuildError<'a, B: Bearer> {
    pub error: E,
    pub session: EstablishedSession<'a>,
    pub bearer: B,
}
impl<'a> EstablishedSession<'a> {
    /// One-time handoff. The supplied guard must belong to the endpoint's exact
    /// canonical owner. This performs no transport I/O or ledger release.
    pub fn into_protected_channel<B: Bearer>(
        mut self,
        bearer: B,
        current: &CurrentAuthorityGuard<'_>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<ProtectedChannel<'a, B>, ChannelBuildError<'a, B>> {
        let result = (|| {
            self.route.check(&bearer)?;
            if self.mtu < 1280 || self.mtu > self.route.mtu {
                return Err(E::Capacity);
            }
            for _ in 0..2 {
                let now = clock();
                self.authority.check_guard(current, now)?;
                pins.recheck(&self.receipt, &self.peer_commitment.0, now)?;
            }
            let keys = self.keys.as_ref().ok_or(E::Closed)?;
            PacketProtection::from_keys(*keys.send_key(), *keys.recv_key(), keys.generation())
        })();
        let protection = match result {
            Ok(p) => p,
            Err(error) => {
                return Err(ChannelBuildError {
                    error,
                    session: self,
                    bearer,
                });
            }
        };
        let reservation = match self.reservation.as_mut() {
            Some(original) => match original.retain_for_channel() {
                Ok(r) => r,
                Err(error) => {
                    return Err(ChannelBuildError {
                        error,
                        session: self,
                        bearer,
                    });
                }
            },
            None => {
                return Err(ChannelBuildError {
                    error: E::Closed,
                    session: self,
                    bearer,
                });
            }
        };
        self.reservation.take(); // Empty original owner: no release or relock.
        self.keys.take(); // InstalledSessionKeys zeroizes; only PacketProtection remains.
        Ok(ProtectedChannel {
            core: ChannelCore {
                authority: self.authority,
                protection: Some(protection),
                receipt: self.receipt,
                peer_commitment: self.peer_commitment,
                transcript: self.transcript,
                role: self.role,
                intent: self.intent,
                route: self.route,
                mtu: self.mtu,
                pending: 0,
                wire: Zeroizing::new([0; FRAME_BYTES]),
            },
            bearer,
            reservation,
            transport_closed: false,
            closed: false,
        })
    }
}
impl ChannelCore<'_> {
    pub fn check(
        &self,
        current: &CurrentAuthorityGuard<'_>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), E> {
        if self.protection.is_none() {
            return Err(E::Closed);
        }
        for _ in 0..2 {
            let now = clock();
            self.authority.check_guard(current, now)?;
            pins.recheck(&self.receipt, &self.peer_commitment.0, now)?;
        }
        Ok(())
    }
}
impl super::super::super::common::Abort for ChannelCore<'_> {
    fn abort(&mut self) {
        self.protection.take();
        self.wire.zeroize();
        self.pending = 0;
    }
}
impl<B: Bearer> ProtectedChannel<'_, B> {
    pub const fn transcript(&self) -> StrongDigest {
        self.core.transcript
    }
    pub const fn intent(&self) -> Intent {
        self.core.intent
    }
    pub const fn role(&self) -> Role {
        self.core.role
    }
    pub const fn route(&self) -> Route {
        self.core.route
    }
    pub const fn negotiated_mtu(&self) -> u16 {
        self.core.mtu
    }
    pub const fn pending_frame_bytes(&self) -> usize {
        self.core.pending
    }
    /// Terminal for new data; actual transport/runtime drain still belongs to
    /// their owners. No charge is released by cancellation.
    pub fn cancel(&mut self) -> Result<(), E> {
        use super::super::super::common::Abort;
        self.core.abort();
        let mut current = self.core.authority.current_guard()?;
        self.core.authority.revoke_guard(&mut current)
    }
    /// Only actual adapter shutdown success allows canonical charge release.
    /// On failure this same non-Copy owner remains available for retry.
    pub fn try_close(&mut self) -> Result<(), E> {
        use super::super::super::common::Abort;
        if self.closed {
            return Err(E::DoubleRelease);
        }
        self.core.abort();
        if !self.transport_closed {
            self.bearer.shutdown()?;
            self.transport_closed = true;
        }
        self.reservation.release_after_shutdown()?;
        self.closed = true;
        Ok(())
    }
}
impl<B: Bearer> Drop for ProtectedChannel<'_, B> {
    fn drop(&mut self) {
        use super::super::super::common::Abort;
        self.core.abort();
        if !self.transport_closed {
            let _ = self.bearer.shutdown();
        }
        // RetainedEndpointReservation intentionally has no releasing Drop.
    }
}
// Original endpoint charge already includes24KiB local transient headroom.
// The actual bearer allocation is its separately admitted owner's obligation.
const _: () = assert!(
    core::mem::size_of::<ChannelCore<'static>>()
        + core::mem::size_of::<RetainedEndpointReservation<'static>>()
        + 2 * (1500 + CHUNK_BYTES + FRAME_BYTES)
        + 4096
        <= 24 * 1024
);
