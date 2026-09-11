//! Tagged QSR lookup outcomes. Authenticated lookup never returns bool.

/// Result of an exact QSR lookup against a named snapshot or cover.
///
/// `Found` carries a bounded candidate count. Targets are copied into the
/// caller buffer by the traversal; this tag is not a negative or positive
/// existence claim by itself without that copy.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QsrOutcome {
    /// Authenticated exact match. `count` is the number of targets staged.
    Found { count: u8 } = 1,
    /// Named snapshot has no matching key under a cover that includes the prefix.
    EmptyInSnapshot = 2,
    /// Bounded progress; caller must continue with remaining budget.
    NeedContinuation = 3,
    /// Cover or evidence does not speak to this prefix. Not a negative answer.
    Incomplete = 4,
    /// Required generation does not match the snapshot (unless Generation(0)).
    Stale = 5,
    /// Conflicting values for the same full key in the snapshot.
    Conflict = 6,
}

impl QsrOutcome {
    #[inline]
    pub const fn found_count(self) -> Option<u8> {
        match self {
            Self::Found { count } => Some(count),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn found_carries_count() {
        assert_eq!(QsrOutcome::Found { count: 1 }.found_count(), Some(1));
        assert_eq!(QsrOutcome::EmptyInSnapshot.found_count(), None);
    }

    #[test]
    fn outcomes_are_distinct() {
        assert_ne!(QsrOutcome::Found { count: 1 }, QsrOutcome::EmptyInSnapshot);
        assert_ne!(QsrOutcome::EmptyInSnapshot, QsrOutcome::Incomplete);
        assert_ne!(QsrOutcome::Stale, QsrOutcome::Conflict);
    }
}
