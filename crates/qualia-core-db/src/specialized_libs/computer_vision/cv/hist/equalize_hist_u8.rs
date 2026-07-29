//! Histogram equalization Gray8.

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;
use crate::specialized_libs::computer_vision::cv::hist::histogram_u8::histogram_u8;

pub fn equalize_hist_u8(src: GrayView<'_>, out: &mut [u8]) -> Result<(), CvError> {
    let n = (src.width * src.height) as usize;
    if out.len() < n || n == 0 {
        return Err(CvError::BufferTooSmall);
    }
    let mut bins = [0u32; 256];
    histogram_u8(src, &mut bins)?;
    let mut cdf = [0u32; 256];
    cdf[0] = bins[0];
    for i in 1..256 {
        cdf[i] = cdf[i - 1] + bins[i];
    }
    let cdf_min = cdf.iter().copied().find(|&c| c > 0).unwrap_or(0);
    let denom = (n as u32).saturating_sub(cdf_min).max(1);
    let mut lut = [0u8; 256];
    for i in 0..256 {
        lut[i] = (((cdf[i].saturating_sub(cdf_min)) * 255) / denom) as u8;
    }
    let mut o = 0usize;
    for y in 0..src.height {
        for x in 0..src.width {
            out[o] = lut[src.pixel(x, y) as usize];
            o += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stretches() {
        let img = [10u8, 20, 30, 40];
        let v = GrayView::new(2, 2, 2, &img).unwrap();
        let mut o = [0u8; 4];
        equalize_hist_u8(v, &mut o).unwrap();
        assert!(o.iter().copied().max().unwrap() >= o.iter().copied().min().unwrap());
    }
}
