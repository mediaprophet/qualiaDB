//! Section 19 (`IMPO`): Directional Octahedral Impostor Atlas for `.10d` Living Containers.
//!
//! Provides the runtime data container and projection mathematics for Tier 3 rendering
//! (Screen-Space Radius: 8–64 px). At this distance, drawing full geometry or even dense meshlets
//! triggers the GPU 2x2 quad-overdraw penalty. An octahedral impostor replaces the entire geometry
//! with a single camera-facing quad (2 triangles) that samples an offline-baked atlas of directional
//! views.
//!
//! # Mathematical Foundations
//!
//! 1. **Hemi-Octahedral Projection**: Maps upper-hemisphere directional rays $V \in S^2$ ($z \ge 0$)
//!    onto a unit square UV coordinate $[0, 1]^2$:
//!    $$u = \frac{x}{2(|x| + |y| + z)} + 0.5, \quad v = \frac{y}{2(|x| + |y| + z)} + 0.5$$
//!    This provides uniform solid-angle texel distribution with minimal distortion compared to
//!    equirectangular maps.
//!
//! 2. **Barycentric View Interpolation**: At runtime, a camera view vector falls between grid cells.
//!    The runtime determines the 3 closest discrete camera angle viewpoints on the octahedral grid
//!    and blends them with barycentric weights, providing parallax-free rotation without popping.
//!
//! 3. **Packed Channels**:
//!    - Slice 0: Albedo RGB + Radial Depth A (for depth-buffer reconstruction and soft intersections).
//!    - Slice 1: Octahedral normal map + Metallic/Roughness.

use bytemuck::{Pod, Zeroable};

pub const IMPOSTOR_SECTION_VERSION: u16 = 1;
pub const IMPOSTOR_MINI_HEADER_SIZE: usize = 64;
pub const MAGIC_IMPO: [u8; 4] = *b"IMPO";

pub const FLAG_HEMI_OCTAHEDRAL: u16 = 1 << 0;
pub const FLAG_DEPTH_IN_ALPHA: u16 = 1 << 1;
pub const FLAG_OCT32_NORMALS: u16 = 1 << 2;
pub const FLAG_PBR_METALLIC_ROUGHNESS: u16 = 1 << 3;

/// 64-byte POD header encoding octahedral impostor atlas in SectionType::ImpostorAtlas (19).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct ImpostorAtlasMiniHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    /// Grid dimension along one axis (e.g., 8 for an 8x8 grid = 64 views, 16 for 16x16 = 256 views).
    pub grid_dimension: u16,
    /// Resolution in texels per view (e.g., 64 for 64x64 or 128 for 128x128).
    pub view_resolution: u16,
    /// Local-space bounding sphere center.
    pub sphere_center: [f32; 3],
    /// Local-space bounding sphere radius.
    pub sphere_radius: f32,
    /// Total byte length of the texture payload following this header.
    pub atlas_byte_size: u32,
    /// 0 = RGBA8_UNORM, 1 = BC7, 2 = ASTC_4x4.
    pub channel_format: u8,
    pub reserved: [u8; 11],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImpostorSectionError {
    PayloadTooShort { need: usize, have: usize },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    BufferTooSmall { need: usize, have: usize },
    InvalidGridDimension(u16),
    PayloadLengthMismatch { declared: usize, actual: usize },
}

impl std::fmt::Display for ImpostorSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { need, have } => {
                write!(f, "impostor section payload too short: have {have}, need {need}")
            }
            Self::BadMagic(m) => write!(f, "bad impostor section magic: {m:?}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported impostor section version {v}"),
            Self::BufferTooSmall { need, have } => {
                write!(f, "buffer too small: have {have}, need {need}")
            }
            Self::InvalidGridDimension(d) => write!(f, "invalid impostor grid dimension: {d}"),
            Self::PayloadLengthMismatch { declared, actual } => {
                write!(f, "impostor payload length mismatch: declared {declared}, actual {actual}")
            }
        }
    }
}

impl std::error::Error for ImpostorSectionError {}

/// Encode impostor header and binary atlas payload into caller-supplied buffer. Zero heap.
pub fn encode_impostor_section(
    header: &ImpostorAtlasMiniHeader,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, ImpostorSectionError> {
    let total_len = IMPOSTOR_MINI_HEADER_SIZE + payload.len();
    if out.len() < total_len {
        return Err(ImpostorSectionError::BufferTooSmall {
            need: total_len,
            have: out.len(),
        });
    }
    let header_bytes = bytemuck::bytes_of(header);
    out[..IMPOSTOR_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
    out[IMPOSTOR_MINI_HEADER_SIZE..total_len].copy_from_slice(payload);
    Ok(total_len)
}

/// Decode impostor header and extract zero-copy slice of atlas payload. Zero heap.
pub fn decode_impostor_section<'a>(
    payload: &'a [u8],
) -> Result<(ImpostorAtlasMiniHeader, &'a [u8]), ImpostorSectionError> {
    if payload.len() < IMPOSTOR_MINI_HEADER_SIZE {
        return Err(ImpostorSectionError::PayloadTooShort {
            need: IMPOSTOR_MINI_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let header: ImpostorAtlasMiniHeader =
        *bytemuck::from_bytes(&payload[..IMPOSTOR_MINI_HEADER_SIZE]);
    if header.magic != MAGIC_IMPO {
        return Err(ImpostorSectionError::BadMagic(header.magic));
    }
    if header.version != IMPOSTOR_SECTION_VERSION {
        return Err(ImpostorSectionError::UnsupportedVersion(header.version));
    }
    if header.grid_dimension == 0 || header.grid_dimension > 64 {
        return Err(ImpostorSectionError::InvalidGridDimension(header.grid_dimension));
    }
    let data_len = payload.len() - IMPOSTOR_MINI_HEADER_SIZE;
    if header.atlas_byte_size as usize != data_len {
        return Err(ImpostorSectionError::PayloadLengthMismatch {
            declared: header.atlas_byte_size as usize,
            actual: data_len,
        });
    }
    Ok((header, &payload[IMPOSTOR_MINI_HEADER_SIZE..]))
}

// ───────────────────────────────────────────────────────────────────────────
//  Hemi-Octahedral Projection Mathematics (Zero-Heap Hot-Path Helpers)
// ───────────────────────────────────────────────────────────────────────────

/// Project a normalized 3D direction vector $V = (x, y, z)$ on the upper hemisphere
/// ($z \ge 0$) onto 2D octahedral UV coordinates $[0.0, 1.0]^2$.
#[inline]
pub fn hemi_octahedral_encode(dir: [f32; 3]) -> [f32; 2] {
    let l1_norm = dir[0].abs() + dir[1].abs() + dir[2].max(0.0);
    if l1_norm < 1e-7 {
        return [0.5, 0.5];
    }
    let inv_l1 = 1.0 / l1_norm;
    let u = (dir[0] - dir[1]) * inv_l1 * 0.5 + 0.5;
    let v = (dir[0] + dir[1]) * inv_l1 * 0.5 + 0.5;
    [u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)]
}

/// Unproject 2D octahedral UV coordinates $[0.0, 1.0]^2$ back to a normalized
/// 3D directional vector on the upper hemisphere.
#[inline]
pub fn hemi_octahedral_decode(uv: [f32; 2]) -> [f32; 3] {
    // Map UV from [0, 1] to [-1, 1]
    let u = uv[0] * 2.0 - 1.0;
    let v = uv[1] * 2.0 - 1.0;
    let x = (u + v) * 0.5;
    let y = (v - u) * 0.5;
    let z = 1.0 - x.abs() - y.abs();
    let len = (x * x + y * y + z * z).sqrt();
    if len < 1e-7 {
        [0.0, 0.0, 1.0]
    } else {
        [x / len, y / len, (z / len).max(0.0)]
    }
}

/// Viewpoint blending information for smooth camera interpolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpostorBlendSample {
    /// 3 nearest view grid indices (row * grid_dimension + col).
    pub view_indices: [u16; 3],
    /// Barycentric weights summing to 1.0.
    pub weights: [f32; 3],
}

/// Calculate the 3 closest discrete camera viewpoints on the octahedral grid and their
/// barycentric blending weights.
pub fn sample_impostor_blend_weights(view_dir: [f32; 3], grid_dim: u16) -> ImpostorBlendSample {
    if grid_dim < 2 {
        return ImpostorBlendSample {
            view_indices: [0, 0, 0],
            weights: [1.0, 0.0, 0.0],
        };
    }
    let uv = hemi_octahedral_encode(view_dir);
    let n = (grid_dim - 1) as f32;
    let gx = (uv[0] * n).clamp(0.0, n);
    let gy = (uv[1] * n).clamp(0.0, n);

    let x0 = gx.floor() as u16;
    let y0 = gy.floor() as u16;
    let x1 = (x0 + 1).min(grid_dim - 1);
    let y1 = (y0 + 1).min(grid_dim - 1);

    let fx = gx - x0 as f32;
    let fy = gy - y0 as f32;

    let idx00 = y0 * grid_dim + x0;
    let idx10 = y0 * grid_dim + x1;
    let idx01 = y1 * grid_dim + x0;
    let idx11 = y1 * grid_dim + x1;

    // Split quad into two triangles along diagonal
    if fx + fy <= 1.0 {
        ImpostorBlendSample {
            view_indices: [idx00, idx10, idx01],
            weights: [1.0 - fx - fy, fx, fy],
        }
    } else {
        ImpostorBlendSample {
            view_indices: [idx11, idx10, idx01],
            weights: [fx + fy - 1.0, 1.0 - fy, 1.0 - fx],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impostor_header_roundtrip() {
        let header = ImpostorAtlasMiniHeader {
            magic: MAGIC_IMPO,
            version: IMPOSTOR_SECTION_VERSION,
            flags: FLAG_HEMI_OCTAHEDRAL | FLAG_DEPTH_IN_ALPHA,
            grid_dimension: 16,
            view_resolution: 64,
            sphere_center: [0.0, 1.5, 0.0],
            sphere_radius: 2.2,
            atlas_byte_size: 16,
            channel_format: 0,
            reserved: [0; 11],
        };
        let dummy_payload = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mut buffer = [0u8; 128];

        let written = encode_impostor_section(&header, &dummy_payload, &mut buffer).unwrap();
        assert_eq!(written, 64 + 16);

        let (decoded_hdr, decoded_data) = decode_impostor_section(&buffer[..written]).unwrap();
        assert_eq!(decoded_hdr, header);
        assert_eq!(decoded_data, &dummy_payload);
    }

    #[test]
    fn test_hemi_octahedral_projection_cardinal() {
        // Up direction (0, 0, 1) should map to center (0.5, 0.5)
        let uv = hemi_octahedral_encode([0.0, 0.0, 1.0]);
        assert!((uv[0] - 0.5).abs() < 1e-4);
        assert!((uv[1] - 0.5).abs() < 1e-4);

        let recovered = hemi_octahedral_decode(uv);
        assert!((recovered[0] - 0.0).abs() < 1e-4);
        assert!((recovered[1] - 0.0).abs() < 1e-4);
        assert!((recovered[2] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_barycentric_blend_weights() {
        let sample = sample_impostor_blend_weights([0.0, 0.0, 1.0], 8);
        let sum = sample.weights[0] + sample.weights[1] + sample.weights[2];
        assert!((sum - 1.0).abs() < 1e-5);
    }
}
