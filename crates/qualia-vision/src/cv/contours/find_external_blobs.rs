//! External blob bounding boxes via flood fill (caller-buffered).

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

#[derive(Debug, Clone, Copy, Default)]
pub struct BlobBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub area: u32,
}

/// Find up to `out.len()` external blobs where pixel ≥ `thresh`.
pub fn find_external_blobs(
    src: GrayView<'_>,
    thresh: u8,
    out: &mut [BlobBox],
) -> Result<usize, CvError> {
    let w = src.width as usize;
    let h = src.height as usize;
    let mut seen = vec![false; w * h];
    let mut n = 0usize;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if seen[i] || src.pixel(x as u32, y as u32) < thresh {
                continue;
            }
            if n >= out.len() {
                return Ok(n);
            }
            // BFS flood
            let mut q = vec![(x, y)];
            seen[i] = true;
            let mut min_x = x;
            let mut max_x = x;
            let mut min_y = y;
            let mut max_y = y;
            let mut area = 0u32;
            while let Some((cx, cy)) = q.pop() {
                area += 1;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);
                for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let nx = nx as usize;
                    let ny = ny as usize;
                    let ni = ny * w + nx;
                    if seen[ni] || src.pixel(nx as u32, ny as u32) < thresh {
                        continue;
                    }
                    seen[ni] = true;
                    q.push((nx, ny));
                }
            }
            out[n] = BlobBox {
                x: min_x as u32,
                y: min_y as u32,
                w: (max_x - min_x + 1) as u32,
                h: (max_y - min_y + 1) as u32,
                area,
            };
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_square() {
        let mut img = vec![0u8; 64];
        for y in 2..6 {
            for x in 2..6 {
                img[y * 8 + x] = 255;
            }
        }
        let v = GrayView::new(8, 8, 8, &img).unwrap();
        let mut boxes = [BlobBox::default(); 4];
        let n = find_external_blobs(v, 128, &mut boxes).unwrap();
        assert_eq!(n, 1);
        assert_eq!(boxes[0].area, 16);
    }
}
