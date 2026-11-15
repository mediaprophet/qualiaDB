//! QAC streaming profile over a complete authenticated current `.qac` entry.
//! Complete-entry remains the transfer unit. Scientific signal is optional.
use super::qac::{QacAdmitError, QacCompleteView};
use super::AuthenticatedRange;
use crate::audio::qac::{signal, QacStorageOptions};
use crate::bundle::stream::plan::RationalInterval;
use crate::q42::field_semantics::{Limits, Scratch};
/// Version/profile capability. Archival is never playable audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QacProfile {
    CompleteEntry,
    SignalScientific,
    OpaqueArchival,
}
/// Admitted complete `.qac` plus the streaming interpretation that succeeded.
pub struct QacStreamView<'a> {
    complete: QacCompleteView<'a>,
    profile: QacProfile,
}
impl<'a> QacStreamView<'a> {
    /// Whole selected entry only. Profile starts as complete-entry fallback.
    pub fn admit(
        range: AuthenticatedRange<'a>,
        options: QacStorageOptions,
    ) -> Result<Self, QacAdmitError> {
        Ok(Self {
            complete: QacCompleteView::admit(range, options)?,
            profile: QacProfile::CompleteEntry,
        })
    }
    /// Scientific inner entry. Unsupported / Profile / unknown critical stay archival.
    pub fn with_signal(
        self,
        selected: u32,
        scratch: &mut Scratch<'_>,
        shared: Limits,
        limits: signal::Limits,
    ) -> Result<Self, QacAdmitError> {
        let profile = match self
            .complete
            .admit_signal(selected, scratch, shared, limits)
        {
            Ok(_) => QacProfile::SignalScientific,
            Err(QacAdmitError::Unsupported) => QacProfile::OpaqueArchival,
            Err(e) => return Err(e),
        };
        Ok(Self {
            complete: self.complete,
            profile,
        })
    }
    pub const fn profile(&self) -> QacProfile {
        self.profile
    }
    pub fn complete(&self) -> &QacCompleteView<'a> {
        &self.complete
    }
    /// Complete-entry transfer unit or admitted scientific signal. Archival is not ready.
    pub const fn ready_for_plan(&self) -> bool {
        matches!(
            self.profile,
            QacProfile::CompleteEntry | QacProfile::SignalScientific
        )
    }
    /// Source-declared interval from the first complete-entry envelope row.
    /// Timed flag is `flags() & 2`. Missing flag is `None`, not epoch zero.
    pub fn declared_interval(&self) -> Option<RationalInterval> {
        let entry = self.complete.storage().structure().entry(1).ok()?;
        if entry.flags() & 2 == 0 {
            return None;
        }
        let start = entry.start_tick();
        let count = i64::try_from(entry.time_count()).ok()?;
        let end = start.checked_add(count)?;
        RationalInterval::declared(start, 1, end, 1).ok()
    }
}
/// Planner `present` id. Successful complete-entry or scientific admit only.
pub fn qac_present_id(entry_id: u32, view: &QacStreamView<'_>) -> Option<u32> {
    view.ready_for_plan().then_some(entry_id)
}
#[cfg(test)]
#[path = "qac_stream_tests.rs"]
mod qac_stream_tests;
