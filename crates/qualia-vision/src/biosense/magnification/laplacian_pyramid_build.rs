//! Laplacian pyramid from a packed Gaussian pyramid (caller-buffered).

use super::gaussian_pyramid_build::{PyramidLevelMeta, MAX_PYRAMID_LEVELS};
use crate::cv::error::CvError;

/// Expand a coarser level to parent resolution by nearest-neighbor upsample + light blur.
fn expand_to(
    coarse: &[f32],
    cw: usize,
    ch: usize,
    parent_w: usize,
    parent_h: usize,
    dest: &mut [f32],
) {
    debug_assert!(dest.len() >= parent_w * parent_h);
    for y in 0..parent_h {
        let cy = (y / 2).min(ch - 1);
        for x in 0..parent_w {
            let cx = (x / 2).min(cw - 1);
            dest[y * parent_w + x] = coarse[cy * cw + cx];
        }
    }
}

/// Build Laplacian levels into `lap_packed` (same layout as Gaussian: L_i = G_i − expand(G_{i+1}),
/// top residual L_{n-1} = G_{n-1}).
///
/// `gauss_packed` / `gauss_meta` from [`super::gaussian_pyramid_build::gaussian_pyramid_build`].
/// `n_levels` must match the Gaussian pyramid depth.
///
/// Scratch: `expand_scratch` ≥ max parent level size (level 0 size is safe).
pub fn laplacian_pyramid_build(
    gauss_packed: &[f32],
    gauss_meta: &[PyramidLevelMeta; MAX_PYRAMID_LEVELS],
    n_levels: usize,
    lap_packed: &mut [f32],
    expand_scratch: &mut [f32],
) -> Result<(), CvError> {
    if n_levels == 0 || n_levels > MAX_PYRAMID_LEVELS {
        return Err(CvError::InvalidParameter);
    }
    let total: usize = (0..n_levels).map(|i| gauss_meta[i].len).sum();
    if gauss_packed.len() < total || lap_packed.len() < total {
        return Err(CvError::BufferTooSmall);
    }

    // Top residual
    let top = n_levels - 1;
    let to = gauss_meta[top].offset;
    let tl = gauss_meta[top].len;
    lap_packed[to..to + tl].copy_from_slice(&gauss_packed[to..to + tl]);

    if n_levels == 1 {
        return Ok(());
    }

    for i in (0..n_levels - 1).rev() {
        let parent = &gauss_meta[i];
        let child = &gauss_meta[i + 1];
        let pw = parent.width as usize;
        let ph = parent.height as usize;
        if expand_scratch.len() < parent.len {
            return Err(CvError::BufferTooSmall);
        }
        let coarse = &gauss_packed[child.offset..child.offset + child.len];
        expand_to(
            coarse,
            child.width as usize,
            child.height as usize,
            pw,
            ph,
            expand_scratch,
        );
        let g = &gauss_packed[parent.offset..parent.offset + parent.len];
        let dest = &mut lap_packed[parent.offset..parent.offset + parent.len];
        for k in 0..parent.len {
            dest[k] = g[k] - expand_scratch[k];
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::magnification::gaussian_pyramid_build::{
        gaussian_pyramid_build, gaussian_pyramid_scratch_elems,
    };

    #[test]
    fn top_equals_gaussian_top() {
        let w = 8u32;
        let h = 8u32;
        let src: Vec<f32> = (0..64).map(|i| (i % 7) as f32).collect();
        let need = gaussian_pyramid_scratch_elems(w, h, 3);
        let mut g = vec![0.0f32; need];
        let mut meta = [PyramidLevelMeta::default(); MAX_PYRAMID_LEVELS];
        let n = gaussian_pyramid_build(&src, w, h, 3, &mut g, &mut meta).unwrap();
        let mut lap = vec![0.0f32; need];
        let mut scratch = vec![0.0f32; (w * h) as usize];
        laplacian_pyramid_build(&g, &meta, n, &mut lap, &mut scratch).unwrap();
        let top = n - 1;
        let o = meta[top].offset;
        let l = meta[top].len;
        for k in 0..l {
            assert!((lap[o + k] - g[o + k]).abs() < 1e-5);
        }
    }
}
