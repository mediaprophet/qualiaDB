//! Edge-directed ("NEDI-lite") upscale RGB packed u8 by integer scale 2|3|4.
//!
//! Classical, no learned weights. For each output pixel we estimate the local
//! gradient direction from the 4 nearest source pixels and interpolate ALONG
//! the edge rather than across it, so diagonal edges stay sharp instead of
//! staircasing/blurring the way plain bilinear does.
//!
//! Decision (per 2×2 source cell, using a luma proxy `r+g+b`):
//!   * `d_main = |l00 - l11|` — variation along the main diagonal `\` (through
//!     the top-left / bottom-right corners, which are co-linear on that edge).
//!   * `d_anti = |l10 - l01|` — variation along the anti-diagonal `/` (through
//!     the top-right / bottom-left corners).
//! The edge runs along whichever diagonal is *flattest*. When one diagonal is
//! clearly flatter than the other (and the cell actually contains an edge), we
//! interpolate along that diagonal — averaging the two co-linear neighbours
//! (which are near-equal on a clean edge, so no graying) and ramping across it.
//! Axis-aligned edges and flat regions are left to bilinear, which is already
//! jaggy-free on them; the diagonal case is the whole point of this kernel.

use crate::specialized_libs::computer_vision::cv::buffer::RgbView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// A diagonal must be flatter than the other by at least this much luma
/// (0..765 scale, sum of 3 channels) before we prefer directional interp.
const EDGE_MARGIN: i32 = 24;
/// The cell must contain at least this much diagonal contrast to count as an
/// edge; below this we treat it as flat/noise and fall back to bilinear.
const EDGE_FLOOR: i32 = 48;

/// Edge-directed upsample RGB8. `out` ≥ `w*scale * h*scale * 3`. Scale ∈ {2,3,4}.
///
/// Caller-buffered: writes into `out`. Zero heap allocation in the hot loop.
pub fn edge_directed_lite(src: RgbView<'_>, scale: u8, out: &mut [u8]) -> Result<(), CvError> {
    if scale < 2 || scale > 4 {
        return Err(CvError::InvalidParameter);
    }
    let w = src.width;
    let h = src.height;
    if w == 0 || h == 0 {
        return Err(CvError::EmptyInput);
    }
    let out_w = w
        .checked_mul(scale as u32)
        .ok_or(CvError::InvalidParameter)?;
    let out_h = h
        .checked_mul(scale as u32)
        .ok_or(CvError::InvalidParameter)?;
    let need = (out_w as usize)
        .checked_mul(out_h as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(CvError::InvalidParameter)?;
    if out.len() < need {
        return Err(CvError::BufferTooSmall);
    }

    let s = scale as f32;
    let wm1 = (w.saturating_sub(1)) as f32;
    let hm1 = (h.saturating_sub(1)) as f32;

    for oy in 0..out_h {
        let sy = ((oy as f32 + 0.5) / s - 0.5).clamp(0.0, hm1);
        let y0 = sy.floor() as u32;
        let y1 = (y0 + 1).min(h - 1);
        let fy = sy - y0 as f32;
        for ox in 0..out_w {
            let sx = ((ox as f32 + 0.5) / s - 0.5).clamp(0.0, wm1);
            let x0 = sx.floor() as u32;
            let x1 = (x0 + 1).min(w - 1);
            let fx = sx - x0 as f32;

            let (r00, g00, b00) = src.pixel(x0, y0);
            let (r10, g10, b10) = src.pixel(x1, y0);
            let (r01, g01, b01) = src.pixel(x0, y1);
            let (r11, g11, b11) = src.pixel(x1, y1);

            // Luma proxy (sum of channels, 0..765) for a channel-consistent
            // direction decision — all three channels use the same edge.
            let l00 = r00 as i32 + g00 as i32 + b00 as i32;
            let l10 = r10 as i32 + g10 as i32 + b10 as i32;
            let l01 = r01 as i32 + g01 as i32 + b01 as i32;
            let l11 = r11 as i32 + g11 as i32 + b11 as i32;

            let d_main = (l00 - l11).abs(); // flatness along `\`
            let d_anti = (l10 - l01).abs(); // flatness along `/`
            let dir = classify(d_main, d_anti);

            let doff = ((oy * out_w + ox) * 3) as usize;
            match dir {
                Dir::Bilinear => {
                    out[doff] = bilinear(r00, r10, r01, r11, fx, fy);
                    out[doff + 1] = bilinear(g00, g10, g01, g11, fx, fy);
                    out[doff + 2] = bilinear(b00, b10, b01, b11, fx, fy);
                }
                Dir::DiagMain => {
                    // Edge along `\` (c00,c11 co-linear). Across-edge axis is the
                    // anti-diagonal: t = (fx - fy + 1)/2, samples c01→mid→c10.
                    let t = 0.5 * (fx - fy + 1.0);
                    out[doff] = diag3(r01, r00, r11, r10, t);
                    out[doff + 1] = diag3(g01, g00, g11, g10, t);
                    out[doff + 2] = diag3(b01, b00, b11, b10, t);
                }
                Dir::DiagAnti => {
                    // Edge along `/` (c10,c01 co-linear). Across-edge axis is the
                    // main diagonal: t = (fx + fy)/2, samples c00→mid→c11.
                    let t = 0.5 * (fx + fy);
                    out[doff] = diag3(r00, r10, r01, r11, t);
                    out[doff + 1] = diag3(g00, g10, g01, g11, t);
                    out[doff + 2] = diag3(b00, b10, b01, b11, t);
                }
            }
        }
    }
    Ok(())
}

/// Which direction to interpolate for a cell, given the two diagonal flatness
/// measures. Prefer the flatter diagonal only when it wins by a clear margin
/// and the cell actually contains an edge; otherwise bilinear.
#[derive(Clone, Copy)]
enum Dir {
    Bilinear,
    DiagMain,
    DiagAnti,
}

#[inline]
fn classify(d_main: i32, d_anti: i32) -> Dir {
    if d_main.max(d_anti) < EDGE_FLOOR {
        return Dir::Bilinear;
    }
    if d_anti + EDGE_MARGIN < d_main {
        Dir::DiagAnti
    } else if d_main + EDGE_MARGIN < d_anti {
        Dir::DiagMain
    } else {
        Dir::Bilinear
    }
}

/// Standard bilinear blend of the 4 corners at fractional (fx, fy).
#[inline]
fn bilinear(c00: u8, c10: u8, c01: u8, c11: u8, fx: f32, fy: f32) -> u8 {
    let v0 = c00 as f32 + (c10 as f32 - c00 as f32) * fx;
    let v1 = c01 as f32 + (c11 as f32 - c01 as f32) * fx;
    let v = v0 + (v1 - v0) * fy;
    v.round().clamp(0.0, 255.0) as u8
}

/// Three-point linear interpolation across an edge: `p0` at t=0, the midpoint
/// `(m_a + m_b)/2` at t=0.5, and `p1` at t=1. On a clean edge `m_a == m_b`, so
/// the midpoint is the true edge value (no graying), and the result ramps
/// sharply along the across-edge axis. `t` is clamped to [0,1].
#[inline]
fn diag3(p0: u8, m_a: u8, m_b: u8, p1: u8, t: f32) -> u8 {
    let t = t.clamp(0.0, 1.0);
    let mid = 0.5 * (m_a as f32 + m_b as f32);
    let v = if t <= 0.5 {
        let f = t * 2.0;
        p0 as f32 + (mid - p0 as f32) * f
    } else {
        let f = (t - 0.5) * 2.0;
        mid + (p1 as f32 - mid) * f
    };
    v.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::cv::sr::bilinear_u8::bilinear_u8;

    /// Peak signal-to-noise ratio (dB) between two equal-length RGB8 buffers.
    /// Returns `f64::INFINITY` for identical buffers.
    fn psnr(a: &[u8], b: &[u8]) -> f64 {
        assert_eq!(a.len(), b.len());
        let mut sse = 0.0f64;
        for (&x, &y) in a.iter().zip(b.iter()) {
            let d = x as f64 - y as f64;
            sse += d * d;
        }
        if sse == 0.0 {
            return f64::INFINITY;
        }
        let mse = sse / a.len() as f64;
        20.0 * (255.0f64).log10() - 10.0 * mse.log10()
    }

    /// Build an anti-diagonal step image: 0 below the `/` line, 255 above.
    fn diagonal_step(n: u32, threshold: i64) -> Vec<u8> {
        let mut img = vec![0u8; (n * n * 3) as usize];
        for y in 0..n {
            for x in 0..n {
                let v: u8 = if (x as i64 + y as i64) < threshold {
                    0
                } else {
                    255
                };
                let o = ((y * n + x) * 3) as usize;
                img[o] = v;
                img[o + 1] = v;
                img[o + 2] = v;
            }
        }
        img
    }

    #[test]
    fn edge_directed_beats_bilinear_on_diagonal() {
        // Low-res diagonal edge, and the ideal high-res version of the same
        // continuous edge. Upscale 2×; edge-directed should reconstruct the
        // sharp diagonal closer to ideal than bilinear does.
        let n = 16u32;
        let scale = 2u8;
        let hn = n * scale as u32;

        // LR edge at x+y < n-1 ; the matching continuous HR edge is at
        // ox+oy < scale*(n-1) (same line under the (o+0.5)/s-0.5 mapping).
        let lr = diagonal_step(n, (n - 1) as i64);
        let ideal = diagonal_step(hn, (scale as i64) * (n as i64 - 1));

        let v = RgbView::new(n, n, n * 3, &lr).unwrap();
        let mut ed = vec![0u8; (hn * hn * 3) as usize];
        let mut bl = vec![0u8; (hn * hn * 3) as usize];
        edge_directed_lite(v, scale, &mut ed).unwrap();
        bilinear_u8(v, scale, &mut bl).unwrap();

        let psnr_ed = psnr(&ed, &ideal);
        let psnr_bl = psnr(&bl, &ideal);
        println!("diagonal PSNR: edge_directed = {psnr_ed:.3} dB, bilinear = {psnr_bl:.3} dB");

        // The whole point: edge-directed is at least as good, and here better.
        assert!(
            psnr_ed >= psnr_bl,
            "edge-directed PSNR {psnr_ed:.3} should be >= bilinear PSNR {psnr_bl:.3}"
        );
    }

    #[test]
    fn flat_image_stays_constant() {
        // Constant colour must upscale to the same constant — no artefacts.
        let n = 5u32;
        let mut img = vec![0u8; (n * n * 3) as usize];
        for p in img.chunks_exact_mut(3) {
            p[0] = 137;
            p[1] = 42;
            p[2] = 211;
        }
        let v = RgbView::new(n, n, n * 3, &img).unwrap();
        let hn = n * 3;
        let mut out = vec![0u8; (hn * hn * 3) as usize];
        edge_directed_lite(v, 3, &mut out).unwrap();
        for p in out.chunks_exact(3) {
            assert_eq!(p, &[137, 42, 211]);
        }
    }

    #[test]
    fn output_dimensions_correct() {
        // 2×2 → scale 4 → 8×8; buffer exactly out_w*out_h*3 must succeed,
        // one byte short must be rejected.
        let img = [10u8, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let v = RgbView::new(2, 2, 6, &img).unwrap();
        let mut out = vec![0u8; 8 * 8 * 3];
        edge_directed_lite(v, 4, &mut out).unwrap();
        assert_eq!(out.len(), 8 * 8 * 3);

        let mut short = vec![0u8; 8 * 8 * 3 - 1];
        assert_eq!(
            edge_directed_lite(v, 4, &mut short),
            Err(CvError::BufferTooSmall)
        );
    }

    #[test]
    fn rejects_bad_scale() {
        let img = [1u8, 2, 3];
        let v = RgbView::new(1, 1, 3, &img).unwrap();
        let mut out = [0u8; 48];
        assert_eq!(
            edge_directed_lite(v, 5, &mut out),
            Err(CvError::InvalidParameter)
        );
        assert_eq!(
            edge_directed_lite(v, 1, &mut out),
            Err(CvError::InvalidParameter)
        );
    }
}
