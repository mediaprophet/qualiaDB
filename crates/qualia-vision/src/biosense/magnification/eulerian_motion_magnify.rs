//! Eulerian motion magnification (simplified spatial high-pass × temporal band on Gray).

use crate::cv::error::CvError;

/// `frames` gray packed n*w*h. Amplifies micro-motion-like differences.
pub fn eulerian_motion_magnify(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    alpha: f32,
    out: &mut [u8],
) -> Result<(), CvError> {
    let px = (width * height) as usize;
    if n_frames < 3 || frames.len() < n_frames * px || out.len() < n_frames * px {
        return Err(CvError::BufferTooSmall);
    }
    out[..n_frames * px].copy_from_slice(&frames[..n_frames * px]);
    let gain = alpha.clamp(0.0, 80.0);
    let w = width as usize;
    for i in 1..n_frames - 1 {
        for y in 1..height as usize - 1 {
            for x in 1..w - 1 {
                let idx = |fi: usize, xx: usize, yy: usize| fi * px + yy * w + xx;
                let cur = frames[idx(i, x, y)] as f32;
                // spatial laplacian
                let lap = frames[idx(i, x + 1, y)] as f32
                    + frames[idx(i, x - 1, y)] as f32
                    + frames[idx(i, x, y + 1)] as f32
                    + frames[idx(i, x, y - 1)] as f32
                    - 4.0 * cur;
                let prev = frames[idx(i - 1, x, y)] as f32;
                let next = frames[idx(i + 1, x, y)] as f32;
                let band = cur - 0.5 * (prev + next);
                let v = (cur + gain * 0.1 * lap * band.signum() * band.abs().sqrt()).clamp(0.0, 255.0);
                out[idx(i, x, y)] = v as u8;
            }
        }
    }
    Ok(())
}
