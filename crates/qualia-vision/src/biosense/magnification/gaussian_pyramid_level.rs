//! One Gaussian pyramid downsample (2×) into caller buffer.

use crate::cv::error::CvError;

/// Downsample gray `src` (w×h) → `dst` ((w/2)×(h/2)) with 2×2 box (approx Gaussian).
pub fn gaussian_pyramid_down_u8(
    src: &[u8],
    width: u32,
    height: u32,
    dst: &mut [u8],
) -> Result<(u32, u32), CvError> {
    if width < 2 || height < 2 {
        return Err(CvError::InvalidParameter);
    }
    let dw = width / 2;
    let dh = height / 2;
    let need_src = (width * height) as usize;
    let need_dst = (dw * dh) as usize;
    if src.len() < need_src || dst.len() < need_dst {
        return Err(CvError::BufferTooSmall);
    }
    let w = width as usize;
    for y in 0..dh as usize {
        for x in 0..dw as usize {
            let x0 = x * 2;
            let y0 = y * 2;
            let s = src[y0 * w + x0] as u32
                + src[y0 * w + x0 + 1] as u32
                + src[(y0 + 1) * w + x0] as u32
                + src[(y0 + 1) * w + x0 + 1] as u32;
            dst[y * dw as usize + x] = (s / 4) as u8;
        }
    }
    Ok((dw, dh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_half() {
        let src = [10u8; 16]; // 4x4
        let mut dst = [0u8; 4];
        let (w, h) = gaussian_pyramid_down_u8(&src, 4, 4, &mut dst).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(dst[0], 10);
    }
}
