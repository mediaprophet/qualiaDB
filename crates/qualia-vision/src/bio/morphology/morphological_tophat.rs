//! White / black morphological top-hat via existing 3×3 erode / dilate.
//!
//! White top-hat (bright structure on dark): `I − open(I) = I − dilate(erode(I))`.
//! Black top-hat (dark structure on bright): `close(I) − I = erode(dilate(I)) − I`.
//! RMP-style structure enhancement for nuclei / stain peaks.

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;
use crate::cv::morph::{dilate_u8, erode_u8};

/// Top-hat polarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopHatKind {
    /// Bright peaks: image − opening.
    White,
    /// Dark pits: closing − image.
    Black,
}

/// Morphological top-hat into caller buffer (`width * height` bytes).
pub fn morphological_tophat(
    src: GrayView<'_>,
    kind: TopHatKind,
    out: &mut [u8],
) -> Result<(), CvError> {
    let w = src.width as usize;
    let h = src.height as usize;
    let n = w.checked_mul(h).ok_or(CvError::InvalidParameter)?;
    if n == 0 {
        return Err(CvError::EmptyInput);
    }
    if out.len() < n {
        return Err(CvError::BufferTooSmall);
    }

    let mut tmp_a = vec![0u8; n];
    let mut tmp_b = vec![0u8; n];

    match kind {
        TopHatKind::White => {
            // open = dilate(erode(src))
            erode_u8(src, &mut tmp_a)?;
            let eroded = GrayView::new(src.width, src.height, src.width, &tmp_a)
                .ok_or(CvError::InvalidParameter)?;
            dilate_u8(eroded, &mut tmp_b)?;
            for i in 0..n {
                let s = src.bytes[(i / w) * src.stride as usize + (i % w)];
                out[i] = s.saturating_sub(tmp_b[i]);
            }
        }
        TopHatKind::Black => {
            // close = erode(dilate(src))
            dilate_u8(src, &mut tmp_a)?;
            let dilated = GrayView::new(src.width, src.height, src.width, &tmp_a)
                .ok_or(CvError::InvalidParameter)?;
            erode_u8(dilated, &mut tmp_b)?;
            for i in 0..n {
                let s = src.bytes[(i / w) * src.stride as usize + (i % w)];
                out[i] = tmp_b[i].saturating_sub(s);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_tophat_highlights_peak() {
        let mut img = [0u8; 25];
        img[12] = 255;
        let v = GrayView::new(5, 5, 5, &img).unwrap();
        let mut out = [0u8; 25];
        morphological_tophat(v, TopHatKind::White, &mut out).unwrap();
        // Peak should have non-zero response relative to flat background.
        assert!(out[12] > 0 || out.iter().any(|&p| p > 0));
        // Corners of flat zero stay zero after open of a single pixel (erode kills it).
        assert_eq!(out[0], 0);
    }

    #[test]
    fn black_tophat_highlights_pit() {
        let mut img = [255u8; 25];
        img[12] = 0;
        let v = GrayView::new(5, 5, 5, &img).unwrap();
        let mut out = [0u8; 25];
        morphological_tophat(v, TopHatKind::Black, &mut out).unwrap();
        assert!(out.iter().any(|&p| p > 0));
    }

    #[test]
    fn buffer_too_small() {
        let img = [1u8; 4];
        let v = GrayView::new(2, 2, 2, &img).unwrap();
        let mut out = [0u8; 1];
        assert_eq!(
            morphological_tophat(v, TopHatKind::White, &mut out),
            Err(CvError::BufferTooSmall)
        );
    }
}
