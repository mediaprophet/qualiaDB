//! SFace embedding from a caller-supplied float tensor (runtime-agnostic).
//!
//! Weight: OpenCV Zoo SFace **Apache-2.0** (PermissiveReady). Embedding is for
//! sanctuary template vault — never packed into NQuin payloads.

use crate::cv::error::CvError;

/// SFace typically emits 128-d float features.
pub const SFACE_EMBED_DIM: usize = 128;

/// Copy and L2-normalize embedding into `out` (must be ≥ SFACE_EMBED_DIM).
pub fn sface_embed_from_tensor(raw: &[f32], out: &mut [f32]) -> Result<usize, CvError> {
    if raw.len() < SFACE_EMBED_DIM {
        return Err(CvError::BufferTooSmall);
    }
    if out.len() < SFACE_EMBED_DIM {
        return Err(CvError::BufferTooSmall);
    }
    let mut norm = 0.0f32;
    for i in 0..SFACE_EMBED_DIM {
        let v = raw[i];
        out[i] = v;
        norm += v * v;
    }
    let n = norm.sqrt();
    if n > 1e-8 {
        for i in 0..SFACE_EMBED_DIM {
            out[i] /= n;
        }
    }
    Ok(SFACE_EMBED_DIM)
}

/// Cosine similarity for two L2-normalized embeddings.
pub fn sface_cosine(a: &[f32], b: &[f32]) -> Result<f32, CvError> {
    if a.len() < SFACE_EMBED_DIM || b.len() < SFACE_EMBED_DIM {
        return Err(CvError::BufferTooSmall);
    }
    let mut dot = 0.0f32;
    for i in 0..SFACE_EMBED_DIM {
        dot += a[i] * b[i];
    }
    Ok(dot.clamp(-1.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_unit() {
        let mut raw = [0.0f32; SFACE_EMBED_DIM];
        raw[0] = 3.0;
        raw[1] = 4.0;
        let mut out = [0.0f32; SFACE_EMBED_DIM];
        sface_embed_from_tensor(&raw, &mut out).unwrap();
        let n: f32 = out.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-5);
    }

    #[test]
    fn identical_cosine_one() {
        let raw = [0.1f32; SFACE_EMBED_DIM];
        let mut a = [0.0f32; SFACE_EMBED_DIM];
        let mut b = [0.0f32; SFACE_EMBED_DIM];
        sface_embed_from_tensor(&raw, &mut a).unwrap();
        sface_embed_from_tensor(&raw, &mut b).unwrap();
        let c = sface_cosine(&a, &b).unwrap();
        assert!((c - 1.0).abs() < 1e-5);
    }
}
