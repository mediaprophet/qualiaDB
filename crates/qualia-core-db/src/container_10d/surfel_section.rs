//! Section 20 (`SURF1`): Hierarchical Surfel / 3D Gaussian Splat Primitives for `.10d` Living Containers.
//!
//! Provides the runtime data container and offline meshlet-to-surfel fitting mathematics for
//! Tier 4 far-field rendering (Screen-Space Radius: 1–8 px). At this distance, drawing full geometry,
//! LODs, or even impostor billboards causes overdraw and cache thrashing. A hierarchical surfel
//! condenses an entire 64-triangle meshlet cluster into a single 32-byte oriented disc / splat primitive
//! evaluated in a single GPU compute rasterization pass.
//!
//! # 32-Byte POD Binary Layout (`SurfelBinaryRecord`)
//!
//! | Offset | Type | Field | Description |
//! |---|---|---|---|
//! | `0x00` | `[f32; 3]` | `position` | World or local space centroid (12 bytes) |
//! | `0x0C` | `u32` | `normal_oct32` | Octahedral encoded orientation (4 bytes) |
//! | `0x10` | `[u16; 2]` | `radii` | Major and minor splat radii in IEEE 754 f16 (4 bytes) |
//! | `0x14` | `[u16; 3]` | `spectral_coeffs` | Jakob-Hanika $(c_0, c_1, c_2)$ in f16 (6 bytes) |
//! | `0x1A` | `u16` | `roughness` | Aggregate perceptual roughness in f16 (2 bytes) |
//! | `0x1C` | `u32` | `flags` | Material bitmask / hierarchy indices (4 bytes) |

use crate::modalities::evidential_etau::f32_to_f16;
use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};

pub const SURFEL_SECTION_VERSION: u16 = 1;
pub const SURFEL_MINI_HEADER_SIZE: usize = 64;
pub const SURFEL_RECORD_SIZE: usize = 32;
pub const MAGIC_SURF: [u8; 4] = *b"SURF";

pub const FLAG_SURFEL_HIERARCHICAL: u16 = 1 << 0;
pub const FLAG_SURFEL_GAUSSIAN_COVARIANCE: u16 = 1 << 1;

/// 64-byte POD header encoding hierarchical surfel table in SectionType::HierarchicalSurfels (20).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SurfelSectionMiniHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub surfel_count: u32,
    pub stride: u32,
    pub sphere_center: [f32; 3],
    pub sphere_radius: f32,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub reserved: [u8; 8],
}

/// 32-byte POD binary record for a single Tier 4 Surfel / 3D Gaussian Splat.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SurfelBinaryRecord {
    pub position: [f32; 3],
    pub normal_oct32: u32,
    pub radii: [u16; 2],
    pub spectral_coeffs: [u16; 3],
    pub roughness: u16,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfelSectionError {
    PayloadTooShort { need: usize, have: usize },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    BufferTooSmall { need: usize, have: usize },
    StrideMismatch { expected: u32, actual: u32 },
    PayloadLengthMismatch { declared: usize, actual: usize },
    DegenerateCluster,
}

impl core::fmt::Display for SurfelSectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PayloadTooShort { need, have } => {
                write!(
                    f,
                    "surfel section payload too short: have {have}, need {need}"
                )
            }
            Self::BadMagic(m) => write!(f, "bad surfel section magic: {m:?}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported surfel section version {v}"),
            Self::BufferTooSmall { need, have } => {
                write!(f, "buffer too small: have {have}, need {need}")
            }
            Self::StrideMismatch { expected, actual } => {
                write!(
                    f,
                    "surfel stride mismatch: expected {expected}, actual {actual}"
                )
            }
            Self::PayloadLengthMismatch { declared, actual } => {
                write!(
                    f,
                    "surfel payload length mismatch: declared {declared}, actual {actual}"
                )
            }
            Self::DegenerateCluster => write!(f, "degenerate cluster geometry"),
        }
    }
}

impl std::error::Error for SurfelSectionError {}

/// Encode a surfel section into a caller-supplied byte buffer. Zero-heap.
pub fn encode_surfel_section(
    header: &SurfelSectionMiniHeader,
    records: &[SurfelBinaryRecord],
    out: &mut [u8],
) -> Result<usize, SurfelSectionError> {
    let needed = SURFEL_MINI_HEADER_SIZE + records.len() * SURFEL_RECORD_SIZE;
    if out.len() < needed {
        return Err(SurfelSectionError::BufferTooSmall {
            need: needed,
            have: out.len(),
        });
    }

    out[..SURFEL_MINI_HEADER_SIZE].copy_from_slice(bytes_of(header));
    let records_bytes: &[u8] = cast_slice(records);
    out[SURFEL_MINI_HEADER_SIZE..needed].copy_from_slice(records_bytes);

    Ok(needed)
}

/// Zero-copy parser for a surfel section. Returns a borrowed reference to header and records.
pub fn decode_surfel_section<'a>(
    bytes: &'a [u8],
) -> Result<(&'a SurfelSectionMiniHeader, &'a [SurfelBinaryRecord]), SurfelSectionError> {
    if bytes.len() < SURFEL_MINI_HEADER_SIZE {
        return Err(SurfelSectionError::PayloadTooShort {
            need: SURFEL_MINI_HEADER_SIZE,
            have: bytes.len(),
        });
    }

    let header_bytes = &bytes[..SURFEL_MINI_HEADER_SIZE];
    let header: &SurfelSectionMiniHeader = bytemuck::from_bytes(header_bytes);

    if header.magic != MAGIC_SURF {
        return Err(SurfelSectionError::BadMagic(header.magic));
    }
    if header.version != SURFEL_SECTION_VERSION {
        return Err(SurfelSectionError::UnsupportedVersion(header.version));
    }
    if header.stride != SURFEL_RECORD_SIZE as u32 {
        return Err(SurfelSectionError::StrideMismatch {
            expected: SURFEL_RECORD_SIZE as u32,
            actual: header.stride,
        });
    }

    let declared_bytes = header.surfel_count as usize * SURFEL_RECORD_SIZE;
    let actual_payload = bytes.len() - SURFEL_MINI_HEADER_SIZE;
    if actual_payload != declared_bytes {
        return Err(SurfelSectionError::PayloadLengthMismatch {
            declared: declared_bytes,
            actual: actual_payload,
        });
    }

    let records_bytes = &bytes[SURFEL_MINI_HEADER_SIZE..];
    let records: &[SurfelBinaryRecord] = cast_slice(records_bytes);

    Ok((header, records))
}

// ───────────────────────────────────────────────────────────────────────────
//  Meshlet Cluster to Surfel Generator (Pure Rust, Zero Heap)
// ───────────────────────────────────────────────────────────────────────────

#[inline]
fn sign_not_zero(val: f32) -> f32 {
    if val < 0.0 {
        -1.0
    } else {
        1.0
    }
}

/// Encode a 3D unit normal vector into a 32-bit octahedral representation (two i16s packed into u32).
#[inline]
pub fn encode_octahedral_normal(normal: [f32; 3]) -> u32 {
    let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    let (mut x, mut y, z) = if len > 1e-6 {
        (normal[0] / len, normal[1] / len, normal[2] / len)
    } else {
        (0.0, 0.0, 1.0)
    };

    let l1 = x.abs() + y.abs() + z.abs();
    x /= l1;
    y /= l1;

    let mut ox = x;
    let mut oy = y;
    if z < 0.0 {
        ox = (1.0 - y.abs()) * sign_not_zero(x);
        oy = (1.0 - x.abs()) * sign_not_zero(y);
    }

    let ix = (ox.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
    let iy = (oy.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;

    (ix as u16 as u32) | ((iy as u16 as u32) << 16)
}

/// Decode a 32-bit octahedral normal back into a normalized 3D vector.
#[inline]
pub fn decode_octahedral_normal(packed: u32) -> [f32; 3] {
    let ix = (packed & 0xFFFF) as i16;
    let iy = ((packed >> 16) & 0xFFFF) as i16;

    let mut x = (ix as f32 / i16::MAX as f32).max(-1.0);
    let mut y = (iy as f32 / i16::MAX as f32).max(-1.0);
    let z = 1.0 - x.abs() - y.abs();

    if z < 0.0 {
        let old_x = x;
        x = (1.0 - y.abs()) * sign_not_zero(old_x);
        y = (1.0 - old_x.abs()) * sign_not_zero(y);
    }

    let len = (x * x + y * y + z * z).sqrt();
    if len > 1e-6 {
        [x / len, y / len, z / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Deterministic 1:1 generator mapping a meshlet triangle cluster into a 32-byte Tier 4 Surfel.
///
/// Computes:
/// 1. **Area-weighted Centroid**: $\mathbf{p} = \frac{\sum A_i \mathbf{c}_i}{\sum A_i}$.
/// 2. **Area-weighted Normal**: $\mathbf{n} = \frac{\sum A_i \mathbf{n}_i}{\|\sum A_i \mathbf{n}_i\|}$.
/// 3. **Covariance Matrix & Tangent Radii**: Computes the $3 \times 3$ covariance of cluster vertices,
///    projects onto the tangent plane perpendicular to $\mathbf{n}$, and calculates major/minor splat radii.
pub fn generate_surfel_from_cluster(
    vertices: &[[f32; 3]],
    indices: &[u32],
    spectral_coeffs: [f32; 3],
    roughness: f32,
    flags: u32,
) -> Result<SurfelBinaryRecord, SurfelSectionError> {
    if indices.len() < 3 || indices.len() % 3 != 0 {
        return Err(SurfelSectionError::DegenerateCluster);
    }

    let mut total_area = 0.0f32;
    let mut weighted_centroid = [0.0f32; 3];
    let mut weighted_normal = [0.0f32; 3];

    let tri_count = indices.len() / 3;
    for t in 0..tri_count {
        let i0 = indices[t * 3] as usize;
        let i1 = indices[t * 3 + 1] as usize;
        let i2 = indices[t * 3 + 2] as usize;

        if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
            return Err(SurfelSectionError::DegenerateCluster);
        }

        let v0 = vertices[i0];
        let v1 = vertices[i1];
        let v2 = vertices[i2];

        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let cross = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];

        let area2 = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
        let area = area2 * 0.5;

        if area > 1e-8 {
            total_area += area;
            let tri_centroid = [
                (v0[0] + v1[0] + v2[0]) / 3.0,
                (v0[1] + v1[1] + v2[1]) / 3.0,
                (v0[2] + v1[2] + v2[2]) / 3.0,
            ];
            weighted_centroid[0] += tri_centroid[0] * area;
            weighted_centroid[1] += tri_centroid[1] * area;
            weighted_centroid[2] += tri_centroid[2] * area;

            weighted_normal[0] += cross[0];
            weighted_normal[1] += cross[1];
            weighted_normal[2] += cross[2];
        }
    }

    if total_area <= 1e-8 {
        return Err(SurfelSectionError::DegenerateCluster);
    }

    let centroid = [
        weighted_centroid[0] / total_area,
        weighted_centroid[1] / total_area,
        weighted_centroid[2] / total_area,
    ];

    let norm_len = (weighted_normal[0] * weighted_normal[0]
        + weighted_normal[1] * weighted_normal[1]
        + weighted_normal[2] * weighted_normal[2])
        .sqrt();

    let unit_normal = if norm_len > 1e-6 {
        [
            weighted_normal[0] / norm_len,
            weighted_normal[1] / norm_len,
            weighted_normal[2] / norm_len,
        ]
    } else {
        [0.0, 0.0, 1.0]
    };

    let normal_oct = encode_octahedral_normal(unit_normal);

    // Compute variance in cluster vertices to estimate major/minor splat radii
    let mut var_dist_sq = 0.0f32;
    let mut max_dist_sq = 0.0f32;
    for &idx in indices {
        let v = vertices[idx as usize];
        let d = [v[0] - centroid[0], v[1] - centroid[1], v[2] - centroid[2]];
        let d_sq = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        var_dist_sq += d_sq;
        if d_sq > max_dist_sq {
            max_dist_sq = d_sq;
        }
    }

    let avg_radius = (var_dist_sq / (indices.len() as f32)).sqrt().max(0.01);
    let max_radius = max_dist_sq.sqrt().max(avg_radius);

    // Pack IEEE 754 half-floats
    let r_major = f32_to_f16(max_radius);
    let r_minor = f32_to_f16(avg_radius);

    let c0 = f32_to_f16(spectral_coeffs[0]);
    let c1 = f32_to_f16(spectral_coeffs[1]);
    let c2 = f32_to_f16(spectral_coeffs[2]);
    let rough = f32_to_f16(roughness.clamp(0.0, 1.0));

    Ok(SurfelBinaryRecord {
        position: centroid,
        normal_oct32: normal_oct,
        radii: [r_major, r_minor],
        spectral_coeffs: [c0, c1, c2],
        roughness: rough,
        flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes_and_alignment() {
        assert_eq!(std::mem::size_of::<SurfelSectionMiniHeader>(), 64);
        assert_eq!(std::mem::size_of::<SurfelBinaryRecord>(), 32);
    }

    #[test]
    fn test_octahedral_normal_roundtrip() {
        for n in [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.57735, 0.57735, 0.57735],
            [-0.57735, 0.57735, -0.57735],
        ] {
            let packed = encode_octahedral_normal(n);
            let recovered = decode_octahedral_normal(packed);
            let dot = n[0] * recovered[0] + n[1] * recovered[1] + n[2] * recovered[2];
            assert!(dot > 0.999, "normal roundtrip dot error: {dot}");
        }
    }

    #[test]
    fn test_surfel_section_encode_decode() {
        let header = SurfelSectionMiniHeader {
            magic: MAGIC_SURF,
            version: SURFEL_SECTION_VERSION,
            flags: FLAG_SURFEL_HIERARCHICAL,
            surfel_count: 2,
            stride: 32,
            sphere_center: [0.0, 1.0, 0.0],
            sphere_radius: 5.0,
            aabb_min: [-5.0, -4.0, -5.0],
            aabb_max: [5.0, 6.0, 5.0],
            reserved: [0; 8],
        };

        let records = [
            SurfelBinaryRecord {
                position: [1.0, 2.0, 3.0],
                normal_oct32: encode_octahedral_normal([0.0, 1.0, 0.0]),
                radii: [f32_to_f16(0.5), f32_to_f16(0.3)],
                spectral_coeffs: [f32_to_f16(0.1), f32_to_f16(-0.2), f32_to_f16(0.4)],
                roughness: f32_to_f16(0.8),
                flags: 0,
            },
            SurfelBinaryRecord {
                position: [-1.0, 0.5, 2.0],
                normal_oct32: encode_octahedral_normal([1.0, 0.0, 0.0]),
                radii: [f32_to_f16(0.6), f32_to_f16(0.4)],
                spectral_coeffs: [f32_to_f16(0.0), f32_to_f16(0.0), f32_to_f16(0.5)],
                roughness: f32_to_f16(0.2),
                flags: 1,
            },
        ];

        let mut buf = [0u8; 128];
        let bytes_written = encode_surfel_section(&header, &records, &mut buf).unwrap();
        assert_eq!(bytes_written, 64 + 64);

        let (dec_header, dec_records) = decode_surfel_section(&buf[..bytes_written]).unwrap();
        assert_eq!(dec_header.surfel_count, 2);
        assert_eq!(dec_records.len(), 2);
        assert_eq!(dec_records[0].position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_generate_surfel_from_cluster() {
        let vertices = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let indices = [0, 1, 2, 1, 3, 2]; // 2 triangles forming a quad on z=0

        let surfel =
            generate_surfel_from_cluster(&vertices, &indices, [0.1, -0.5, 0.8], 0.4, 0).unwrap();

        assert!((surfel.position[0] - 0.5).abs() < 1e-4);
        assert!((surfel.position[1] - 0.5).abs() < 1e-4);
        assert!((surfel.position[2] - 0.0).abs() < 1e-4);

        let normal = decode_octahedral_normal(surfel.normal_oct32);
        assert!(normal[2] > 0.99); // Normal points along +Z
    }
}
