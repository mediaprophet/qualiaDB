//! Bounded, deterministic planning for combining independently sourced samples.
//!
//! This module selects and aligns references; it does not merge physical sample
//! values, perform coordinate transforms, authenticate inputs, or create
//! evidence. A caller must apply the referenced mapping and preserve each
//! source's physical semantics when executing the plan.

use super::fusion_time::align_time;
use super::{PhysicalFieldSemantics, SemanticId, SemanticProjection};

pub use super::fusion_time::{FusionClockStamp, FusionTarget, FusionTimeWindow, TimeAlignment};

/// Hard bound on candidate input size and output-plan size.
pub const MAX_FUSION_CANDIDATES: usize = 64;

/// Explicit spatial-coordinate mapping reference. `maximum_error` is expressed
/// in the declared nonzero error unit. This metadata is not the transform and
/// does not prove that the transform was applied correctly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoordinateMapping {
    mapping_id: u64,
    source_frame_id: u64,
    target_frame_id: u64,
    maximum_error: f64,
    error_unit_id: SemanticId,
}

impl CoordinateMapping {
    pub fn new(
        mapping_id: u64,
        source_frame_id: u64,
        target_frame_id: u64,
        maximum_error: f64,
        error_unit_id: SemanticId,
    ) -> Result<Self, FusionError> {
        if mapping_id == 0 || source_frame_id == 0 || target_frame_id == 0 {
            return Err(FusionError::InvalidIdentifier);
        }
        if !maximum_error.is_finite() || maximum_error < 0.0 {
            return Err(FusionError::InvalidCoordinateMapping);
        }
        if error_unit_id.value().is_none() {
            return Err(FusionError::UnknownMappingErrorUnit);
        }
        Ok(Self {
            mapping_id,
            source_frame_id,
            target_frame_id,
            maximum_error,
            error_unit_id,
        })
    }
}

/// Measurement quality is independent from source trust. Unknown is preserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleQuality {
    Unknown,
    Known(SampleQualityMetrics),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleQualityMetrics {
    score_millionths: u32,
    uncertainty_millionths: u32,
}

impl SampleQuality {
    pub const SCALE: u32 = 1_000_000;

    pub const fn known(
        score_millionths: u32,
        uncertainty_millionths: u32,
    ) -> Result<Self, FusionError> {
        if score_millionths > Self::SCALE || uncertainty_millionths > Self::SCALE {
            return Err(FusionError::InvalidQuality);
        }
        Ok(Self::Known(SampleQualityMetrics {
            score_millionths,
            uncertainty_millionths,
        }))
    }
}

impl SampleQualityMetrics {
    pub const fn score_millionths(self) -> u32 {
        self.score_millionths
    }

    pub const fn uncertainty_millionths(self) -> u32 {
        self.uncertainty_millionths
    }
}

/// Candidate sample reference. Payload bytes remain in the caller's storage;
/// this record carries identity and interpretation metadata only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FusionSample {
    sample_id: u64,
    physical: PhysicalFieldSemantics,
    clock: Option<FusionClockStamp>,
    coordinate_frame_id: SemanticId,
    projection: Option<SemanticProjection>,
    quality: SampleQuality,
    priority: i32,
}

impl FusionSample {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sample_id: u64,
        physical: PhysicalFieldSemantics,
        clock: Option<FusionClockStamp>,
        coordinate_frame_id: SemanticId,
        projection: Option<SemanticProjection>,
        quality: SampleQuality,
        priority: i32,
    ) -> Result<Self, FusionError> {
        if sample_id == 0 {
            return Err(FusionError::InvalidIdentifier);
        }
        if projection
            .as_ref()
            .is_some_and(|projection| projection.physical() != &physical)
        {
            return Err(FusionError::ProjectionSemanticMismatch);
        }
        Ok(Self {
            sample_id,
            physical,
            clock,
            coordinate_frame_id,
            projection,
            quality,
            priority,
        })
    }
}

/// Maximum number of selected samples and their maximum aligned time span.
/// The time span is the union bound `latest_upper - earliest_lower` in the
/// target clock. Callers must choose this temporal-coherence limit explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FusionPolicy {
    maximum_samples: usize,
    maximum_time_span_ns: u64,
}

impl FusionPolicy {
    pub const fn new(
        maximum_samples: usize,
        maximum_time_span_ns: u64,
    ) -> Result<Self, FusionError> {
        if maximum_samples == 0 || maximum_samples > MAX_FUSION_CANDIDATES {
            return Err(FusionError::InvalidPolicy);
        }
        Ok(Self {
            maximum_samples,
            maximum_time_span_ns,
        })
    }
}

/// A selected plan entry, not a fused observation or evidence receipt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FusionPlanEntry {
    sample_id: u64,
    input_index: usize,
    physical: PhysicalFieldSemantics,
    aligned_window: FusionTimeWindow,
    alignment_id: Option<u64>,
    mapping_id: u64,
    mapping_maximum_error: f64,
    mapping_error_unit_id: SemanticId,
    projection: Option<SemanticProjection>,
    quality: SampleQuality,
}

impl FusionPlanEntry {
    pub const fn sample_id(self) -> u64 {
        self.sample_id
    }

    pub const fn input_index(self) -> usize {
        self.input_index
    }

    pub const fn physical(&self) -> &PhysicalFieldSemantics {
        &self.physical
    }

    pub const fn aligned_window(self) -> FusionTimeWindow {
        self.aligned_window
    }

    pub const fn alignment_id(self) -> Option<u64> {
        self.alignment_id
    }

    pub const fn mapping_id(self) -> u64 {
        self.mapping_id
    }

    pub const fn mapping_maximum_error(self) -> f64 {
        self.mapping_maximum_error
    }

    pub const fn mapping_error_unit_id(self) -> SemanticId {
        self.mapping_error_unit_id
    }

    pub const fn projection(self) -> Option<SemanticProjection> {
        self.projection
    }

    pub const fn quality(self) -> SampleQuality {
        self.quality
    }
}

/// Build a deterministic bounded plan in caller-owned `out` storage.
///
/// Every candidate must have a known coordinate frame and exactly one explicit
/// coordinate mapping into the target. Same-domain clocks use their exact
/// declared domain; cross-clock or cross-restart candidates require exactly
/// one matching valid `TimeAlignment`. Missing/ambiguous mappings, unknown
/// time, or a selected union interval wider than the policy limit fail the whole
/// plan before output is modified. Selection sorts by descending priority, then
/// ascending sample ID. Physical modality is carried only by physical metadata;
/// unknown projection axes and quality remain explicit.
pub fn plan_fusion(
    samples: &[FusionSample],
    alignments: &[TimeAlignment],
    mappings: &[CoordinateMapping],
    target: FusionTarget,
    policy: FusionPolicy,
    out: &mut [Option<FusionPlanEntry>],
) -> Result<usize, FusionError> {
    if samples.len() > MAX_FUSION_CANDIDATES {
        return Err(FusionError::TooManyCandidates);
    }
    validate_unique_ids(samples)?;

    // Validate every candidate and its required bindings before touching out.
    for sample in samples {
        resolve_mapping(
            sample.coordinate_frame_id,
            target.coordinate_frame_id,
            mappings,
        )?;
        align_time(sample.clock, target, alignments)?;
    }

    let selected_count = samples.len().min(policy.maximum_samples);
    if out.len() < selected_count {
        return Err(FusionError::OutputTooSmall);
    }

    // Build selection order in bounded stack storage; caller output is never
    // used as scratch for a plan that may still fail temporal coherence.
    let mut selected_indices = [usize::MAX; MAX_FUSION_CANDIDATES];
    for rank in 0..selected_count {
        let mut best_index: Option<usize> = None;
        for (index, sample) in samples.iter().enumerate() {
            if selected_indices[..rank]
                .iter()
                .any(|&selected| samples[selected].sample_id == sample.sample_id)
            {
                continue;
            }
            best_index = match best_index {
                None => Some(index),
                Some(current) => {
                    let incumbent = &samples[current];
                    if sample.priority > incumbent.priority
                        || (sample.priority == incumbent.priority
                            && sample.sample_id < incumbent.sample_id)
                    {
                        Some(index)
                    } else {
                        Some(current)
                    }
                }
            };
        }
        let input_index = best_index.ok_or(FusionError::InvalidPlanState)?;
        selected_indices[rank] = input_index;
    }

    let mut planned = [None; MAX_FUSION_CANDIDATES];
    let mut earliest_lower = u64::MAX;
    let mut latest_upper = 0u64;
    for rank in 0..selected_count {
        let input_index = selected_indices[rank];
        let entry = make_entry(
            input_index,
            &samples[input_index],
            target,
            alignments,
            mappings,
        )?;
        earliest_lower = earliest_lower.min(entry.aligned_window.lower_ns);
        latest_upper = latest_upper.max(entry.aligned_window.upper_ns);
        planned[rank] = Some(entry);
    }
    if selected_count > 0 && latest_upper - earliest_lower > policy.maximum_time_span_ns {
        return Err(FusionError::TemporalSpanExceeded);
    }

    // All checks have passed; publish the planned prefix atomically from stack
    // storage. The unused caller-buffer tail is left untouched.
    out[..selected_count].copy_from_slice(&planned[..selected_count]);
    Ok(selected_count)
}

fn validate_unique_ids(samples: &[FusionSample]) -> Result<(), FusionError> {
    for i in 0..samples.len() {
        for j in (i + 1)..samples.len() {
            if samples[i].sample_id == samples[j].sample_id {
                return Err(FusionError::DuplicateSampleId);
            }
        }
    }
    Ok(())
}

fn resolve_mapping(
    source_frame_id: SemanticId,
    target_frame_id: u64,
    mappings: &[CoordinateMapping],
) -> Result<CoordinateMapping, FusionError> {
    let source_frame_id = source_frame_id
        .value()
        .ok_or(FusionError::UnknownCoordinateFrame)?;
    let mut found = None;
    for mapping in mappings {
        if mapping.source_frame_id == source_frame_id && mapping.target_frame_id == target_frame_id
        {
            if found.is_some() {
                return Err(FusionError::AmbiguousCoordinateMapping);
            }
            found = Some(*mapping);
        }
    }
    found.ok_or(FusionError::MissingCoordinateMapping)
}

fn make_entry(
    input_index: usize,
    sample: &FusionSample,
    target: FusionTarget,
    alignments: &[TimeAlignment],
    mappings: &[CoordinateMapping],
) -> Result<FusionPlanEntry, FusionError> {
    let (aligned_window, alignment_id) = align_time(sample.clock, target, alignments)?;
    let mapping = resolve_mapping(
        sample.coordinate_frame_id,
        target.coordinate_frame_id,
        mappings,
    )?;
    Ok(FusionPlanEntry {
        sample_id: sample.sample_id,
        input_index,
        physical: sample.physical,
        aligned_window,
        alignment_id,
        mapping_id: mapping.mapping_id,
        mapping_maximum_error: mapping.maximum_error,
        mapping_error_unit_id: mapping.error_unit_id,
        projection: sample.projection,
        quality: sample.quality,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FusionError {
    InvalidIdentifier,
    InvalidTime,
    UnknownTime,
    UnknownCoordinateFrame,
    MissingTimeAlignment,
    AmbiguousTimeAlignment,
    InvalidTimeAlignment,
    TimeAlignmentOutsideValidity,
    MissingCoordinateMapping,
    AmbiguousCoordinateMapping,
    UnknownMappingErrorUnit,
    InvalidCoordinateMapping,
    InvalidQuality,
    InvalidPolicy,
    ProjectionSemanticMismatch,
    TemporalSpanExceeded,
    TooManyCandidates,
    DuplicateSampleId,
    OutputTooSmall,
    InvalidPlanState,
}

#[cfg(test)]
#[path = "fusion_tests.rs"]
mod tests;
