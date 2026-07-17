//! Reinhard stain normalization in CIE Lab (D65).
//!
//! Match source per-channel Lab mean/std to a target (caller-supplied or H&E defaults).
//! Operates on packed RGB8 frames; caller owns output RGB buffer.

use super::lab_to_rgb::lab_f32_to_rgb_u8;
use super::rgb_to_lab::rgb_u8_to_lab_f32;
use super::HistoError;

/// Lab mean (L, a, b).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabStats {
    pub mean: [f32; 3],
    pub std: [f32; 3],
}

/// Approximate H&E appearance targets in CIE Lab (D65).
///
/// Honest defaults for a typical hematoxylin–eosin look when no reference slide
/// is available; pass your own `LabStats` from a gold-standard tile when you have one.
pub const DEFAULT_HE_TARGET_MEAN: [f32; 3] = [55.240, 14.850, -9.870];
/// Companion std for [`DEFAULT_HE_TARGET_MEAN`].
pub const DEFAULT_HE_TARGET_STD: [f32; 3] = [15.320, 9.480, 9.650];

/// Compute Lab mean/std over packed RGB8 (interleaved).
pub fn lab_stats_of_rgb(rgb: &[u8]) -> Result<LabStats, HistoError> {
    if rgb.is_empty() || rgb.len() % 3 != 0 {
        return Err(if rgb.is_empty() {
            HistoError::EmptyInput
        } else {
            HistoError::InvalidParameter
        });
    }
    let n = (rgb.len() / 3) as f64;
    let mut sum = [0.0f64; 3];
    let mut sum_sq = [0.0f64; 3];
    for i in 0..(rgb.len() / 3) {
        let base = i * 3;
        let (l, a, b) = rgb_u8_to_lab_f32(rgb[base], rgb[base + 1], rgb[base + 2]);
        let v = [l as f64, a as f64, b as f64];
        for c in 0..3 {
            sum[c] += v[c];
            sum_sq[c] += v[c] * v[c];
        }
    }
    let mut mean = [0.0f32; 3];
    let mut std = [0.0f32; 3];
    for c in 0..3 {
        let m = sum[c] / n;
        let var = (sum_sq[c] / n - m * m).max(0.0);
        mean[c] = m as f32;
        std[c] = (var.sqrt() as f32).max(1e-6);
    }
    Ok(LabStats { mean, std })
}

/// Reinhard-normalize packed RGB8 toward `target` Lab stats (or H&E defaults if `None`).
///
/// `out` length ≥ `rgb.len()`. In-place is allowed if `out` and `rgb` alias the same slice
/// only when they are the same buffer via separate non-overlapping paths — prefer distinct buffers.
pub fn reinhard_normalize(
    rgb: &[u8],
    target: Option<LabStats>,
    out: &mut [u8],
) -> Result<LabStats, HistoError> {
    if rgb.is_empty() {
        return Err(HistoError::EmptyInput);
    }
    if rgb.len() % 3 != 0 {
        return Err(HistoError::InvalidParameter);
    }
    if out.len() < rgb.len() {
        return Err(HistoError::BufferTooSmall);
    }
    let src = lab_stats_of_rgb(rgb)?;
    let tgt = target.unwrap_or(LabStats {
        mean: DEFAULT_HE_TARGET_MEAN,
        std: DEFAULT_HE_TARGET_STD,
    });
    // Avoid divide-by-zero / explosion on flat tiles.
    let mut scale = [0.0f32; 3];
    for c in 0..3 {
        scale[c] = tgt.std[c] / src.std[c].max(1e-6);
    }
    let n = rgb.len() / 3;
    for i in 0..n {
        let base = i * 3;
        let (l, a, b) = rgb_u8_to_lab_f32(rgb[base], rgb[base + 1], rgb[base + 2]);
        let lab = [l, a, b];
        let mut mapped = [0.0f32; 3];
        for c in 0..3 {
            mapped[c] = (lab[c] - src.mean[c]) * scale[c] + tgt.mean[c];
        }
        let (r, g, bb) = lab_f32_to_rgb_u8(mapped[0], mapped[1], mapped[2]);
        out[base] = r;
        out[base + 1] = g;
        out[base + 2] = bb;
    }
    Ok(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_biased_tile() -> Vec<u8> {
        // Reddish-pink H&E-ish patch: higher R, mid B.
        let mut v = Vec::with_capacity(16 * 16 * 3);
        for y in 0..16u32 {
            for x in 0..16u32 {
                let r = (180 + (x % 40)) as u8;
                let g = (80 + (y % 30)) as u8;
                let b = (120 + ((x + y) % 20)) as u8;
                v.push(r);
                v.push(g);
                v.push(b);
            }
        }
        v
    }

    #[test]
    fn reinhard_moves_stats_toward_target() {
        let src = make_biased_tile();
        let target = LabStats {
            mean: DEFAULT_HE_TARGET_MEAN,
            std: DEFAULT_HE_TARGET_STD,
        };
        let before = lab_stats_of_rgb(&src).unwrap();
        let mut out = vec![0u8; src.len()];
        reinhard_normalize(&src, Some(target), &mut out).unwrap();
        let after = lab_stats_of_rgb(&out).unwrap();
        // Mean of each Lab channel should move closer to the target.
        for c in 0..3 {
            let err_before = (before.mean[c] - target.mean[c]).abs();
            let err_after = (after.mean[c] - target.mean[c]).abs();
            assert!(
                err_after < err_before + 0.5,
                "channel {c}: before err={err_before} after err={err_after} (src mean {} → {})",
                before.mean[c],
                after.mean[c]
            );
            // Tight: after mean near target (float/quantize residual).
            assert!(
                err_after < 3.0,
                "channel {c} mean not near target: got {} want {}",
                after.mean[c],
                target.mean[c]
            );
        }
    }

    #[test]
    fn default_he_target_is_used() {
        let src = make_biased_tile();
        let mut out = vec![0u8; src.len()];
        reinhard_normalize(&src, None, &mut out).unwrap();
        let after = lab_stats_of_rgb(&out).unwrap();
        assert!((after.mean[0] - DEFAULT_HE_TARGET_MEAN[0]).abs() < 3.0);
    }
}
