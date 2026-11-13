//! Local actual-value identity only; never a budget token or wire authority.
use crate::net::qdnf::errors::QdnfError;
use core::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CanonicalIncarnation(usize);
impl CanonicalIncarnation {
    pub(crate) const UNASSIGNED: Self = Self(0);
    pub(crate) const fn assigned(self) -> bool {
        self.0 != 0
    }
    pub(crate) fn bind(&mut self) -> Result<Self, QdnfError> {
        if !self.assigned() {
            *self = Self(next(&NEXT)?);
        }
        Ok(*self)
    }
    pub(crate) fn fresh() -> Result<Self, QdnfError> {
        Ok(Self(next(&NEXT)?))
    }
}
fn next(source: &AtomicUsize) -> Result<usize, QdnfError> {
    source
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| QdnfError::StaleGeneration)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn once_bound_is_stable_and_new_value_is_distinct() {
        let mut a = CanonicalIncarnation::UNASSIGNED;
        let mut b = CanonicalIncarnation::UNASSIGNED;
        assert!(!a.assigned());
        let first = a.bind().unwrap();
        assert!(first.assigned());
        assert_eq!(a.bind(), Ok(first));
        assert_ne!(b.bind().unwrap(), first);
        let moved = a;
        assert_eq!(moved, first);
    }
    #[test]
    fn exhaustion_never_wraps_or_changes_source() {
        let source = AtomicUsize::new(usize::MAX);
        assert_eq!(next(&source), Err(QdnfError::StaleGeneration));
        assert_eq!(source.load(Ordering::Relaxed), usize::MAX);
    }
}
