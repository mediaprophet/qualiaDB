//! Exact queued work identity, without removing or classifying foreign work.
use super::{FairScheduler, ScheduledWork};
impl FairScheduler {
    pub(crate) fn contains_work(&self, expected: ScheduledWork) -> bool {
        self.scopes
            .iter()
            .filter(|scope| scope.occupied)
            .any(|scope| {
                scope
                    .control
                    .iter()
                    .chain(scope.interactive.iter())
                    .chain(scope.background.iter())
                    .flatten()
                    .any(|work| *work == expected)
            })
    }
}
