//! Differential-oracle comparison result types and the numeric comparator, plus the
//! [`GpuEvaluation`] evidence bundle that every evaluator returns.

use serde::{Deserialize, Serialize};

use super::params::OracleTolerance;
use crate::wgsl_forge::{AdapterConstraints, AdapterIdentity, TimingSummary};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub compared: usize,
    pub mismatch_count: usize,
    pub first_mismatch: Option<usize>,
    pub max_absolute_error: f32,
    pub max_relative_error: f32,
}

impl ComparisonReport {
    pub const fn passed(&self) -> bool {
        self.mismatch_count == 0
    }
}

pub fn compare_f32(
    expected: &[f32],
    actual: &[f32],
    tolerance: OracleTolerance,
) -> ComparisonReport {
    let compared = expected.len().max(actual.len());
    let mut mismatch_count = expected.len().abs_diff(actual.len());
    let mut first_mismatch =
        (expected.len() != actual.len()).then(|| expected.len().min(actual.len()));
    let mut max_absolute_error = 0.0f32;
    let mut max_relative_error = 0.0f32;

    for (index, (&expected, &actual)) in expected.iter().zip(actual.iter()).enumerate() {
        let (matches, absolute, relative) = compare_value(expected, actual, tolerance);
        max_absolute_error = max_absolute_error.max(absolute);
        max_relative_error = max_relative_error.max(relative);
        if !matches {
            mismatch_count += 1;
            if first_mismatch
                .map(|current| index < current)
                .unwrap_or(true)
            {
                first_mismatch = Some(index);
            }
        }
    }

    ComparisonReport {
        compared,
        mismatch_count,
        first_mismatch,
        max_absolute_error,
        max_relative_error,
    }
}

fn compare_value(expected: f32, actual: f32, tolerance: OracleTolerance) -> (bool, f32, f32) {
    if expected.is_nan() || actual.is_nan() {
        return (false, f32::INFINITY, f32::INFINITY);
    }
    if expected.is_infinite() || actual.is_infinite() {
        return (
            expected.to_bits() == actual.to_bits(),
            if expected.to_bits() == actual.to_bits() {
                0.0
            } else {
                f32::INFINITY
            },
            f32::INFINITY,
        );
    }
    let absolute = (expected - actual).abs();
    let denominator = expected.abs().max(actual.abs()).max(f32::MIN_POSITIVE);
    let relative = absolute / denominator;
    (
        absolute <= tolerance.absolute || relative <= tolerance.relative,
        absolute,
        relative,
    )
}

#[derive(Debug, Clone)]
pub struct GpuEvaluation {
    pub adapter: AdapterIdentity,
    pub constraints: AdapterConstraints,
    pub oracle: ComparisonReport,
    pub timing: TimingSummary,
    pub samples_ns: Vec<u64>,
    /// The exact correctness tolerance (absolute, relative) this kernel's GPU
    /// result was verified against, recorded so it can be folded into the reuse
    /// cache key (plan §8) — a coarser tolerance must not silently reuse evidence.
    pub tolerance: (f32, f32),
    /// Deterministic seed of the test vector this run was checked against, when the
    /// vector is seed-derived (`None` for fixed-scene kernels like ray-probe).
    pub vector_seed: Option<u64>,
    /// blake3 hex of the expected CPU-reference output bytes — pins the vector.
    pub vector_hash: String,
}

/// blake3 hex of a `&[f32]` expected vector's little-endian bytes. Used to record
/// exactly which CPU-reference output a certified manifest was checked against.
pub(super) fn vector_hash_f32(expected: &[f32]) -> String {
    blake3::hash(bytemuck::cast_slice(expected))
        .to_hex()
        .to_string()
}
