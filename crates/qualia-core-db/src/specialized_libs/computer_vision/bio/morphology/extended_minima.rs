//! H-minima / extended minima on Gray8 (suppress shallow minima).
//!
//! Classic morphology: `HMIN_h(f) = R^ε_f(f+h)` (reconstruction by erosion),
//! then extended minima = regional minima of `HMIN_h(f)`.
//! Output is a binary map: 255 = extended-minimum pixel, 0 otherwise.

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Compute extended minima of a gray topographic surface.
///
/// `h` is the depth threshold in intensity units: minima shallower than `h`
/// are suppressed. `out` must hold at least `width * height` bytes.
pub fn extended_minima(src: GrayView<'_>, h: u8, out: &mut [u8]) -> Result<(), CvError> {
    let w = src.width as usize;
    let hgt = src.height as usize;
    let n = w.checked_mul(hgt).ok_or(CvError::InvalidParameter)?;
    if n == 0 {
        return Err(CvError::EmptyInput);
    }
    if out.len() < n {
        return Err(CvError::BufferTooSmall);
    }

    // Marker = min(src + h, 255); mask = src. Reconstruct by erosion under mask.
    let mut recon = vec![0u8; n];
    for y in 0..hgt {
        for x in 0..w {
            let v = src.pixel(x as u32, y as u32);
            recon[y * w + x] = v.saturating_add(h);
        }
    }

    // Geodesic reconstruction by erosion: iterate erode then max(mask) until stable.
    // Bounded by n iterations worst-case; early exit on no change.
    let mut next = recon.clone();
    let max_iters = n.saturating_mul(2).max(1);
    for _ in 0..max_iters {
        let mut changed = false;
        for y in 0..hgt {
            for x in 0..w {
                let i = y * w + x;
                let mut m = recon[i];
                for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= hgt as i32 {
                        continue;
                    }
                    m = m.min(recon[ny as usize * w + nx as usize]);
                }
                let mask = src.pixel(x as u32, y as u32);
                let v = m.max(mask);
                if v != recon[i] {
                    changed = true;
                }
                next[i] = v;
            }
        }
        core::mem::swap(&mut recon, &mut next);
        if !changed {
            break;
        }
    }

    // Regional minima of reconstruction: flat zones whose exterior neighbors are strictly higher.
    out[..n].fill(0);
    let mut seen = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut component: Vec<usize> = Vec::new();

    for seed in 0..n {
        if seen[seed] {
            continue;
        }
        let level = recon[seed];
        stack.clear();
        component.clear();
        stack.push(seed);
        seen[seed] = true;
        let mut is_minimum = true;

        while let Some(i) = stack.pop() {
            component.push(i);
            let x = i % w;
            let y = i / w;
            for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= hgt as i32 {
                    continue;
                }
                let ni = ny as usize * w + nx as usize;
                let nv = recon[ni];
                if nv < level {
                    // Neighbor lower ⇒ this plateau is not a minimum.
                    is_minimum = false;
                } else if nv == level {
                    if !seen[ni] {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                } else {
                    // nv > level: exterior higher — ok for a minimum.
                }
            }
        }

        if is_minimum {
            for &i in &component {
                out[i] = 255;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_basin_survives_shallow_suppressed() {
        // 5x5: deep center (0) and a shallow dimple (100) on plateaus of 150 / 200.
        let mut img = [150u8; 25];
        img[12] = 0; // deep minimum at center
        img[2] = 100; // shallow
        img[3] = 150;
        img[1] = 150;
        img[7] = 150;
        let v = GrayView::new(5, 5, 5, &img).unwrap();
        let mut out = [0u8; 25];
        extended_minima(v, 40, &mut out).unwrap();
        // Deep basin remains a minimum.
        assert_eq!(out[12], 255);
        // Shallow dip of depth 50 relative to 150 should be suppressed with h=40? depth = 150-100=50 > 40
        // so it may survive. Use h=60 so depth 50 is suppressed.
        let mut out2 = [0u8; 25];
        extended_minima(v, 60, &mut out2).unwrap();
        assert_eq!(out2[12], 255);
        assert_eq!(out2[2], 0);
    }

    #[test]
    fn empty_buffer_errors() {
        let img = [0u8; 4];
        let v = GrayView::new(2, 2, 2, &img).unwrap();
        let mut out = [0u8; 2];
        assert_eq!(
            extended_minima(v, 1, &mut out),
            Err(CvError::BufferTooSmall)
        );
    }

    #[test]
    fn flat_image_all_minima() {
        let img = [42u8; 9];
        let v = GrayView::new(3, 3, 3, &img).unwrap();
        let mut out = [0u8; 9];
        extended_minima(v, 0, &mut out).unwrap();
        assert!(out.iter().all(|&p| p == 255));
    }
}
