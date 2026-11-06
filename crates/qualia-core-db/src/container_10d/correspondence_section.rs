//! Section 9 (`FEMC`): Volumetric-to-Surface Correspondence Map Section.
//!
//! Binds high-resolution visual surface mesh vertices (`QuantizedMesh` / `ClusterDag`)
//! to volumetric tetrahedral finite elements (`Reconstruction` / `FEM` / `XPBD`) via
//! 16-byte [`VertexTetBinding`] records with `unorm16` partition-of-unity barycentric
//! weights (`w0 + w1 + w2 + w3 == 65535`) and octahedral reference normal encodings.

use bytemuck::{bytes_of, pod_read_unaligned, Pod, Zeroable};

use super::cluster_dag_section::{decode_oct16_normal, encode_oct16_normal};

/// 4-byte ASCII magic for `.10d` Volumetric-to-Surface Correspondence sections (`"FEMC"`).
pub const MAGIC_FEMC: [u8; 4] = *b"FEMC";

/// Current format version of the `FEMC` section.
pub const CORRESPONDENCE_SECTION_VERSION: u16 = 1;

/// Fixed byte size of [`CorrespondenceMiniHeader`].
pub const CORRESPONDENCE_HEADER_SIZE: usize = 16;

/// Fixed byte size of [`VertexTetBinding`].
pub const VERTEX_TET_BINDING_SIZE: usize = 16;

/// Normalization denominator for `unorm16` barycentric coordinates.
pub const BARYCENTRIC_UNORM16_SUM: u32 = 65_535;

/// Maximum bindings admitted per `FEMC` section.
pub const MAX_CORRESPONDENCE_BINDINGS: usize = 1_048_576;

/// Section flag: all bindings strictly lie inside or on the boundary of their host tetrahedron.
pub const FLAG_FEMC_CONFORMING_BARYCENTRIC: u16 = 1 << 0;

/// Section flag: `normal_rotation_oct16` stores rest-frame local normal directions.
pub const FLAG_FEMC_HAS_NORMAL_ROTATION: u16 = 1 << 1;

/// 16-byte POD mini-header for `SectionType::CorrespondenceMap` (`FEMC`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct CorrespondenceMiniHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub binding_count: u32,
    pub tet_cell_count: u32,
}

const _: () =
    assert!(core::mem::size_of::<CorrespondenceMiniHeader>() == CORRESPONDENCE_HEADER_SIZE);

impl Default for CorrespondenceMiniHeader {
    fn default() -> Self {
        Self {
            magic: MAGIC_FEMC,
            version: CORRESPONDENCE_SECTION_VERSION,
            flags: FLAG_FEMC_CONFORMING_BARYCENTRIC | FLAG_FEMC_HAS_NORMAL_ROTATION,
            binding_count: 0,
            tet_cell_count: 0,
        }
    }
}

/// 16-byte POD record binding one surface vertex to a volumetric tetrahedron.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct VertexTetBinding {
    /// Index of the host tetrahedron in the volumetric element array (4B).
    pub tet_element_index: u32,
    /// Quantized barycentric weights `[w0, w1, w2, w3]` summing to `65535` (8B).
    pub barycentric_unorm16: [u16; 4],
    /// Rest-pose surface normal or local frame axis in octahedral `snorm16` (4B).
    pub normal_rotation_oct16: [i16; 2],
}

const _: () = assert!(core::mem::size_of::<VertexTetBinding>() == VERTEX_TET_BINDING_SIZE);

impl VertexTetBinding {
    /// Quantize floating-point barycentric weights `[w0, w1, w2, w3]` and a 3D normal
    /// into a 16-byte [`VertexTetBinding`] whose `barycentric_unorm16` sums to `65535`.
    #[inline]
    pub fn from_barycentric(
        tet_element_index: u32,
        weights: [f32; 4],
        normal_dir: [f32; 3],
    ) -> Self {
        let mut clamped = [0.0f32; 4];
        for i in 0..4 {
            let w = weights[i];
            clamped[i] = if w.is_finite() && w > 0.0 { w } else { 0.0 };
        }
        let sum = clamped[0] + clamped[1] + clamped[2] + clamped[3];
        let unorm = if sum <= 1e-12 {
            [65_535u16, 0, 0, 0]
        } else {
            let mut q = [0u16; 4];
            let mut max_idx = 0usize;
            let mut q_sum = 0i32;
            for i in 0..4 {
                if clamped[i] > clamped[max_idx] {
                    max_idx = i;
                }
                let scaled = ((clamped[i] / sum) * 65_535.0).round().clamp(0.0, 65_535.0) as u16;
                q[i] = scaled;
                q_sum += scaled as i32;
            }
            let diff = 65_535i32 - q_sum;
            q[max_idx] = (q[max_idx] as i32 + diff).clamp(0, 65_535) as u16;
            q
        };

        Self {
            tet_element_index,
            barycentric_unorm16: unorm,
            normal_rotation_oct16: encode_oct16_normal(normal_dir),
        }
    }

    /// Convert quantized `unorm16` weights back to normalized `[f32; 4]` weights.
    #[inline]
    pub fn barycentric_weights(&self) -> [f32; 4] {
        const INV: f32 = 1.0 / 65_535.0;
        [
            self.barycentric_unorm16[0] as f32 * INV,
            self.barycentric_unorm16[1] as f32 * INV,
            self.barycentric_unorm16[2] as f32 * INV,
            self.barycentric_unorm16[3] as f32 * INV,
        ]
    }

    /// Interpolate the deformed 3D surface vertex position from deformed tetrahedron corners.
    #[inline]
    pub fn interpolate_deformed_vertex(&self, tet_corners: [[f32; 3]; 4]) -> [f32; 3] {
        let w = self.barycentric_weights();
        [
            w[0] * tet_corners[0][0]
                + w[1] * tet_corners[1][0]
                + w[2] * tet_corners[2][0]
                + w[3] * tet_corners[3][0],
            w[0] * tet_corners[0][1]
                + w[1] * tet_corners[1][1]
                + w[2] * tet_corners[2][1]
                + w[3] * tet_corners[3][1],
            w[0] * tet_corners[0][2]
                + w[1] * tet_corners[1][2]
                + w[2] * tet_corners[2][2]
                + w[3] * tet_corners[3][2],
        ]
    }

    /// Decode the octahedral normal rotation field back into a unit 3D vector.
    #[inline]
    pub fn decoded_normal_dir(&self) -> [f32; 3] {
        decode_oct16_normal(self.normal_rotation_oct16)
    }
}

/// Errors emitted when encoding, decoding, or evaluating a `FEMC` correspondence section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrespondenceSectionError {
    PayloadTooShort {
        need: usize,
        have: usize,
    },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    TooManyBindings {
        count: usize,
        max: usize,
    },
    PayloadLengthMismatch {
        declared_bindings: u32,
        payload_bytes: usize,
    },
    BufferTooSmall {
        need: usize,
        have: usize,
    },
    TetIndexOutOfBounds {
        binding_index: usize,
        tet_index: u32,
        tet_cell_count: u32,
    },
    InvalidBarycentricSum {
        binding_index: usize,
        sum: u32,
    },
    TetVertexIndexOutOfBounds {
        tet_index: usize,
        vertex_index: u32,
        vertex_count: usize,
    },
}

impl core::fmt::Display for CorrespondenceSectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PayloadTooShort { need, have } => {
                write!(f, "FEMC payload too short: have {have}, need {need}")
            }
            Self::BadMagic(m) => write!(f, "FEMC bad magic: {m:?}"),
            Self::UnsupportedVersion(v) => write!(f, "FEMC unsupported version: {v}"),
            Self::TooManyBindings { count, max } => {
                write!(f, "FEMC binding count {count} exceeds max {max}")
            }
            Self::PayloadLengthMismatch {
                declared_bindings,
                payload_bytes,
            } => write!(
                f,
                "FEMC payload length {payload_bytes} does not match {declared_bindings} bindings"
            ),
            Self::BufferTooSmall { need, have } => {
                write!(f, "FEMC buffer too small: have {have}, need {need}")
            }
            Self::TetIndexOutOfBounds {
                binding_index,
                tet_index,
                tet_cell_count,
            } => write!(
                f,
                "FEMC binding[{binding_index}] tet_index {tet_index} >= tet_cell_count {tet_cell_count}"
            ),
            Self::InvalidBarycentricSum { binding_index, sum } => write!(
                f,
                "FEMC binding[{binding_index}] barycentric sum {sum} != {BARYCENTRIC_UNORM16_SUM}"
            ),
            Self::TetVertexIndexOutOfBounds {
                tet_index,
                vertex_index,
                vertex_count,
            } => write!(
                f,
                "FEMC tet[{tet_index}] vertex {vertex_index} >= vertex_count {vertex_count}"
            ),
        }
    }
}

impl std::error::Error for CorrespondenceSectionError {}

/// Compute exact byte length for a `FEMC` section with `binding_count` records.
#[inline]
pub const fn correspondence_encoded_len(binding_count: usize) -> Option<usize> {
    if binding_count > MAX_CORRESPONDENCE_BINDINGS {
        return None;
    }
    match binding_count.checked_mul(VERTEX_TET_BINDING_SIZE) {
        Some(body) => CORRESPONDENCE_HEADER_SIZE.checked_add(body),
        None => None,
    }
}

fn validate_binding(
    index: usize,
    binding: &VertexTetBinding,
    tet_cell_count: u32,
) -> Result<(), CorrespondenceSectionError> {
    if tet_cell_count > 0 && binding.tet_element_index >= tet_cell_count {
        return Err(CorrespondenceSectionError::TetIndexOutOfBounds {
            binding_index: index,
            tet_index: binding.tet_element_index,
            tet_cell_count,
        });
    }
    let sum = binding.barycentric_unorm16[0] as u32
        + binding.barycentric_unorm16[1] as u32
        + binding.barycentric_unorm16[2] as u32
        + binding.barycentric_unorm16[3] as u32;
    if sum != BARYCENTRIC_UNORM16_SUM {
        return Err(CorrespondenceSectionError::InvalidBarycentricSum {
            binding_index: index,
            sum,
        });
    }
    Ok(())
}

/// Encode a `FEMC` section into a caller-supplied byte slice without heap allocation.
pub fn encode_correspondence_section(
    header: &CorrespondenceMiniHeader,
    bindings: &[VertexTetBinding],
    out: &mut [u8],
) -> Result<usize, CorrespondenceSectionError> {
    if header.magic != MAGIC_FEMC {
        return Err(CorrespondenceSectionError::BadMagic(header.magic));
    }
    if header.version != CORRESPONDENCE_SECTION_VERSION {
        return Err(CorrespondenceSectionError::UnsupportedVersion(
            header.version,
        ));
    }
    let needed = correspondence_encoded_len(bindings.len()).ok_or(
        CorrespondenceSectionError::TooManyBindings {
            count: bindings.len(),
            max: MAX_CORRESPONDENCE_BINDINGS,
        },
    )?;
    if out.len() < needed {
        return Err(CorrespondenceSectionError::BufferTooSmall {
            need: needed,
            have: out.len(),
        });
    }
    for (idx, b) in bindings.iter().enumerate() {
        validate_binding(idx, b, header.tet_cell_count)?;
    }

    let mut canonical_header = *header;
    canonical_header.binding_count = bindings.len() as u32;
    out[..CORRESPONDENCE_HEADER_SIZE].copy_from_slice(bytes_of(&canonical_header));

    let mut offset = CORRESPONDENCE_HEADER_SIZE;
    for b in bindings {
        out[offset..offset + VERTEX_TET_BINDING_SIZE].copy_from_slice(bytes_of(b));
        offset += VERTEX_TET_BINDING_SIZE;
    }
    Ok(needed)
}

/// Decode and validate a `FEMC` section into caller-provided `VertexTetBinding` storage (Zero-Heap).
pub fn decode_correspondence_section(
    payload: &[u8],
    out_bindings: &mut [VertexTetBinding],
) -> Result<(CorrespondenceMiniHeader, usize), CorrespondenceSectionError> {
    if payload.len() < CORRESPONDENCE_HEADER_SIZE {
        return Err(CorrespondenceSectionError::PayloadTooShort {
            need: CORRESPONDENCE_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let header: CorrespondenceMiniHeader = pod_read_unaligned(&payload[..CORRESPONDENCE_HEADER_SIZE]);
    if header.magic != MAGIC_FEMC {
        return Err(CorrespondenceSectionError::BadMagic(header.magic));
    }
    if header.version != CORRESPONDENCE_SECTION_VERSION {
        return Err(CorrespondenceSectionError::UnsupportedVersion(
            header.version,
        ));
    }
    let count = header.binding_count as usize;
    let needed_bytes = correspondence_encoded_len(count).ok_or(
        CorrespondenceSectionError::TooManyBindings {
            count,
            max: MAX_CORRESPONDENCE_BINDINGS,
        },
    )?;
    if payload.len() != needed_bytes {
        return Err(CorrespondenceSectionError::PayloadLengthMismatch {
            declared_bindings: header.binding_count,
            payload_bytes: payload.len(),
        });
    }
    if out_bindings.len() < count {
        return Err(CorrespondenceSectionError::BufferTooSmall {
            need: count,
            have: out_bindings.len(),
        });
    }

    let mut offset = CORRESPONDENCE_HEADER_SIZE;
    for (idx, slot) in out_bindings[..count].iter_mut().enumerate() {
        let binding: VertexTetBinding =
            pod_read_unaligned(&payload[offset..offset + VERTEX_TET_BINDING_SIZE]);
        validate_binding(idx, &binding, header.tet_cell_count)?;
        *slot = binding;
        offset += VERTEX_TET_BINDING_SIZE;
    }
    Ok((header, count))
}

/// Evaluate deformed surface vertex positions across a `FEMC` binding slice in zero heap memory.
pub fn deform_surface_vertices_zero_heap(
    bindings: &[VertexTetBinding],
    tet_elements: &[[u32; 4]],
    tet_vertices: &[[f32; 3]],
    out_surface_vertices: &mut [[f32; 3]],
) -> Result<usize, CorrespondenceSectionError> {
    if out_surface_vertices.len() < bindings.len() {
        return Err(CorrespondenceSectionError::BufferTooSmall {
            need: bindings.len(),
            have: out_surface_vertices.len(),
        });
    }
    for (idx, binding) in bindings.iter().enumerate() {
        let tet_idx = binding.tet_element_index as usize;
        if tet_idx >= tet_elements.len() {
            return Err(CorrespondenceSectionError::TetIndexOutOfBounds {
                binding_index: idx,
                tet_index: binding.tet_element_index,
                tet_cell_count: tet_elements.len() as u32,
            });
        }
        let tet = tet_elements[tet_idx];
        let mut corners = [[0.0f32; 3]; 4];
        for k in 0..4 {
            let vi = tet[k] as usize;
            if vi >= tet_vertices.len() {
                return Err(CorrespondenceSectionError::TetVertexIndexOutOfBounds {
                    tet_index: tet_idx,
                    vertex_index: tet[k],
                    vertex_count: tet_vertices.len(),
                });
            }
            corners[k] = tet_vertices[vi];
        }
        out_surface_vertices[idx] = binding.interpolate_deformed_vertex(corners);
    }
    Ok(bindings.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn femc_layout_and_barycentric_partition_of_unity() {
        assert_eq!(core::mem::size_of::<CorrespondenceMiniHeader>(), 16);
        assert_eq!(core::mem::size_of::<VertexTetBinding>(), 16);

        let b = VertexTetBinding::from_barycentric(
            0,
            [0.25, 0.25, 0.25, 0.25],
            [0.0, 1.0, 0.0],
        );
        let sum: u32 = b.barycentric_unorm16.iter().map(|&w| w as u32).sum();
        assert_eq!(sum, BARYCENTRIC_UNORM16_SUM);

        let corners = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 2.0],
        ];
        let pos = b.interpolate_deformed_vertex(corners);
        assert!((pos[0] - 0.5).abs() < 1e-4);
        assert!((pos[1] - 0.5).abs() < 1e-4);
        assert!((pos[2] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn femc_encode_decode_and_deform_roundtrip() {
        let bindings = [
            VertexTetBinding::from_barycentric(0, [1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            VertexTetBinding::from_barycentric(0, [0.1, 0.2, 0.3, 0.4], [1.0, 0.0, 0.0]),
        ];
        let header = CorrespondenceMiniHeader {
            binding_count: 2,
            tet_cell_count: 1,
            ..CorrespondenceMiniHeader::default()
        };
        let mut buf = [0u8; 16 + 2 * 16];
        let n = encode_correspondence_section(&header, &bindings, &mut buf).unwrap();
        assert_eq!(n, buf.len());

        let mut decoded = [VertexTetBinding::zeroed(); 2];
        let (dec_hdr, count) = decode_correspondence_section(&buf, &mut decoded).unwrap();
        assert_eq!(count, 2);
        assert_eq!(dec_hdr, header);
        assert_eq!(decoded, bindings);

        let tets = [[0u32, 1, 2, 3]];
        let verts = [
            [1.0, 2.0, 3.0],
            [2.0, 2.0, 3.0],
            [1.0, 4.0, 3.0],
            [1.0, 2.0, 7.0],
        ];
        let mut out_surface = [[0.0f32; 3]; 2];
        let count_def =
            deform_surface_vertices_zero_heap(&decoded, &tets, &verts, &mut out_surface).unwrap();
        assert_eq!(count_def, 2);
        assert!((out_surface[0][0] - 1.0).abs() < 1e-5);
        assert!((out_surface[0][1] - 2.0).abs() < 1e-5);
        assert!((out_surface[0][2] - 3.0).abs() < 1e-5);
    }
}
