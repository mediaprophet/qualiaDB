//! Simplified Canny: Sobel mag + dual threshold (no full hysteresis graph).

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::edges::sobel_mag_u8::sobel_mag_u8;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Approximate Canny edges. `out` binary-ish 0/255. `low`/`high` thresholds on magnitude.
pub fn canny_u8(src: GrayView<'_>, low: u8, high: u8, out: &mut [u8]) -> Result<(), CvError> {
    let n = (src.width * src.height) as usize;
    if out.len() < n {
        return Err(CvError::BufferTooSmall);
    }
    let mut mag = vec![0u8; n];
    sobel_mag_u8(src, &mut mag)?;
    let lo = low.min(high);
    let hi = high.max(low);
    for i in 0..n {
        out[i] = if mag[i] >= hi {
            255
        } else if mag[i] >= lo {
            128
        } else {
            0
        };
    }
    // Simple hysteresis pass: weak becomes strong if neighbour strong
    let w = src.width as usize;
    let h = src.height as usize;
    let mut changed = true;
    while changed {
        changed = false;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if out[i] != 128 {
                    continue;
                }
                let mut strong = false;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let xx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
                        let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
                        if out[yy * w + xx] == 255 {
                            strong = true;
                        }
                    }
                }
                if strong {
                    out[i] = 255;
                    changed = true;
                }
            }
        }
    }
    for v in out.iter_mut().take(n) {
        if *v == 128 {
            *v = 0;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs() {
        let img = [0u8, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255];
        let v = GrayView::new(4, 4, 4, &img).unwrap();
        let mut o = [0u8; 16];
        canny_u8(v, 20, 80, &mut o).unwrap();
    }
}
