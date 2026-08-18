//! Zero-copy WASM buffer views and std140-aligned uniform layouts.
//!
//! Provides:
//! 1. **std140-aligned uniform structs** — `#[repr(C, align(16))]` structs with
//!    compile-time size assertions, matching the GLSL ES 300 / WebGPU uniform
//!    buffer layout rules. These are the host-side packing types for uniform
//!    buffer uploads.
//! 2. **Std140Layout calculator** — a standalone calculator that computes byte
//!    offsets for struct fields following std140 rules (scalars 4B/align4,
//!    vec3 12B/align16, mat4 64B/align16, arrays 16B stride, structs 16B align).
//! 3. **WASM zero-copy views** — `Float32Array::view` / `Uint32Array::view`
//!    helpers (cfg'd to `wasm32`) that create JS typed array views over Rust
//!    memory without copying. This is the bridge between Rust-side buffer
//!    packing and WebGL2 buffer upload on the browser path.
//!
//! ## std140 rules summary
//!
//! | Type | Size | Alignment |
//! |------|------|-----------|
//! | scalar (f32, u32, i32) | 4 | 4 |
//! | vec2<f32> | 8 | 8 |
//! | vec3<f32> | 12 | 16 |
//! | vec4<f32> | 16 | 16 |
//! | mat4x4<f32> | 64 | 16 |
//! | mat3x3<f32> | 48 | 16 |
//! | array<T> | 16 × N | 16 |
//! | struct | sum (rounded to 16) | 16 |

use bytemuck::{Pod, Zeroable};

// ── std140-aligned uniform structs ─────────────────────────────────────────

/// Camera uniform block: view + projection matrices (std140 layout).
///
/// `mat4x4<f32>` is 64 bytes each, aligned to 16 bytes. Total: 128 bytes.
/// Matches the WGSL struct:
/// ```wgsl
/// struct Camera {
///     view: mat4x4<f32>,
///     proj: mat4x4<f32>,
/// };
/// ```
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
}

const _: [(); 128] = [(); std::mem::size_of::<CameraUniform>()];

impl CameraUniform {
    pub fn new(view: [[f32; 4]; 4], proj: [[f32; 4]; 4]) -> Self {
        Self { view, proj }
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// Model uniform block: model matrix + time scalar (std140 layout).
///
/// `mat4x4<f32>` (64 bytes) + `f32` time (4 bytes) + 12 bytes padding to reach
/// 80 bytes (next multiple of 16). Total: 80 bytes.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ModelUniform {
    pub model: [[f32; 4]; 4],
    pub time: f32,
    pub _pad: [u8; 12],
}

const _: [(); 80] = [(); std::mem::size_of::<ModelUniform>()];

impl ModelUniform {
    pub fn new(model: [[f32; 4]; 4], time: f32) -> Self {
        Self {
            model,
            time,
            _pad: [0; 12],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// Time uniform block: sim time + resolution (std140 layout).
///
/// 4 × f32 = 16 bytes, aligned to 16. Total: 16 bytes.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TimeUniform {
    pub time: f32,
    pub width: f32,
    pub height: f32,
    pub _pad: f32,
}

const _: [(); 16] = [(); std::mem::size_of::<TimeUniform>()];

impl TimeUniform {
    pub fn new(time: f32, width: f32, height: f32) -> Self {
        Self {
            time,
            width,
            height,
            _pad: 0.0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

// ── std140 layout calculator ───────────────────────────────────────────────

/// A field in a std140 layout calculation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Std140Field {
    /// Field name (for diagnostic purposes).
    pub name: String,
    /// Byte offset within the uniform buffer.
    pub offset: u32,
    /// Byte size of the field.
    pub size: u32,
    /// Byte alignment of the field.
    pub align: u32,
}

/// std140 field type descriptor (used by the layout calculator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Std140Type {
    Scalar,
    Vec2,
    Vec3,
    Vec4,
    Mat4,
    Mat3,
    Array { elem: Std140TypeSimple, count: u32 },
    Struct { size: u32, align: u32 },
}

/// Simplified type for array elements (arrays can't contain arrays in this
/// simplified calculator, but can contain scalars/vectors/matrices).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Std140TypeSimple {
    Scalar,
    Vec2,
    Vec3,
    Vec4,
    Mat4,
    Mat3,
}

impl Std140Type {
    /// Compute (size, alignment) for this type under std140 rules.
    pub fn size_align(&self) -> (u32, u32) {
        match self {
            Std140Type::Scalar => (4, 4),
            Std140Type::Vec2 => (8, 8),
            Std140Type::Vec3 => (12, 16),
            Std140Type::Vec4 => (16, 16),
            Std140Type::Mat4 => (64, 16),
            Std140Type::Mat3 => (48, 16),
            Std140Type::Array { elem, count } => {
                let (elem_size, _) = elem.size_align();
                // std140: array stride is rounded up to vec4 (16 bytes)
                let stride = ((elem_size + 15) / 16) * 16;
                (stride * count, stride)
            }
            Std140Type::Struct { size, align } => (*size, *align),
        }
    }
}

impl Std140TypeSimple {
    fn size_align(&self) -> (u32, u32) {
        match self {
            Std140TypeSimple::Scalar => (4, 4),
            Std140TypeSimple::Vec2 => (8, 8),
            Std140TypeSimple::Vec3 => (12, 16),
            Std140TypeSimple::Vec4 => (16, 16),
            Std140TypeSimple::Mat4 => (64, 16),
            Std140TypeSimple::Mat3 => (48, 16),
        }
    }
}

/// Calculate the std140 layout for a list of named fields.
///
/// Returns the field offsets and the total buffer size (rounded up to the
/// struct alignment, which is 16 bytes under std140).
pub fn std140_layout(fields: &[(&str, Std140Type)]) -> (Vec<Std140Field>, u32) {
    let mut layout = Vec::with_capacity(fields.len());
    let mut offset = 0u32;

    for (name, ty) in fields {
        let (size, align) = ty.size_align();
        // Align offset up to the field's alignment.
        offset = ((offset + align - 1) / align) * align;
        layout.push(Std140Field {
            name: name.to_string(),
            offset,
            size,
            align,
        });
        offset += size;
    }

    // Round up struct size to 16 (std140 struct alignment).
    let total = ((offset + 15) / 16) * 16;
    (layout, total)
}

// ── WASM zero-copy buffer views ─────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm_views {
    use js_sys::{Float32Array, Uint32Array, Uint8Array};

    /// Create a zero-copy `Float32Array` view over a Rust `&[f32]` slice.
    ///
    /// The returned `Float32Array` shares memory with the Rust slice — no
    /// allocation, no copy. The Rust slice must outlive the JS view. This is
    /// the bridge for uploading uniform/buffer data to WebGL2 without
    /// allocating a new JS typed array each frame.
    ///
    /// # Safety
    ///
    /// The caller must ensure the Rust source slice remains valid (not moved,
    /// not dropped) for the lifetime of the returned `Float32Array`. In
    /// practice this means the slice should be a reference to a stack-local
    /// or static buffer that is consumed by a WebGL2 `bufferData` call
    /// before the function returns.
    pub fn view_f32(data: &[f32]) -> Float32Array {
        Float32Array::view(data)
    }

    /// Create a zero-copy `Uint32Array` view over a Rust `&[u32]` slice.
    ///
    /// See [`view_f32`] for safety notes.
    pub fn view_u32(data: &[u32]) -> Uint32Array {
        Uint32Array::view(data)
    }

    /// Create a zero-copy `Uint8Array` view over a Rust `&[u8]` slice.
    ///
    /// See [`view_f32`] for safety notes.
    pub fn view_u8(data: &[u8]) -> Uint8Array {
        Uint8Array::view(data)
    }

    /// Create a zero-copy `Float32Array` view over the bytes of any
    /// `#[repr(C, align(16))]` Pod struct (e.g. [`super::CameraUniform`]).
    ///
    /// This is the primary uniform-upload path: pack a std140 struct on the
    /// Rust stack, then view its bytes as a `Float32Array` for
    /// `gl.bufferData()` without an intermediate copy.
    pub fn view_uniform_bytes(bytes: &[u8]) -> Float32Array {
        // Reinterpret bytes as f32 slice — safe because std140 structs are
        // align(16) and Pod, so the byte slice is 4-byte aligned and its
        // length is a multiple of 4.
        let f32_len = bytes.len() / 4;
        let ptr = bytes.as_ptr() as *const f32;
        let slice = unsafe { std::slice::from_raw_parts(ptr, f32_len) };
        Float32Array::view(slice)
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_views::{view_f32, view_u32, view_u8, view_uniform_bytes};

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_uniform_is_128_bytes() {
        assert_eq!(std::mem::size_of::<CameraUniform>(), 128);
    }

    #[test]
    fn model_uniform_is_80_bytes() {
        assert_eq!(std::mem::size_of::<ModelUniform>(), 80);
    }

    #[test]
    fn time_uniform_is_16_bytes() {
        assert_eq!(std::mem::size_of::<TimeUniform>(), 16);
    }

    #[test]
    fn camera_uniform_alignment_is_16() {
        assert_eq!(std::mem::align_of::<CameraUniform>(), 16);
    }

    #[test]
    fn std140_layout_camera_struct() {
        let (layout, total) = std140_layout(&[
            ("view", Std140Type::Mat4),
            ("proj", Std140Type::Mat4),
        ]);
        assert_eq!(layout.len(), 2);
        assert_eq!(layout[0].name, "view");
        assert_eq!(layout[0].offset, 0);
        assert_eq!(layout[0].size, 64);
        assert_eq!(layout[1].name, "proj");
        assert_eq!(layout[1].offset, 64);
        assert_eq!(layout[1].size, 64);
        assert_eq!(total, 128);
    }

    #[test]
    fn std140_layout_vec3_then_scalar() {
        // vec3 at offset 0 (size 12, align 16), then f32 at offset 12
        // (fits in the vec4 padding, no extra alignment needed since 12 is
        // 4-byte aligned for a scalar).
        let (layout, total) = std140_layout(&[
            ("a", Std140Type::Vec3),
            ("b", Std140Type::Scalar),
        ]);
        assert_eq!(layout[0].offset, 0);
        assert_eq!(layout[0].size, 12);
        assert_eq!(layout[0].align, 16);
        assert_eq!(layout[1].offset, 12);
        assert_eq!(layout[1].size, 4);
        // Total: 16 (rounded up from 16 to 16)
        assert_eq!(total, 16);
    }

    #[test]
    fn std140_layout_array_of_floats() {
        // array<f32, 4>: stride 16 (std140 rounds up to vec4), total 64
        let (layout, total) = std140_layout(&[(
            "data",
            Std140Type::Array {
                elem: Std140TypeSimple::Scalar,
                count: 4,
            },
        )]);
        assert_eq!(layout[0].offset, 0);
        assert_eq!(layout[0].size, 64); // 4 × 16
        assert_eq!(layout[0].align, 16); // stride
        assert_eq!(total, 64);
    }

    #[test]
    fn std140_layout_mixed_struct() {
        // mat4 (64B) + vec3 (12B, aligned 16) + f32 (4B)
        let (layout, total) = std140_layout(&[
            ("model", Std140Type::Mat4),
            ("color", Std140Type::Vec3),
            ("intensity", Std140Type::Scalar),
        ]);
        assert_eq!(layout[0].offset, 0);
        assert_eq!(layout[0].size, 64);
        assert_eq!(layout[1].offset, 64); // 64 is 16-aligned
        assert_eq!(layout[1].size, 12);
        assert_eq!(layout[2].offset, 76); // 64+12=76, 76 is 4-aligned
        assert_eq!(layout[2].size, 4);
        // Total: 80 (76+4=80, already 16-aligned)
        assert_eq!(total, 80);
    }

    #[test]
    fn camera_uniform_bytes_roundtrip() {
        let cam = CameraUniform::new(
            [[1.0; 4]; 4],
            [[2.0; 4]; 4],
        );
        let bytes = cam.as_bytes();
        assert_eq!(bytes.len(), 128);
        // First 4 bytes should be 1.0f32 (view matrix [0][0])
        let first_f32 = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(first_f32, 1.0);
        // Byte 64 should be 2.0f32 (proj matrix [0][0])
        let proj_f32 = f32::from_le_bytes(bytes[64..68].try_into().unwrap());
        assert_eq!(proj_f32, 2.0);
    }

    #[test]
    fn time_uniform_bytes_roundtrip() {
        let t = TimeUniform::new(1.5, 800.0, 600.0);
        let bytes = t.as_bytes();
        assert_eq!(bytes.len(), 16);
        let time = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(time, 1.5);
        let width = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(width, 800.0);
    }
}
