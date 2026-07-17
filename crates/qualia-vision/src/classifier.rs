//! Phase 4 / V4 — frozen embedding + compact linear classifier head.
//!
//! Uses the existing CPU-reference embedding (or any `VisualModel` embed) and a
//! caller-supplied weight matrix for a closed vocabulary. No Python, no deep
//! training loop — head can be filled offline or by simple accumulators.

use crate::semantic::q_hash;
use crate::types::{
    Detection, ImageView, VisionError, VisualCapabilities, VisualModel, VisualOutputCounts,
    MAX_DETECTIONS,
};

/// Compact linear head: `logits[c] = bias[c] + sum_i embed[i] * weight[c * dim + i]`.
#[derive(Debug, Clone)]
pub struct LinearHead {
    pub dim: usize,
    pub n_classes: usize,
    /// Row-major `[n_classes * dim]`.
    pub weight: Vec<f32>,
    pub bias: Vec<f32>,
    /// Class IRI hashes parallel to rows.
    pub class_hashes: Vec<u64>,
}

impl LinearHead {
    /// Build a zero head (always abstains until weights set).
    pub fn zeros(dim: usize, class_iris: &[&str]) -> Self {
        let n = class_iris.len();
        Self {
            dim,
            n_classes: n,
            weight: vec![0.0; n * dim],
            bias: vec![0.0; n],
            class_hashes: class_iris.iter().map(|s| q_hash(s)).collect(),
        }
    }

    /// Nearest-centroid style: set each class row to the provided centroid vector.
    pub fn from_centroids(dim: usize, class_iris: &[&str], centroids: &[&[f32]]) -> Result<Self, VisionError> {
        if class_iris.len() != centroids.len() {
            return Err(VisionError::MalformedImage);
        }
        let mut h = Self::zeros(dim, class_iris);
        for (c, cent) in centroids.iter().enumerate() {
            if cent.len() < dim {
                return Err(VisionError::OutputBufferTooSmall);
            }
            for i in 0..dim {
                h.weight[c * dim + i] = cent[i];
            }
        }
        Ok(h)
    }

    /// Softmax-free argmax score into detection (whole-image box).
    pub fn classify_embedding(
        &self,
        embed: &[f32],
        min_score: f32,
    ) -> Option<Detection> {
        if embed.len() < self.dim || self.n_classes == 0 {
            return None;
        }
        let mut best_c = 0usize;
        let mut best = f32::NEG_INFINITY;
        for c in 0..self.n_classes {
            let mut logit = self.bias[c];
            for i in 0..self.dim {
                logit += embed[i] * self.weight[c * self.dim + i];
            }
            if logit > best {
                best = logit;
                best_c = c;
            }
        }
        // Map logit to (0,1) via tanh for score_u16 packing.
        let score = ((best.tanh() + 1.0) * 0.5).clamp(0.0, 1.0);
        if score < min_score {
            return None;
        }
        let mut d = Detection::empty();
        d.class_hash = self.class_hashes[best_c];
        d.instance_hash = d.class_hash ^ (best.to_bits() as u64);
        d.score_u16 = (score * 65535.0) as u16;
        d.x_min_u16 = 0;
        d.y_min_u16 = 0;
        d.x_max_u16 = 65535;
        d.y_max_u16 = 65535;
        d.flags = Detection::FLAG_LOW_ASSURANCE;
        Some(d)
    }
}

/// Wraps a base visual model: takes its embedding, runs linear head, emits 0–1 detection.
pub struct LinearProbeVision<M: VisualModel> {
    pub base: M,
    pub head: LinearHead,
    pub min_score: f32,
    pub model_hash: u64,
}

impl<M: VisualModel> LinearProbeVision<M> {
    pub fn new(base: M, head: LinearHead, min_score: f32) -> Self {
        let model_hash = q_hash("qualia-vision-linear-probe-v1");
        Self {
            base,
            head,
            min_score,
            model_hash,
        }
    }

    pub fn model_hash(&self) -> u64 {
        self.model_hash
    }
}

impl<M: VisualModel> VisualModel for LinearProbeVision<M> {
    fn capabilities(&self) -> VisualCapabilities {
        let mut c = self.base.capabilities();
        c.supports_boxes = true;
        c.max_detections = 1;
        c.is_reference_backend = c.is_reference_backend || true;
        c
    }

    fn infer(
        &mut self,
        image: ImageView<'_>,
        detections_out: &mut [Detection],
        embedding_out: &mut [f32],
        workspace: &mut [u8],
    ) -> Result<VisualOutputCounts, VisionError> {
        if detections_out.is_empty() {
            return Err(VisionError::OutputBufferTooSmall);
        }
        // Base may write multiple dets; we only need embed.
        let mut scratch_dets = [Detection::empty(); MAX_DETECTIONS];
        let base_counts =
            self.base
                .infer(image, &mut scratch_dets, embedding_out, workspace)?;
        let emb_n = base_counts.embedding_written.min(embedding_out.len());
        for d in detections_out.iter_mut() {
            *d = Detection::empty();
        }
        let mut det_n = 0usize;
        if emb_n >= self.head.dim {
            if let Some(d) =
                self.head
                    .classify_embedding(&embedding_out[..emb_n], self.min_score)
            {
                detections_out[0] = d;
                det_n = 1;
            }
        }
        Ok(VisualOutputCounts {
            detections: det_n,
            embedding_written: emb_n,
        })
    }
}

/// Train a 2-class head by averaging embeddings of two example batches (centroid).
pub fn fit_two_class_centroids(
    dim: usize,
    class_a: &str,
    embeds_a: &[&[f32]],
    class_b: &str,
    embeds_b: &[&[f32]],
) -> Result<LinearHead, VisionError> {
    if embeds_a.is_empty() || embeds_b.is_empty() {
        return Err(VisionError::MalformedImage);
    }
    let mut ca = vec![0.0f32; dim];
    let mut cb = vec![0.0f32; dim];
    for e in embeds_a {
        if e.len() < dim {
            return Err(VisionError::OutputBufferTooSmall);
        }
        for i in 0..dim {
            ca[i] += e[i];
        }
    }
    for e in embeds_b {
        if e.len() < dim {
            return Err(VisionError::OutputBufferTooSmall);
        }
        for i in 0..dim {
            cb[i] += e[i];
        }
    }
    let na = embeds_a.len() as f32;
    let nb = embeds_b.len() as f32;
    for i in 0..dim {
        ca[i] /= na;
        cb[i] /= nb;
    }
    LinearHead::from_centroids(dim, &[class_a, class_b], &[&ca, &cb])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_reference::{CpuReferenceVision, CLASS_MOSTLY_BLUE, CLASS_MOSTLY_RED};
    use crate::types::PixelFormat;

    fn solid(rgb: [u8; 3], w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 3) as usize];
        for p in v.chunks_mut(3) {
            p.copy_from_slice(&rgb);
        }
        v
    }

    #[test]
    fn linear_probe_prefers_matching_centroid() {
        let mut base = CpuReferenceVision::new();
        let red = solid([220, 10, 10], 8, 8);
        let blue = solid([10, 10, 220], 8, 8);
        let img_r = ImageView {
            bytes: &red,
            width: 8,
            height: 8,
            row_stride: 24,
            format: PixelFormat::Rgb8,
        };
        let img_b = ImageView {
            bytes: &blue,
            width: 8,
            height: 8,
            row_stride: 24,
            format: PixelFormat::Rgb8,
        };
        let mut dets = [Detection::empty(); 8];
        let mut emb_r = [0.0f32; 16];
        let mut emb_b = [0.0f32; 16];
        let mut ws = [0u8; 64];
        base.infer(img_r, &mut dets, &mut emb_r, &mut ws).unwrap();
        base.infer(img_b, &mut dets, &mut emb_b, &mut ws).unwrap();
        let head = fit_two_class_centroids(
            16,
            CLASS_MOSTLY_RED,
            &[&emb_r],
            CLASS_MOSTLY_BLUE,
            &[&emb_b],
        )
        .unwrap();
        let mut probe = LinearProbeVision::new(base, head, 0.01);
        let c = probe
            .infer(img_r, &mut dets, &mut emb_r, &mut ws)
            .unwrap();
        assert_eq!(c.detections, 1);
        assert_eq!(dets[0].class_hash, q_hash(CLASS_MOSTLY_RED));
    }
}
