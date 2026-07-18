//! Anti-alias lowpass cutoff design for sample-rate conversion.
//!
//! When downsampling, any source content above the *destination* Nyquist
//! frequency must be removed before decimation, otherwise it folds back
//! ("aliases") into the audible band as spurious tones. The band-limited
//! resamplers in this module ([`super::windowed_sinc`], [`super::polyphase`])
//! achieve that by convolving with a windowed-sinc lowpass whose cutoff is
//! chosen here.
//!
//! The cutoff is expressed in **cycles per source sample** (a normalised
//! frequency in `[0, 0.5]`, where `0.5` is the source Nyquist):
//!
//! - Upsampling or equal rate → `0.5` (pass the full source band).
//! - Downsampling by ratio `r = dst/src < 1` → `0.5 · r`, i.e. the destination
//!   Nyquist re-expressed in source-sample units. Content above it is rejected.

/// Anti-alias lowpass cutoff in cycles per **source** sample (range `[0, 0.5]`).
///
/// Returns `0.0` for a degenerate zero rate. The consuming resamplers derive
/// their sinc time-scale as `2 · cutoff` (`= min(1, dst/src)`).
pub fn antialias_cutoff(src_rate: u32, dst_rate: u32) -> f32 {
    if src_rate == 0 || dst_rate == 0 {
        return 0.0;
    }
    let ratio = dst_rate as f32 / src_rate as f32;
    0.5 * ratio.min(1.0)
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
