//! Explicit clock-domain alignment and conservative uncertainty windows.

use super::fusion::FusionError;

/// A source clock timestamp with an integer nanosecond worst-case absolute
/// error radius. The true timestamp is assumed to lie in the inclusive interval
/// `[max(0, ticks_ns - uncertainty_ns), ticks_ns + uncertainty_ns]`.
/// Clock IDs identify monotonic domains; they do not imply UTC or host time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FusionClockStamp {
    clock_id: u64,
    restart_epoch: u32,
    ticks_ns: u64,
    uncertainty_ns: u64,
}

impl FusionClockStamp {
    pub const fn new(
        clock_id: u64,
        restart_epoch: u32,
        ticks_ns: u64,
        uncertainty_ns: u64,
    ) -> Result<Self, FusionError> {
        if clock_id == 0 {
            return Err(FusionError::InvalidIdentifier);
        }
        if ticks_ns.checked_add(uncertainty_ns).is_none() {
            return Err(FusionError::InvalidTime);
        }
        Ok(Self {
            clock_id,
            restart_epoch,
            ticks_ns,
            uncertainty_ns,
        })
    }
}

/// An explicit, constant-offset mapping from a source clock/epoch into the
/// selected target clock. It is valid only over the declared source-time range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeAlignment {
    alignment_id: u64,
    source_clock_id: u64,
    source_epoch: u32,
    target_clock_id: u64,
    target_epoch: u32,
    valid_from_ns: u64,
    valid_through_ns: u64,
    offset_ns: i128,
    /// Worst-case absolute error bound on `offset_ns` over the validity range.
    /// It is conservatively added to the sample's absolute timestamp bound.
    uncertainty_ns: u64,
}

impl TimeAlignment {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        alignment_id: u64,
        source_clock_id: u64,
        source_epoch: u32,
        target_clock_id: u64,
        target_epoch: u32,
        valid_from_ns: u64,
        valid_through_ns: u64,
        offset_ns: i128,
        uncertainty_ns: u64,
    ) -> Result<Self, FusionError> {
        if alignment_id == 0 || source_clock_id == 0 || target_clock_id == 0 {
            return Err(FusionError::InvalidIdentifier);
        }
        if valid_from_ns > valid_through_ns {
            return Err(FusionError::InvalidTimeAlignment);
        }
        Ok(Self {
            alignment_id,
            source_clock_id,
            source_epoch,
            target_clock_id,
            target_epoch,
            valid_from_ns,
            valid_through_ns,
            offset_ns,
            uncertainty_ns,
        })
    }
}

/// Target clock and coordinate frame for a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FusionTarget {
    pub(super) clock_id: u64,
    pub(super) restart_epoch: u32,
    pub(super) coordinate_frame_id: u64,
}

impl FusionTarget {
    pub const fn new(
        clock_id: u64,
        restart_epoch: u32,
        coordinate_frame_id: u64,
    ) -> Result<Self, FusionError> {
        if clock_id == 0 || coordinate_frame_id == 0 {
            return Err(FusionError::InvalidIdentifier);
        }
        Ok(Self {
            clock_id,
            restart_epoch,
            coordinate_frame_id,
        })
    }
}

/// Inclusive aligned time interval, expressed only in the target clock domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FusionTimeWindow {
    pub(super) clock_id: u64,
    pub(super) restart_epoch: u32,
    pub(super) lower_ns: u64,
    pub(super) upper_ns: u64,
}

impl FusionTimeWindow {
    pub const fn clock_id(self) -> u64 {
        self.clock_id
    }

    pub const fn restart_epoch(self) -> u32 {
        self.restart_epoch
    }

    pub const fn lower_ns(self) -> u64 {
        self.lower_ns
    }

    pub const fn upper_ns(self) -> u64 {
        self.upper_ns
    }
}

pub(super) fn align_time(
    stamp: Option<FusionClockStamp>,
    target: FusionTarget,
    alignments: &[TimeAlignment],
) -> Result<(FusionTimeWindow, Option<u64>), FusionError> {
    let stamp = stamp.ok_or(FusionError::UnknownTime)?;
    let matching_domain =
        stamp.clock_id == target.clock_id && stamp.restart_epoch == target.restart_epoch;
    let (center_ns, alignment_id, alignment_uncertainty) = if matching_domain {
        (stamp.ticks_ns, None, 0)
    } else {
        let mut found = None;
        for alignment in alignments {
            if alignment.source_clock_id == stamp.clock_id
                && alignment.source_epoch == stamp.restart_epoch
                && alignment.target_clock_id == target.clock_id
                && alignment.target_epoch == target.restart_epoch
            {
                if found.is_some() {
                    return Err(FusionError::AmbiguousTimeAlignment);
                }
                found = Some(*alignment);
            }
        }
        let alignment = found.ok_or(FusionError::MissingTimeAlignment)?;
        let source_lower = stamp.ticks_ns.saturating_sub(stamp.uncertainty_ns);
        let source_upper = stamp
            .ticks_ns
            .checked_add(stamp.uncertainty_ns)
            .ok_or(FusionError::InvalidTime)?;
        if source_lower < alignment.valid_from_ns || source_upper > alignment.valid_through_ns {
            return Err(FusionError::TimeAlignmentOutsideValidity);
        }
        let center = i128::from(stamp.ticks_ns)
            .checked_add(alignment.offset_ns)
            .ok_or(FusionError::InvalidTime)?;
        if center < 0 || center > i128::from(u64::MAX) {
            return Err(FusionError::InvalidTime);
        }
        (
            center as u64,
            Some(alignment.alignment_id),
            alignment.uncertainty_ns,
        )
    };
    let uncertainty_ns = stamp
        .uncertainty_ns
        .checked_add(alignment_uncertainty)
        .ok_or(FusionError::InvalidTime)?;
    let upper_ns = center_ns
        .checked_add(uncertainty_ns)
        .ok_or(FusionError::InvalidTime)?;
    let lower_ns = center_ns.saturating_sub(uncertainty_ns);
    Ok((
        FusionTimeWindow {
            clock_id: target.clock_id,
            restart_epoch: target.restart_epoch,
            lower_ns,
            upper_ns,
        },
        alignment_id,
    ))
}
