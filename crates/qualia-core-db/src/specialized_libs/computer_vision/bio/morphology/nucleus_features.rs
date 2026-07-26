//! Per-label nucleus geometry: area, circularity, axis ratio (PCA of region points).
//!
//! Caller supplies a label map (`u16`, 0 = background) and a fixed `NucleusFeature`
//! output array. Returns the number of distinct non-zero labels written (capped
//! by `out.len()` and `MAX_NUCLEUS_LABELS`).

use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Maximum distinct labels tracked in one pass (stack/table bound).
pub const MAX_NUCLEUS_LABELS: usize = 512;

/// Fixed-layout region feature (no heap).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NucleusFeature {
    pub label: u16,
    pub area: u32,
    /// Circularity × 1000 (`4πA / P²`). Perfect disk ≈ 1000.
    pub circularity_x1000: u16,
    /// Major/minor axis ratio × 1000 from 2D PCA eigenvalues (disk ≈ 1000).
    pub axis_ratio_x1000: u16,
    /// Centroid x (pixel units).
    pub cx: u32,
    /// Centroid y (pixel units).
    pub cy: u32,
}

#[derive(Clone, Copy)]
struct Acc {
    label: u16,
    area: u32,
    sum_x: u64,
    sum_y: u64,
    sum_xx: u64,
    sum_yy: u64,
    sum_xy: i64,
    perimeter: u32,
}

impl Default for Acc {
    fn default() -> Self {
        Self {
            label: 0,
            area: 0,
            sum_x: 0,
            sum_y: 0,
            sum_xx: 0,
            sum_yy: 0,
            sum_xy: 0,
            perimeter: 0,
        }
    }
}

/// Extract geometric features for each labeled nucleus region.
///
/// `labels` length must be `width * height` (row-major, no stride).
pub fn nucleus_features(
    labels: &[u16],
    width: u32,
    height: u32,
    out: &mut [NucleusFeature],
) -> Result<usize, CvError> {
    let w = width as usize;
    let h = height as usize;
    let n = w.checked_mul(h).ok_or(CvError::InvalidParameter)?;
    if n == 0 {
        return Err(CvError::EmptyInput);
    }
    if labels.len() < n {
        return Err(CvError::BufferTooSmall);
    }
    if out.is_empty() {
        return Ok(0);
    }

    // Sparse label → slot map via linear scan of a small table (bounded).
    let mut accs = [Acc::default(); MAX_NUCLEUS_LABELS];
    let mut used = 0usize;

    let find_or_insert =
        |lab: u16, accs: &mut [Acc; MAX_NUCLEUS_LABELS], used: &mut usize| -> Option<usize> {
            for i in 0..*used {
                if accs[i].label == lab {
                    return Some(i);
                }
            }
            if *used >= MAX_NUCLEUS_LABELS {
                return None;
            }
            let i = *used;
            accs[i] = Acc {
                label: lab,
                ..Acc::default()
            };
            *used += 1;
            Some(i)
        };

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let lab = labels[i];
            if lab == 0 {
                continue;
            }
            let Some(slot) = find_or_insert(lab, &mut accs, &mut used) else {
                continue;
            };
            let a = &mut accs[slot];
            a.area = a.area.saturating_add(1);
            a.sum_x = a.sum_x.saturating_add(x as u64);
            a.sum_y = a.sum_y.saturating_add(y as u64);
            a.sum_xx = a.sum_xx.saturating_add((x as u64) * (x as u64));
            a.sum_yy = a.sum_yy.saturating_add((y as u64) * (y as u64));
            a.sum_xy = a.sum_xy.saturating_add((x as i64) * (y as i64));

            // 4-neighborhood perimeter: edge if neighbor missing or different label.
            for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    a.perimeter = a.perimeter.saturating_add(1);
                    continue;
                }
                let ni = ny as usize * w + nx as usize;
                if labels[ni] != lab {
                    a.perimeter = a.perimeter.saturating_add(1);
                }
            }
        }
    }

    let write_n = used.min(out.len());
    for i in 0..write_n {
        let a = &accs[i];
        let area = a.area.max(1);
        let cx = (a.sum_x / area as u64) as u32;
        let cy = (a.sum_y / area as u64) as u32;

        // Circularity = 4πA / P²
        let p = a.perimeter.max(1) as f64;
        let circ = (4.0 * core::f64::consts::PI * (area as f64) / (p * p)).clamp(0.0, 2.0);
        let circularity_x1000 = (circ * 1000.0).round().clamp(0.0, 65535.0) as u16;

        // Covariance / PCA eigenvalues for axis ratio.
        let n_f = area as f64;
        let mean_x = a.sum_x as f64 / n_f;
        let mean_y = a.sum_y as f64 / n_f;
        let cxx = a.sum_xx as f64 / n_f - mean_x * mean_x;
        let cyy = a.sum_yy as f64 / n_f - mean_y * mean_y;
        let cxy = a.sum_xy as f64 / n_f - mean_x * mean_y;
        // Eigenvalues of [[cxx,cxy],[cxy,cyy]]
        let tr = cxx + cyy;
        let det = cxx * cyy - cxy * cxy;
        let disc = (tr * tr * 0.25 - det).max(0.0).sqrt();
        let l1 = tr * 0.5 + disc;
        let l2 = (tr * 0.5 - disc).max(1e-12);
        let ratio = if l1 > l2 { (l1 / l2).sqrt() } else { 1.0 };
        let axis_ratio_x1000 = (ratio * 1000.0).round().clamp(0.0, 65535.0) as u16;

        out[i] = NucleusFeature {
            label: a.label,
            area: a.area,
            circularity_x1000,
            axis_ratio_x1000,
            cx,
            cy,
        };
    }

    // Clear unused slots if any expectation of defaults.
    for i in write_n..out.len() {
        out[i] = NucleusFeature::default();
    }

    Ok(write_n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_region_features() {
        // 8x8 with a 4x4 square labeled 1.
        let mut labels = [0u16; 64];
        for y in 2..6 {
            for x in 2..6 {
                labels[y * 8 + x] = 1;
            }
        }
        let mut feats = [NucleusFeature::default(); 4];
        let n = nucleus_features(&labels, 8, 8, &mut feats).unwrap();
        assert_eq!(n, 1);
        assert_eq!(feats[0].label, 1);
        assert_eq!(feats[0].area, 16);
        assert_eq!(feats[0].cx, 3); // mean of 2,3,4,5
        assert_eq!(feats[0].cy, 3);
        // Square is reasonably circular but not a disk; circ < 1000, > 0.
        assert!(feats[0].circularity_x1000 > 200);
        assert!(feats[0].axis_ratio_x1000 >= 900); // near isotropic
    }

    #[test]
    fn two_labels() {
        let mut labels = [0u16; 16];
        labels[0] = 1;
        labels[1] = 1;
        labels[15] = 2;
        let mut feats = [NucleusFeature::default(); 4];
        let n = nucleus_features(&labels, 4, 4, &mut feats).unwrap();
        assert_eq!(n, 2);
        let areas: Vec<u32> = feats[..n].iter().map(|f| f.area).collect();
        assert!(areas.contains(&2));
        assert!(areas.contains(&1));
    }

    #[test]
    fn empty_labels() {
        let labels = [0u16; 9];
        let mut feats = [NucleusFeature::default(); 2];
        let n = nucleus_features(&labels, 3, 3, &mut feats).unwrap();
        assert_eq!(n, 0);
    }
}
