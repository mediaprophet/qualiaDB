//! Nearest-neighbour perspective warp Gray8.
//! Homography H 3x3 row-major maps output → input (x',y',w').

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

pub fn warp_perspective_u8(
    src: GrayView<'_>,
    h: [f32; 9],
    out_w: u32,
    out_h: u32,
    out: &mut [u8],
) -> Result<(), CvError> {
    let n = (out_w * out_h) as usize;
    if out.len() < n {
        return Err(CvError::BufferTooSmall);
    }
    for y in 0..out_h {
        for x in 0..out_w {
            let xf = x as f32;
            let yf = y as f32;
            let w = h[6] * xf + h[7] * yf + h[8];
            if w.abs() < 1e-8 {
                out[(y * out_w + x) as usize] = 0;
                continue;
            }
            let xs = (h[0] * xf + h[1] * yf + h[2]) / w;
            let ys = (h[3] * xf + h[4] * yf + h[5]) / w;
            let xi = xs.round() as i32;
            let yi = ys.round() as i32;
            let v = if xi >= 0 && yi >= 0 && xi < src.width as i32 && yi < src.height as i32 {
                src.pixel(xi as u32, yi as u32)
            } else {
                0
            };
            out[(y * out_w + x) as usize] = v;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_h() {
        let img = [9u8, 8, 7, 6];
        let v = GrayView::new(2, 2, 2, &img).unwrap();
        let mut o = [0u8; 4];
        let h = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        warp_perspective_u8(v, h, 2, 2, &mut o).unwrap();
        assert_eq!(o, img);
    }
}
