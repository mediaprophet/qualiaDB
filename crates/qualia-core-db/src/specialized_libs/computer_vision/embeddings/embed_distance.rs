//! Distances for local visual embedding proxies (hash Hamming + float cosine).
//!
//! Cosine helpers assume caller-normalized vectors when using
//! [`cosine_distance`] for CBIR ranking (identical L2 unit vectors → 0).

use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Popcount of XOR bits between two 64-bit perceptual hashes.
#[inline]
pub fn hamming_distance_u64(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Cosine similarity `dot(a,b) / (|a||b|)` over equal-length slices.
///
/// Empty or mismatched lengths → error. Zero vectors → 0.0 similarity.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32, CvError> {
    if a.is_empty() || b.is_empty() {
        return Err(CvError::EmptyInput);
    }
    if a.len() != b.len() {
        return Err(CvError::DimensionMismatch);
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom <= 1e-12 {
        return Ok(0.0);
    }
    Ok((dot / denom).clamp(-1.0, 1.0))
}

/// Cosine distance `1 − cosine_similarity` (identical unit vectors → 0).
pub fn cosine_distance(a: &[f32], b: &[f32]) -> Result<f32, CvError> {
    Ok(1.0 - cosine_similarity(a, b)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::cv::buffer::{GrayView, RgbView};
    use crate::specialized_libs::computer_vision::embeddings::color_hist_embed::{
        color_hist_embed_rgb, COLOR_HIST_EMBED_DIM,
    };
    use crate::specialized_libs::computer_vision::embeddings::perceptual_hash_u64::{
        ahash_u64, dhash_u64,
    };

    #[test]
    fn hamming_identical_zero() {
        assert_eq!(
            hamming_distance_u64(0xDEAD_BEEF_CAFE_BABE, 0xDEAD_BEEF_CAFE_BABE),
            0
        );
    }

    #[test]
    fn hamming_one_bit() {
        assert_eq!(hamming_distance_u64(0, 1), 1);
        assert_eq!(hamming_distance_u64(0b1111, 0b0000), 4);
    }

    #[test]
    fn cosine_identical_distance_zero() {
        let v = [0.6f32, 0.8];
        assert!((cosine_distance(&v, &v).unwrap()).abs() < 1e-6);
        assert!((cosine_similarity(&v, &v).unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal() {
        let a = [1.0f32, 0.0];
        let b = [0.0f32, 1.0];
        assert!((cosine_similarity(&a, &b).unwrap()).abs() < 1e-6);
        assert!((cosine_distance(&a, &b).unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn end_to_end_identical_images_distance_zero() {
        // Gray ramp for hashes
        let mut gray_bytes = [0u8; 16];
        for i in 0..16 {
            gray_bytes[i] = (i as u8).wrapping_mul(17);
        }
        let g = GrayView::new(4, 4, 4, &gray_bytes).unwrap();
        let ha = ahash_u64(g).unwrap();
        let hb = ahash_u64(g).unwrap();
        assert_eq!(hamming_distance_u64(ha, hb), 0);
        let da = dhash_u64(g).unwrap();
        let db = dhash_u64(g).unwrap();
        assert_eq!(hamming_distance_u64(da, db), 0);

        // RGB for colour embed
        let rgb_bytes = [
            40u8, 80, 120, 40, 80, 120, 40, 80, 120, 40, 80, 120, 40, 80, 120, 40, 80, 120, 40, 80,
            120, 40, 80, 120, 40, 80, 120,
        ];
        let r = RgbView::new(3, 3, 9, &rgb_bytes).unwrap();
        let mut e0 = [0.0f32; COLOR_HIST_EMBED_DIM];
        let mut e1 = [0.0f32; COLOR_HIST_EMBED_DIM];
        color_hist_embed_rgb(r, &mut e0).unwrap();
        color_hist_embed_rgb(r, &mut e1).unwrap();
        assert!((cosine_distance(&e0, &e1).unwrap()).abs() < 1e-6);
    }

    #[test]
    fn end_to_end_different_images_differ() {
        // Solid vs decreasing ramp: aHash mean threshold + dHash all-equal vs all-falling.
        let solid = [100u8; 64];
        let mut ramp = [0u8; 64];
        for y in 0..8 {
            for x in 0..8 {
                // Strictly decreasing left→right so dHash bits are 0 (right < left).
                ramp[y * 8 + x] = 255u8.saturating_sub((x as u8).saturating_mul(32));
            }
        }
        let gs = GrayView::new(8, 8, 8, &solid).unwrap();
        let gr = GrayView::new(8, 8, 8, &ramp).unwrap();
        assert!(hamming_distance_u64(ahash_u64(gs).unwrap(), ahash_u64(gr).unwrap()) > 0);
        assert!(hamming_distance_u64(dhash_u64(gs).unwrap(), dhash_u64(gr).unwrap()) > 0);

        let red = [220u8, 20, 20, 220, 20, 20, 220, 20, 20, 220, 20, 20];
        let green = [20u8, 220, 20, 20, 220, 20, 20, 220, 20, 20, 220, 20];
        let mut er = [0.0f32; COLOR_HIST_EMBED_DIM];
        let mut eg = [0.0f32; COLOR_HIST_EMBED_DIM];
        color_hist_embed_rgb(RgbView::new(2, 2, 6, &red).unwrap(), &mut er).unwrap();
        color_hist_embed_rgb(RgbView::new(2, 2, 6, &green).unwrap(), &mut eg).unwrap();
        assert!(cosine_distance(&er, &eg).unwrap() > 0.1);
    }

    #[test]
    fn dim_mismatch() {
        assert_eq!(
            cosine_similarity(&[1.0], &[1.0, 0.0]),
            Err(CvError::DimensionMismatch)
        );
    }
}
