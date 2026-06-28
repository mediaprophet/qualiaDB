use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct AffineParams {
    pub length: u32,
    pub scale: f32,
    pub bias: f32,
    pub _pad: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OracleTolerance {
    pub absolute: f32,
    pub relative: f32,
}

impl Default for OracleTolerance {
    fn default() -> Self {
        Self {
            absolute: 1.0e-6,
            relative: 1.0e-5,
        }
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct OracleCase {
    pub seed: u64,
    pub input: Vec<f32>,
    pub expected: Vec<f32>,
    pub params: AffineParams,
}

impl OracleCase {
    pub fn affine(length: usize, seed: u64, scale: f32, bias: f32) -> Self {
        let length = length.min(u32::MAX as usize);
        let mut state = seed.max(1);
        let mut input = Vec::with_capacity(length);
        for _ in 0..length {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let unit = (state as u32) as f32 / u32::MAX as f32;
            input.push(unit.mul_add(2.0, -1.0));
        }
        let params = AffineParams {
            length: length as u32,
            scale,
            bias,
            _pad: 0,
        };
        let expected = affine_cpu(&input, params);
        Self {
            seed,
            input,
            expected,
            params,
        }
    }
}

pub fn affine_cpu(input: &[f32], params: AffineParams) -> Vec<f32> {
    input
        .iter()
        .take(params.length as usize)
        .map(|value| value.mul_add(params.scale, params.bias))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_vectors_are_reproducible() {
        let first = OracleCase::affine(17, 42, 1.5, -0.25);
        let second = OracleCase::affine(17, 42, 1.5, -0.25);
        assert_eq!(first, second);
        assert!(compare_f32(
            &first.expected,
            &second.expected,
            OracleTolerance::default()
        )
        .passed());
    }

    #[test]
    fn comparator_reports_tail_and_numeric_errors() {
        let report = compare_f32(&[1.0, 2.0, 3.0], &[1.0, 2.1], OracleTolerance::default());
        assert_eq!(report.mismatch_count, 2);
        assert_eq!(report.first_mismatch, Some(1));
    }

    #[test]
    fn nan_never_certifies() {
        assert!(!compare_f32(&[f32::NAN], &[f32::NAN], OracleTolerance::default()).passed());
    }
}
