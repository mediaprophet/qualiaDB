//! Cold bounded fact capture; all request grounding reads the retained owner.
#![allow(dead_code)]
use super::{fact_store, GroundingFact};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, marker::PhantomData, rc::Rc, sync::Arc};
thread_local! { static FACTS:RefCell<Option<Arc<[GroundingFact]>>>=const{RefCell::new(None)}; }
pub(crate) fn read<T>(f: impl FnOnce(&[GroundingFact]) -> T) -> Option<T> {
    FACTS.with(|s| s.borrow().as_ref().map(|facts| f(facts)))
}
pub(crate) struct GroundingScope {
    previous: Option<Arc<[GroundingFact]>>,
    facts: Arc<[GroundingFact]>,
    digest: [u8; 32],
    retained_bytes: usize,
    _thread: PhantomData<Rc<()>>,
}
impl GroundingScope {
    pub(crate) fn capture(budget: usize) -> Result<Self, &'static str> {
        let source = fact_store()
            .lock()
            .map_err(|_| "grounding store poisoned")?;
        let mut required = source
            .len()
            .checked_mul(std::mem::size_of::<GroundingFact>())
            .and_then(|n| n.checked_add(2 * std::mem::size_of::<usize>()))
            .ok_or("grounding budget overflow")?;
        for fact in source.iter() {
            for strings in [&fact.prompt_needles, &fact.answer_ok] {
                required = required
                    .checked_add(
                        strings
                            .len()
                            .checked_mul(std::mem::size_of::<String>())
                            .ok_or("grounding budget overflow")?,
                    )
                    .ok_or("grounding budget overflow")?;
                for s in strings {
                    required = required
                        .checked_add(s.len())
                        .ok_or("grounding budget overflow")?;
                }
            }
            required = required
                .checked_add(fact.repair.len())
                .and_then(|n| n.checked_add(fact.reason.len()))
                .ok_or("grounding budget overflow")?;
        }
        if required > budget {
            return Err("grounding budget exceeded");
        }
        let facts: Arc<[GroundingFact]> = source.as_slice().into();
        let mut hash = Sha256::new();
        hash.update(b"qualia:owned-grounding-facts:v1\0");
        hash.update((facts.len() as u64).to_le_bytes());
        for fact in facts.iter() {
            let q = fact.quin;
            for w in [
                q.subject,
                q.predicate,
                q.object,
                q.context,
                q.metadata,
                q.parity,
            ] {
                hash.update(w.to_le_bytes());
            }
            for strings in [&fact.prompt_needles, &fact.answer_ok] {
                hash.update((strings.len() as u64).to_le_bytes());
                for s in strings {
                    hash.update((s.len() as u64).to_le_bytes());
                    hash.update(s.as_bytes());
                }
            }
            for s in [&fact.repair, &fact.reason] {
                hash.update((s.len() as u64).to_le_bytes());
                hash.update(s.as_bytes());
            }
        }
        let previous = FACTS.with(|slot| slot.replace(Some(Arc::clone(&facts))));
        Ok(Self {
            previous,
            facts,
            digest: hash.finalize().into(),
            retained_bytes: required,
            _thread: PhantomData,
        })
    }
    pub(crate) fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub(crate) fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    pub(crate) fn owner(&self) -> Arc<[GroundingFact]> {
        Arc::clone(&self.facts)
    }
}
impl Drop for GroundingScope {
    fn drop(&mut self) {
        FACTS.with(|slot| slot.replace(self.previous.take()));
    }
}
pub(crate) fn owns(owner: &Arc<[GroundingFact]>) -> bool {
    FACTS.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|f| Arc::ptr_eq(f, owner))
    })
}
