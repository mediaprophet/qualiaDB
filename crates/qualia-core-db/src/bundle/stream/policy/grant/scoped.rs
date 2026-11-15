//! Bounded multi-barrier effects under the existing local grant.
use super::{
    AuthorityBinding, AuthorityOwner, BoundClosure, E, LocalWholeSourceGrant, MutationHandle,
    PinAuthority,
};

/// Current-authority gate with the same concrete owner and private grant.
/// No permit escapes; failed checks and cancellation are terminal even when
/// the callback catches an error. This is not a remote Share/consent decision.
pub struct LocalEffectControl<'a, 'data> {
    owner: &'a mut AuthorityOwner,
    closure: &'a BoundClosure<'data>,
    pins: &'a PinAuthority,
    clock: &'a dyn Fn() -> u64,
    handle: MutationHandle,
    binding: AuthorityBinding,
    ceiling: u64,
    terminal: bool,
}
impl LocalEffectControl<'_, '_> {
    /// Samples trusted host time internally and checks BOTH current paired pin
    /// receipts and this exact local grant. Call immediately before each bounded
    /// effect and after blocking I/O. This cannot undo already completed writes.
    pub fn recheck(&mut self) -> Result<(), E> {
        if self.terminal {
            return Err(E::Spent);
        }
        self.terminal = true;
        for _ in 0..2 {
            let now = (self.clock)();
            self.closure.recheck(self.pins, now)?;
            self.owner
                .issue_permit(self.handle, self.binding, now, self.ceiling)
                .map_err(E::Authority)?;
        }
        self.terminal = false;
        Ok(())
    }
    pub fn cancel(&mut self) -> Result<(), E> {
        self.terminal = true;
        self.owner.revoke(self.handle).map_err(E::Authority)
    }
    pub fn closure(&self) -> &BoundClosure<'_> {
        self.closure
    }
}
impl<'data> LocalWholeSourceGrant<'_, 'data> {
    /// One synchronous effect with repeated current-authority checks. The
    /// callback must recheck before each externally meaningful operation;
    /// entry/exit checks additionally prevent publishing a successful result
    /// after expiry. Failure does not roll back staging I/O; callers retain
    /// uncertain bytes as unaccepted candidates and drain through their owner.
    /// Exact owner/pin borrows remain held only for this bounded call. Do not
    /// await, reenter these owners, or retain plaintext without its own lease.
    pub fn accept_local_scoped<T>(
        &mut self,
        pins: &PinAuthority,
        clock: impl Fn() -> u64,
        effect: impl for<'g> FnOnce(&mut LocalEffectControl<'g, 'data>) -> Result<T, E>,
    ) -> Result<T, E> {
        if self.spent {
            return Err(E::Spent);
        }
        self.spent = true;
        let mut control = LocalEffectControl {
            owner: self.owner,
            closure: self.closure,
            pins,
            clock: &clock,
            handle: self.handle,
            binding: self.binding,
            ceiling: self.ceiling,
            terminal: false,
        };
        control.recheck()?;
        let result = effect(&mut control)?;
        control.recheck()?;
        Ok(result)
    }
}
