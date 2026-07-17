//! Gaussian image pyramid (caller-buffered levels, fixed max depth).
//!
//! Tier-2 cold construction may pre-size buffers; hot downsample uses fixed 5-tap
//! separable-style box average (no heap in the pixel loop).

use crate::cv::error::CvError;

/// Maximum pyramid depth (level 0 = full resolution).
pub const MAX_PYRAMID_LEVELS: usize = 6;

/// Per-level geometry written by [`gaussian_pyramid_build`].
#[derive(Debug, Clone, Copy, Default)]
pub struct PyramidLevelMeta {
    pub width: u32,
    pub height: u32,
    /// Byte/element offset into the packed level buffer.
    pub offset: usize,
    pub len: usize,
}

/// Bytes (f32 elements) required for `n_levels` of a `width×height` Gaussian pyramid.
pub fn gaussian_pyramid_scratch_elems(width: u32, height: u32, n_levels: usize) -> usize {
    let n = n_levels.clamp(1, MAX_PYRAMID_LEVELS);
    let mut w = width as usize;
    let mut h = height as usize;
    let mut total = 0usize;
    for _ in 0..n {
        total = total.saturating_add(w.saturating_mul(h));
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    total
}

/// Build a Gaussian pyramid into a packed caller buffer.
///
/// Level 0 copies `src` (`width*height` f32). Each subsequent level is a 2×
/// spatial downsample of the previous (4-neighbor mean with edge clamp).
///
/// Returns number of levels written (≤ `n_levels`, stops early if dim stalls at 1×1).
pub fn gaussian_pyramid_build(
    src: &[f32],
    width: u32,
    height: u32,
    n_levels: usize,
    out_packed: &mut [f32],
    meta: &mut [PyramidLevelMeta; MAX_PYRAMID_LEVELS],
) -> Result<usize, CvError> {
    if width == 0 || height == 0 || n_levels == 0 {
        return Err(CvError::InvalidParameter);
    }
    let n_req = n_levels.min(MAX_PYRAMID_LEVELS);
    let need = width as usize * height as usize;
    if src.len() < need {
        return Err(CvError::BufferTooSmall);
    }
    let scratch_need = gaussian_pyramid_scratch_elems(width, height, n_req);
    if out_packed.len() < scratch_need {
        return Err(CvError::BufferTooSmall);
    }

    // Level 0
    out_packed[..need].copy_from_slice(&src[..need]);
    meta[0] = PyramidLevelMeta {
        width,
        height,
        offset: 0,
        len: need,
    };

    let mut levels = 1usize;
    let mut prev_w = width as usize;
    let mut prev_h = height as usize;
    let mut prev_off = 0usize;

    while levels < n_req {
        let next_w = (prev_w / 2).max(1);
        let next_h = (prev_h / 2).max(1);
        if next_w == prev_w && next_h == prev_h {
            break; // cannot refine further
        }
        let next_len = next_w * next_h;
        let next_off = prev_off + prev_w * prev_h;
        if next_off + next_len > out_packed.len() {
            return Err(CvError::BufferTooSmall);
        }

        for y in 0..next_h {
            for x in 0..next_w {
                let x0 = (x * 2).min(prev_w - 1);
                let y0 = (y * 2).min(prev_h - 1);
                let x1 = (x0 + 1).min(prev_w - 1);
                let y1 = (y0 + 1).min(prev_h - 1);
                let base = prev_off;
                let a = out_packed[base + y0 * prev_w + x0];
                let b = out_packed[base + y0 * prev_w + x1];
                let c = out_packed[base + y1 * prev_w + x0];
                let d = out_packed[base + y1 * prev_w + x1];
                out_packed[next_off + y * next_w + x] = 0.25 * (a + b + c + d);
            }
        }

        meta[levels] = PyramidLevelMeta {
            width: next_w as u32,
            height: next_h as u32,
            offset: next_off,
            len: next_len,
        };
        prev_w = next_w;
        prev_h = next_h;
        prev_off = next_off;
        levels += 1;
    }

    // Zero unused meta slots
    for m in meta.iter_mut().skip(levels) {
        *m = PyramidLevelMeta::default();
    }
    Ok(levels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level0_matches_source() {
        let w = 4u32;
        let h = 4u32;
        let mut src = [0.0f32; 16];
        for i in 0..16 {
            src[i] = i as f32;
        }
        let need = gaussian_pyramid_scratch_elems(w, h, 3);
        let mut out = vec![0.0f32; need];
        let mut meta = [PyramidLevelMeta::default(); MAX_PYRAMID_LEVELS];
        let n = gaussian_pyramid_build(&src, w, h, 3, &mut out, &mut meta).unwrap();
        assert!(n >= 2);
        assert_eq!(meta[0].width, 4);
        assert_eq!(meta[0].height, 4);
        for i in 0..16 {
            assert!((out[i] - src[i]).abs() < 1e-6);
        }
        assert_eq!(meta[1].width, 2);
        assert_eq!(meta[1].height, 2);
    }

    #[test]
    fn rejects_empty() {
        let mut out = [0.0f32; 4];
        let mut meta = [PyramidLevelMeta::default(); MAX_PYRAMID_LEVELS];
        assert!(gaussian_pyramid_build(&[], 0, 0, 2, &mut out, &mut meta).is_err());
    }
}
