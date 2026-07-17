//! Stable visual ABI (fixed-layout, caller-buffered).

/// Maximum detections written per `infer` call (caller must supply at least this).
pub const MAX_DETECTIONS: usize = 64;
/// Maximum embedding dim written (caller buffer may be larger; we fill `min`).
pub const MAX_EMBED_DIM: usize = 128;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Gray8 = 0,
    Rgb8 = 1,
    Rgba8 = 2,
    Bgr8 = 3,
    RgbF32 = 4,
}

/// Borrowed image view — no ownership of pixels.
#[derive(Debug, Clone, Copy)]
pub struct ImageView<'a> {
    pub bytes: &'a [u8],
    pub width: u32,
    pub height: u32,
    /// Bytes between row starts (≥ width × bpp).
    pub row_stride: u32,
    pub format: PixelFormat,
}

impl<'a> ImageView<'a> {
    #[inline]
    pub fn bytes_per_pixel(self) -> u32 {
        match self.format {
            PixelFormat::Gray8 => 1,
            PixelFormat::Rgb8 | PixelFormat::Bgr8 => 3,
            PixelFormat::Rgba8 => 4,
            PixelFormat::RgbF32 => 12,
        }
    }

    /// True if the view has enough bytes for the declared geometry.
    pub fn is_well_formed(self) -> bool {
        if self.width == 0 || self.height == 0 || self.row_stride == 0 {
            return false;
        }
        let need = self
            .row_stride
            .saturating_mul(self.height.saturating_sub(1))
            .saturating_add(self.width.saturating_mul(self.bytes_per_pixel()));
        self.bytes.len() as u64 >= need as u64
    }
}

/// One detection: fixed-width so native/WASM/serde stay stable.
/// Coordinates are normalized × 65535 (0…1 mapped into u16).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detection {
    pub class_hash: u64,
    pub instance_hash: u64,
    pub score_u16: u16,
    pub x_min_u16: u16,
    pub y_min_u16: u16,
    pub x_max_u16: u16,
    pub y_max_u16: u16,
    pub frame_index: u32,
    pub track_id: u32,
    pub flags: u32,
}

impl Detection {
    pub const FLAG_REFERENCE_BACKEND: u32 = 1;
    pub const FLAG_LOW_ASSURANCE: u32 = 2;

    #[inline]
    pub fn empty() -> Self {
        Self {
            class_hash: 0,
            instance_hash: 0,
            score_u16: 0,
            x_min_u16: 0,
            y_min_u16: 0,
            x_max_u16: 0,
            y_max_u16: 0,
            frame_index: 0,
            track_id: 0,
            flags: 0,
        }
    }

    #[inline]
    pub fn score_f32(self) -> f32 {
        self.score_u16 as f32 / 65535.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualCapabilities {
    pub max_detections: u16,
    pub embed_dim: u16,
    pub supports_boxes: bool,
    pub supports_embedding: bool,
    /// Backend is a development/reference path — never treat as certified detector.
    pub is_reference_backend: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualOutputCounts {
    pub detections: usize,
    pub embedding_written: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionError {
    MalformedImage,
    OutputBufferTooSmall,
    WorkspaceTooSmall,
    BackendUnavailable,
}

/// Backend-agnostic visual model.
pub trait VisualModel {
    fn capabilities(&self) -> VisualCapabilities;

    fn infer(
        &mut self,
        image: ImageView<'_>,
        detections_out: &mut [Detection],
        embedding_out: &mut [f32],
        workspace: &mut [u8],
    ) -> Result<VisualOutputCounts, VisionError>;
}
