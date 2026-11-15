//! Bounded CPU reference for cutoff-specific alpha coverage at mip levels.

const GPU_ALPHA_ROUNDING_BIAS: f32 = 2.0 / 255.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AlphaCoveragePlan {
    pub scales: [f32; 32],
    pub rounding_biases: [f32; 32],
    /// Covered fraction of the uncorrected base mip at the authored cutoff.
    pub base_coverage: f32,
    /// Signed covered-fraction difference from mip zero after RGBA8 quantization.
    pub residuals: [f32; 32],
    pub level_count: usize,
}

impl Default for AlphaCoveragePlan {
    fn default() -> Self {
        Self {
            scales: [1.0; 32],
            rounding_biases: [0.0; 32],
            base_coverage: 0.0,
            residuals: [0.0; 32],
            level_count: 1,
        }
    }
}

/// Derive deterministic alpha multipliers and coverage residuals for a complete mip chain.
///
/// The base-level covered fraction is retained as closely as each quantized mip's discrete pixel
/// count allows. Scratch contains alpha only and is bounded by the supplied image dimensions.
pub(super) fn alpha_coverage_plan(
    rgba8: &[u8],
    width: u32,
    height: u32,
    cutoff: f32,
) -> AlphaCoveragePlan {
    let mut plan = AlphaCoveragePlan::default();
    if width == 0 || height == 0 || !cutoff.is_finite() || cutoff <= 0.0 || cutoff > 1.0 {
        return plan;
    }
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return plan;
    };
    if rgba8.len() != pixel_count.saturating_mul(4) {
        return plan;
    }
    let mut current: Vec<u8> = rgba8.chunks_exact(4).map(|pixel| pixel[3]).collect();
    let base_covered = current
        .iter()
        .filter(|&&alpha| f32::from(alpha) / 255.0 >= cutoff)
        .count();
    let base_coverage = base_covered as f64 / current.len() as f64;
    plan.base_coverage = base_coverage as f32;
    let (mut source_width, mut source_height) = (width, height);
    let mut level = 1usize;
    while (source_width > 1 || source_height > 1) && level < plan.scales.len() {
        let dest_width = (source_width / 2).max(1);
        let dest_height = (source_height / 2).max(1);
        let mut raw = Vec::with_capacity(dest_width as usize * dest_height as usize);
        let source_scale = [
            source_width as f32 / dest_width as f32,
            source_height as f32 / dest_height as f32,
        ];
        for y in 0..dest_height {
            for x in 0..dest_width {
                let begin_x = x as f32 * source_scale[0];
                let end_x = (x + 1) as f32 * source_scale[0];
                let begin_y = y as f32 * source_scale[1];
                let end_y = (y + 1) as f32 * source_scale[1];
                let x0 = begin_x.floor() as u32;
                let x1 = end_x.ceil() as u32;
                let y0 = begin_y.floor() as u32;
                let y1 = end_y.ceil() as u32;
                let (mut sum, mut weight_sum) = (0.0f32, 0.0f32);
                for sy in y0..y1 {
                    for sx in x0..x1 {
                        let overlap_x =
                            (end_x.min((sx + 1) as f32) - begin_x.max(sx as f32)).max(0.0);
                        let overlap_y =
                            (end_y.min((sy + 1) as f32) - begin_y.max(sy as f32)).max(0.0);
                        let weight = overlap_x * overlap_y;
                        let source_index = (sy * source_width + sx) as usize;
                        sum += (f32::from(current[source_index]) / 255.0) * weight;
                        weight_sum += weight;
                    }
                }
                raw.push(if weight_sum > 0.0 {
                    sum / weight_sum
                } else {
                    0.0
                });
            }
        }
        let target_covered = base_coverage * raw.len() as f64;
        let level_bias =
            if raw.iter().all(|&alpha| alpha == 0.0) || raw.iter().all(|&alpha| alpha == 1.0) {
                0.0
            } else {
                GPU_ALPHA_ROUNDING_BIAS
            };
        plan.rounding_biases[level] = level_bias;
        // Preserve mixed-alpha coverage one UNORM code above the authored cutoff. The GPU
        // shader's two-code bias offsets sRGB target rounding measured on the GL adapter.
        let planning_cutoff = (cutoff + if level_bias > 0.0 { 1.0 / 255.0 } else { 0.0 }).min(1.0);
        let mut low = 0.0f32;
        let mut high = raw
            .iter()
            .copied()
            .filter(|&alpha| alpha > 0.0)
            .map(|alpha| ((planning_cutoff - level_bias).max(0.0)) / alpha)
            .fold(1.0f32, f32::max)
            .min(1.0e12);
        for _ in 0..24 {
            let mid = (low + high) * 0.5;
            let covered = quantized_coverage_count(&raw, mid, planning_cutoff, level_bias) as f64;
            if covered > target_covered {
                high = mid;
            } else {
                low = mid;
            }
        }
        let low_error = (quantized_coverage_count(&raw, low, planning_cutoff, level_bias) as f64
            - target_covered)
            .abs();
        let high_error = (quantized_coverage_count(&raw, high, planning_cutoff, level_bias) as f64
            - target_covered)
            .abs();
        let correction = if low_error < high_error {
            low
        } else if high_error < low_error {
            high
        } else if (low - 1.0).abs() <= (high - 1.0).abs() {
            low
        } else {
            high
        };
        plan.scales[level] = correction;
        current = raw
            .into_iter()
            .map(|alpha| ((alpha * correction + level_bias).clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();
        let actual_coverage = current
            .iter()
            .filter(|&&alpha| f32::from(alpha) / 255.0 >= cutoff)
            .count() as f64
            / current.len() as f64;
        plan.residuals[level] = (actual_coverage - base_coverage) as f32;
        source_width = dest_width;
        source_height = dest_height;
        level += 1;
    }
    plan.level_count = level;
    plan
}

fn quantized_coverage_count(alpha: &[f32], scale: f32, cutoff: f32, bias: f32) -> usize {
    alpha
        .iter()
        .filter(|&&value| {
            let quantized = ((value * scale + bias).clamp(0.0, 1.0) * 255.0).round() as u8;
            f32::from(quantized) / 255.0 >= cutoff
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_coverage_scale_is_deterministic_for_non_power_of_two_chain() {
        let mut pixels = vec![0u8; 5 * 3 * 4];
        // Three fifths of the base texels survive the 0.5 material cutoff.
        for x in 0..3 {
            for y in 0..3 {
                pixels[(y * 5 + x) * 4 + 3] = 255;
            }
        }
        let first = alpha_coverage_plan(&pixels, 5, 3, 0.5);
        let second = alpha_coverage_plan(&pixels, 5, 3, 0.5);
        assert_eq!(first, second);
        assert_eq!(first.level_count, 3);
        assert!(first.scales[1].is_finite() && first.scales[1] > 0.0);
        assert!(first.scales[2].is_finite() && first.scales[2] > 0.0);
        assert!(first.residuals[1].is_finite());
        assert!(first.residuals[2].is_finite());
    }

    #[test]
    fn alpha_coverage_scale_uses_material_cutoff() {
        let mut pixels = vec![0u8; 16 * 4];
        for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
            pixel[3] = if index % 4 == 3 { 0 } else { 255 };
        }
        let at_half = alpha_coverage_plan(&pixels, 4, 4, 0.5);
        let at_point_nine = alpha_coverage_plan(&pixels, 4, 4, 0.9);
        assert_ne!(
            at_half.scales[1].to_bits(),
            at_point_nine.scales[1].to_bits()
        );
        assert!(at_point_nine.scales[1] > at_half.scales[1]);
        assert!((at_half.base_coverage - 0.75).abs() < f32::EPSILON);
        assert_eq!(at_half.residuals[1], 0.25);
        assert_eq!(at_half.residuals[2], 0.25);
    }

    #[test]
    fn invalid_or_single_texel_input_has_neutral_scale() {
        assert_eq!(
            alpha_coverage_plan(&[], 0, 1, 0.5),
            AlphaCoveragePlan::default()
        );
        assert_eq!(
            alpha_coverage_plan(&[], 1, 1, 0.5),
            AlphaCoveragePlan::default()
        );
        assert_eq!(
            alpha_coverage_plan(&[], 2, 2, f32::NAN),
            AlphaCoveragePlan::default()
        );
    }

    #[test]
    fn opaque_and_transparent_extremes_keep_neutral_coverage() {
        for alpha in [0u8, 255u8] {
            let mut rgba = vec![0u8; 7 * 5 * 4];
            for pixel in rgba.chunks_exact_mut(4) {
                pixel[3] = alpha;
            }
            let plan = alpha_coverage_plan(&rgba, 7, 5, 0.5);
            assert_eq!(plan.level_count, 3);
            assert!(plan.scales[..plan.level_count]
                .iter()
                .all(|&scale| (scale - 1.0).abs() < 1e-5));
            assert!(plan.residuals[..plan.level_count]
                .iter()
                .all(|&residual| residual.abs() < 1e-6));
        }
    }
}
