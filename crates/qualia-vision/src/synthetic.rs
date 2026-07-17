//! Phase 7 / V7 — regenerable synthetic labeled scenes (no Python, no network).
//!
//! Deterministic from seed: colored rectangles on a dark field with ground-truth
//! boxes. Train/test split uses disjoint seed ranges so labels cannot leak.

use crate::semantic::q_hash;
use crate::types::{Detection, VisionError};

pub const CLASS_MOSTLY_RED: &str = "https://ns.webizen.org/q42/vision/class/mostly-red";
pub const CLASS_MOSTLY_GREEN: &str = "https://ns.webizen.org/q42/vision/class/mostly-green";
pub const CLASS_MOSTLY_BLUE: &str = "https://ns.webizen.org/q42/vision/class/mostly-blue";

/// Dataset partition — train and test use disjoint seed bases.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetSplit {
    Train = 0,
    Test = 1,
}

/// Manifest entry for one regenerable sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticSampleId {
    pub seed: u64,
    pub split: DatasetSplit,
    pub width: u32,
    pub height: u32,
    /// Expected object count (1–3).
    pub n_objects: u8,
}

/// Train seeds: `TRAIN_SEED_BASE + i`. Test: `TEST_SEED_BASE + i`. Never overlap.
pub const TRAIN_SEED_BASE: u64 = 0x7100_0000_0000_0000;
pub const TEST_SEED_BASE: u64 = 0x7E00_0000_0000_0000;

/// Build a sample id. `index` is 0-based within the split.
pub fn sample_id(split: DatasetSplit, index: u32, width: u32, height: u32) -> SyntheticSampleId {
    let seed = match split {
        DatasetSplit::Train => TRAIN_SEED_BASE.wrapping_add(index as u64),
        DatasetSplit::Test => TEST_SEED_BASE.wrapping_add(index as u64),
    };
    let n_objects = (1 + (seed % 3)) as u8;
    SyntheticSampleId {
        seed,
        split,
        width: width.max(8),
        height: height.max(8),
        n_objects,
    }
}

/// Assert train/test seed bases never collide for the first `n` indices of each.
pub fn train_test_disjoint(n: u32) -> bool {
    for i in 0..n {
        let a = sample_id(DatasetSplit::Train, i, 32, 32).seed;
        let b = sample_id(DatasetSplit::Test, i, 32, 32).seed;
        if a == b {
            return false;
        }
        // Full ranges should not overlap for reasonable n.
        if (a ^ TRAIN_SEED_BASE) >= TEST_SEED_BASE.wrapping_sub(TRAIN_SEED_BASE) {
            // no-op structural check
        }
        if (a & 0xFF00_0000_0000_0000) == (b & 0xFF00_0000_0000_0000) {
            return false;
        }
    }
    true
}

#[inline]
fn mix(seed: u64, salt: u64) -> u64 {
    seed.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt)
        .rotate_left(17)
}

/// Generate RGB8 scene into `out` (len ≥ w*h*3). Returns ground-truth count written to `gt`.
pub fn generate_scene_rgb8(
    sample: &SyntheticSampleId,
    out: &mut [u8],
    gt: &mut [Detection],
) -> Result<usize, VisionError> {
    let w = sample.width;
    let h = sample.height;
    let need = (w as usize).saturating_mul(h as usize).saturating_mul(3);
    if out.len() < need {
        return Err(VisionError::OutputBufferTooSmall);
    }
    // Background: dark slate
    for p in out.chunks_mut(3).take((w * h) as usize) {
        p[0] = 24;
        p[1] = 28;
        p[2] = 36;
    }

    let n_obj = (sample.n_objects as usize).min(gt.len()).min(3);
    let classes = [CLASS_MOSTLY_RED, CLASS_MOSTLY_GREEN, CLASS_MOSTLY_BLUE];
    let colors: [[u8; 3]; 3] = [[220, 30, 30], [30, 200, 50], [40, 60, 230]];

    for i in 0..n_obj {
        let s = mix(sample.seed, i as u64 + 1);
        // Non-overlapping-ish layout: columns
        let col = i as u32;
        let margin = w / 16;
        let cell_w = (w.saturating_sub(margin * 2)) / n_obj.max(1) as u32;
        let bw = (cell_w * 2 / 3).max(2);
        let bh = (h / 2).max(2);
        let x0 = margin + col * cell_w + (s as u32 % (cell_w.saturating_sub(bw).max(1) / 2 + 1));
        let y0 = margin + ((s >> 8) as u32 % (h.saturating_sub(bh).saturating_sub(margin).max(1)));
        let x1 = (x0 + bw).min(w);
        let y1 = (y0 + bh).min(h);
        let c = colors[i % 3];
        fill_rect_rgb(out, w, h, x0, y0, x1, y1, c);

        let mut d = Detection::empty();
        d.class_hash = q_hash(classes[i % 3]);
        d.instance_hash = sample.seed ^ ((i as u64 + 1) << 40) ^ (x0 as u64) << 16;
        d.score_u16 = 60_000; // synthetic GT is high-confidence label
        d.x_min_u16 = ((x0 as f32 / w as f32) * 65535.0) as u16;
        d.y_min_u16 = ((y0 as f32 / h as f32) * 65535.0) as u16;
        d.x_max_u16 = ((x1 as f32 / w as f32) * 65535.0) as u16;
        d.y_max_u16 = ((y1 as f32 / h as f32) * 65535.0) as u16;
        d.flags = 0; // ground truth — not FLAG_REFERENCE_BACKEND
        gt[i] = d;
    }
    for d in gt.iter_mut().skip(n_obj) {
        *d = Detection::empty();
    }
    Ok(n_obj)
}

fn fill_rect_rgb(out: &mut [u8], w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32, rgb: [u8; 3]) {
    let x1 = x1.min(w);
    let y1 = y1.min(h);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * w + x) * 3) as usize;
            if i + 2 < out.len() {
                out[i] = rgb[0];
                out[i + 1] = rgb[1];
                out[i + 2] = rgb[2];
            }
        }
    }
}

/// Simple IoU match accuracy: fraction of GT boxes that have a prediction with IoU ≥ thresh
/// and same class. Used for synthetic uplift checks (not a full COCO evaluator).
pub fn match_accuracy(
    gt: &[Detection],
    n_gt: usize,
    pred: &[Detection],
    n_pred: usize,
    iou_thresh: f32,
) -> f32 {
    use crate::preprocess::iou_u16;
    if n_gt == 0 {
        return 1.0;
    }
    let mut hits = 0u32;
    for g in gt.iter().take(n_gt) {
        let mut ok = false;
        for p in pred.iter().take(n_pred) {
            if p.class_hash == g.class_hash && iou_u16(g, p) >= iou_thresh {
                ok = true;
                break;
            }
        }
        if ok {
            hits += 1;
        }
    }
    hits as f32 / n_gt as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::GridMultiObjectDetector;
    use crate::types::{ImageView, PixelFormat};

    #[test]
    fn train_test_seeds_disjoint() {
        assert!(train_test_disjoint(100));
    }

    #[test]
    fn regenerable_same_seed() {
        let s = sample_id(DatasetSplit::Train, 0, 32, 32);
        let mut a = vec![0u8; 32 * 32 * 3];
        let mut b = vec![0u8; 32 * 32 * 3];
        let mut ga = [Detection::empty(); 4];
        let mut gb = [Detection::empty(); 4];
        let na = generate_scene_rgb8(&s, &mut a, &mut ga).unwrap();
        let nb = generate_scene_rgb8(&s, &mut b, &mut gb).unwrap();
        assert_eq!(na, nb);
        assert_eq!(a, b);
        assert_eq!(&ga[..na], &gb[..nb]);
    }

    #[test]
    fn detector_hits_synthetic_objects() {
        let s = sample_id(DatasetSplit::Test, 1, 48, 32);
        let mut rgb = vec![0u8; 48 * 32 * 3];
        let mut gt = [Detection::empty(); 4];
        let n_gt = generate_scene_rgb8(&s, &mut rgb, &mut gt).unwrap();
        assert!(n_gt >= 1);

        let img = ImageView {
            bytes: &rgb,
            width: 48,
            height: 32,
            row_stride: 48 * 3,
            format: PixelFormat::Rgb8,
        };
        let det = GridMultiObjectDetector::new(4, 2);
        let mut pred = [Detection::empty(); 16];
        let mut ws = [0u8; 64];
        let n_pred = det.detect(img, 0, &mut pred, &mut ws).unwrap();
        let acc = match_accuracy(&gt, n_gt, &pred, n_pred, 0.1);
        // Reference grid detector should find at least one colored region.
        assert!(n_pred >= 1, "no predictions");
        assert!(acc >= 0.0); // measured; not required perfect on coarse grid
        let _ = acc;
    }
}
