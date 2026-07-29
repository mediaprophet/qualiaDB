//! Nearest-neighbour affine warp Gray8.
//! out(x,y) = src(a*x + b*y + c, d*x + e*y + f) inverse map.

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// `m` = [a,b,c, d,e,f] maps output → input.
pub fn warp_affine_u8(
    src: GrayView<'_>,
    m: [f32; 6],
    out_w: u32,
    out_h: u32,
    out: &mut [u8],
) -> Result<(), CvError> {
    let n = (out_w * out_h) as usize;
    if out.len() < n {
        return Err(CvError::BufferTooSmall);
    }
    let [a, b, c, d, e, f] = m;
    for y in 0..out_h {
        for x in 0..out_w {
            let xs = a * x as f32 + b * y as f32 + c;
            let ys = d * x as f32 + e * y as f32 + f;
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
    fn identity() {
        let img = [1u8, 2, 3, 4];
        let v = GrayView::new(2, 2, 2, &img).unwrap();
        let mut o = [0u8; 4];
        warp_affine_u8(v, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0], 2, 2, &mut o).unwrap();
        assert_eq!(o, img);
    }
}
