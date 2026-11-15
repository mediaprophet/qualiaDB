//! 256-bin histogram of Gray8.

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Write 256 bin counts into `bins`.
pub fn histogram_u8(src: GrayView<'_>, bins: &mut [u32; 256]) -> Result<(), CvError> {
    if src.stride == src.width {
        let len = (src.width as usize).saturating_mul(src.height as usize);
        *bins = etalon_signal::vision::histogram_u8(&src.bytes[..len]);
        return Ok(());
    }
    bins.fill(0);
    for y in 0..src.height {
        let row_start = (y as usize) * (src.stride as usize);
        let row_end = row_start + (src.width as usize);
        let row_bins = etalon_signal::vision::histogram_u8(&src.bytes[row_start..row_end]);
        for i in 0..256 {
            bins[i] = bins[i].saturating_add(row_bins[i]);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts() {
        let img = [0u8, 0, 255, 128];
        let v = GrayView::new(2, 2, 2, &img).unwrap();
        let mut b = [0u32; 256];
        histogram_u8(v, &mut b).unwrap();
        assert_eq!(b[0], 2);
        assert_eq!(b[255], 1);
        assert_eq!(b[128], 1);
    }
}
