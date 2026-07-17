//! Phase 5 / V5 — multi-object detector pipeline (grid head + NMS).
//!
//! Honest scope: pure-Rust, fixed-buffer path over a configurable cell grid.
//! Not a neural YOLO head — scores each cell by channel mean / edge energy
//! (same family as `CpuReferenceVision`) then applies class-agnostic NMS.
//! Replace the cell scorer with a P64 encoder when weights land; keep the
//! pipeline + `VisualModel` surface.

use crate::preprocess::nms_class_agnostic;
use crate::semantic::q_hash;
use crate::types::{
    Detection, ImageView, PixelFormat, VisionError, VisualCapabilities, VisualModel,
    VisualOutputCounts, MAX_DETECTIONS,
};

/// Stable class IRIs (shared taxonomy with cpu_reference).
pub const CLASS_MOSTLY_RED: &str = "https://ns.webizen.org/q42/vision/class/mostly-red";
pub const CLASS_MOSTLY_GREEN: &str = "https://ns.webizen.org/q42/vision/class/mostly-green";
pub const CLASS_MOSTLY_BLUE: &str = "https://ns.webizen.org/q42/vision/class/mostly-blue";
pub const CLASS_HIGH_EDGE: &str = "https://ns.webizen.org/q42/vision/class/high-edge-energy";
pub const CLASS_LOW_CONTRAST: &str = "https://ns.webizen.org/q42/vision/class/low-contrast";

const MODEL_ID: &str = "qualia-vision-grid-detector-v1";

/// Maximum grid dimension (NxN cells before NMS). 8×8 = 64 = MAX_DETECTIONS.
pub const MAX_GRID: u32 = 8;

/// Configurable multi-object grid detector.
#[derive(Debug, Clone)]
pub struct GridMultiObjectDetector {
    pub grid_w: u32,
    pub grid_h: u32,
    pub min_score: f32,
    pub nms_iou: f32,
    model_hash: u64,
}

impl Default for GridMultiObjectDetector {
    fn default() -> Self {
        Self::new(2, 2)
    }
}

impl GridMultiObjectDetector {
    pub fn new(grid_w: u32, grid_h: u32) -> Self {
        let gw = grid_w.clamp(1, MAX_GRID);
        let gh = grid_h.clamp(1, MAX_GRID);
        Self {
            grid_w: gw,
            grid_h: gh,
            min_score: 0.15,
            nms_iou: 0.45,
            model_hash: q_hash(MODEL_ID) ^ ((gw as u64) << 8) ^ (gh as u64),
        }
    }

    pub fn model_hash(&self) -> u64 {
        self.model_hash
    }

    fn sample_rgb(img: ImageView<'_>, x: u32, y: u32) -> (u8, u8, u8) {
        let bpp = img.bytes_per_pixel() as usize;
        let off = (y as usize)
            .saturating_mul(img.row_stride as usize)
            .saturating_add((x as usize).saturating_mul(bpp));
        if off + bpp > img.bytes.len() {
            return (0, 0, 0);
        }
        match img.format {
            PixelFormat::Gray8 => {
                let g = img.bytes[off];
                (g, g, g)
            }
            PixelFormat::Rgb8 | PixelFormat::Rgba8 => {
                (img.bytes[off], img.bytes[off + 1], img.bytes[off + 2])
            }
            PixelFormat::Bgr8 => (img.bytes[off + 2], img.bytes[off + 1], img.bytes[off]),
            PixelFormat::RgbF32 => (0, 0, 0),
        }
    }

    /// Score one cell into a Detection (class + box + score). Returns None if below min_score.
    fn score_cell(
        &self,
        image: ImageView<'_>,
        cx: u32,
        cy: u32,
        frame_index: u32,
    ) -> Option<Detection> {
        let w = image.width;
        let h = image.height;
        let x0 = cx * w / self.grid_w;
        let y0 = cy * h / self.grid_h;
        let x1 = if cx + 1 >= self.grid_w {
            w
        } else {
            (cx + 1) * w / self.grid_w
        };
        let y1 = if cy + 1 >= self.grid_h {
            h
        } else {
            (cy + 1) * h / self.grid_h
        };
        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        let mut sum_r = 0u64;
        let mut sum_g = 0u64;
        let mut sum_b = 0u64;
        let mut edge = 0u64;
        let mut n = 0u64;
        let mut y = y0;
        while y < y1 {
            let mut x = x0;
            while x < x1 {
                let (r, g, b) = Self::sample_rgb(image, x, y);
                sum_r += r as u64;
                sum_g += g as u64;
                sum_b += b as u64;
                if x + 1 < x1 {
                    let (r2, g2, b2) = Self::sample_rgb(image, x + 1, y);
                    let d = (r as i16 - r2 as i16).unsigned_abs() as u64
                        + (g as i16 - g2 as i16).unsigned_abs() as u64
                        + (b as i16 - b2 as i16).unsigned_abs() as u64;
                    edge += d;
                }
                n += 1;
                x += 2;
            }
            y += 2;
        }
        if n == 0 {
            return None;
        }
        let mr = sum_r / n;
        let mg = sum_g / n;
        let mb = sum_b / n;
        let mean_edge = edge / n;

        let (class_iri, score) = if mean_edge > 40 {
            (CLASS_HIGH_EDGE, (mean_edge.min(255) as f32) / 255.0)
        } else if mr >= mg && mr >= mb && mr > 40 {
            (CLASS_MOSTLY_RED, (mr as f32) / 255.0)
        } else if mg >= mr && mg >= mb && mg > 40 {
            (CLASS_MOSTLY_GREEN, (mg as f32) / 255.0)
        } else if mb >= mr && mb >= mg && mb > 40 {
            (CLASS_MOSTLY_BLUE, (mb as f32) / 255.0)
        } else {
            (CLASS_LOW_CONTRAST, 0.35)
        };

        if score < self.min_score {
            return None;
        }

        let mut d = Detection::empty();
        d.class_hash = q_hash(class_iri);
        d.instance_hash = self
            .model_hash
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ ((cx as u64) << 32)
            ^ (cy as u64)
            ^ (mr << 16)
            ^ mg
            ^ ((frame_index as u64) << 48);
        d.score_u16 = (score.clamp(0.0, 1.0) * 65535.0) as u16;
        d.x_min_u16 = ((x0 as f32 / w as f32) * 65535.0) as u16;
        d.y_min_u16 = ((y0 as f32 / h as f32) * 65535.0) as u16;
        d.x_max_u16 = ((x1 as f32 / w as f32) * 65535.0) as u16;
        d.y_max_u16 = ((y1 as f32 / h as f32) * 65535.0) as u16;
        d.frame_index = frame_index;
        d.flags = Detection::FLAG_REFERENCE_BACKEND | Detection::FLAG_LOW_ASSURANCE;
        Some(d)
    }

    /// Emit raw cell proposals (no NMS). Writes up to `out.len()` detections.
    pub fn propose_cells(
        &self,
        image: ImageView<'_>,
        frame_index: u32,
        out: &mut [Detection],
    ) -> Result<usize, VisionError> {
        if !image.is_well_formed() {
            return Err(VisionError::MalformedImage);
        }
        if out.is_empty() {
            return Err(VisionError::OutputBufferTooSmall);
        }
        let mut n = 0usize;
        for cy in 0..self.grid_h {
            for cx in 0..self.grid_w {
                if n >= out.len() || n >= MAX_DETECTIONS {
                    return Ok(n);
                }
                if let Some(d) = self.score_cell(image, cx, cy, frame_index) {
                    out[n] = d;
                    n += 1;
                }
            }
        }
        for d in out.iter_mut().skip(n) {
            *d = Detection::empty();
        }
        Ok(n)
    }

    /// Full pipeline: cell proposals → NMS → `out`.
    /// `workspace` must hold ≥ MAX_DETECTIONS bytes (NMS flags) + room for proposal scratch
    /// if using the in-place path; we use stack proposal buffer of MAX_DETECTIONS.
    pub fn detect(
        &self,
        image: ImageView<'_>,
        frame_index: u32,
        out: &mut [Detection],
        workspace: &mut [u8],
    ) -> Result<usize, VisionError> {
        let mut proposals = [Detection::empty(); MAX_DETECTIONS];
        let n_prop = self.propose_cells(image, frame_index, &mut proposals)?;
        if n_prop == 0 {
            for d in out.iter_mut() {
                *d = Detection::empty();
            }
            return Ok(0);
        }
        nms_class_agnostic(&proposals, n_prop, self.nms_iou, out, workspace)
    }
}

impl VisualModel for GridMultiObjectDetector {
    fn capabilities(&self) -> VisualCapabilities {
        let max_cells = (self.grid_w * self.grid_h).min(MAX_DETECTIONS as u32) as u16;
        VisualCapabilities {
            max_detections: max_cells,
            embed_dim: 0,
            supports_boxes: true,
            supports_embedding: false,
            is_reference_backend: true,
        }
    }

    fn infer(
        &mut self,
        image: ImageView<'_>,
        detections_out: &mut [Detection],
        embedding_out: &mut [f32],
        workspace: &mut [u8],
    ) -> Result<VisualOutputCounts, VisionError> {
        let n = self.detect(image, 0, detections_out, workspace)?;
        for e in embedding_out.iter_mut() {
            *e = 0.0;
        }
        Ok(VisualOutputCounts {
            detections: n,
            embedding_written: 0,
        })
    }
}

/// Deterministic frame indices for video sampling (cold path helper).
/// Writes every `stride`-th frame into `out`, starting at 0. Returns count written.
/// Overflow: if more indices than `out.len()`, only first `out.len()` are kept (deterministic).
pub fn sample_frame_indices(total_frames: u32, stride: u32, out: &mut [u32]) -> usize {
    if total_frames == 0 || out.is_empty() {
        return 0;
    }
    let step = stride.max(1);
    let mut w = 0usize;
    let mut i = 0u32;
    while i < total_frames && w < out.len() {
        out[w] = i;
        w += 1;
        i = i.saturating_add(step);
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkerboard_rgb(w: u32, h: u32) -> Vec<u8> {
        // Left half red, right half blue — two objects for multi-det.
        let mut v = vec![0u8; (w * h * 3) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 3) as usize;
                if x < w / 2 {
                    v[i] = 220;
                    v[i + 1] = 10;
                    v[i + 2] = 10;
                } else {
                    v[i] = 10;
                    v[i + 1] = 10;
                    v[i + 2] = 220;
                }
            }
        }
        v
    }

    #[test]
    fn multi_object_two_cells() {
        let px = checkerboard_rgb(16, 8);
        let img = ImageView {
            bytes: &px,
            width: 16,
            height: 8,
            row_stride: 48,
            format: PixelFormat::Rgb8,
        };
        let det = GridMultiObjectDetector::new(2, 1);
        let mut out = [Detection::empty(); 16];
        let mut ws = [0u8; 64];
        let n = det.detect(img, 0, &mut out, &mut ws).unwrap();
        assert!(n >= 2, "expected multi-object, got {n}");
        let classes: Vec<u64> = out[..n].iter().map(|d| d.class_hash).collect();
        assert!(classes.contains(&q_hash(CLASS_MOSTLY_RED)));
        assert!(classes.contains(&q_hash(CLASS_MOSTLY_BLUE)));
    }

    #[test]
    fn frame_sampler_stride() {
        let mut out = [0u32; 8];
        let n = sample_frame_indices(10, 3, &mut out);
        assert_eq!(n, 4);
        assert_eq!(&out[..4], &[0, 3, 6, 9]);
    }

    #[test]
    fn frame_sampler_overflow_deterministic() {
        let mut out = [0u32; 2];
        let n = sample_frame_indices(100, 1, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out[0], 0);
        assert_eq!(out[1], 1);
    }

    #[test]
    fn nms_path_via_visual_model() {
        let px = checkerboard_rgb(8, 8);
        let img = ImageView {
            bytes: &px,
            width: 8,
            height: 8,
            row_stride: 24,
            format: PixelFormat::Rgb8,
        };
        let mut det = GridMultiObjectDetector::new(4, 4);
        let mut dets = [Detection::empty(); MAX_DETECTIONS];
        let mut emb = [0.0f32; 4];
        let mut ws = [0u8; MAX_DETECTIONS];
        let c = det.infer(img, &mut dets, &mut emb, &mut ws).unwrap();
        assert!(c.detections >= 1);
        assert!(c.detections <= MAX_DETECTIONS);
    }
}
