//! Apply CT Hounsfield Unit window/level → 8-bit display.

/// Medical imaging helper errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MedicalError {
    EmptyInput,
    InvalidParameter,
    BufferTooSmall,
    DimensionMismatch,
    NonConvergence,
}

impl core::fmt::Display for MedicalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "empty input"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::BufferTooSmall => write!(f, "output buffer too small"),
            Self::DimensionMismatch => write!(f, "dimension mismatch"),
            Self::NonConvergence => write!(f, "unmix did not converge"),
        }
    }
}

/// Window/level on `i16` HU values → `u8` display.
///
/// Display range is `[level - window/2, level + window/2]` mapped linearly to 0..255.
/// Values outside are clamped. `window` must be > 0.
///
/// Returns number of pixels written.
pub fn apply_hu_window_i16(
    hu: &[i16],
    window: f32,
    level: f32,
    out: &mut [u8],
) -> Result<usize, MedicalError> {
    if hu.is_empty() {
        return Err(MedicalError::EmptyInput);
    }
    if !(window > 0.0) || !window.is_finite() || !level.is_finite() {
        return Err(MedicalError::InvalidParameter);
    }
    if out.len() < hu.len() {
        return Err(MedicalError::BufferTooSmall);
    }
    let lo = level - window * 0.5;
    let inv = 255.0 / window;
    for (i, &v) in hu.iter().enumerate() {
        let t = (v as f32 - lo) * inv;
        out[i] = t.clamp(0.0, 255.0) as u8;
    }
    Ok(hu.len())
}

/// Window/level on `f32` intensity (HU or similar) → `u8` display.
pub fn apply_hu_window_f32(
    values: &[f32],
    window: f32,
    level: f32,
    out: &mut [u8],
) -> Result<usize, MedicalError> {
    if values.is_empty() {
        return Err(MedicalError::EmptyInput);
    }
    if !(window > 0.0) || !window.is_finite() || !level.is_finite() {
        return Err(MedicalError::InvalidParameter);
    }
    if out.len() < values.len() {
        return Err(MedicalError::BufferTooSmall);
    }
    let lo = level - window * 0.5;
    let inv = 255.0 / window;
    for (i, &v) in values.iter().enumerate() {
        let t = (v - lo) * inv;
        out[i] = t.clamp(0.0, 255.0) as u8;
    }
    Ok(values.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_tissue_window_center() {
        // window=400, level=40 → range [-160, 240]
        let hu = [-160i16, 40, 240, 1000, -1000];
        let mut out = [0u8; 5];
        apply_hu_window_i16(&hu, 400.0, 40.0, &mut out).unwrap();
        assert_eq!(out[0], 0);
        assert!((out[1] as i16 - 128).abs() <= 1); // mid
        assert_eq!(out[2], 255);
        assert_eq!(out[3], 255); // clamped high
        assert_eq!(out[4], 0); // clamped low
    }

    #[test]
    fn f32_path_matches_scale() {
        let v = [0.0f32, 100.0, 200.0];
        let mut out = [0u8; 3];
        apply_hu_window_f32(&v, 200.0, 100.0, &mut out).unwrap();
        assert_eq!(out[0], 0);
        assert!((out[1] as i16 - 128).abs() <= 1);
        assert_eq!(out[2], 255);
    }

    #[test]
    fn zero_window_rejects() {
        let hu = [0i16];
        let mut out = [0u8; 1];
        assert_eq!(
            apply_hu_window_i16(&hu, 0.0, 0.0, &mut out).unwrap_err(),
            MedicalError::InvalidParameter
        );
    }

    #[test]
    fn buffer_too_small() {
        let hu = [0i16, 1];
        let mut out = [0u8; 1];
        assert_eq!(
            apply_hu_window_i16(&hu, 100.0, 0.0, &mut out).unwrap_err(),
            MedicalError::BufferTooSmall
        );
    }
}
