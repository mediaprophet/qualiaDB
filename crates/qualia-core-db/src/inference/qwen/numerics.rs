//! Small caller-buffered numerical kernels used by the Qwen4Exp decoder.
//!
//! These are deliberately independent of storage.  A layer executor obtains
//! quantized rows from the C:/E: readers, then applies these deterministic
//! vector kernels without allocating in the decode loop.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QwenNumericError {
    BufferTooSmall,
    InvalidEpsilon,
}

impl core::fmt::Display for QwenNumericError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Qwen numeric caller buffer is too small"),
            Self::InvalidEpsilon => write!(f, "Qwen RMSNorm epsilon must be finite and positive"),
        }
    }
}

impl std::error::Error for QwenNumericError {}

/// Apply Qwen RMSNorm with a separately stored scale vector.
pub fn rms_norm_into(
    input: &[f32],
    weight: &[f32],
    epsilon: f32,
    out: &mut [f32],
) -> Result<(), QwenNumericError> {
    if weight.len() < input.len() || out.len() < input.len() {
        return Err(QwenNumericError::BufferTooSmall);
    }
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return Err(QwenNumericError::InvalidEpsilon);
    }
    let mut squared_sum = 0.0f32;
    for &value in input {
        squared_sum += value * value;
    }
    let inv_rms = 1.0 / (squared_sum / input.len().max(1) as f32 + epsilon).sqrt();
    for index in 0..input.len() {
        out[index] = input[index] * inv_rms * weight[index];
    }
    Ok(())
}

/// Add `update` into `state` in place, avoiding a temporary residual vector.
pub fn add_assign(state: &mut [f32], update: &[f32]) -> Result<(), QwenNumericError> {
    if update.len() < state.len() {
        return Err(QwenNumericError::BufferTooSmall);
    }
    for index in 0..state.len() {
        state[index] += update[index];
    }
    Ok(())
}

/// Copy a supplied vector into an output buffer and add a second vector.  This
/// is useful where the decoder must preserve a pre-norm residual separately.
pub fn add_into(base: &[f32], update: &[f32], out: &mut [f32]) -> Result<(), QwenNumericError> {
    if update.len() < base.len() || out.len() < base.len() {
        return Err(QwenNumericError::BufferTooSmall);
    }
    for index in 0..base.len() {
        out[index] = base[index] + update[index];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_norm_is_scale_aware() {
        let mut out = [0.0f32; 2];
        rms_norm_into(&[3.0, 4.0], &[1.0, 2.0], 1e-6, &mut out).unwrap();
        assert!((out[0] - 0.8485).abs() < 0.001);
        assert!((out[1] - 2.2627).abs() < 0.001);
    }
}
