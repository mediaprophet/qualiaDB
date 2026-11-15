//! Anti-alias lowpass cutoff design for sample-rate conversion.
//!
//! Delegated to [`etalon_signal::resample::antialias_cutoff_cycles`].

/// Anti-alias lowpass cutoff in cycles per **source** sample (range `[0, 0.5]`).
///
/// Returns `0.0` for a degenerate zero rate. The consuming resamplers derive
/// their sinc time-scale as `2 · cutoff` (`= min(1, dst/src)`).
#[inline]
pub fn antialias_cutoff(src_rate: u32, dst_rate: u32) -> f32 {
    etalon_signal::resample::antialias_cutoff_cycles(src_rate, dst_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsample_passes_full_band() {
        assert!((antialias_cutoff(8000, 48000) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn equal_rate_passes_full_band() {
        assert!((antialias_cutoff(44100, 44100) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn downsample_lowers_cutoff_to_dst_nyquist() {
        // 48k → 8k: cutoff = 0.5 · 8000/48000 = 0.08333… cycles/source-sample.
        let c = antialias_cutoff(48000, 8000);
        assert!((c - (0.5 * 8000.0 / 48000.0)).abs() < 1e-6, "got {c}");
        assert!(c < 0.5);
    }

    #[test]
    fn zero_rate_is_zero() {
        assert_eq!(antialias_cutoff(0, 8000), 0.0);
        assert_eq!(antialias_cutoff(8000, 0), 0.0);
    }
}
