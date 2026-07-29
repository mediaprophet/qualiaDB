//! Swarm W — evaluation metrics (synthetic always; real H1 when corpus supplied).
//!
//! Synthetic and real metrics are **never mixed** into one score.

use crate::preprocess::iou_u16;
use crate::synthetic::{generate_scene_rgb8, match_accuracy, sample_id, DatasetSplit};
use crate::types::{Detection, ImageView, PixelFormat, VisualModel, MAX_DETECTIONS};
use crate::weights::VisionBackendKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricsReport {
    pub backend: VisionBackendKind,
    pub model_hash: u64,
    pub n_samples: u32,
    pub mean_match_acc: f32,
    pub mean_detections: f32,
    /// True only when metrics ran on an external real set (H1).
    pub is_real_eval: bool,
    /// Always set when synthetic partition used.
    pub is_synthetic_eval: bool,
}

/// Run detector on `n` synthetic **test** samples; returns mean match accuracy vs GT.
pub fn evaluate_synthetic<M: VisualModel>(
    model: &mut M,
    backend: VisionBackendKind,
    model_hash: u64,
    n_samples: u32,
    width: u32,
    height: u32,
) -> MetricsReport {
    let n = n_samples.max(1).min(64);
    let mut sum_acc = 0.0f32;
    let mut sum_det = 0.0f32;
    let px = (width as usize) * (height as usize) * 3;
    let mut rgb = vec![0u8; px];
    let mut gt = [Detection::empty(); MAX_DETECTIONS];
    let mut pred = [Detection::empty(); MAX_DETECTIONS];
    let mut emb = [0.0f32; 32];
    let mut ws = [0u8; MAX_DETECTIONS];

    for i in 0..n {
        let sample = sample_id(DatasetSplit::Test, i, width, height);
        let n_gt = generate_scene_rgb8(&sample, &mut rgb, &mut gt).unwrap_or(0);
        let img = ImageView {
            bytes: &rgb,
            width,
            height,
            row_stride: width * 3,
            format: PixelFormat::Rgb8,
        };
        let counts = model.infer(img, &mut pred, &mut emb, &mut ws).unwrap_or(
            crate::types::VisualOutputCounts {
                detections: 0,
                embedding_written: 0,
            },
        );
        sum_acc += match_accuracy(&gt, n_gt, &pred, counts.detections, 0.15);
        sum_det += counts.detections as f32;
    }
    MetricsReport {
        backend,
        model_hash,
        n_samples: n,
        mean_match_acc: sum_acc / n as f32,
        mean_detections: sum_det / n as f32,
        is_real_eval: false,
        is_synthetic_eval: true,
    }
}

/// Placeholder for H1 real eval: caller supplies (image, gt) pairs.
/// Returns None if empty — never invents real metrics.
pub fn evaluate_real_held_out(
    pairs: &[(ImageView<'_>, &[Detection])],
    model: &mut impl VisualModel,
    backend: VisionBackendKind,
    model_hash: u64,
    iou_thresh: f32,
) -> Option<MetricsReport> {
    if pairs.is_empty() {
        return None;
    }
    let mut sum_acc = 0.0f32;
    let mut sum_det = 0.0f32;
    let mut pred = [Detection::empty(); MAX_DETECTIONS];
    let mut emb = [0.0f32; 32];
    let mut ws = [0u8; MAX_DETECTIONS];
    for (img, gt) in pairs {
        let counts = model.infer(*img, &mut pred, &mut emb, &mut ws).ok()?;
        sum_acc += match_accuracy(gt, gt.len(), &pred, counts.detections, iou_thresh);
        sum_det += counts.detections as f32;
    }
    let n = pairs.len() as u32;
    Some(MetricsReport {
        backend,
        model_hash,
        n_samples: n,
        mean_match_acc: sum_acc / n as f32,
        mean_detections: sum_det / n as f32,
        is_real_eval: true,
        is_synthetic_eval: false,
    })
}

/// Mean IoU of best-matching same-class pairs (diagnostic).
pub fn mean_best_iou(gt: &[Detection], n_gt: usize, pred: &[Detection], n_pred: usize) -> f32 {
    if n_gt == 0 {
        return 1.0;
    }
    let mut s = 0.0f32;
    for g in gt.iter().take(n_gt) {
        let mut best = 0.0f32;
        for p in pred.iter().take(n_pred) {
            if p.class_hash == g.class_hash {
                best = best.max(iou_u16(g, p));
            }
        }
        s += best;
    }
    s / n_gt as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::{GridMultiObjectDetector, CLASS_MOSTLY_BLUE, CLASS_MOSTLY_RED};
    use crate::weights::{ProductionVision, VisionWeightBundle};

    #[test]
    fn synthetic_metrics_reference() {
        let mut m = GridMultiObjectDetector::new(4, 3);
        let mh = m.model_hash();
        let r = evaluate_synthetic(&mut m, VisionBackendKind::Reference, mh, 4, 48, 32);
        assert!(r.is_synthetic_eval);
        assert!(!r.is_real_eval);
        assert_eq!(r.n_samples, 4);
        assert!(r.mean_detections >= 0.0);
    }

    #[test]
    fn synthetic_metrics_production() {
        let b = VisionWeightBundle::from_seed(7, 16, &[CLASS_MOSTLY_RED, CLASS_MOSTLY_BLUE]);
        let h = b.model_hash();
        let mut m = ProductionVision::new(b);
        let r = evaluate_synthetic(&mut m, VisionBackendKind::ProductionWeights, h, 3, 32, 32);
        assert!(r.is_synthetic_eval);
        assert_eq!(r.backend, VisionBackendKind::ProductionWeights);
    }

    #[test]
    fn real_eval_empty_is_none() {
        let mut m = GridMultiObjectDetector::new(2, 2);
        assert!(
            evaluate_real_held_out(&[], &mut m, VisionBackendKind::Reference, 0, 0.5).is_none()
        );
    }
}
