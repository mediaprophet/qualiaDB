//! Internal provider-current staging. This does not supply application authority.
use super::{ManagedObjectPublication, ManagedObjectSealer};
use crate::crypto::network::errors::CryptoError as E;
impl ManagedObjectPublication<'_> {
    /// The caller must already hold the actual application guard where required.
    /// Effect must be bounded, synchronous and must not reenter this control or
    /// manager. The control lock covers the effect and fresh post-effect clock.
    pub(crate) fn with_current_effect<T>(
        &mut self,
        effect: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        self.attempt(|p| {
            let state = p.control.lock()?;
            p.check_locked(&state, (p.clock)())?;
            let result = effect()?;
            p.check_locked(&state, (p.clock)())?;
            Ok(result)
        })
    }
}
impl ManagedObjectSealer<'_, '_> {
    pub(crate) fn with_current_effect<T>(
        &mut self,
        effect: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        self.publication.with_current_effect(effect)
    }
}
