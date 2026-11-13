use super::super::super::common::guarded;
use super::{
    owner::{ChannelCore, ProtectedChannel},
    E,
};
use crate::crypto::network::authority::PinAuthority;
use crate::net::{
    peer::runtime::ReservationLedger,
    qdnf::{authority::CurrentAuthorityGuard, bearer::contract::Bearer},
};

/// Resource scope excludes the charge owner: close/drop cannot relock this ledger.
pub(crate) struct ChannelIo<'s, 'a, B: Bearer> {
    pub(super) core: &'s mut ChannelCore<'a>,
    pub(super) bearer: &'s mut B,
}
/// One bounded effect. No authority lock lives in this transport view.
pub(crate) struct FrameIo<'s, 'a, B: Bearer> {
    pub(super) core: &'s mut ChannelCore<'a>,
    pub(super) bearer: &'s mut B,
}
impl<'a, B: Bearer> ProtectedChannel<'a, B> {
    /// Lock order: original ledger -> SHORT authority guard. This callback is bounded and
    /// nonblocking; never loop/wait for a peer or retain authority across polls.
    /// Existing TransferRuntime cannot survive releasing this borrowed ledger: a
    /// canonical persistent-state split is required before asynchronous service
    /// integration. No reference can escape this callback.
    pub(crate) fn with_resources<T>(
        &mut self,
        effect: impl for<'s> FnOnce(ChannelIo<'s, 'a, B>, &'s mut ReservationLedger) -> Result<T, E>,
    ) -> Result<T, E> {
        let Self {
            core,
            bearer,
            reservation,
            ..
        } = self;
        if core.protection.is_none() {
            return Err(E::Closed);
        }
        guarded(core, |core| {
            reservation.with_ledger(|ledger| {
                effect(
                    ChannelIo {
                        core: &mut *core,
                        bearer: &mut *bearer,
                    },
                    ledger,
                )
            })
        })
    }
}
impl<'a, B: Bearer> ChannelIo<'_, 'a, B> {
    /// Acquires ONE current actual-owner guard for this bounded effect only.
    /// Both endpoint and application checks use this guard. Never wait/await or
    /// call a blocking filesystem operation inside effect. A WouldBlock return
    /// drops the guard; the next effect observes real inter-frame revocation.
    pub(crate) fn with_current<T>(
        &mut self,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        effect: impl for<'g> FnOnce(
            FrameIo<'g, 'a, B>,
            &'g mut CurrentAuthorityGuard<'a>,
        ) -> Result<T, E>,
    ) -> Result<T, E> {
        let core = &mut *self.core;
        let bearer = &mut *self.bearer;
        guarded(core, |core| {
            let mut current = core.authority.current_guard()?;
            core.check(&current, pins, clock)?;
            let result = effect(
                FrameIo {
                    core: &mut *core,
                    bearer: &mut *bearer,
                },
                &mut current,
            );
            match result {
                Err(error) if error != E::WouldBlock => Err(error),
                result => {
                    core.check(&current, pins, clock)?;
                    result
                }
            }
        })
    }
}
