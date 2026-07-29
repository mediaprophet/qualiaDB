//! Reconstruct an image from a Laplacian pyramid (caller-buffered).

use super::gaussian_pyramid_build::{PyramidLevelMeta, MAX_PYRAMID_LEVELS};
use crate::cv::error::CvError;

fn expand_add(
    coarse: &[f32],
    cw: usize,
    ch: usize,
    parent_w: usize,
    parent_h: usize,
    lap: &[f32],
    dest: &mut [f32],
) {
    for y in 0..parent_h {
        let cy = (y / 2).min(ch - 1);
        for x in 0..parent_w {
            let cx = (x / 2).min(cw - 1);
            let idx = y * parent_w + x;
            dest[idx] = lap[idx] + coarse[cy * cw + cx];
        }
    }
}

/// Reconstruct level-0 image from Laplacian pyramid into `out` (`meta[0].len` elements).
///
/// `work` must hold at least the largest intermediate level (≥ level-0 size is sufficient).
pub fn pyramid_reconstruct(
    lap_packed: &[f32],
    meta: &[PyramidLevelMeta; MAX_PYRAMID_LEVELS],
    n_levels: usize,
    out: &mut [f32],
    work: &mut [f32],
) -> Result<(), CvError> {
    if n_levels == 0 || n_levels > MAX_PYRAMID_LEVELS {
        return Err(CvError::InvalidParameter);
    }
    let l0 = meta[0].len;
    if out.len() < l0 || work.len() < l0 {
        return Err(CvError::BufferTooSmall);
    }

    if n_levels == 1 {
        let o = meta[0].offset;
        out[..l0].copy_from_slice(&lap_packed[o..o + l0]);
        return Ok(());
    }

    // Start from top residual into a side buffer sized to top, then walk down.
    // Use `work` for "current coarse", `out` for next parent when parent is level 0.
    let top = n_levels - 1;
    let mut cur_w = meta[top].width as usize;
    let mut cur_h = meta[top].height as usize;
    let mut cur_len = meta[top].len;

    // Keep current reconstruction in a stack-friendly two-buffer ping-pong via work/out.
    // Allocate conceptually: copy top into work[0..cur_len]
    if work.len() < cur_len {
        return Err(CvError::BufferTooSmall);
    }
    let to = meta[top].offset;
    work[..cur_len].copy_from_slice(&lap_packed[to..to + cur_len]);

    for i in (0..n_levels - 1).rev() {
        let parent = &meta[i];
        let pw = parent.width as usize;
        let ph = parent.height as usize;
        let plen = parent.len;
        let lap = &lap_packed[parent.offset..parent.offset + plen];

        if i == 0 {
            if out.len() < plen {
                return Err(CvError::BufferTooSmall);
            }
            expand_add(&work[..cur_len], cur_w, cur_h, pw, ph, lap, out);
        } else {
            // Need temporary for expanded parent — write into out then copy back to work
            if out.len() < plen || work.len() < plen {
                return Err(CvError::BufferTooSmall);
            }
            expand_add(&work[..cur_len], cur_w, cur_h, pw, ph, lap, out);
            work[..plen].copy_from_slice(&out[..plen]);
            cur_w = pw;
            cur_h = ph;
            cur_len = plen;
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
    use crate::biosense::magnification::laplacian_pyramid_build::laplacian_pyramid_build;

    #[test]
    fn reconstruct_approx_input() {
        let w = 16u32;
        let h = 16u32;
        let n_px = (w * h) as usize;
        let mut src = vec![0.0f32; n_px];
        for y in 0..h as usize {
            for x in 0..w as usize {
                src[y * w as usize + x] = (x as f32 * 0.5 + y as f32 * 0.25).sin() * 40.0 + 100.0;
            }
        }
        let levels = 4usize;
        let need = gaussian_pyramid_scratch_elems(w, h, levels);
        let mut g = vec![0.0f32; need];
        let mut meta = [PyramidLevelMeta::default(); MAX_PYRAMID_LEVELS];
        let n = gaussian_pyramid_build(&src, w, h, levels, &mut g, &mut meta).unwrap();
        let mut lap = vec![0.0f32; need];
        let mut expand = vec![0.0f32; n_px];
        laplacian_pyramid_build(&g, &meta, n, &mut lap, &mut expand).unwrap();
        let mut recon = vec![0.0f32; n_px];
        let mut work = vec![0.0f32; n_px];
        pyramid_reconstruct(&lap, &meta, n, &mut recon, &mut work).unwrap();

        let mut max_err = 0.0f32;
        for i in 0..n_px {
            max_err = max_err.max((recon[i] - src[i]).abs());
        }
        // Nearest-neighbor expand is lossy but should stay close for smooth fields.
        assert!(max_err < 25.0, "max_err={max_err}");
        let mean_err: f32 = recon
            .iter()
            .zip(src.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / n_px as f32;
        assert!(mean_err < 5.0, "mean_err={mean_err}");
    }
}
