//! Particle features from binary / labelled masks — centroids for Crocker–Grier linking.
//!
//! Pure Rust. Connected components on `u8` intensity (threshold or pre-labelled).
//! Centroid list is caller-buffered (`&mut [ParticleCentroid]`).
//!
//! Cold path may allocate a visit mask / BFS queue (bounded by image size); public
//! output is fixed-capacity and suitable for the particle linker.

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Maximum particles reported from a single frame (stack-friendly bound for callers).
pub const MAX_PARTICLES_PER_FRAME: usize = 512;

/// 2D particle feature used by [`super::crocker_grier_link`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleCentroid {
    /// Sub-pixel centroid X (image coords, origin top-left).
    pub x: f32,
    /// Sub-pixel centroid Y.
    pub y: f32,
    /// Frame index (set by caller when packing multi-frame inputs).
    pub frame: u32,
    /// Connected-component area in pixels.
    pub area: u32,
    /// Optional label id when source is a labelled mask (0 = threshold path).
    pub label: u32,
}

impl ParticleCentroid {
    pub const EMPTY: Self = Self {
        x: 0.0,
        y: 0.0,
        frame: 0,
        area: 0,
        label: 0,
    };
}

impl Default for ParticleCentroid {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Extract centroids of connected components where pixel ≥ `thresh` (4-connected).
///
/// Writes up to `out.len()` particles. Returns the count written.
/// When the buffer fills, remaining blobs are skipped (deterministic raster order).
pub fn centroids_from_binary(
    src: GrayView<'_>,
    thresh: u8,
    frame: u32,
    out: &mut [ParticleCentroid],
) -> Result<usize, CvError> {
    if out.is_empty() {
        return Ok(0);
    }
    let w = src.width as usize;
    let h = src.height as usize;
    if w == 0 || h == 0 {
        return Ok(0);
    }

    let mut seen = vec![false; w * h];
    let mut n = 0usize;

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if seen[i] || src.pixel(x as u32, y as u32) < thresh {
                continue;
            }
            if n >= out.len() {
                return Ok(n);
            }

            // BFS flood — accumulate mass moments for centroid.
            let mut q = vec![(x, y)];
            seen[i] = true;
            let mut sum_x = 0u64;
            let mut sum_y = 0u64;
            let mut area = 0u32;

            while let Some((cx, cy)) = q.pop() {
                area = area.saturating_add(1);
                sum_x = sum_x.saturating_add(cx as u64);
                sum_y = sum_y.saturating_add(cy as u64);
                for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let nx = nx as usize;
                    let ny = ny as usize;
                    let ni = ny * w + nx;
                    if seen[ni] || src.pixel(nx as u32, ny as u32) < thresh {
                        continue;
                    }
                    seen[ni] = true;
                    q.push((nx, ny));
                }
            }

            if area == 0 {
                continue;
            }
            out[n] = ParticleCentroid {
                x: sum_x as f32 / area as f32,
                y: sum_y as f32 / area as f32,
                frame,
                area,
                label: 0,
            };
            n += 1;
        }
    }
    Ok(n)
}

/// Extract centroids from a pre-labelled `u8` mask (label 0 = background).
///
/// Each non-zero grey value is treated as a distinct label class; connected
/// components of the same label are separate particles. Labels > 0 only.
pub fn centroids_from_labels(
    labels: GrayView<'_>,
    frame: u32,
    out: &mut [ParticleCentroid],
) -> Result<usize, CvError> {
    if out.is_empty() {
        return Ok(0);
    }
    let w = labels.width as usize;
    let h = labels.height as usize;
    if w == 0 || h == 0 {
        return Ok(0);
    }

    let mut seen = vec![false; w * h];
    let mut n = 0usize;

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let lab = labels.pixel(x as u32, y as u32);
            if seen[i] || lab == 0 {
                continue;
            }
            if n >= out.len() {
                return Ok(n);
            }

            let mut q = vec![(x, y)];
            seen[i] = true;
            let mut sum_x = 0u64;
            let mut sum_y = 0u64;
            let mut area = 0u32;

            while let Some((cx, cy)) = q.pop() {
                area = area.saturating_add(1);
                sum_x = sum_x.saturating_add(cx as u64);
                sum_y = sum_y.saturating_add(cy as u64);
                for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let nx = nx as usize;
                    let ny = ny as usize;
                    let ni = ny * w + nx;
                    if seen[ni] || labels.pixel(nx as u32, ny as u32) != lab {
                        continue;
                    }
                    seen[ni] = true;
                    q.push((nx, ny));
                }
            }

            if area == 0 {
                continue;
            }
            out[n] = ParticleCentroid {
                x: sum_x as f32 / area as f32,
                y: sum_y as f32 / area as f32,
                frame,
                area,
                label: lab as u32,
            };
            n += 1;
        }
    }
    Ok(n)
}

/// Centroid of a bounding box (integer mid-point) — convenience for box→particle bridges.
#[inline]
pub fn centroid_from_bbox(
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    frame: u32,
    area: u32,
) -> ParticleCentroid {
    let cx = x as f32 + (w.saturating_sub(1) as f32) * 0.5;
    let cy = y as f32 + (h.saturating_sub(1) as f32) * 0.5;
    ParticleCentroid {
        x: cx,
        y: cy,
        frame,
        area,
        label: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_square_centroid_at_centre() {
        // 4×4 block at (2,2)..(5,5) on 8×8 → centre (3.5, 3.5)
        let mut img = vec![0u8; 64];
        for y in 2..6 {
            for x in 2..6 {
                img[y * 8 + x] = 255;
            }
        }
        let v = GrayView::new(8, 8, 8, &img).unwrap();
        let mut out = [ParticleCentroid::EMPTY; 4];
        let n = centroids_from_binary(v, 128, 3, &mut out).unwrap();
        assert_eq!(n, 1);
        assert!((out[0].x - 3.5).abs() < 1e-4);
        assert!((out[0].y - 3.5).abs() < 1e-4);
        assert_eq!(out[0].area, 16);
        assert_eq!(out[0].frame, 3);
    }

    #[test]
    fn two_blobs_two_centroids() {
        let mut img = vec![0u8; 64];
        img[1 * 8 + 1] = 255;
        img[1 * 8 + 2] = 255;
        img[6 * 8 + 6] = 255;
        let v = GrayView::new(8, 8, 8, &img).unwrap();
        let mut out = [ParticleCentroid::EMPTY; 4];
        let n = centroids_from_binary(v, 1, 0, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!(out[0].area, 2);
        assert_eq!(out[1].area, 1);
    }

    #[test]
    fn labels_separate_by_value_and_connectivity() {
        let mut img = vec![0u8; 16];
        // label 1 at (0,0); label 2 at (3,0) and (3,1) connected
        img[0] = 1;
        img[3] = 2;
        img[4 + 3] = 2;
        let v = GrayView::new(4, 4, 4, &img).unwrap();
        let mut out = [ParticleCentroid::EMPTY; 4];
        let n = centroids_from_labels(v, 1, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!(out[0].label, 1);
        assert_eq!(out[0].area, 1);
        assert_eq!(out[1].label, 2);
        assert_eq!(out[1].area, 2);
    }

    #[test]
    fn empty_image_zero() {
        let img = vec![0u8; 16];
        let v = GrayView::new(4, 4, 4, &img).unwrap();
        let mut out = [ParticleCentroid::EMPTY; 2];
        assert_eq!(centroids_from_binary(v, 128, 0, &mut out).unwrap(), 0);
    }

    #[test]
    fn bbox_centroid_mid() {
        let c = centroid_from_bbox(10, 20, 5, 5, 2, 25);
        assert!((c.x - 12.0).abs() < 1e-4);
        assert!((c.y - 22.0).abs() < 1e-4);
        assert_eq!(c.frame, 2);
    }
}
