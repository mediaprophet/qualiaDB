//! Swarm G — native image generation ABI (Phase 8).
//!
//! Deterministic seeded generator using pure-Rust noise + iterative smooth/refine.
//! **Honest scope:** this is a native, licence-clear reference generator for the
//! ABI, receipts, and media-store path — not a foundation DiT/UNet checkpoint.
//! Swap the `step` body for Forge-backed denoiser weights when a licence-approved
//! model is selected (G0 audit). No Python / ComfyUI.

use crate::semantic::{media_digest, q_hash, MediaDigest};
use crate::types::VisionError;

pub const GENERATOR_MODEL_ID: &str = "qualia-vision-native-generator-ref-v1";

/// Immutable generation receipt for Q42 / sidecars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationReceipt {
    pub model_hash: u64,
    pub seed: u64,
    pub steps: u32,
    pub width: u32,
    pub height: u32,
    pub prompt_hash: u64,
    pub output_digest: MediaDigest,
    /// True for the built-in reference path (not a third-party foundation model).
    pub is_reference_generator: bool,
}

/// Native image generator session (cold construct; hot path is `generate_rgb8`).
#[derive(Debug, Clone)]
pub struct NativeImageGenerator {
    pub model_hash: u64,
    pub is_reference: bool,
}

impl Default for NativeImageGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeImageGenerator {
    pub fn new() -> Self {
        Self {
            model_hash: q_hash(GENERATOR_MODEL_ID),
            is_reference: true,
        }
    }

    /// Generate RGB8 into `out` (len ≥ w*h*3). Prompt is hashed only (no string retained).
    pub fn generate_rgb8(
        &self,
        prompt: &str,
        seed: u64,
        steps: u32,
        width: u32,
        height: u32,
        out: &mut [u8],
    ) -> Result<GenerationReceipt, VisionError> {
        let w = width.max(1);
        let h = height.max(1);
        let need = (w as usize).saturating_mul(h as usize).saturating_mul(3);
        if out.len() < need {
            return Err(VisionError::OutputBufferTooSmall);
        }
        let steps = steps.clamp(1, 64);
        let prompt_hash = q_hash(prompt);
        let mut state = seed ^ prompt_hash ^ self.model_hash;

        // Init noise in f32 buffer (heap cold — generation is not Tier-1 hot path).
        let n = (w * h) as usize;
        let mut rch = vec![0.0f32; n];
        let mut gch = vec![0.0f32; n];
        let mut bch = vec![0.0f32; n];
        for i in 0..n {
            state = splitmix64(state);
            rch[i] = (state as f32 / u64::MAX as f32) * 2.0 - 1.0;
            state = splitmix64(state);
            gch[i] = (state as f32 / u64::MAX as f32) * 2.0 - 1.0;
            state = splitmix64(state);
            bch[i] = (state as f32 / u64::MAX as f32) * 2.0 - 1.0;
        }

        // Prompt-conditioned color bias
        let br = ((prompt_hash) & 0xFF) as f32 / 255.0;
        let bg = ((prompt_hash >> 8) & 0xFF) as f32 / 255.0;
        let bb = ((prompt_hash >> 16) & 0xFF) as f32 / 255.0;

        for step in 0..steps {
            let t = (step + 1) as f32 / steps as f32;
            blur3_into(&rch.clone(), w, h, &mut rch);
            blur3_into(&gch.clone(), w, h, &mut gch);
            blur3_into(&bch.clone(), w, h, &mut bch);
            for i in 0..n {
                rch[i] = rch[i] * (1.0 - 0.15 * t) + br * 0.15 * t;
                gch[i] = gch[i] * (1.0 - 0.15 * t) + bg * 0.15 * t;
                bch[i] = bch[i] * (1.0 - 0.15 * t) + bb * 0.15 * t;
            }
        }

        for i in 0..n {
            out[i * 3] = ((rch[i].tanh() * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
            out[i * 3 + 1] = ((gch[i].tanh() * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
            out[i * 3 + 2] = ((bch[i].tanh() * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
        }

        let dig = media_digest(&out[..need]);
        Ok(GenerationReceipt {
            model_hash: self.model_hash,
            seed,
            steps,
            width: w,
            height: h,
            prompt_hash,
            output_digest: dig,
            is_reference_generator: self.is_reference,
        })
    }
}

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn blur3_into(src: &[f32], w: u32, h: u32, dst: &mut [f32]) {
    let w = w as usize;
    let h = h as usize;
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0f32;
            let mut c = 0.0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let xx = x as i32 + dx;
                    let yy = y as i32 + dy;
                    if xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h {
                        s += src[yy as usize * w + xx as usize];
                        c += 1.0;
                    }
                }
            }
            dst[y * w + x] = s / c.max(1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_deterministic() {
        let g = NativeImageGenerator::new();
        let mut a = vec![0u8; 16 * 16 * 3];
        let mut b = vec![0u8; 16 * 16 * 3];
        let ra = g.generate_rgb8("teal field", 99, 4, 16, 16, &mut a).unwrap();
        let rb = g.generate_rgb8("teal field", 99, 4, 16, 16, &mut b).unwrap();
        assert_eq!(a, b);
        assert_eq!(ra.output_digest, rb.output_digest);
        assert!(ra.is_reference_generator);
    }

    #[test]
    fn different_seeds_differ() {
        let g = NativeImageGenerator::new();
        let mut a = vec![0u8; 8 * 8 * 3];
        let mut b = vec![0u8; 8 * 8 * 3];
        g.generate_rgb8("x", 1, 2, 8, 8, &mut a).unwrap();
        g.generate_rgb8("x", 2, 2, 8, 8, &mut b).unwrap();
        assert_ne!(a, b);
    }
}
