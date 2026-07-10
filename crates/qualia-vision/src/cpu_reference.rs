//! Deterministic CPU reference “vision” backend.
//!
//! **Honest scope:** not a neural detector. Partitions the image into a 2×2 grid,
//! scores each cell by channel mean and simple horizontal edge energy, and emits
//! up to four class proposals. Used to:
//! - prove the ABI end-to-end on consumer machines without GPU weights;
//! - feed epistemic observation quins for rights/audit demos;
//! - regression-test semantic packing.
//!
//! Replace with a P64-backed encoder when available — keep `VisualModel`.

use crate::semantic::q_hash;
use crate::types::{
    Detection, ImageView, PixelFormat, VisionError, VisualCapabilities, VisualModel,
    VisualOutputCounts, MAX_DETECTIONS, MAX_EMBED_DIM,
};

/// Stable class IRIs for the reference taxonomy.
pub const CLASS_MOSTLY_RED: &str = "https://ns.webizen.org/q42/vision/class/mostly-red";
pub const CLASS_MOSTLY_GREEN: &str = "https://ns.webizen.org/q42/vision/class/mostly-green";
pub const CLASS_MOSTLY_BLUE: &str = "https://ns.webizen.org/q42/vision/class/mostly-blue";
pub const CLASS_HIGH_EDGE: &str = "https://ns.webizen.org/q42/vision/class/high-edge-energy";
pub const CLASS_LOW_CONTRAST: &str = "https://ns.webizen.org/q42/vision/class/low-contrast";

const MODEL_ID: &str = "qualia-vision-cpu-reference-v1";

pub struct CpuReferenceVision {
    model_hash: u64,
}

impl Default for CpuReferenceVision {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuReferenceVision {
    pub fn new() -> Self {
        Self {
            model_hash: q_hash(MODEL_ID),
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
            PixelFormat::RgbF32 => {
                // cold path: clamp first channel bytes if misaligned — treat as zero
                (0, 0, 0)
            }
        }
    }
}

impl VisualModel for CpuReferenceVision {
    fn capabilities(&self) -> VisualCapabilities {
        VisualCapabilities {
            max_detections: 4,
            embed_dim: 16,
            supports_boxes: true,
            supports_embedding: true,
            is_reference_backend: true,
        }
    }

    fn infer(
        &mut self,
        image: ImageView<'_>,
        detections_out: &mut [Detection],
        embedding_out: &mut [f32],
        _workspace: &mut [u8],
    ) -> Result<VisualOutputCounts, VisionError> {
        if !image.is_well_formed() {
            return Err(VisionError::MalformedImage);
        }
        if detections_out.len() < 4 {
            return Err(VisionError::OutputBufferTooSmall);
        }

        let w = image.width;
        let h = image.height;
        let mut det_n = 0usize;

        // 2×2 cells
        for cy in 0..2u32 {
            for cx in 0..2u32 {
                let x0 = cx * w / 2;
                let y0 = cy * h / 2;
                let x1 = if cx == 1 { w } else { (cx + 1) * w / 2 };
                let y1 = if cy == 1 { h } else { (cy + 1) * h / 2 };
                if x1 <= x0 || y1 <= y0 {
                    continue;
                }

                let mut sum_r = 0u64;
                let mut sum_g = 0u64;
                let mut sum_b = 0u64;
                let mut edge = 0u64;
                let mut n = 0u64;
                // Subsample for speed (every 2nd pixel).
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
                    continue;
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

                let mut d = Detection::empty();
                d.class_hash = q_hash(class_iri);
                d.instance_hash = self
                    .model_hash
                    .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                    ^ ((cx as u64) << 32)
                    ^ (cy as u64)
                    ^ (mr << 16)
                    ^ mg;
                d.score_u16 = (score.clamp(0.0, 1.0) * 65535.0) as u16;
                // Normalized box in u16.
                d.x_min_u16 = ((x0 as f32 / w as f32) * 65535.0) as u16;
                d.y_min_u16 = ((y0 as f32 / h as f32) * 65535.0) as u16;
                d.x_max_u16 = ((x1 as f32 / w as f32) * 65535.0) as u16;
                d.y_max_u16 = ((y1 as f32 / h as f32) * 65535.0) as u16;
                d.flags = Detection::FLAG_REFERENCE_BACKEND | Detection::FLAG_LOW_ASSURANCE;
                detections_out[det_n] = d;
                det_n += 1;
                if det_n >= MAX_DETECTIONS.min(4) {
                    break;
                }
            }
            if det_n >= 4 {
                break;
            }
        }

        // Tiny embedding: global mean RGB + edge + aspect (16 slots).
        let emb_n = embedding_out.len().min(MAX_EMBED_DIM).min(16);
        if emb_n > 0 {
            let (r, g, b) = Self::sample_rgb(image, w / 2, h / 2);
            embedding_out[..emb_n].fill(0.0);
            if emb_n > 0 {
                embedding_out[0] = r as f32 / 255.0;
            }
            if emb_n > 1 {
                embedding_out[1] = g as f32 / 255.0;
            }
            if emb_n > 2 {
                embedding_out[2] = b as f32 / 255.0;
            }
            if emb_n > 3 {
                embedding_out[3] = w as f32 / (h.max(1) as f32);
            }
            for i in 0..det_n.min(emb_n.saturating_sub(4)) {
                embedding_out[4 + i] = detections_out[i].score_f32();
            }
        }

        // Clear unused detection slots.
        for d in detections_out.iter_mut().skip(det_n) {
            *d = Detection::empty();
        }

        Ok(VisualOutputCounts {
            detections: det_n,
            embedding_written: emb_n,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{compile_observation_quins, media_digest};

    #[test]
    fn red_cell_emits_detection() {
        // 4×4 solid red RGB
        let mut px = vec![0u8; 4 * 4 * 3];
        for p in px.chunks_mut(3) {
            p[0] = 220;
            p[1] = 10;
            p[2] = 10;
        }
        let img = ImageView {
            bytes: &px,
            width: 4,
            height: 4,
            row_stride: 12,
            format: PixelFormat::Rgb8,
        };
        let mut model = CpuReferenceVision::new();
        let mut dets = [Detection::empty(); 8];
        let mut emb = [0.0f32; 16];
        let mut ws = [0u8; 64];
        let counts = model
            .infer(img, &mut dets, &mut emb, &mut ws)
            .expect("infer");
        assert!(counts.detections >= 1);
        assert!(dets[0].score_u16 > 0);
        assert!(model.capabilities().is_reference_backend);

        let digest = media_digest(&px);
        let mut quins = [crate::semantic::VisionQuin::with_parity(0, 0, 0, 0, 0); 16];
        let n = compile_observation_quins(digest, &dets[..counts.detections], model.model_hash(), &mut quins);
        assert!(n >= 2);
    }
}
