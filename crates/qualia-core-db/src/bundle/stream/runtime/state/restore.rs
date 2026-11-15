//! Unconditional, infallible restore of the exact existing runtime state.
use super::super::TransferRuntime;
use super::data::RuntimeData;
pub(super) struct Restore<'destination, 'parents> {
    destination: &'destination mut Option<RuntimeData>,
    runtime: Option<TransferRuntime<'parents>>,
}
impl<'d, 'p> Restore<'d, 'p> {
    pub fn new(destination: &'d mut Option<RuntimeData>, runtime: TransferRuntime<'p>) -> Self {
        Self {
            destination,
            runtime: Some(runtime),
        }
    }
    pub fn runtime(&mut self) -> &mut TransferRuntime<'p> {
        // The Option is never taken until this guard's own Drop.
        self.runtime
            .as_mut()
            .expect("private runtime restore invariant")
    }
}
impl Drop for Restore<'_, '_> {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            // Simple non-Copy field moves only. No validation/locks/allocation/
            // user callback/assertion is allowed in this restoration path.
            *self.destination = Some(RuntimeData::take(runtime));
        }
    }
}
