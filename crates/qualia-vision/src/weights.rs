//! Swarm W — production weight bundle + encoder/detector backend.
//!
//! Native **QVWT** format (Qualia Vision Weights): mmap-friendly flat f32 tables.
//! Offline conversion from GGUF/P64 may write this format; no Python at runtime.
//!
//! Honest labels: `VisionBackendKind::ProductionWeights` only when a bundle is loaded.
//! Seed-built fixtures are regenerable test weights, not a third-party foundation model.

use crate::classifier::LinearHead;
use crate::semantic::q_hash;
use crate::types::{
    Detection, ImageView, PixelFormat, VisionError, VisualCapabilities, VisualModel,
    VisualOutputCounts, MAX_DETECTIONS, MAX_EMBED_DIM,
};

/// File magic: `QVWT` little-endian.
pub const QVWT_MAGIC: u32 = 0x5457_5651; // 'QVWT' as LE bytes Q V W T
pub const QVWT_VERSION: u32 = 1;

/// Which detector path is active (product must label this).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionBackendKind {
    /// Heuristic grid / CPU reference — not production eval.
    Reference = 0,
    /// Loaded QVWT (or equivalent) weight tables.
    ProductionWeights = 1,
}

/// On-disk / in-memory vision weight bundle.
#[derive(Debug, Clone)]
pub struct VisionWeightBundle {
    pub embed_dim: usize,
    pub n_classes: usize,
    /// Projects 16-d image features → embed_dim. Row-major `[embed_dim * 16]`.
    pub proj: Vec<f32>,
    pub head: LinearHead,
    pub model_id: String,
    pub content_hash: u64,
}

impl VisionWeightBundle {
    pub fn model_hash(&self) -> u64 {
        self.content_hash
    }

    /// Regenerable fixture weights from seed (tests + offline demo without external files).
    pub fn from_seed(seed: u64, embed_dim: usize, class_iris: &[&str]) -> Self {
        let dim = embed_dim.min(MAX_EMBED_DIM).max(8);
        let n = class_iris.len().max(1);
        let mut proj = vec![0.0f32; dim * 16];
        let mut h = seed ^ 0xA5A5_5A5A_C3C3_3C3C;
        for v in proj.iter_mut() {
            h = h
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                .rotate_left(13)
                .wrapping_add(1);
            *v = ((h as i64 as f32) / (u64::MAX as f32)) * 0.5;
        }
        let mut head = LinearHead::zeros(dim, class_iris);
        for c in 0..n {
            for i in 0..dim {
                h = h.wrapping_mul(0x85eb_ca6b).wrapping_add(c as u64 + 1);
                head.weight[c * dim + i] = ((h % 1000) as f32 / 1000.0) - 0.5;
            }
            head.bias[c] = 0.01 * (c as f32 + 1.0);
        }
        let content_hash = q_hash(&format!("qvwt-seed:{seed}:{dim}:{n}"));
        Self {
            embed_dim: dim,
            n_classes: n,
            proj,
            head,
            model_id: format!("qvwt-seed-{seed:016x}"),
            content_hash,
        }
    }

    /// Serialize to QVWT bytes (cold path).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + (self.proj.len() + self.head.weight.len() + self.head.bias.len()) * 4);
        out.extend_from_slice(&QVWT_MAGIC.to_le_bytes());
        out.extend_from_slice(&QVWT_VERSION.to_le_bytes());
        out.extend_from_slice(&(self.embed_dim as u32).to_le_bytes());
        out.extend_from_slice(&(self.n_classes as u32).to_le_bytes());
        out.extend_from_slice(&self.content_hash.to_le_bytes());
        // pad to 32
        while out.len() < 32 {
            out.push(0);
        }
        for f in self.proj.iter().chain(self.head.weight.iter()).chain(self.head.bias.iter()) {
            out.extend_from_slice(&f.to_le_bytes());
        }
        // class hashes
        for &ch in &self.head.class_hashes {
            out.extend_from_slice(&ch.to_le_bytes());
        }
        out
    }

    pub fn save_path(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, self.to_bytes()).map_err(|e| e.to_string())
    }

    pub fn load_path(path: &std::path::Path, class_iris: &[&str]) -> Result<Self, String> {
        let b = std::fs::read(path).map_err(|e| e.to_string())?;
        Self::from_bytes(&b, class_iris).map_err(|e| format!("{e:?}"))
    }

    /// Load QVWT from bytes. Class IRIs optional: if fewer names than n_classes, hashes from file used.
    pub fn from_bytes(bytes: &[u8], class_iris: &[&str]) -> Result<Self, VisionError> {
        if bytes.len() < 32 {
            return Err(VisionError::MalformedImage);
        }
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != QVWT_MAGIC {
            return Err(VisionError::BackendUnavailable);
        }
        let ver = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if ver != QVWT_VERSION {
            return Err(VisionError::BackendUnavailable);
        }
        let embed_dim = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let n_classes = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let content_hash = u64::from_le_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
        ]);
        if embed_dim == 0 || embed_dim > MAX_EMBED_DIM || n_classes == 0 || n_classes > 64 {
            return Err(VisionError::MalformedImage);
        }
        let mut off = 32usize;
        let proj_n = embed_dim * 16;
        let w_n = n_classes * embed_dim;
        let need = (proj_n + w_n + n_classes) * 4 + n_classes * 8;
        if bytes.len() < off + need {
            return Err(VisionError::OutputBufferTooSmall);
        }
        let mut proj = vec![0.0f32; proj_n];
        for v in &mut proj {
            *v = f32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
            off += 4;
        }
        let names: Vec<&str> = if class_iris.len() >= n_classes {
            class_iris[..n_classes].to_vec()
        } else {
            // placeholder iris; hashes overwritten from file
            (0..n_classes).map(|_| "https://ns.webizen.org/q42/vision/class/loaded").collect()
        };
        // Build head then fill
        let mut head = LinearHead::zeros(embed_dim, &names);
        for v in &mut head.weight {
            *v = f32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
            off += 4;
        }
        for v in &mut head.bias {
            *v = f32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
            off += 4;
        }
        for c in 0..n_classes {
            let ch = u64::from_le_bytes([
                bytes[off],
                bytes[off + 1],
                bytes[off + 2],
                bytes[off + 3],
                bytes[off + 4],
                bytes[off + 5],
                bytes[off + 6],
                bytes[off + 7],
            ]);
            off += 8;
            if c < head.class_hashes.len() {
                head.class_hashes[c] = ch;
            }
        }
        Ok(Self {
            embed_dim,
            n_classes,
            proj,
            head,
            model_id: format!("qvwt-{content_hash:016x}"),
            content_hash,
        })
    }
}

/// Production path: image features → projection → linear head → whole-image or grid cells.
pub struct ProductionVision {
    pub bundle: VisionWeightBundle,
    pub min_score: f32,
    pub grid: u32,
}

impl ProductionVision {
    pub fn new(bundle: VisionWeightBundle) -> Self {
        Self {
            bundle,
            min_score: 0.2,
            grid: 2,
        }
    }

    pub fn backend_kind(&self) -> VisionBackendKind {
        VisionBackendKind::ProductionWeights
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

    fn cell_features(img: ImageView<'_>, x0: u32, y0: u32, x1: u32, y1: u32) -> [f32; 16] {
        let mut sum = [0.0f32; 3];
        let mut n = 0u32;
        let mut edge = 0.0f32;
        let mut y = y0;
        while y < y1 {
            let mut x = x0;
            while x < x1 {
                let (r, g, b) = Self::sample_rgb(img, x, y);
                sum[0] += r as f32 / 255.0;
                sum[1] += g as f32 / 255.0;
                sum[2] += b as f32 / 255.0;
                if x + 1 < x1 {
                    let (r2, g2, b2) = Self::sample_rgb(img, x + 1, y);
                    edge += ((r as i16 - r2 as i16).unsigned_abs() as f32
                        + (g as i16 - g2 as i16).unsigned_abs() as f32
                        + (b as i16 - b2 as i16).unsigned_abs() as f32)
                        / (3.0 * 255.0);
                }
                n += 1;
                x += 2;
            }
            y += 2;
        }
        let inv = if n == 0 { 0.0 } else { 1.0 / n as f32 };
        let mut f = [0.0f32; 16];
        f[0] = sum[0] * inv;
        f[1] = sum[1] * inv;
        f[2] = sum[2] * inv;
        f[3] = edge * inv;
        f[4] = (x1 - x0) as f32 / img.width.max(1) as f32;
        f[5] = (y1 - y0) as f32 / img.height.max(1) as f32;
        for i in 6..16 {
            f[i] = f[i % 3] * f[3];
        }
        f
    }

    fn project(&self, feat: &[f32; 16], out: &mut [f32]) {
        let dim = self.bundle.embed_dim.min(out.len());
        for e in 0..dim {
            let mut s = 0.0f32;
            for i in 0..16 {
                s += feat[i] * self.bundle.proj[e * 16 + i];
            }
            out[e] = s.tanh();
        }
    }
}

impl VisualModel for ProductionVision {
    fn capabilities(&self) -> VisualCapabilities {
        VisualCapabilities {
            max_detections: (self.grid * self.grid).min(MAX_DETECTIONS as u32) as u16,
            embed_dim: self.bundle.embed_dim as u16,
            supports_boxes: true,
            supports_embedding: true,
            is_reference_backend: false,
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
        if detections_out.is_empty() {
            return Err(VisionError::OutputBufferTooSmall);
        }
        let g = self.grid.max(1);
        let w = image.width;
        let h = image.height;
        let mut det_n = 0usize;
        let mut global_emb = [0.0f32; MAX_EMBED_DIM];
        let gf = Self::cell_features(image, 0, 0, w, h);
        self.project(&gf, &mut global_emb);
        let emb_n = embedding_out.len().min(self.bundle.embed_dim);
        embedding_out[..emb_n].copy_from_slice(&global_emb[..emb_n]);

        for cy in 0..g {
            for cx in 0..g {
                if det_n >= detections_out.len() || det_n >= MAX_DETECTIONS {
                    break;
                }
                let x0 = cx * w / g;
                let y0 = cy * h / g;
                let x1 = if cx + 1 >= g { w } else { (cx + 1) * w / g };
                let y1 = if cy + 1 >= g { h } else { (cy + 1) * h / g };
                let feat = Self::cell_features(image, x0, y0, x1, y1);
                let mut emb = [0.0f32; MAX_EMBED_DIM];
                self.project(&feat, &mut emb);
                if let Some(mut d) = self
                    .bundle
                    .head
                    .classify_embedding(&emb[..self.bundle.embed_dim], self.min_score)
                {
                    d.x_min_u16 = ((x0 as f32 / w as f32) * 65535.0) as u16;
                    d.y_min_u16 = ((y0 as f32 / h as f32) * 65535.0) as u16;
                    d.x_max_u16 = ((x1 as f32 / w as f32) * 65535.0) as u16;
                    d.y_max_u16 = ((y1 as f32 / h as f32) * 65535.0) as u16;
                    d.instance_hash ^= self.bundle.content_hash ^ ((cx as u64) << 8) ^ cy as u64;
                    d.flags = 0; // production path — not FLAG_REFERENCE_BACKEND
                    detections_out[det_n] = d;
                    det_n += 1;
                }
            }
        }
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
    use crate::detector::{CLASS_MOSTLY_BLUE, CLASS_MOSTLY_RED};

    #[test]
    fn qvwt_roundtrip() {
        let b = VisionWeightBundle::from_seed(42, 16, &[CLASS_MOSTLY_RED, CLASS_MOSTLY_BLUE]);
        let bytes = b.to_bytes();
        let b2 = VisionWeightBundle::from_bytes(&bytes, &[CLASS_MOSTLY_RED, CLASS_MOSTLY_BLUE]).unwrap();
        assert_eq!(b.embed_dim, b2.embed_dim);
        assert_eq!(b.content_hash, b2.content_hash);
        assert_eq!(b.proj, b2.proj);
    }

    #[test]
    fn production_not_labelled_reference() {
        let b = VisionWeightBundle::from_seed(1, 16, &[CLASS_MOSTLY_RED, CLASS_MOSTLY_BLUE]);
        let mut m = ProductionVision::new(b);
        assert_eq!(m.backend_kind(), VisionBackendKind::ProductionWeights);
        assert!(!m.capabilities().is_reference_backend);
        let mut rgb = vec![200u8; 8 * 8 * 3];
        for p in rgb.chunks_mut(3) {
            p[0] = 220;
            p[1] = 20;
            p[2] = 20;
        }
        let img = ImageView {
            bytes: &rgb,
            width: 8,
            height: 8,
            row_stride: 24,
            format: PixelFormat::Rgb8,
        };
        let mut dets = [Detection::empty(); 8];
        let mut emb = [0.0f32; 16];
        let mut ws = [0u8; 8];
        let c = m.infer(img, &mut dets, &mut emb, &mut ws).unwrap();
        assert!(c.embedding_written > 0);
    }
}
