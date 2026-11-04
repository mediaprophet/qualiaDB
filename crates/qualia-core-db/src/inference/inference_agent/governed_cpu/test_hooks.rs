//! Test-only deterministic loss after actual prompt prefill; no production hook.
use std::cell::{Cell, RefCell};
thread_local! {
    static NEXT: Cell<u64> = const { Cell::new(0) };
    static AFTER_PREFILL: RefCell<Option<(u64, Box<dyn FnOnce()>)>> = const { RefCell::new(None) };
}
pub(super) struct HookGuard(u64);
impl Drop for HookGuard {
    fn drop(&mut self) {
        AFTER_PREFILL.with(|slot| {
            if slot.borrow().as_ref().is_some_and(|(id, _)| *id == self.0) {
                slot.borrow_mut().take();
            }
        });
    }
}
pub(super) fn install_after_prefill(hook: impl FnOnce() + 'static) -> HookGuard {
    let id = NEXT.with(|n| {
        let id = n.get().checked_add(1).expect("bounded test hooks");
        n.set(id);
        id
    });
    AFTER_PREFILL.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some((id, Box::new(hook)));
    });
    HookGuard(id)
}
pub(super) fn after_prefill() {
    let hook = AFTER_PREFILL.with(|slot| slot.borrow_mut().take());
    if let Some((_, hook)) = hook {
        hook();
    }
}
