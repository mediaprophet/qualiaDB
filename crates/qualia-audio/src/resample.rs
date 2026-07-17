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
    if src_rate == dst_rate {
        let n = src.len().min(out.len());
        out[..n].copy_from_slice(&src[..n]);
        return Ok(n);
    }
    let out_n = ((src.len() as u64 * dst_rate as u64) / src_rate as u64) as usize;
    let out_n = out_n.min(out.len());
    if out_n == 0 {
        return Ok(0);
    }
    let ratio = src_rate as f64 / dst_rate as f64;
    for i in 0..out_n {
        let pos = i as f64 * ratio;
        let i0 = pos.floor() as usize;
        let frac = (pos - i0 as f64) as f32;
        let s0 = src.get(i0).copied().unwrap_or(0.0);
        let s1 = src.get(i0 + 1).copied().unwrap_or(s0);
        out[i] = s0 * (1.0 - frac) + s1 * frac;
    }
    Ok(out_n)
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
