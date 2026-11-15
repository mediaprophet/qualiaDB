use super::{RetainedRecoverySource, SourceError, SourceReleaseError};
use crate::{
    bundle::stream::runtime::TransferRuntime,
    net::{
        peer::runtime::{ReservationHandle, ResourceBudget, WorkClass},
        qdnf::errors::QdnfError,
    },
    sensing::local_custody::quota::recovery::RecoveryRootScope,
    storage_driver::{
        ranges::{recovery::StagingLocator, RangeLimits},
        MmapDriver,
    },
};
impl TransferRuntime<'_> {
    /// Reserve actual retained source/metadata and conservative4-handle IO
    /// profile BEFORE cold opening. No persistent quota is reconstructed.
    pub fn acquire_recovery_source<'root>(
        &mut self,
        driver: &MmapDriver,
        scope: &RecoveryRootScope<'root>,
        locator: StagingLocator,
        limits: RangeLimits,
    ) -> Result<RetainedRecoverySource<'root>, SourceError> {
        self.require_open()?;
        let index = self
            .recovery_sources
            .iter()
            .position(|s| s.charge.is_none() && s.generation.0 != u64::MAX)
            .ok_or(QdnfError::Capacity)?;
        let generation = self.recovery_sources[index].generation.next()?;
        let count = self.source_live.checked_add(1).ok_or(QdnfError::Range)?;
        let headroom = 30usize
            .saturating_sub(self.workspace_live)
            .saturating_sub(count);
        if self.live > self.profile.inflight_limit().min(headroom) {
            return Err(QdnfError::Capacity.into());
        }
        let bytes =
            u64::try_from(RetainedRecoverySource::charged_bytes()).map_err(|_| QdnfError::Range)?;
        let window = self.used_bytes.checked_add(bytes).ok_or(QdnfError::Range)?;
        if window > self.profile.window_bytes() {
            return Err(QdnfError::BudgetExhausted.into());
        }
        let charge = self.governor.admit_and_reserve(
            self.ledger,
            ResourceBudget {
                bytes,
                work: 16,
                io: 4,
            },
            WorkClass::Background,
            true,
        )?;
        let source = match driver.open_range_recovery_source(scope, locator, limits) {
            Ok(s) => s,
            Err(e) => {
                self.ledger.release(charge)?;
                return Err(e.into());
            }
        };
        self.recovery_sources[index].generation = generation;
        self.recovery_sources[index].scope = Some(source.scope_key());
        self.recovery_sources[index].attempt = source.attempt_id();
        self.recovery_sources[index].limits = Some(source.limits());
        self.recovery_sources[index].charge = Some(charge);
        self.source_live = count;
        self.used_bytes = window;
        Ok(RetainedRecoverySource {
            owner: self.owner,
            slot: index as u8,
            generation,
            source,
        })
    }
    pub const fn retained_recovery_sources(&self) -> usize {
        self.source_live
    }
    pub fn recovery_inflight_ceiling(&self) -> usize {
        self.profile.inflight_limit().min(
            30usize
                .saturating_sub(self.workspace_live)
                .saturating_sub(self.source_live),
        )
    }
    pub(in super::super) fn validate_recovery_source(
        &self,
        source: &RetainedRecoverySource<'_>,
    ) -> Result<usize, QdnfError> {
        self.require_open()?;
        if source.owner != self.owner {
            return Err(QdnfError::StaleGeneration);
        }
        let i = usize::from(source.slot);
        let slot = self
            .recovery_sources
            .get(i)
            .ok_or(QdnfError::StaleGeneration)?;
        if slot.generation != source.generation
            || slot.charge.is_none()
            || slot.scope != Some(source.source.scope_key())
            || slot.attempt != source.attempt_id()
            || slot.limits != Some(source.limits())
        {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(i)
    }
    pub fn release_recovery_source<'root>(
        &mut self,
        source: RetainedRecoverySource<'root>,
    ) -> Result<(), SourceReleaseError<'root>> {
        match self.release_recovery_source_inner(&source) {
            Ok(()) => Ok(()),
            Err(error) => Err(SourceReleaseError { error, source }),
        }
    }
    fn release_recovery_source_inner(
        &mut self,
        source: &RetainedRecoverySource<'_>,
    ) -> Result<(), QdnfError> {
        let i = self.validate_recovery_source(source)?;
        let bytes =
            u64::try_from(RetainedRecoverySource::charged_bytes()).map_err(|_| QdnfError::Range)?;
        let window = self
            .used_bytes
            .checked_sub(bytes)
            .ok_or(QdnfError::Conflict)?;
        let count = self.source_live.checked_sub(1).ok_or(QdnfError::Conflict)?;
        let charge = self.recovery_sources[i]
            .charge
            .take()
            .ok_or(QdnfError::Incomplete)?;
        let original = charge.as_ref();
        if let Err(e) = self.ledger.release_checked_bytes(charge, bytes) {
            self.recovery_sources[i].charge = Some(ReservationHandle::from_ref(original));
            return Err(e);
        }
        self.recovery_sources[i].scope = None;
        self.recovery_sources[i].attempt = [0; 32];
        self.recovery_sources[i].limits = None;
        self.source_live = count;
        self.used_bytes = window;
        Ok(())
    }
}
