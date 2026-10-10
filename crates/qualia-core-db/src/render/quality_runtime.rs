//! Allocation-free runtime quality adaptation with conservative safety gates.
//!
//! The policy changes presentation budgets only. It must never alter simulation cadence, logical
//! time, semantic state, or deterministic gameplay decisions. Platform probes feed this portable
//! policy normalized samples; a later VibeScript mirror can use the same thresholds and reason ids.

use super::acceptance_contract::PerformanceEvidence;
use super::quality_profiles::QualityTier;

pub const RECEIPT_CAPACITY: usize = 16;
pub const DEFAULT_OVER_BUDGET_FRAMES: u8 = 4;
pub const DEFAULT_RECOVERY_FRAMES: u16 = 120;
pub const DEFAULT_MINIMUM_DWELL_MS: u64 = 8_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ThermalState {
    Cool = 0,
    Warm = 1,
    Critical = 2,
    Unknown = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourcePressure {
    Normal = 0,
    Elevated = 1,
    High = 2,
    Critical = 3,
    Unknown = 4,
}

/// One bounded presentation-health observation. Missing measurements are explicit and fail safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQualitySample {
    pub now_ms: Option<u64>,
    pub frame_time_us: Option<u32>,
    pub target_frame_time_us: Option<u32>,
    pub thermal: ThermalState,
    pub resource_pressure: ResourcePressure,
}

/// Fixed-memory collector for the performance evidence consumed by quality admission. It uses
/// the worst valid frame rather than an optimistic average and rejects changing frame targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerformanceEvidenceAccumulator {
    sample_count: u16,
    worst_frame_time_us: Option<u32>,
    target_frame_time_us: Option<u32>,
    target_consistent: bool,
}

impl Default for PerformanceEvidenceAccumulator {
    fn default() -> Self {
        Self {
            sample_count: 0,
            worst_frame_time_us: None,
            target_frame_time_us: None,
            target_consistent: true,
        }
    }
}

impl PerformanceEvidenceAccumulator {
    pub fn observe(&mut self, sample: RuntimeQualitySample) {
        let (Some(frame_time_us), Some(target_frame_time_us)) =
            (sample.frame_time_us, sample.target_frame_time_us)
        else {
            return;
        };
        if target_frame_time_us == 0 {
            return;
        }

        self.sample_count = self.sample_count.saturating_add(1);
        self.worst_frame_time_us = Some(
            self.worst_frame_time_us
                .map_or(frame_time_us, |worst| worst.max(frame_time_us)),
        );
        match self.target_frame_time_us {
            Some(target) if target != target_frame_time_us => self.target_consistent = false,
            Some(_) => {}
            None => self.target_frame_time_us = Some(target_frame_time_us),
        }
    }

    pub const fn evidence(self) -> PerformanceEvidence {
        PerformanceEvidence {
            sample_count: self.sample_count,
            worst_frame_time_us: self.worst_frame_time_us,
            target_frame_time_us: if self.target_consistent || self.sample_count == 0 {
                self.target_frame_time_us
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQualityConfig {
    /// Clamp profile changes to this inclusive tier range.
    pub minimum_tier: QualityTier,
    pub maximum_tier: QualityTier,
    pub over_budget_frames_to_downgrade: u8,
    pub recovery_frames_to_upgrade: u16,
    pub minimum_dwell_ms: u64,
}

impl Default for RuntimeQualityConfig {
    fn default() -> Self {
        Self {
            minimum_tier: QualityTier::Conservative,
            maximum_tier: QualityTier::Ultra,
            over_budget_frames_to_downgrade: DEFAULT_OVER_BUDGET_FRAMES,
            recovery_frames_to_upgrade: DEFAULT_RECOVERY_FRAMES,
            minimum_dwell_ms: DEFAULT_MINIMUM_DWELL_MS,
        }
    }
}

impl RuntimeQualityConfig {
    fn normalized(self) -> Self {
        Self {
            minimum_tier: self.minimum_tier.min(self.maximum_tier),
            maximum_tier: self.maximum_tier.max(self.minimum_tier),
            over_budget_frames_to_downgrade: self.over_budget_frames_to_downgrade.max(1),
            recovery_frames_to_upgrade: self.recovery_frames_to_upgrade.max(1),
            minimum_dwell_ms: self.minimum_dwell_ms,
        }
    }

    fn clamp(self, tier: QualityTier) -> QualityTier {
        tier.max(self.minimum_tier).min(self.maximum_tier)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum QualityChangeReason {
    Stable = 0,
    SustainedFrameTime = 1,
    ThermalPressure = 2,
    ResourcePressure = 3,
    RecoveryWindow = 4,
    UnknownCapabilityOrSample = 5,
    InvalidMonotonicClock = 6,
    TierClamp = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum QualityRuntimeEvent {
    Held = 0,
    Downgraded = 1,
    Upgraded = 2,
    ForcedConservative = 3,
}

/// Result for a sample. This policy owns its tier rather than resynchronizing from a potentially
/// stale profile. When a tier changes, update the capability snapshot's `tier_hint` and rebuild
/// the profile on the cold path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQualityDecision {
    /// Tier active before this sample's recommendation is applied.
    pub from_tier: QualityTier,
    pub recommended_tier: QualityTier,
    pub reason: QualityChangeReason,
    pub event: QualityRuntimeEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualityRuntimeReceipt {
    pub sequence: u32,
    pub now_ms: Option<u64>,
    pub from_tier: QualityTier,
    pub to_tier: QualityTier,
    pub reason: QualityChangeReason,
    pub event: QualityRuntimeEvent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QualityRuntimeCounters {
    pub samples: u32,
    pub downgrades: u32,
    pub upgrades: u32,
    pub conservative_fallbacks: u32,
    pub unknown_samples: u32,
    pub pressure_refusals: u32,
    pub receipts_overwritten: u32,
}

/// Fixed-memory state. Receipt storage is a ring; no sample allocates and no policy path changes
/// semantic or simulation state.
#[derive(Debug, Clone)]
pub struct QualityRuntimePolicy {
    config: RuntimeQualityConfig,
    tier: QualityTier,
    last_change_ms: Option<u64>,
    last_sample_ms: Option<u64>,
    dwell_waiting_for_clock: bool,
    over_budget_frames: u8,
    recovery_frames: u16,
    next_sequence: u32,
    receipt_start: usize,
    receipt_len: usize,
    receipts: [Option<QualityRuntimeReceipt>; RECEIPT_CAPACITY],
    counters: QualityRuntimeCounters,
    performance: PerformanceEvidenceAccumulator,
}

impl QualityRuntimePolicy {
    pub fn new(initial_tier: QualityTier, config: RuntimeQualityConfig) -> Self {
        let config = config.normalized();
        Self {
            config,
            tier: config.clamp(initial_tier),
            last_change_ms: None,
            last_sample_ms: None,
            dwell_waiting_for_clock: false,
            over_budget_frames: 0,
            recovery_frames: 0,
            next_sequence: 0,
            receipt_start: 0,
            receipt_len: 0,
            receipts: [None; RECEIPT_CAPACITY],
            counters: QualityRuntimeCounters::default(),
            performance: PerformanceEvidenceAccumulator::default(),
        }
    }

    pub fn tier(&self) -> QualityTier {
        self.tier
    }

    pub fn counters(&self) -> QualityRuntimeCounters {
        self.counters
    }

    /// Return measured performance evidence for feeding a later capability snapshot. Unknown or
    /// inconsistent samples remain unadmitted by `PerformanceEvidence::gate`.
    pub const fn performance_evidence(&self) -> PerformanceEvidence {
        self.performance.evidence()
    }

    /// Physical ring storage and its oldest index. Inspect at most `receipt_count()` entries,
    /// wrapping at the end of the array to enumerate oldest-to-newest without allocation.
    pub fn receipt_storage(
        &self,
    ) -> (
        &[Option<QualityRuntimeReceipt>; RECEIPT_CAPACITY],
        usize,
        usize,
    ) {
        (&self.receipts, self.receipt_start, self.receipt_len)
    }

    pub fn receipt_count(&self) -> usize {
        self.receipt_len
    }

    pub fn observe(&mut self, sample: RuntimeQualitySample) -> RuntimeQualityDecision {
        self.counters.samples = self.counters.samples.saturating_add(1);
        self.performance.observe(sample);
        let current = self.tier;
        let clock_regressed = self.clock_regressed(sample.now_ms);
        let decision = if self.has_unknown(sample) {
            self.counters.unknown_samples = self.counters.unknown_samples.saturating_add(1);
            self.force_conservative(
                sample.now_ms.filter(|_| !clock_regressed),
                QualityChangeReason::UnknownCapabilityOrSample,
            )
        } else if clock_regressed {
            self.force_conservative(None, QualityChangeReason::InvalidMonotonicClock)
        } else if sample.thermal == ThermalState::Critical
            || sample.resource_pressure == ResourcePressure::Critical
        {
            let reason = if sample.thermal == ThermalState::Critical {
                QualityChangeReason::ThermalPressure
            } else {
                QualityChangeReason::ResourcePressure
            };
            self.force_conservative(sample.now_ms, reason)
        } else {
            self.observe_known(current, sample)
        };

        if let Some(now) = sample.now_ms.filter(|_| !clock_regressed) {
            self.last_sample_ms = Some(self.last_sample_ms.map_or(now, |last| last.max(now)));
        }

        self.push_receipt(sample.now_ms, decision);
        decision
    }

    fn observe_known(
        &mut self,
        current: QualityTier,
        sample: RuntimeQualitySample,
    ) -> RuntimeQualityDecision {
        let frame_time = sample.frame_time_us.unwrap_or(0);
        let target = sample.target_frame_time_us.unwrap_or(0);
        // Require a 15% overrun before accumulating load. A 15% headroom is required to count
        // toward recovery, preventing oscillation around the frame budget.
        let over_budget = (frame_time as u64) * 100 > (target as u64) * 115;
        let healthy = (frame_time as u64) * 100 <= (target as u64) * 85
            && sample.thermal == ThermalState::Cool
            && sample.resource_pressure == ResourcePressure::Normal;
        if over_budget
            || sample.thermal == ThermalState::Warm
            || sample.resource_pressure == ResourcePressure::High
        {
            self.over_budget_frames = self.over_budget_frames.saturating_add(1);
            self.recovery_frames = 0;
            self.counters.pressure_refusals = self.counters.pressure_refusals.saturating_add(1);
        } else {
            self.over_budget_frames = 0;
            if healthy {
                self.recovery_frames = self.recovery_frames.saturating_add(1);
            } else {
                self.recovery_frames = 0;
                self.counters.pressure_refusals = self.counters.pressure_refusals.saturating_add(1);
            }
        }

        let enough_dwell = self.dwell_elapsed(sample.now_ms);
        // Ordinary degradation is deliberately fast-down/slow-up: sustained load can lower
        // several tiers as repeated over-budget windows arrive. Critical pressure bypasses even
        // those windows via `force_conservative` above.
        if self.over_budget_frames >= self.config.over_budget_frames_to_downgrade {
            let to = self.config.clamp(previous_tier(current));
            if to < current {
                let reason = if sample.thermal == ThermalState::Warm {
                    QualityChangeReason::ThermalPressure
                } else if sample.resource_pressure == ResourcePressure::High {
                    QualityChangeReason::ResourcePressure
                } else {
                    QualityChangeReason::SustainedFrameTime
                };
                return self.change(current, to, sample.now_ms, reason);
            }
            self.over_budget_frames = 0;
        }

        if self.recovery_frames >= self.config.recovery_frames_to_upgrade
            && enough_dwell
            && sample.resource_pressure == ResourcePressure::Normal
            && sample.thermal == ThermalState::Cool
        {
            let to = self.config.clamp(next_tier(current));
            if to > current {
                return self.change(
                    current,
                    to,
                    sample.now_ms,
                    QualityChangeReason::RecoveryWindow,
                );
            }
            self.recovery_frames = 0;
        }

        RuntimeQualityDecision {
            from_tier: current,
            recommended_tier: current,
            reason: if self.recovery_frames >= self.config.recovery_frames_to_upgrade
                && !enough_dwell
            {
                QualityChangeReason::TierClamp
            } else {
                QualityChangeReason::Stable
            },
            event: QualityRuntimeEvent::Held,
        }
    }

    fn has_unknown(&self, sample: RuntimeQualitySample) -> bool {
        sample.now_ms.is_none()
            || sample.frame_time_us.is_none()
            || sample.target_frame_time_us.is_none()
            || sample.target_frame_time_us == Some(0)
            || sample.thermal == ThermalState::Unknown
            || sample.resource_pressure == ResourcePressure::Unknown
    }

    fn clock_regressed(&self, now_ms: Option<u64>) -> bool {
        matches!((self.last_sample_ms, now_ms), (Some(last), Some(now)) if now < last)
    }

    fn dwell_elapsed(&mut self, now_ms: Option<u64>) -> bool {
        if self.dwell_waiting_for_clock {
            if let Some(now) = now_ms {
                self.last_change_ms = Some(now);
                self.dwell_waiting_for_clock = false;
            }
            return false;
        }
        match (self.last_change_ms, now_ms) {
            (Some(last), Some(now)) => now.saturating_sub(last) >= self.config.minimum_dwell_ms,
            // On initial startup there is no prior transition to dwell from.
            (None, Some(_)) => true,
            _ => false,
        }
    }

    fn force_conservative(
        &mut self,
        now_ms: Option<u64>,
        reason: QualityChangeReason,
    ) -> RuntimeQualityDecision {
        self.reset_streaks();
        let to = self.config.clamp(QualityTier::Conservative);
        let from = self.tier;
        if to < from {
            self.tier = to;
            self.last_change_ms = now_ms;
            self.dwell_waiting_for_clock = now_ms.is_none();
            self.counters.downgrades = self.counters.downgrades.saturating_add(1);
            self.counters.conservative_fallbacks =
                self.counters.conservative_fallbacks.saturating_add(1);
            RuntimeQualityDecision {
                from_tier: from,
                recommended_tier: to,
                reason,
                event: QualityRuntimeEvent::ForcedConservative,
            }
        } else {
            RuntimeQualityDecision {
                from_tier: from,
                recommended_tier: from,
                reason,
                event: QualityRuntimeEvent::Held,
            }
        }
    }

    fn change(
        &mut self,
        from: QualityTier,
        to: QualityTier,
        now_ms: Option<u64>,
        reason: QualityChangeReason,
    ) -> RuntimeQualityDecision {
        self.tier = to;
        self.last_change_ms = now_ms;
        self.dwell_waiting_for_clock = false;
        self.reset_streaks();
        let downgrade = to < from;
        if downgrade {
            self.counters.downgrades = self.counters.downgrades.saturating_add(1);
        } else {
            self.counters.upgrades = self.counters.upgrades.saturating_add(1);
        }
        RuntimeQualityDecision {
            from_tier: from,
            recommended_tier: to,
            reason,
            event: if downgrade {
                QualityRuntimeEvent::Downgraded
            } else {
                QualityRuntimeEvent::Upgraded
            },
        }
    }

    fn reset_streaks(&mut self) {
        self.over_budget_frames = 0;
        self.recovery_frames = 0;
    }

    fn push_receipt(&mut self, now_ms: Option<u64>, decision: RuntimeQualityDecision) {
        let receipt = QualityRuntimeReceipt {
            sequence: self.next_sequence,
            now_ms,
            from_tier: decision.from_tier,
            to_tier: decision.recommended_tier,
            reason: decision.reason,
            event: decision.event,
        };
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let index = if self.receipt_len < RECEIPT_CAPACITY {
            let index = (self.receipt_start + self.receipt_len) % RECEIPT_CAPACITY;
            self.receipt_len += 1;
            index
        } else {
            let index = self.receipt_start;
            self.receipt_start = (self.receipt_start + 1) % RECEIPT_CAPACITY;
            self.counters.receipts_overwritten =
                self.counters.receipts_overwritten.saturating_add(1);
            index
        };
        self.receipts[index] = Some(receipt);
    }
}

#[inline]
fn previous_tier(tier: QualityTier) -> QualityTier {
    match tier {
        QualityTier::Conservative => QualityTier::Conservative,
        QualityTier::Low => QualityTier::Conservative,
        QualityTier::Balanced => QualityTier::Low,
        QualityTier::High => QualityTier::Balanced,
        QualityTier::Ultra => QualityTier::High,
    }
}

#[inline]
fn next_tier(tier: QualityTier) -> QualityTier {
    match tier {
        QualityTier::Conservative => QualityTier::Low,
        QualityTier::Low => QualityTier::Balanced,
        QualityTier::Balanced => QualityTier::High,
        QualityTier::High => QualityTier::Ultra,
        QualityTier::Ultra => QualityTier::Ultra,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        QualityChangeReason, QualityRuntimeEvent, QualityRuntimePolicy, ResourcePressure,
        RuntimeQualityConfig, RuntimeQualitySample, ThermalState,
    };
    use crate::render::quality_profiles::QualityTier;

    fn sample(now_ms: u64, frame_time_us: u32) -> RuntimeQualitySample {
        RuntimeQualitySample {
            now_ms: Some(now_ms),
            frame_time_us: Some(frame_time_us),
            target_frame_time_us: Some(16_667),
            thermal: ThermalState::Cool,
            resource_pressure: ResourcePressure::Normal,
        }
    }

    fn quick_config() -> RuntimeQualityConfig {
        RuntimeQualityConfig {
            minimum_tier: QualityTier::Conservative,
            maximum_tier: QualityTier::Ultra,
            over_budget_frames_to_downgrade: 3,
            recovery_frames_to_upgrade: 3,
            minimum_dwell_ms: 100,
        }
    }

    #[test]
    fn sustained_load_downgrades_once_after_threshold() {
        let mut policy = QualityRuntimePolicy::new(QualityTier::High, quick_config());
        for index in 0..2 {
            let decision = policy.observe(sample(index * 20, 25_000));
            assert_eq!(decision.event, QualityRuntimeEvent::Held);
        }
        let decision = policy.observe(sample(40, 25_000));
        assert_eq!(decision.recommended_tier, QualityTier::Balanced);
        assert_eq!(decision.reason, QualityChangeReason::SustainedFrameTime);
        assert_eq!(policy.counters().downgrades, 1);
    }

    #[test]
    fn transient_spike_does_not_lower_quality() {
        let mut policy = QualityRuntimePolicy::new(QualityTier::High, quick_config());
        policy.observe(sample(0, 25_000));
        let decision = policy.observe(sample(20, 15_000));
        assert_eq!(decision.recommended_tier, QualityTier::High);
        assert_eq!(policy.counters().downgrades, 0);
    }

    #[test]
    fn recovery_requires_clean_frames_and_minimum_dwell() {
        let mut policy = QualityRuntimePolicy::new(QualityTier::Balanced, quick_config());
        let heavy = sample(0, 25_000);
        for ms in [0, 20, 40] {
            policy.observe(RuntimeQualitySample {
                now_ms: Some(ms),
                ..heavy
            });
        }
        assert_eq!(policy.tier(), QualityTier::Low);
        for ms in [60, 80, 100, 120] {
            let decision = policy.observe(sample(ms, 12_000));
            assert_eq!(decision.recommended_tier, QualityTier::Low);
        }
        let decision = policy.observe(sample(140, 12_000));
        assert_eq!(decision.recommended_tier, QualityTier::Balanced);
        assert_eq!(decision.reason, QualityChangeReason::RecoveryWindow);
    }

    #[test]
    fn configured_tier_clamps_are_respected() {
        let config = RuntimeQualityConfig {
            minimum_tier: QualityTier::Low,
            maximum_tier: QualityTier::Balanced,
            ..quick_config()
        };
        let mut policy = QualityRuntimePolicy::new(QualityTier::Ultra, config);
        assert_eq!(policy.tier(), QualityTier::Balanced);
        let conservative = RuntimeQualitySample {
            now_ms: None,
            frame_time_us: None,
            target_frame_time_us: None,
            thermal: ThermalState::Unknown,
            resource_pressure: ResourcePressure::Unknown,
        };
        let decision = policy.observe(conservative);
        assert_eq!(decision.recommended_tier, QualityTier::Low);
    }

    #[test]
    fn unknown_sample_falls_back_and_critical_pressure_is_immediate() {
        let mut policy = QualityRuntimePolicy::new(QualityTier::Ultra, quick_config());
        let unknown = RuntimeQualitySample {
            now_ms: None,
            frame_time_us: Some(10_000),
            target_frame_time_us: Some(16_667),
            thermal: ThermalState::Cool,
            resource_pressure: ResourcePressure::Normal,
        };
        let decision = policy.observe(unknown);
        assert_eq!(decision.recommended_tier, QualityTier::Conservative);
        assert_eq!(decision.event, QualityRuntimeEvent::ForcedConservative);
        let mut critical = sample(1_000, 10_000);
        critical.resource_pressure = ResourcePressure::Critical;
        let decision = policy.observe(critical);
        assert_eq!(decision.recommended_tier, QualityTier::Conservative);
        assert_eq!(decision.reason, QualityChangeReason::ResourcePressure);
    }

    #[test]
    fn elevated_pressure_refuses_recovery_and_receipts_are_bounded() {
        let mut policy = QualityRuntimePolicy::new(QualityTier::Low, quick_config());
        for i in 0..24 {
            let mut observation = sample(i * 20, 12_000);
            observation.resource_pressure = ResourcePressure::Elevated;
            let decision = policy.observe(observation);
            assert_eq!(decision.recommended_tier, QualityTier::Low);
        }
        assert_eq!(policy.counters().upgrades, 0);
        assert_eq!(policy.receipt_count(), super::RECEIPT_CAPACITY);
        assert_eq!(policy.counters().receipts_overwritten, 8);
        let (storage, start, len) = policy.receipt_storage();
        assert_eq!(len, super::RECEIPT_CAPACITY);
        assert!(storage[start].is_some());
    }

    #[test]
    fn clock_regression_preserves_high_water_mark_until_time_recovers() {
        let mut policy = QualityRuntimePolicy::new(QualityTier::High, quick_config());
        policy.observe(sample(1_000, 15_000));
        let regressed = policy.observe(sample(900, 15_000));
        assert_eq!(regressed.reason, QualityChangeReason::InvalidMonotonicClock);
        assert_eq!(regressed.recommended_tier, QualityTier::Conservative);
        let still_regressed = policy.observe(sample(950, 15_000));
        assert_eq!(
            still_regressed.reason,
            QualityChangeReason::InvalidMonotonicClock
        );
        assert_eq!(still_regressed.recommended_tier, QualityTier::Conservative);
        policy.observe(sample(1_001, 15_000));
        assert_eq!(policy.tier(), QualityTier::Conservative);
    }

    #[test]
    fn missing_clock_starts_recovery_dwell_at_first_valid_timestamp() {
        let mut policy = QualityRuntimePolicy::new(QualityTier::Ultra, quick_config());
        let mut unknown = sample(0, 12_000);
        unknown.now_ms = None;
        let decision = policy.observe(unknown);
        assert_eq!(decision.recommended_tier, QualityTier::Conservative);
        for ms in [0, 20, 40, 60, 80] {
            let decision = policy.observe(sample(ms, 12_000));
            assert_eq!(decision.recommended_tier, QualityTier::Conservative);
        }
        let decision = policy.observe(sample(100, 12_000));
        assert_eq!(decision.recommended_tier, QualityTier::Low);
    }

    #[test]
    fn performance_evidence_is_bounded_and_fails_closed() {
        let mut policy = QualityRuntimePolicy::new(QualityTier::Balanced, quick_config());
        assert_eq!(
            policy.performance_evidence().gate(),
            super::super::acceptance_contract::CapabilityEvidence::Unknown
        );

        for now_ms in [0, 20, 40] {
            policy.observe(sample(now_ms, 12_000));
        }
        let evidence = policy.performance_evidence();
        assert_eq!(evidence.sample_count, 3);
        assert_eq!(evidence.worst_frame_time_us, Some(12_000));
        assert_eq!(evidence.target_frame_time_us, Some(16_667));
        assert_eq!(
            evidence.gate(),
            super::super::acceptance_contract::CapabilityEvidence::Confirmed
        );

        policy.observe(RuntimeQualitySample {
            target_frame_time_us: Some(8_000),
            ..sample(60, 12_000)
        });
        assert_eq!(
            policy.performance_evidence().gate(),
            super::super::acceptance_contract::CapabilityEvidence::Unknown
        );
    }
}
