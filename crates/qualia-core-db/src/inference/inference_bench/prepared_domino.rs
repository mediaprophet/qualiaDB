//! Request-owned grammar execution. Evolving grammar state never writes back to
//! the process template or becomes the starting state of another request.
use crate::inference::speculative_decode::PreparedDomino;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    marker::PhantomData,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

static NEXT: AtomicU64 = AtomicU64::new(1);
struct Slot {
    identity: u64,
    grammar: Option<PreparedDomino>,
    valid: Arc<AtomicBool>,
}
thread_local! { static SLOT:RefCell<Option<Slot>> = const { RefCell::new(None) }; }

#[derive(Clone)]
pub struct PreparedDominoLease {
    identity: u64,
    digest: [u8; 32],
    valid: Arc<AtomicBool>,
}
impl PreparedDominoLease {
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn live(&self) -> bool {
        self.valid.load(Ordering::Acquire)
            && SLOT.with(|slot| {
                slot.borrow().as_ref().is_some_and(|slot| {
                    slot.identity == self.identity
                        && slot.grammar.as_ref().is_none_or(PreparedDomino::valid)
                })
            })
    }
}
pub struct PreparedDominoScope {
    previous: Option<Slot>,
    lease: PreparedDominoLease,
    _thread: PhantomData<Rc<()>>,
}
impl PreparedDominoScope {
    pub(crate) fn enter(grammar: Option<PreparedDomino>) -> Result<Self, &'static str> {
        let identity = NEXT
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
            .map_err(|_| "DOMINO owner identity exhausted")?;
        let valid = Arc::new(AtomicBool::new(true));
        let digest = grammar
            .as_ref()
            .map(PreparedDomino::digest)
            .unwrap_or_else(|| Sha256::digest(b"qualia:prepared-domino-absent:v1\0").into());
        let lease = PreparedDominoLease {
            identity,
            digest,
            valid: Arc::clone(&valid),
        };
        let previous = SLOT.with(|slot| {
            slot.replace(Some(Slot {
                identity,
                grammar,
                valid,
            }))
        });
        Ok(Self {
            previous,
            lease,
            _thread: PhantomData,
        })
    }
    pub fn lease(&self) -> PreparedDominoLease {
        self.lease.clone()
    }
}
impl Drop for PreparedDominoScope {
    fn drop(&mut self) {
        self.lease.valid.store(false, Ordering::Release);
        SLOT.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}

pub(crate) fn with<T>(operation: impl FnOnce(&mut Option<PreparedDomino>) -> T) -> Option<T> {
    SLOT.with(|slot| {
        let mut slot = slot.borrow_mut();
        let slot = slot.as_mut()?;
        let result = operation(&mut slot.grammar);
        if slot
            .grammar
            .as_ref()
            .is_some_and(|grammar| !grammar.valid())
        {
            slot.valid.store(false, Ordering::Release);
        }
        Some(result)
    })
}
pub(crate) fn invalidate() -> bool {
    SLOT.with(|slot| {
        let slot = slot.borrow();
        if let Some(slot) = slot.as_ref() {
            slot.valid.store(false, Ordering::Release);
            true
        } else {
            false
        }
    })
}
