//! Marker-controlled watershed flooding on a gray topographic surface.
//!
//! Seeds grow by priority (lower gray first). When fronts from distinct labels
//! meet, the contested pixel stays unlabeled (0) as a watershed ridge.
//! Output labels are written into a caller-owned `u16` buffer.

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

/// Marker-controlled watershed.
///
/// - `topography`: elevation (low = basin).
/// - `markers`: same length as image; non-zero seeds grow their label.
/// - `out_labels`: caller buffer, length ≥ width×height; receives final labels
///   (0 = background / watershed line).
pub fn watershed_markers(
    topography: GrayView<'_>,
    markers: &[u16],
    out_labels: &mut [u16],
) -> Result<(), CvError> {
    let w = topography.width as usize;
    let h = topography.height as usize;
    let n = w.checked_mul(h).ok_or(CvError::InvalidParameter)?;
    if n == 0 {
        return Err(CvError::EmptyInput);
    }
    if markers.len() < n || out_labels.len() < n {
        return Err(CvError::BufferTooSmall);
    }

    out_labels[..n].fill(0);

    // Bucket queue by gray level (256 buckets) — deterministic Meyer-style flood.
    let mut buckets: [Vec<usize>; 256] = core::array::from_fn(|_| Vec::new());
    let mut enqueued = vec![false; n];

    for i in 0..n {
        if markers[i] != 0 {
            out_labels[i] = markers[i];
            // Seed neighbors into the queue at their elevation.
            let x = i % w;
            let y = i / w;
            for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let ni = ny as usize * w + nx as usize;
                if markers[ni] == 0 && !enqueued[ni] {
                    let g = topography.pixel(nx as u32, ny as u32) as usize;
                    buckets[g].push(ni);
                    enqueued[ni] = true;
                }
            }
        }
    }

    for level in 0..256 {
        // Process level with a working list so same-level growth is stable FIFO.
        let mut idx = 0usize;
        while idx < buckets[level].len() {
            let i = buckets[level][idx];
            idx += 1;
            if out_labels[i] != 0 {
                continue;
            }

            let x = i % w;
            let y = i / w;
            let mut label: u16 = 0;
            let mut conflict = false;
            for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let ni = ny as usize * w + nx as usize;
                let nl = out_labels[ni];
                if nl == 0 {
                    continue;
                }
                if label == 0 {
                    label = nl;
                } else if label != nl {
                    conflict = true;
                    break;
                }
            }

            if conflict || label == 0 {
                // Watershed ridge or unseeded isolation — leave 0.
                continue;
            }

            out_labels[i] = label;

            for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let ni = ny as usize * w + nx as usize;
                if out_labels[ni] == 0 && !enqueued[ni] {
                    let g = topography.pixel(nx as u32, ny as u32) as usize;
                    buckets[g].push(ni);
                    enqueued[ni] = true;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_seeds_split_basin() {
        // Flat basin with two seeds → each side gets its label.
        let img = [10u8; 16];
        let v = GrayView::new(4, 4, 4, &img).unwrap();
        let mut markers = [0u16; 16];
        markers[0] = 1;
        markers[15] = 2;
        let mut out = [0u16; 16];
        watershed_markers(v, &markers, &mut out).unwrap();
        assert_eq!(out[0], 1);
        assert_eq!(out[15], 2);
        // Some interior pixels labeled 1 or 2 (or 0 ridge).
        let labeled: usize = out.iter().filter(|&&l| l == 1 || l == 2).count();
        assert!(labeled >= 2);
        assert!(!out.iter().any(|&l| l > 2));
    }

    #[test]
    fn buffer_too_small() {
        let img = [0u8; 4];
        let v = GrayView::new(2, 2, 2, &img).unwrap();
        let markers = [1u16, 0, 0, 2];
        let mut out = [0u16; 2];
        assert_eq!(
            watershed_markers(v, &markers, &mut out),
            Err(CvError::BufferTooSmall)
        );
    }

    #[test]
    fn no_markers_all_zero() {
        let img = [50u8; 9];
        let v = GrayView::new(3, 3, 3, &img).unwrap();
        let markers = [0u16; 9];
        let mut out = [99u16; 9];
        watershed_markers(v, &markers, &mut out).unwrap();
        assert!(out.iter().all(|&l| l == 0));
    }
}
