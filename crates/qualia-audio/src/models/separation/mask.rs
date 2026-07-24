//! Classical spectral masking for source separation. Given a mixture magnitude spectrum and a
//! per-bin soft mask (each element in `[0, 1]`), the separated stem magnitude is the
//! element-wise product `stem = mixture ⊙ mask`. This is the algorithmic core of ideal-ratio /
//! Wiener-style masking; it invents nothing — it only re-weights energy the mixture already
//! contains. Learned mask *estimation* (a demucs/openunmix network) is a separate,
//! `NeedsWeights` concern (see `learned.rs`).
//!
//! Zero-heap: the caller supplies both the mask and the output buffer.

use crate::types::AudioError;

/// Apply a per-bin soft mask to a mixture magnitude spectrum: `out[k] = mixture_mag[k] * mask[k]`.
///
/// - `mixture_mag`: mixture magnitudes (any length `N`).
/// - `mask`: `N` weights, each expected in `[0, 1]` (all-ones passes the mixture through,
///   all-zeros yields silence). Values outside `[0, 1]` are applied as-is (caller's choice).
/// - `out`: at least `N` floats; bins `0..N` are written.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `mask.len() != mixture_mag.len()`.
/// - [`AudioError::OutputBufferTooSmall`] if `out` is shorter than `mixture_mag`.
pub fn apply_soft_mask(
    mixture_mag: &[f32],
    mask: &[f32],
    out: &mut [f32],
) -> Result<(), AudioError> {
    if mask.len() != mixture_mag.len() {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < mixture_mag.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for (o, (&m, &g)) in out.iter_mut().zip(mixture_mag.iter().zip(mask.iter())) {
        *o = m * g;
    }
    Ok(())
}

/// Derive a **binary** mask from two reference magnitude spectra (the "ideal binary mask" when
/// the references are the true stems): bin `k` is assigned to the target (`1.0`) when the target
/// reference has at least as much magnitude there as the other reference, else `0.0`.
///
/// Equivalent to thresholding the ratio `target / (target + other)` at `0.5`, but computed by
/// direct comparison to avoid a divide (and the `0/0` case). Zero-heap.
///
/// - `target_ref`, `other_ref`: reference magnitudes, equal length `N`.
/// - `out_mask`: at least `N` floats; each written as `0.0` or `1.0`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if the two references differ in length.
/// - [`AudioError::OutputBufferTooSmall`] if `out_mask` is shorter than the references.
pub fn binary_mask_from_ratio(
    target_ref: &[f32],
    other_ref: &[f32],
    out_mask: &mut [f32],
) -> Result<(), AudioError> {
    if target_ref.len() != other_ref.len() {
        return Err(AudioError::InvalidParameter);
    }
    if out_mask.len() < target_ref.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for (o, (&t, &n)) in out_mask
        .iter_mut()
        .zip(target_ref.iter().zip(other_ref.iter()))
    {
        *o = if t >= n { 1.0 } else { 0.0 };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ones_mask_passes_mixture_unchanged() {
        let mix = [0.5f32, 1.0, 2.0, 0.25];
        let mask = [1.0f32; 4];
        let mut out = [0.0f32; 4];
        apply_soft_mask(&mix, &mask, &mut out).expect("mask");
        assert_eq!(out, mix);
    }

    #[test]
    fn all_zeros_mask_yields_silence() {
        let mix = [0.5f32, 1.0, 2.0, 0.25];
        let mask = [0.0f32; 4];
        let mut out = [9.0f32; 4];
        apply_soft_mask(&mix, &mask, &mut out).expect("mask");
        assert_eq!(out, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn band_mask_isolates_that_band() {
        // Keep only bins 1..3 (a mid band); zero the rest.
        let mix = [1.0f32, 2.0, 3.0, 4.0, 5.0];
        let mask = [0.0f32, 1.0, 1.0, 0.0, 0.0];
        let mut out = [0.0f32; 5];
        apply_soft_mask(&mix, &mask, &mut out).expect("mask");
        assert_eq!(out, [0.0, 2.0, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn soft_mask_scales_energy() {
        let mix = [4.0f32, 8.0];
        let mask = [0.25f32, 0.5];
        let mut out = [0.0f32; 2];
        apply_soft_mask(&mix, &mask, &mut out).expect("mask");
        assert_eq!(out, [1.0, 4.0]);
    }

    #[test]
    fn binary_mask_assigns_dominant_bins() {
        let target = [1.0f32, 0.2, 0.5, 0.0];
        let other = [0.5f32, 0.8, 0.5, 0.1];
        let mut mask = [0.0f32; 4];
        binary_mask_from_ratio(&target, &other, &mut mask).expect("mask");
        // t>=other: bin0 (1>=0.5), bin2 (0.5>=0.5). bin1 (0.2<0.8), bin3 (0<0.1).
        assert_eq!(mask, [1.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn binary_then_apply_extracts_target_bins() {
        let target = [3.0f32, 0.1, 2.0];
        let other = [1.0f32, 0.9, 0.5];
        let mix = [4.0f32, 1.0, 2.5]; // = target + other
        let mut mask = [0.0f32; 3];
        binary_mask_from_ratio(&target, &other, &mut mask).expect("mask");
        let mut out = [0.0f32; 3];
        apply_soft_mask(&mix, &mask, &mut out).expect("apply");
        // target dominant in bins 0 and 2 -> keep mixture there, drop bin 1.
        assert_eq!(out, [4.0, 0.0, 2.5]);
    }

    #[test]
    fn rejects_length_mismatch_and_short_output() {
        let mix = [1.0f32, 2.0];
        let mask = [1.0f32];
        let mut out = [0.0f32; 2];
        assert_eq!(
            apply_soft_mask(&mix, &mask, &mut out),
            Err(AudioError::InvalidParameter)
        );
        let mask2 = [1.0f32, 1.0];
        let mut small = [0.0f32; 1];
        assert_eq!(
            apply_soft_mask(&mix, &mask2, &mut small),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
