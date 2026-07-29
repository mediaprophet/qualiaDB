//! Draw axis-aligned rectangle on Gray8.

use crate::specialized_libs::computer_vision::cv::error::CvError;

pub fn draw_rect_u8(
    img: &mut [u8],
    stride: u32,
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    value: u8,
) -> Result<(), CvError> {
    if img.len() < (stride * height) as usize {
        return Err(CvError::BufferTooSmall);
    }
    let x1 = x.min(width.saturating_sub(1));
    let y1 = y.min(height.saturating_sub(1));
    let x2 = (x + w).min(width).saturating_sub(1);
    let y2 = (y + h).min(height).saturating_sub(1);
    for xx in x1..=x2 {
        img[(y1 * stride + xx) as usize] = value;
        img[(y2 * stride + xx) as usize] = value;
    }
    for yy in y1..=y2 {
        img[(yy * stride + x1) as usize] = value;
        img[(yy * stride + x2) as usize] = value;
    }
    Ok(())
}
