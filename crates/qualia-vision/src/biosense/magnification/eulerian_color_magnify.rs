//! Eulerian colour magnification (simplified temporal band amplify on RGB means map).

use crate::cv::error::CvError;

/// Amplify temporal band of a per-frame scalar signal into `out` frames of RGB delta applied to base.
/// `frames` length = n * w * h * 3 packed RGB. In-place style: write magnified into `out`.
pub fn eulerian_color_magnify(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    alpha: f32,
    out: &mut [u8],
) -> Result<(), CvError> {
    let px = (width * height * 3) as usize;
    if n_frames < 3 || frames.len() < n_frames * px || out.len() < n_frames * px {
        return Err(CvError::BufferTooSmall);
    }
    out[..n_frames * px].copy_from_slice(&frames[..n_frames * px]);
    // Temporal high-pass approx: amplify difference from local mean
    let gain = alpha.clamp(0.0, 50.0);
    for i in 1..n_frames - 1 {
        for p in 0..px {
            let prev = frames[(i - 1) * px + p] as f32;
            let cur = frames[i * px + p] as f32;
            let next = frames[(i + 1) * px + p] as f32;
            let band = cur - (prev + next) * 0.5;
            let v = (cur + gain * band).clamp(0.0, 255.0) as u8;
            out[i * px + p] = v;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs() {
        let n = 5;
        let px = 4 * 3;
        let mut f = vec![100u8; n * px];
        for i in 0..n {
            f[i * px] = (100 + (i as i32 - 2) * 5) as u8;
        }
        let mut o = vec![0u8; n * px];
        eulerian_color_magnify(&f, n, 2, 2, 10.0, &mut o).unwrap();
    }
}
