//! CHROM rPPG chrominance method (simplified).

use crate::cv::error::CvError;

pub fn chrom_rppg_trace(rgb_means: &[f32], n_frames: usize, out: &mut [f32]) -> Result<(), CvError> {
    if n_frames == 0 || rgb_means.len() < n_frames * 3 || out.len() < n_frames {
        return Err(CvError::BufferTooSmall);
    }
    let mut xs = 0.0f32;
    let mut ys = 0.0f32;
    let mut xbuf = vec![0.0f32; n_frames];
    let mut ybuf = vec![0.0f32; n_frames];
    for i in 0..n_frames {
        let r = rgb_means[i * 3];
        let g = rgb_means[i * 3 + 1];
        let b = rgb_means[i * 3 + 2];
        let x = 3.0 * r - 2.0 * g;
        let y = 1.5 * r + g - 1.5 * b;
        xbuf[i] = x;
        ybuf[i] = y;
        xs += x;
        ys += y;
    }
    let n = n_frames as f32;
    let xm = xs / n;
    let ym = ys / n;
    let mut xs2 = 0.0f32;
    let mut ys2 = 0.0f32;
    for i in 0..n_frames {
        let xd = xbuf[i] - xm;
        let yd = ybuf[i] - ym;
        xs2 += xd * xd;
        ys2 += yd * yd;
    }
    let sx = (xs2 / n).sqrt().max(1e-6);
    let sy = (ys2 / n).sqrt().max(1e-6);
    let alpha = sx / sy;
    for i in 0..n_frames {
        out[i] = (xbuf[i] - xm) - alpha * (ybuf[i] - ym);
    }
    Ok(())
}
