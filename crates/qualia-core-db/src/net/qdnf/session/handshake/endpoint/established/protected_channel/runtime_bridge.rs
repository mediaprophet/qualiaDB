//! Concrete bridge shared by QBS09/QBS10; validates original charge, never rights.
use super::super::super::common::guarded;
use super::{owner::ProtectedChannel, ChannelIo, E};
use crate::bundle::stream::runtime::RuntimeAccess;
use crate::net::qdnf::bearer::contract::Bearer;
impl<'a, B: Bearer> ProtectedChannel<'a, B> {
    /// Consume the opaque actual runtime view and lend it beside channel I/O.
    /// The canonical parent ledger is ALREADY locked/checked. Never relock it.
    /// No new charge, authority, keys, slot owner or generation is created.
    pub(crate) fn with_runtime_scope<'b, 'p, T>(
        &mut self,
        access: RuntimeAccess<'b, 'p>,
        effect: impl for<'s> FnOnce(ChannelIo<'s, 'a, B>, RuntimeAccess<'b, 'p>) -> Result<T, E>,
    ) -> Result<T, E> {
        // Foreign resource context refuses before mutating channel state; owner
        // remains available for its correct actual original context or close.
        self.reservation.check_runtime_access(&access)?;
        let Self { core, bearer, .. } = self;
        if core.protection.is_none() {
            return Err(E::Closed);
        }
        guarded(core, |core| {
            effect(
                ChannelIo {
                    core: &mut *core,
                    bearer: &mut *bearer,
                },
                access,
            )
        })
    }
}
