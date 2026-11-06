//! Mean absolute frame difference (motion energy).

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

/// Motion energy.
pub fn motion_energy(prev: GrayView<'_>, next: GrayView<'_>) -> Result<f32, CvError> {
    if prev.width != next.width || prev.height != next.height {
        return Err(CvError::DimensionMismatch);
    }
    if prev.stride == prev.width && next.stride == next.width {
        let len = (prev.width as usize).saturating_mul(prev.height as usize);
        return Ok(etalon_signal::vision::frame_motion_mad_u8(
            &prev.bytes[..len],
            &next.bytes[..len],
        ));
    }
    let mut s = 0.0f32;
    let mut n = 0.0f32;
    for y in 0..prev.height {
        for x in 0..prev.width {
            let d = (prev.pixel(x, y) as i16 - next.pixel(x, y) as i16).unsigned_abs() as f32;
            s += d;
            n += 1.0;
        }
    }
    Ok(s / n.max(1.0))
}
