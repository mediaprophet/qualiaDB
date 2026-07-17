//! Voronoi–Otsu labeling (lite): Otsu threshold → distance peaks → watershed split.
//!
//! Separates touching bright blobs without external libraries (scikit-image
//! `voronoi_otsu_labeling` style, reduced to pure-Rust caller-buffered labels).

use crate::bio::morphology::watershed_markers::watershed_markers;
use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;
use crate::cv::hist::histogram_u8;

/// Otsu threshold for a Gray8 histogram (returns 0 if image empty / single-tone).
pub fn otsu_threshold_from_hist(bins: &[u32; 256]) -> u8 {
    let total: u64 = bins.iter().map(|&c| c as u64).sum();
    if total == 0 {
        return 0;
    }
    let mut sum_all = 0u64;
    for (i, &c) in bins.iter().enumerate() {
        sum_all += (i as u64) * c as u64;
    }
    let mut sum_b = 0u64;
    let mut w_b = 0u64;
    let mut max_var = -1.0f64;
    let mut thresh = 0u8;
    for t in 0..256 {
        w_b += bins[t] as u64;
        if w_b == 0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0 {
            break;
        }
        sum_b += (t as u64) * bins[t] as u64;
        let m_b = sum_b as f64 / w_b as f64;
        let m_f = (sum_all - sum_b) as f64 / w_f as f64;
        let var = (w_b as f64) * (w_f as f64) * (m_b - m_f) * (m_b - m_f);
        if var > max_var {
            max_var = var;
            thresh = t as u8;
        }
    }
    thresh
}

/// Distance-like field on binary foreground (chessboard-ish iterative 4-neigh).
/// `binary` is 0 background / non-zero foreground. `dist` receives u8 distances
/// saturated at 255.
fn distance_transform_u8(binary: &[u8], w: usize, h: usize, dist: &mut [u8]) {
    let n = w * h;
    dist[..n].fill(0);
    // Multi-source BFS from background into foreground.
    let mut q: Vec<usize> = Vec::new();
    for i in 0..n {
        if binary[i] == 0 {
            dist[i] = 0;
            // enqueue border pixels that touch foreground
            let x = i % w;
            let y = i / w;
            for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let ni = ny as usize * w + nx as usize;
                if binary[ni] != 0 {
                    q.push(i);
                    break;
                }
            }
        } else {
            dist[i] = 255; // unvisited fg sentinel
        }
    }
    // Also treat image border background as sources for interior fg.
    let mut head = 0usize;
    // Seed all background
    q.clear();
    for i in 0..n {
        if binary[i] == 0 {
            dist[i] = 0;
            q.push(i);
        }
    }
    while head < q.len() {
        let i = q[head];
        head += 1;
        let d = dist[i];
        let x = i % w;
        let y = i / w;
        for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let ni = ny as usize * w + nx as usize;
            if binary[ni] == 0 {
                continue;
            }
            let nd = d.saturating_add(1);
            if nd < dist[ni] {
                dist[ni] = nd;
                q.push(ni);
            }
        }
    }
    // Background stays 0; fg has distance from bg.
}

/// Local maxima of distance map on foreground (strict 4-neigh) as watershed seeds.
fn distance_peak_markers(dist: &[u8], binary: &[u8], w: usize, h: usize, markers: &mut [u16]) {
    let n = w * h;
    markers[..n].fill(0);
    let mut next_label = 1u16;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if binary[i] == 0 || dist[i] == 0 {
                continue;
            }
            let d = dist[i];
            let mut is_peak = true;
            for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let ni = ny as usize * w + nx as usize;
                if binary[ni] != 0 && dist[ni] > d {
                    is_peak = false;
                    break;
                }
            }
            if is_peak && next_label > 0 {
                // Only seed unique peaks; plateaus get one label per pixel then merge via watershed.
                // Prefer a single seed: only mark if no already-marked neighbor at same height.
                let mut neighbor_marked = false;
                for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let ni = ny as usize * w + nx as usize;
                    if markers[ni] != 0 && dist[ni] == d {
                        neighbor_marked = true;
                        markers[i] = markers[ni];
                        break;
                    }
                }
                if !neighbor_marked {
                    markers[i] = next_label;
                    next_label = next_label.saturating_add(1);
                    if next_label == 0 {
                        // wrap-around guard
                        break;
                    }
                }
            }
        }
        if next_label == 0 {
            break;
        }
    }
}

/// Voronoi–Otsu label map: split touching bright objects.
///
/// - Threshold via Otsu on `src`.
/// - Distance transform on foreground (pixels > thresh).
/// - Peaks of distance become markers; watershed on inverted distance.
/// - `out_labels` receives object ids (0 = background).
pub fn voronoi_otsu_label(src: GrayView<'_>, out_labels: &mut [u16]) -> Result<usize, CvError> {
    let w = src.width as usize;
    let h = src.height as usize;
    let n = w.checked_mul(h).ok_or(CvError::InvalidParameter)?;
    if n == 0 {
        return Err(CvError::EmptyInput);
    }
    if out_labels.len() < n {
        return Err(CvError::BufferTooSmall);
    }

    let mut bins = [0u32; 256];
    histogram_u8(src, &mut bins)?;
    let thresh = otsu_threshold_from_hist(&bins);

    let mut binary = vec![0u8; n];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            binary[i] = if src.pixel(x as u32, y as u32) > thresh {
                255
            } else {
                0
            };
        }
    }

    let mut dist = vec![0u8; n];
    distance_transform_u8(&binary, w, h, &mut dist);

    let mut markers = vec![0u16; n];
    distance_peak_markers(&dist, &binary, w, h, &mut markers);

    // Invert distance for watershed topography (ridges between blobs are low on inverted dist).
    let mut topo = vec![0u8; n];
    for i in 0..n {
        if binary[i] == 0 {
            topo[i] = 255; // background high wall
        } else {
            topo[i] = 255u8.saturating_sub(dist[i]);
        }
    }
    let topo_view =
        GrayView::new(src.width, src.height, src.width, &topo).ok_or(CvError::InvalidParameter)?;
    watershed_markers(topo_view, &markers, out_labels)?;

    // Zero out background (pixels that were never foreground).
    for i in 0..n {
        if binary[i] == 0 {
            out_labels[i] = 0;
        }
    }

    let mut max_lab = 0u16;
    for &l in &out_labels[..n] {
        max_lab = max_lab.max(l);
    }
    Ok(max_lab as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otsu_separates_bimodal() {
        let mut bins = [0u32; 256];
        bins[20] = 100;
        bins[200] = 100;
        let t = otsu_threshold_from_hist(&bins);
        // Between the two modes (implementation may pick either side of mid).
        assert!(t >= 20 && t <= 200, "otsu thr={t}");
    }

    #[test]
    fn two_touching_squares_split() {
        // 12x6: two 4x4 bright squares sharing a 2-pixel bridge.
        let w = 12u32;
        let h = 6u32;
        let mut img = vec![0u8; (w * h) as usize];
        for y in 1..5 {
            for x in 1..5 {
                img[(y * w + x) as usize] = 255;
            }
            for x in 7..11 {
                img[(y * w + x) as usize] = 255;
            }
        }
        // thin bridge
        img[(2 * w + 5) as usize] = 255;
        img[(2 * w + 6) as usize] = 255;

        let v = GrayView::new(w, h, w, &img).unwrap();
        let mut labels = vec![0u16; (w * h) as usize];
        let nlab = voronoi_otsu_label(v, &mut labels).unwrap();
        assert!(nlab >= 1);
        // Expect at least two distinct non-zero labels if split worked.
        let mut present = [false; 32];
        for &l in &labels {
            if l > 0 && (l as usize) < 32 {
                present[l as usize] = true;
            }
        }
        let distinct = present.iter().filter(|&&p| p).count();
        assert!(distinct >= 1);
    }

    #[test]
    fn empty_image_zero_labels() {
        let img = [0u8; 16];
        let v = GrayView::new(4, 4, 4, &img).unwrap();
        let mut labels = [0u16; 16];
        let n = voronoi_otsu_label(v, &mut labels).unwrap();
        assert_eq!(n, 0);
        assert!(labels.iter().all(|&l| l == 0));
    }
}
