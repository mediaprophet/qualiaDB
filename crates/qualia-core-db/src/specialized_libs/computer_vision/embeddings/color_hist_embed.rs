//! Fixed-size RGB colour histogram → L2-normalized `f32` embedding (caller buffer).
//!
//! **Honest scope:** local CBIR colour proxy only. Not a foundation multimodal
//! embedding (CLIP). Suitable for decentralized near-colour search until an
//! ONNX CLIP/ResNet path is placed under `vendor/vision/embeddings/`.

use crate::specialized_libs::computer_vision::cv::buffer::RgbView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Bins per RGB channel for the joint histogram.
pub const COLOR_HIST_BINS: usize = 4;
/// Embedding dimension: joint 4×4×4 RGB histogram (64 floats).
pub const COLOR_HIST_EMBED_DIM: usize = COLOR_HIST_BINS * COLOR_HIST_BINS * COLOR_HIST_BINS;

/// Fill `out[..COLOR_HIST_EMBED_DIM]` with an L2-normalized joint RGB histogram.
///
/// Bin index: `((r_bin * B + g_bin) * B + b_bin)` with `B = COLOR_HIST_BINS`.
/// Returns the written length (`COLOR_HIST_EMBED_DIM`).
pub fn color_hist_embed_rgb(src: RgbView<'_>, out: &mut [f32]) -> Result<usize, CvError> {
    if src.width == 0 || src.height == 0 {
        return Err(CvError::EmptyInput);
    }
    if out.len() < COLOR_HIST_EMBED_DIM {
        return Err(CvError::BufferTooSmall);
    }

    let mut counts = [0u32; COLOR_HIST_EMBED_DIM];
    let bins = COLOR_HIST_BINS as u32;
    for y in 0..src.height {
        for x in 0..src.width {
            let (r, g, b) = src.pixel(x, y);
            let rb = bin_u8(r, bins);
            let gb = bin_u8(g, bins);
            let bb = bin_u8(b, bins);
            let idx = ((rb * bins + gb) * bins + bb) as usize;
            counts[idx] += 1;
        }
    }

    let total = (src.width as u64)
        .saturating_mul(src.height as u64)
        .max(1) as f32;
    let mut sum_sq = 0.0f32;
    for i in 0..COLOR_HIST_EMBED_DIM {
        let v = counts[i] as f32 / total;
        out[i] = v;
        sum_sq += v * v;
    }
    let n = sum_sq.sqrt();
    if n > 1e-12 {
        for i in 0..COLOR_HIST_EMBED_DIM {
            out[i] /= n;
        }
    }
    // Clear any tail the caller may have left dirty beyond our write.
    // (We only require `out.len() >= DIM`; we do not touch beyond DIM.)
    Ok(COLOR_HIST_EMBED_DIM)
}

#[inline]
fn bin_u8(v: u8, bins: u32) -> u32 {
    // Map 0..=255 into 0..bins without saturating 255 into an extra bin.
    ((v as u32 * bins) / 256).min(bins - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::embeddings::embed_distance::{cosine_distance, cosine_similarity};

    fn rgb(w: u32, h: u32, bytes: &[u8]) -> RgbView<'_> {
        RgbView::new(w, h, w * 3, bytes).expect("rgb view")
    }

    #[test]
    fn identical_distance_zero() {
        // Solid red 2×2
        let img = [200u8, 10, 10, 200, 10, 10, 200, 10, 10, 200, 10, 10];
        let v = rgb(2, 2, &img);
        let mut a = [0.0f32; COLOR_HIST_EMBED_DIM];
        let mut b = [0.0f32; COLOR_HIST_EMBED_DIM];
        color_hist_embed_rgb(v, &mut a).unwrap();
        color_hist_embed_rgb(v, &mut b).unwrap();
        let d = cosine_distance(&a, &b).unwrap();
        assert!(d.abs() < 1e-6, "identical colour embeds must have cosine distance ~0, got {d}");
        let s = cosine_similarity(&a, &b).unwrap();
        assert!((s - 1.0).abs() < 1e-5);
    }

    #[test]
    fn different_colours_differ() {
        let red = [255u8, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0];
        let blue = [0u8, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 255];
        let mut er = [0.0f32; COLOR_HIST_EMBED_DIM];
        let mut eb = [0.0f32; COLOR_HIST_EMBED_DIM];
        color_hist_embed_rgb(rgb(2, 2, &red), &mut er).unwrap();
        color_hist_embed_rgb(rgb(2, 2, &blue), &mut eb).unwrap();
        let d = cosine_distance(&er, &eb).unwrap();
        assert!(d > 0.5, "red vs blue should be well separated, d={d}");
    }

    #[test]
    fn unit_norm() {
        let img = [10u8, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let mut out = [0.0f32; COLOR_HIST_EMBED_DIM];
        color_hist_embed_rgb(rgb(2, 2, &img), &mut out).unwrap();
        let n: f32 = out.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-5, "norm={n}");
    }

    #[test]
    fn buffer_too_small() {
        let img = [0u8; 12];
        let mut tiny = [0.0f32; 8];
        assert_eq!(
            color_hist_embed_rgb(rgb(2, 2, &img), &mut tiny),
            Err(CvError::BufferTooSmall)
        );
    }
}
