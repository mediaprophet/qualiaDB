//! Swarm G — native image generation ABI (Phase 8).
//!
//! Deterministic seeded generator using pure-Rust noise + iterative smooth/refine.
//! **Honest scope:** this is a native, licence-clear reference generator for the
//! ABI, receipts, and media-store path — not a foundation DiT/UNet checkpoint.
//! Swap the `step` body for Forge-backed denoiser weights when a licence-approved
//! model is selected (G0 audit). No Python / ComfyUI.

use crate::semantic::{media_digest, q_hash, MediaDigest, VisionQuin};
use crate::types::VisionError;

pub const GENERATOR_MODEL_ID: &str = "qualia-vision-native-generator-ref-v1";
pub const P_GENERATED_IMAGE: &str = "https://ns.webizen.org/q42/generatedImage";
pub const P_GEN_SEED: &str = "https://ns.webizen.org/q42/generationSeed";
pub const P_GEN_PROMPT: &str = "https://ns.webizen.org/q42/generationPromptHash";
pub const CTX_GENERATION: &str = "https://ns.webizen.org/q42/vision-generation";

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
    /// Shared media timeline slot (ms or frame base) for cross-modal join with audio.
    pub media_time_ms: u64,
}

/// Cooperative cancel flag (generation is cold path; checked between steps).
#[derive(Debug, Clone, Copy, Default)]
pub struct CancelFlag {
    pub cancelled: bool,
}

impl CancelFlag {
    pub fn new() -> Self {
        Self { cancelled: false }
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
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
        self.generate_rgb8_cancellable(prompt, seed, steps, width, height, out, None, 0)
    }

    /// Same as `generate_rgb8` with optional cancel between steps and media timeline stamp.
    pub fn generate_rgb8_cancellable(
        &self,
        prompt: &str,
        seed: u64,
        steps: u32,
        width: u32,
        height: u32,
        out: &mut [u8],
        cancel: Option<&CancelFlag>,
        media_time_ms: u64,
    ) -> Result<GenerationReceipt, VisionError> {
        let w = width.max(1);
        let h = height.max(1);
        let need = (w as usize).saturating_mul(h as usize).saturating_mul(3);
        if out.len() < need {
            return Err(VisionError::OutputBufferTooSmall);
        }
        let steps = steps.clamp(1, 64);
        let prompt_hash = q_hash(prompt);
        let state = seed ^ prompt_hash ^ self.model_hash;

        let n = (w * h) as usize;
        let mut rch = vec![0.0f32; n];
        let mut gch = vec![0.0f32; n];
        let mut bch = vec![0.0f32; n];
        // Multi-octave value noise (still deterministic reference, richer structure).
        for i in 0..n {
            let x = (i % w as usize) as f32 / w as f32;
            let y = (i / w as usize) as f32 / h as f32;
            let (nr, ng, nb) = octave_noise(x, y, state);
            rch[i] = nr;
            gch[i] = ng;
            bch[i] = nb;
        }

        let br = ((prompt_hash) & 0xFF) as f32 / 255.0 * 2.0 - 1.0;
        let bg = ((prompt_hash >> 8) & 0xFF) as f32 / 255.0 * 2.0 - 1.0;
        let bb = ((prompt_hash >> 16) & 0xFF) as f32 / 255.0 * 2.0 - 1.0;

        let mut scratch = vec![0.0f32; n];
        for step in 0..steps {
            if cancel.map(|c| c.cancelled).unwrap_or(false) {
                // Leave no partial claim: zero output and fail closed.
                out[..need].fill(0);
                return Err(VisionError::BackendUnavailable);
            }
            let t = (step + 1) as f32 / steps as f32;
            blur3_into(&rch, w, h, &mut scratch);
            rch.copy_from_slice(&scratch);
            blur3_into(&gch, w, h, &mut scratch);
            gch.copy_from_slice(&scratch);
            blur3_into(&bch, w, h, &mut scratch);
            bch.copy_from_slice(&scratch);
            for i in 0..n {
                rch[i] = rch[i] * (1.0 - 0.18 * t) + br * 0.18 * t;
                gch[i] = gch[i] * (1.0 - 0.18 * t) + bg * 0.18 * t;
                bch[i] = bch[i] * (1.0 - 0.18 * t) + bb * 0.18 * t;
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
            media_time_ms,
        })
    }
}

/// Compile generation receipt into epistemic quins (media --generated--> model, seed, prompt).
pub fn compile_generation_receipt_quins(
    receipt: &GenerationReceipt,
    out: &mut [VisionQuin],
) -> usize {
    if out.len() < 3 {
        return 0;
    }
    let media = receipt.output_digest.hash;
    let ctx = q_hash(CTX_GENERATION) ^ receipt.model_hash;
    out[0] = VisionQuin::with_parity(
        media,
        q_hash(P_GENERATED_IMAGE),
        receipt.model_hash,
        ctx,
        receipt.media_time_ms,
    );
    out[1] = VisionQuin::with_parity(
        media,
        q_hash(P_GEN_SEED),
        receipt.seed,
        ctx,
        receipt.steps as u64,
    );
    out[2] = VisionQuin::with_parity(
        media,
        q_hash(P_GEN_PROMPT),
        receipt.prompt_hash,
        ctx,
        ((receipt.width as u64) << 32) | receipt.height as u64,
    );
    3
}

fn octave_noise(x: f32, y: f32, seed: u64) -> (f32, f32, f32) {
    let mut amp = 1.0f32;
    let mut freq = 1.0f32;
    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;
    let mut norm = 0.0f32;
    for o in 0..4u64 {
        let s = splitmix64(seed.wrapping_add(o * 0x9E37));
        let n = value_noise(x * freq, y * freq, s);
        r += n * amp;
        g += value_noise(x * freq + 17.0, y * freq, splitmix64(s)) * amp;
        b += value_noise(x * freq, y * freq + 31.0, splitmix64(s ^ 1)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    let inv = 1.0 / norm.max(1e-6);
    (r * inv, g * inv, b * inv)
}

fn value_noise(x: f32, y: f32, seed: u64) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let n00 = hash2(x0, y0, seed);
    let n10 = hash2(x0 + 1, y0, seed);
    let n01 = hash2(x0, y0 + 1, seed);
    let n11 = hash2(x0 + 1, y0 + 1, seed);
    let ix0 = n00 * (1.0 - sx) + n10 * sx;
    let ix1 = n01 * (1.0 - sx) + n11 * sx;
    ix0 * (1.0 - sy) + ix1 * sy
}

fn hash2(x: i32, y: i32, seed: u64) -> f32 {
    let mut h = seed
        ^ ((x as u64).wrapping_mul(0x85eb_ca6b))
        ^ ((y as u64).wrapping_mul(0xc2b2_ae35));
    h = splitmix64(h);
    (h as f32 / u64::MAX as f32) * 2.0 - 1.0
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

    #[test]
    fn cancel_fails_closed() {
        let g = NativeImageGenerator::new();
        let mut out = vec![0u8; 8 * 8 * 3];
        let mut c = CancelFlag::new();
        c.cancel();
        let r = g.generate_rgb8_cancellable("x", 1, 8, 8, 8, &mut out, Some(&c), 0);
        assert!(r.is_err());
        assert!(out.iter().all(|&b| b == 0));
    }

    #[test]
    fn receipt_quins() {
        let g = NativeImageGenerator::new();
        let mut out = vec![0u8; 8 * 8 * 3];
        let rec = g.generate_rgb8("hi", 3, 2, 8, 8, &mut out).unwrap();
        let mut q = [VisionQuin::with_parity(0, 0, 0, 0, 0); 4];
        let n = compile_generation_receipt_quins(&rec, &mut q);
        assert_eq!(n, 3);
        assert_eq!(q[0].subject, rec.output_digest.hash);
    }
}
