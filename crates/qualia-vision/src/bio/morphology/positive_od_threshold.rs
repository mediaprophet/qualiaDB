//! Optical-density threshold for positive cell fraction (Ki-67 style index).
//!
//! Brightfield gray → approximate optical density (Beer–Lambert), then count
//! labeled nuclei whose mean OD exceeds a threshold. Returns a fixed-point
//! positive fraction for the index.

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

/// Maximum distinct nucleus labels for OD aggregation.
pub const MAX_OD_LABELS: usize = 512;

/// Result of a positive-OD index computation.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OdPositiveIndex {
    /// Nuclei (non-zero labels) considered.
    pub total_nuclei: u32,
    /// Nuclei with mean OD ≥ threshold.
    pub positive_nuclei: u32,
    /// `positive / total * 65535` (0 if total == 0).
    pub fraction_q16: u16,
    /// Mean OD (0–255 scale) over all nucleus pixels.
    pub mean_od_u8: u8,
}

/// Map brightfield intensity to an 8-bit optical-density-like value.
///
/// `OD ∝ −log10(I/I0)` with `I0 = 255`. Dark stain → high OD.
/// Uses a fixed-point approximation filled at call time (no heap LUT stored).
#[inline]
pub fn intensity_to_od_u8(intensity: u8) -> u8 {
    // Avoid log(0): treat I=0 as max OD.
    if intensity == 0 {
        return 255;
    }
    // od = -log10(I/255) = log10(255) - log10(I)
    // Map to 0..255 with od_max ≈ log10(255) ≈ 2.4065 → scale 255/2.4065 ≈ 106
    let t = intensity as f64 / 255.0;
    let od = -t.log10();
    let scaled = (od * (255.0 / 2.406_540_18)).clamp(0.0, 255.0);
    scaled.round() as u8
}

/// Compute Ki-67-style positive index from a gray channel and nucleus label map.
///
/// - `gray`: brightfield luminance (bright = background).
/// - `labels`: same geometry, 0 = background; non-zero = nucleus id.
/// - `od_thresh_u8`: mean-OD threshold on the 0–255 OD scale.
pub fn positive_od_threshold(
    gray: GrayView<'_>,
    labels: &[u16],
    od_thresh_u8: u8,
    out: &mut OdPositiveIndex,
) -> Result<(), CvError> {
    let w = gray.width as usize;
    let h = gray.height as usize;
    let n = w.checked_mul(h).ok_or(CvError::InvalidParameter)?;
    if n == 0 {
        return Err(CvError::EmptyInput);
    }
    if labels.len() < n {
        return Err(CvError::DimensionMismatch);
    }

    struct Acc {
        label: u16,
        sum_od: u64,
        count: u32,
    }
    let mut accs: [Acc; MAX_OD_LABELS] = core::array::from_fn(|_| Acc {
        label: 0,
        sum_od: 0,
        count: 0,
    });
    let mut used = 0usize;
    let mut global_sum = 0u64;
    let mut global_count = 0u32;

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let lab = labels[i];
            if lab == 0 {
                continue;
            }
            let od = intensity_to_od_u8(gray.pixel(x as u32, y as u32)) as u64;
            global_sum = global_sum.saturating_add(od);
            global_count = global_count.saturating_add(1);

            let mut slot = None;
            for j in 0..used {
                if accs[j].label == lab {
                    slot = Some(j);
                    break;
                }
            }
            let j = match slot {
                Some(j) => j,
                None => {
                    if used >= MAX_OD_LABELS {
                        continue;
                    }
                    let j = used;
                    accs[j].label = lab;
                    accs[j].sum_od = 0;
                    accs[j].count = 0;
                    used += 1;
                    j
                }
            };
            accs[j].sum_od = accs[j].sum_od.saturating_add(od);
            accs[j].count = accs[j].count.saturating_add(1);
        }
    }

    let mut positive = 0u32;
    for j in 0..used {
        if accs[j].count == 0 {
            continue;
        }
        let mean = (accs[j].sum_od / accs[j].count as u64) as u8;
        if mean >= od_thresh_u8 {
            positive = positive.saturating_add(1);
        }
    }

    let total = used as u32;
    let fraction_q16 = if total == 0 {
        0
    } else {
        ((positive as u64 * 65535) / total as u64) as u16
    };
    let mean_od_u8 = if global_count == 0 {
        0
    } else {
        (global_sum / global_count as u64) as u8
    };

    *out = OdPositiveIndex {
        total_nuclei: total,
        positive_nuclei: positive,
        fraction_q16,
        mean_od_u8,
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn od_monotonic_dark_higher() {
        assert!(intensity_to_od_u8(10) > intensity_to_od_u8(200));
        assert_eq!(intensity_to_od_u8(0), 255);
    }

    #[test]
    fn positive_fraction_two_nuclei() {
        // 4x4: label 1 dark (positive), label 2 bright (negative).
        let mut gray = [220u8; 16];
        let mut labels = [0u16; 16];
        // nucleus 1: dark pixels
        for i in [0usize, 1, 4, 5] {
            gray[i] = 20;
            labels[i] = 1;
        }
        // nucleus 2: light pixels
        for i in [10usize, 11, 14, 15] {
            gray[i] = 200;
            labels[i] = 2;
        }
        let v = GrayView::new(4, 4, 4, &gray).unwrap();
        let mut idx = OdPositiveIndex::default();
        // Mid OD threshold should catch dark only.
        positive_od_threshold(v, &labels, 80, &mut idx).unwrap();
        assert_eq!(idx.total_nuclei, 2);
        assert_eq!(idx.positive_nuclei, 1);
        assert!(idx.fraction_q16 > 30_000 && idx.fraction_q16 < 40_000); // ~0.5
    }

    #[test]
    fn no_nuclei_zero_index() {
        let gray = [128u8; 9];
        let labels = [0u16; 9];
        let v = GrayView::new(3, 3, 3, &gray).unwrap();
        let mut idx = OdPositiveIndex {
            total_nuclei: 9,
            positive_nuclei: 9,
            fraction_q16: 1,
            mean_od_u8: 1,
        };
        positive_od_threshold(v, &labels, 10, &mut idx).unwrap();
        assert_eq!(idx.total_nuclei, 0);
        assert_eq!(idx.positive_nuclei, 0);
        assert_eq!(idx.fraction_q16, 0);
    }

    #[test]
    fn dimension_mismatch() {
        let gray = [0u8; 4];
        let v = GrayView::new(2, 2, 2, &gray).unwrap();
        let labels = [1u16; 2];
        let mut idx = OdPositiveIndex::default();
        assert_eq!(
            positive_od_threshold(v, &labels, 1, &mut idx),
            Err(CvError::DimensionMismatch)
        );
    }
}
