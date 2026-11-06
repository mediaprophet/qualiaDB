//! Linear resampler (cold/simple; declared latency 0 for linear interp).

use crate::types::AudioError;

/// Resample mono f32 `src` at `src_rate` → `dst_rate` into `out`.
/// Returns number of output frames written.
pub fn resample_linear_mono(
    src: &[f32],
    src_rate: u32,
    dst_rate: u32,
    out: &mut [f32],
) -> Result<usize, AudioError> {
    if src_rate == 0 || dst_rate == 0 || src.is_empty() {
        return Err(AudioError::MalformedAudio);
    }
    Ok(etalon_signal::resample::resample_linear_mono_into(
        src, src_rate, dst_rate, out,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_rate() {
        let s = [0.0f32, 1.0, 0.0];
        let mut o = [0.0f32; 3];
        let n = resample_linear_mono(&s, 16000, 16000, &mut o).unwrap();
        assert_eq!(n, 3);
        assert_eq!(o, s);
    }

    #[test]
    fn upsample_len() {
        let s = [0.0f32; 10];
        let mut o = [0.0f32; 40];
        let n = resample_linear_mono(&s, 8000, 16000, &mut o).unwrap();
        assert_eq!(n, 20);
    }
}
