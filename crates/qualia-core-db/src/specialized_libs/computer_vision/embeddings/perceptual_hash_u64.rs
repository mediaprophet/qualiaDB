//! aHash / dHash perceptual hashes over a gray buffer.
//!
//! **Honest scope:** compact image fingerprints for local near-duplicate / CBIR
//! proxy search — **not** semantic CLIP embeddings. When ONNX CLIP (or similar)
//! lands under `vendor/vision/embeddings/`, use that for open-vocab meaning;
//! keep these hashes for cheap layout/structure pre-filter.

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Side length for aHash (8×8 → 64 bits).
pub const AHASH_SIDE: u32 = 8;
/// dHash samples (WIDTH+1)×HEIGHT so adjacent horizontal diffs yield 64 bits.
pub const DHASH_WIDTH: u32 = 9;
pub const DHASH_HEIGHT: u32 = 8;

/// Average hash: nearest-neighbor sample to 8×8, bit = pixel ≥ mean.
///
/// Returns a 64-bit fingerprint. Identical inputs → identical hash (distance 0).
pub fn ahash_u64(src: GrayView<'_>) -> Result<u64, CvError> {
    if src.width == 0 || src.height == 0 {
        return Err(CvError::EmptyInput);
    }
    let mut samples = [0u8; 64];
    sample_nn(src, AHASH_SIDE, AHASH_SIDE, &mut samples);
    let mut sum = 0u32;
    for &p in &samples {
        sum += p as u32;
    }
    let mean = (sum / 64) as u8;
    let mut hash = 0u64;
    for (i, &p) in samples.iter().enumerate() {
        if p >= mean {
            hash |= 1u64 << i;
        }
    }
    Ok(hash)
}

/// Difference hash: nearest-neighbor sample to 9×8, bit = right ≥ left (row-wise).
pub fn dhash_u64(src: GrayView<'_>) -> Result<u64, CvError> {
    if src.width == 0 || src.height == 0 {
        return Err(CvError::EmptyInput);
    }
    let mut samples = [0u8; (DHASH_WIDTH * DHASH_HEIGHT) as usize];
    sample_nn(src, DHASH_WIDTH, DHASH_HEIGHT, &mut samples);
    let mut hash = 0u64;
    let mut bit = 0u32;
    for y in 0..DHASH_HEIGHT {
        let row = (y * DHASH_WIDTH) as usize;
        for x in 0..(DHASH_WIDTH - 1) {
            let left = samples[row + x as usize];
            let right = samples[row + x as usize + 1];
            if right >= left {
                hash |= 1u64 << bit;
            }
            bit += 1;
        }
    }
    Ok(hash)
}

/// Nearest-neighbour downsample into `out` (must be `w * h` bytes).
fn sample_nn(src: GrayView<'_>, w: u32, h: u32, out: &mut [u8]) {
    debug_assert_eq!(out.len(), (w * h) as usize);
    for y in 0..h {
        let sy = ((y as u64 * src.height as u64) / h as u64) as u32;
        let sy = sy.min(src.height - 1);
        for x in 0..w {
            let sx = ((x as u64 * src.width as u64) / w as u64) as u32;
            let sx = sx.min(src.width - 1);
            out[(y * w + x) as usize] = src.pixel(sx, sy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray(w: u32, h: u32, bytes: &[u8]) -> GrayView<'_> {
        GrayView::new(w, h, w, bytes).expect("view")
    }

    #[test]
    fn ahash_identical_equal() {
        let img = [10u8, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160];
        let v = gray(4, 4, &img);
        let a = ahash_u64(v).unwrap();
        let b = ahash_u64(v).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn dhash_identical_equal() {
        let img = [0u8, 255, 0, 255, 128, 64, 192, 32, 16, 240, 80, 160, 200, 40, 90, 10];
        let v = gray(4, 4, &img);
        assert_eq!(dhash_u64(v).unwrap(), dhash_u64(v).unwrap());
    }

    #[test]
    fn solid_vs_checker_differ() {
        let solid = [128u8; 64];
        let mut checker = [0u8; 64];
        for i in 0..64 {
            checker[i] = if (i / 8 + i % 8) % 2 == 0 { 255 } else { 0 };
        }
        let vs = gray(8, 8, &solid);
        let vc = gray(8, 8, &checker);
        assert_ne!(ahash_u64(vs).unwrap(), ahash_u64(vc).unwrap());
        assert_ne!(dhash_u64(vs).unwrap(), dhash_u64(vc).unwrap());
    }

    #[test]
    fn empty_rejected() {
        // GrayView::new rejects zero size; call path still documents EmptyInput for API.
        let empty: [u8; 0] = [];
        assert!(GrayView::new(0, 0, 0, &empty).is_none());
    }
}
