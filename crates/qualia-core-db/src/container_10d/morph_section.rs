//! `.10d` Morph Target Section (`MRP1`) — sparse and dense quantized blend-shape
//! deltas with optional `EONT1` ontogenetic/metabolic binding.
//!
//! Honors `AGENTS.md` §0 & §0-A:
//! - `encode_morph_section`, `decode_morph_section_into`, and
//!   `apply_morph_targets_zero_heap` perform **zero heap allocations**, operating
//!   exclusively on caller-provided slices.

use bytemuck::{Pod, Zeroable};

use super::section::SectionType;

/// 4-byte magic identifier for `.10d` morph target sections.
pub const MAGIC_MRP1: [u8; 4] = *b"MRP1";

/// Current version of the `MRP1` section format.
pub const MORPH_SECTION_VERSION: u16 = 1;

/// Fixed byte size of [`MorphSectionHeader`].
pub const MORPH_SECTION_HEADER_SIZE: usize = 32;

/// Fixed byte size of [`QuantizedMorphDelta`].
pub const QUANTIZED_MORPH_DELTA_SIZE: usize = 16;

/// Maximum morph targets supported per `.10d` container.
pub const MAX_MORPH_TARGETS: usize = 256;

/// Maximum quantized morph delta records per section.
pub const MAX_MORPH_DELTAS: usize = 1_048_576;

/// Section flags for `MorphSectionHeader::flags`.
pub const FLAG_MORPH_SPARSE: u32 = 1 << 0;
pub const FLAG_MORPH_DENSE: u32 = 1 << 1;
pub const FLAG_MORPH_HAS_NORMALS: u32 = 1 << 2;
pub const FLAG_MORPH_HAS_TANGENTS: u32 = 1 << 3;
pub const FLAG_MORPH_ONTOGENETIC_BIND: u32 = 1 << 4;

impl SectionType {
    /// Logical `.10d` section discriminant for morph targets (`0x0013`).
    pub const MORPH_TARGETS_DISCRIMINANT: u16 = 0x0013;

    /// Canonical `.10d` section tag for `MRP1` morph target payloads.
    #[allow(non_upper_case_globals)]
    pub const MorphTargets: Self = Self::FieldSidecar;
}

/// 32-byte `#[repr(C)]` POD header for a `.10d` `MRP1` morph target section.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MorphSectionHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub target_count: u16,
    pub vertex_count: u32,
    pub flags: u32,
    pub max_displacement: f32,
    pub ontogenetic_bind_mask: u32,
    pub reserved: [u8; 8],
}

const _: [(); MORPH_SECTION_HEADER_SIZE] = [(); core::mem::size_of::<MorphSectionHeader>()];

/// 16-byte `#[repr(C)]` POD quantized morph delta record.
///
/// Layout:
/// - `pos_delta`: `[i16; 3]` quantized to `[-32767, 32767]` relative to `max_displacement`
/// - `norm_delta`: `[i8; 3]` quantized to `[-127, 127]` in `[-2.0, 2.0]` normal space
/// - `target_index`: morph target slot (`0..target_count`)
/// - `tangent_oct_delta`: `[i8; 2]` quantized tangent delta in octahedral coordinates
/// - `vertex_index`: target vertex index (`0..vertex_count`)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct QuantizedMorphDelta {
    pub pos_delta: [i16; 3],
    pub norm_delta: [i8; 3],
    pub target_index: u8,
    pub tangent_oct_delta: [i8; 2],
    pub vertex_index: u32,
}

const _: [(); QUANTIZED_MORPH_DELTA_SIZE] = [(); core::mem::size_of::<QuantizedMorphDelta>()];

/// Errors produced when encoding, decoding, or applying a `.10d` `MRP1` morph section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MorphSectionError {
    PayloadTooShort { need: usize, have: usize },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    NonZeroReserved,
    InvalidHeaderCounts { target_count: u16, vertex_count: u32 },
    InvalidMaxDisplacement,
    TooManyDeltas { count: usize, max: usize },
    PayloadLengthMismatch { payload_len: usize },
    BufferTooSmall { need: usize, have: usize },
    DeltaTargetOutOfRange { index: usize, target_index: u8, target_count: u16 },
    DeltaVertexOutOfRange { index: usize, vertex_index: u32, vertex_count: u32 },
}

impl std::fmt::Display for MorphSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { need, have } => {
                write!(f, "MRP1 payload too short: have {have}, need {need}")
            }
            Self::BadMagic(m) => write!(f, "MRP1 bad magic: {m:?}"),
            Self::UnsupportedVersion(v) => write!(f, "MRP1 unsupported version: {v}"),
            Self::NonZeroReserved => write!(f, "MRP1 header reserved bytes must be zero"),
            Self::InvalidHeaderCounts {
                target_count,
                vertex_count,
            } => write!(
                f,
                "MRP1 invalid counts: target_count={target_count}, vertex_count={vertex_count}"
            ),
            Self::InvalidMaxDisplacement => {
                write!(f, "MRP1 max_displacement must be finite and >= 0")
            }
            Self::TooManyDeltas { count, max } => {
                write!(f, "MRP1 delta count {count} exceeds maximum {max}")
            }
            Self::PayloadLengthMismatch { payload_len } => {
                write!(f, "MRP1 payload length {payload_len} is not aligned to 16B records")
            }
            Self::BufferTooSmall { need, have } => {
                write!(f, "MRP1 output buffer too small: have {have}, need {need}")
            }
            Self::DeltaTargetOutOfRange {
                index,
                target_index,
                target_count,
            } => write!(
                f,
                "MRP1 delta[{index}] target_index {target_index} >= target_count {target_count}"
            ),
            Self::DeltaVertexOutOfRange {
                index,
                vertex_index,
                vertex_count,
            } => write!(
                f,
                "MRP1 delta[{index}] vertex_index {vertex_index} >= vertex_count {vertex_count}"
            ),
        }
    }
}

impl std::error::Error for MorphSectionError {}

/// Compute exact byte length for a `MRP1` section with `delta_count` records.
#[inline]
pub const fn morph_section_encoded_len(delta_count: usize) -> Option<usize> {
    if delta_count > MAX_MORPH_DELTAS {
        return None;
    }
    match delta_count.checked_mul(QUANTIZED_MORPH_DELTA_SIZE) {
        Some(body) => MORPH_SECTION_HEADER_SIZE.checked_add(body),
        None => None,
    }
}

/// Quantize a floating-point position/normal/tangent morph delta into a 16-byte POD record.
#[inline]
pub fn quantize_morph_delta(
    pos_delta: [f32; 3],
    norm_delta: [f32; 3],
    tangent_delta: [f32; 3],
    max_displacement: f32,
    target_index: u8,
    vertex_index: u32,
) -> QuantizedMorphDelta {
    let inv_scale = if max_displacement > 1e-8 {
        32767.0 / max_displacement
    } else {
        0.0
    };
    let q_pos = [
        (pos_delta[0] * inv_scale).round().clamp(-32767.0, 32767.0) as i16,
        (pos_delta[1] * inv_scale).round().clamp(-32767.0, 32767.0) as i16,
        (pos_delta[2] * inv_scale).round().clamp(-32767.0, 32767.0) as i16,
    ];
    let q_norm = [
        (norm_delta[0] * 63.5).round().clamp(-127.0, 127.0) as i8,
        (norm_delta[1] * 63.5).round().clamp(-127.0, 127.0) as i8,
        (norm_delta[2] * 63.5).round().clamp(-127.0, 127.0) as i8,
    ];
    let q_tan = [
        (tangent_delta[0] * 63.5).round().clamp(-127.0, 127.0) as i8,
        (tangent_delta[1] * 63.5).round().clamp(-127.0, 127.0) as i8,
    ];
    QuantizedMorphDelta {
        pos_delta: q_pos,
        norm_delta: q_norm,
        target_index,
        tangent_oct_delta: q_tan,
        vertex_index,
    }
}

/// Dequantize a 16-byte `QuantizedMorphDelta` back into `(position_delta, normal_delta)`.
#[inline]
pub fn dequantize_morph_delta(
    delta: &QuantizedMorphDelta,
    max_displacement: f32,
) -> ([f32; 3], [f32; 3]) {
    let scale = max_displacement / 32767.0;
    let pos = [
        delta.pos_delta[0] as f32 * scale,
        delta.pos_delta[1] as f32 * scale,
        delta.pos_delta[2] as f32 * scale,
    ];
    let norm = [
        delta.norm_delta[0] as f32 / 63.5,
        delta.norm_delta[1] as f32 / 63.5,
        delta.norm_delta[2] as f32 / 63.5,
    ];
    (pos, norm)
}

fn validate_header(header: &MorphSectionHeader) -> Result<(), MorphSectionError> {
    if header.magic != MAGIC_MRP1 {
        return Err(MorphSectionError::BadMagic(header.magic));
    }
    if header.version != MORPH_SECTION_VERSION {
        return Err(MorphSectionError::UnsupportedVersion(header.version));
    }
    if header.reserved != [0u8; 8] {
        return Err(MorphSectionError::NonZeroReserved);
    }
    if header.target_count == 0
        || header.target_count as usize > MAX_MORPH_TARGETS
        || header.vertex_count == 0
    {
        return Err(MorphSectionError::InvalidHeaderCounts {
            target_count: header.target_count,
            vertex_count: header.vertex_count,
        });
    }
    if !header.max_displacement.is_finite() || header.max_displacement < 0.0 {
        return Err(MorphSectionError::InvalidMaxDisplacement);
    }
    Ok(())
}

/// Encode a `MRP1` morph target section into a caller-supplied byte slice (Zero-Heap).
pub fn encode_morph_section(
    header: &MorphSectionHeader,
    deltas: &[QuantizedMorphDelta],
    out: &mut [u8],
) -> Result<usize, MorphSectionError> {
    validate_header(header)?;
    if deltas.len() > MAX_MORPH_DELTAS {
        return Err(MorphSectionError::TooManyDeltas {
            count: deltas.len(),
            max: MAX_MORPH_DELTAS,
        });
    }
    for (index, d) in deltas.iter().enumerate() {
        if d.target_index as u16 >= header.target_count {
            return Err(MorphSectionError::DeltaTargetOutOfRange {
                index,
                target_index: d.target_index,
                target_count: header.target_count,
            });
        }
        if d.vertex_index >= header.vertex_count {
            return Err(MorphSectionError::DeltaVertexOutOfRange {
                index,
                vertex_index: d.vertex_index,
                vertex_count: header.vertex_count,
            });
        }
    }

    let needed = morph_section_encoded_len(deltas.len()).ok_or(MorphSectionError::TooManyDeltas {
        count: deltas.len(),
        max: MAX_MORPH_DELTAS,
    })?;
    if out.len() < needed {
        return Err(MorphSectionError::BufferTooSmall {
            need: needed,
            have: out.len(),
        });
    }

    out[..MORPH_SECTION_HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(header));
    let mut cursor = MORPH_SECTION_HEADER_SIZE;
    for d in deltas {
        out[cursor..cursor + QUANTIZED_MORPH_DELTA_SIZE].copy_from_slice(bytemuck::bytes_of(d));
        cursor += QUANTIZED_MORPH_DELTA_SIZE;
    }
    Ok(needed)
}

/// Decode a `MRP1` morph section into caller-provided `QuantizedMorphDelta` storage (Zero-Heap).
pub fn decode_morph_section_into(
    payload: &[u8],
    out_deltas: &mut [QuantizedMorphDelta],
) -> Result<(MorphSectionHeader, usize), MorphSectionError> {
    if payload.len() < MORPH_SECTION_HEADER_SIZE {
        return Err(MorphSectionError::PayloadTooShort {
            need: MORPH_SECTION_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let body_len = payload.len() - MORPH_SECTION_HEADER_SIZE;
    if body_len % QUANTIZED_MORPH_DELTA_SIZE != 0 {
        return Err(MorphSectionError::PayloadLengthMismatch {
            payload_len: payload.len(),
        });
    }
    let delta_count = body_len / QUANTIZED_MORPH_DELTA_SIZE;
    if delta_count > MAX_MORPH_DELTAS {
        return Err(MorphSectionError::TooManyDeltas {
            count: delta_count,
            max: MAX_MORPH_DELTAS,
        });
    }

    let header: MorphSectionHeader =
        bytemuck::pod_read_unaligned(&payload[..MORPH_SECTION_HEADER_SIZE]);
    validate_header(&header)?;

    if out_deltas.len() < delta_count {
        return Err(MorphSectionError::BufferTooSmall {
            need: delta_count,
            have: out_deltas.len(),
        });
    }

    for (index, slot) in out_deltas[..delta_count].iter_mut().enumerate() {
        let start = MORPH_SECTION_HEADER_SIZE + index * QUANTIZED_MORPH_DELTA_SIZE;
        let d: QuantizedMorphDelta =
            bytemuck::pod_read_unaligned(&payload[start..start + QUANTIZED_MORPH_DELTA_SIZE]);
        if d.target_index as u16 >= header.target_count {
            return Err(MorphSectionError::DeltaTargetOutOfRange {
                index,
                target_index: d.target_index,
                target_count: header.target_count,
            });
        }
        if d.vertex_index >= header.vertex_count {
            return Err(MorphSectionError::DeltaVertexOutOfRange {
                index,
                vertex_index: d.vertex_index,
                vertex_count: header.vertex_count,
            });
        }
        *slot = d;
    }

    Ok((header, delta_count))
}

/// Apply morph target deltas directly into caller-owned vertex buffers (`HotZeroHeap`).
pub fn apply_morph_targets_zero_heap(
    header: &MorphSectionHeader,
    deltas: &[QuantizedMorphDelta],
    weights: &[f32],
    inout_positions: &mut [[f32; 3]],
    mut inout_normals: Option<&mut [[f32; 3]]>,
) -> Result<(), MorphSectionError> {
    validate_header(header)?;
    if inout_positions.len() < header.vertex_count as usize {
        return Err(MorphSectionError::BufferTooSmall {
            need: header.vertex_count as usize,
            have: inout_positions.len(),
        });
    }
    if let Some(norms) = inout_normals.as_deref() {
        if norms.len() < header.vertex_count as usize {
            return Err(MorphSectionError::BufferTooSmall {
                need: header.vertex_count as usize,
                have: norms.len(),
            });
        }
    }

    for (index, d) in deltas.iter().enumerate() {
        if d.target_index as u16 >= header.target_count {
            return Err(MorphSectionError::DeltaTargetOutOfRange {
                index,
                target_index: d.target_index,
                target_count: header.target_count,
            });
        }
        if d.vertex_index >= header.vertex_count {
            return Err(MorphSectionError::DeltaVertexOutOfRange {
                index,
                vertex_index: d.vertex_index,
                vertex_count: header.vertex_count,
            });
        }
        let weight = weights.get(d.target_index as usize).copied().unwrap_or(0.0);
        if weight.abs() <= 1e-7 {
            continue;
        }
        let (p_delta, n_delta) = dequantize_morph_delta(d, header.max_displacement);
        let vi = d.vertex_index as usize;
        inout_positions[vi][0] += p_delta[0] * weight;
        inout_positions[vi][1] += p_delta[1] * weight;
        inout_positions[vi][2] += p_delta[2] * weight;
        if let Some(norms) = inout_normals.as_deref_mut() {
            norms[vi][0] += n_delta[0] * weight;
            norms[vi][1] += n_delta[1] * weight;
            norms[vi][2] += n_delta[2] * weight;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morph_section_encode_decode_and_apply_roundtrips() {
        let header = MorphSectionHeader {
            magic: MAGIC_MRP1,
            version: MORPH_SECTION_VERSION,
            target_count: 2,
            vertex_count: 4,
            flags: FLAG_MORPH_SPARSE | FLAG_MORPH_HAS_NORMALS | FLAG_MORPH_ONTOGENETIC_BIND,
            max_displacement: 2.0,
            ontogenetic_bind_mask: 0b0011,
            reserved: [0u8; 8],
        };

        let d0 = quantize_morph_delta([1.0, -0.5, 0.25], [0.0, 1.0, 0.0], [0.0; 3], 2.0, 0, 1);
        let d1 = quantize_morph_delta([0.0, 2.0, -1.0], [0.5, 0.0, -0.5], [0.0; 3], 2.0, 1, 3);
        let deltas = [d0, d1];

        let mut buf = [0u8; 128];
        let written = encode_morph_section(&header, &deltas, &mut buf).unwrap();
        assert_eq!(written, MORPH_SECTION_HEADER_SIZE + 2 * QUANTIZED_MORPH_DELTA_SIZE);

        let mut decoded_deltas = [QuantizedMorphDelta::zeroed(); 4];
        let (decoded_hdr, count) =
            decode_morph_section_into(&buf[..written], &mut decoded_deltas).unwrap();
        assert_eq!(decoded_hdr, header);
        assert_eq!(count, 2);
        assert_eq!(&decoded_deltas[..2], &deltas);

        let mut positions = [[0.0f32; 3]; 4];
        let mut normals = [[0.0f32; 3]; 4];
        apply_morph_targets_zero_heap(
            &decoded_hdr,
            &decoded_deltas[..count],
            &[0.5, 1.0],
            &mut positions,
            Some(&mut normals),
        )
        .unwrap();

        assert!((positions[1][0] - 0.5).abs() < 1e-3);
        assert!((positions[1][1] + 0.25).abs() < 1e-3);
        assert!((positions[3][1] - 2.0).abs() < 1e-3);
        assert!((normals[1][1] - 0.5).abs() < 2e-2);
    }

    #[test]
    fn morph_section_rejects_out_of_range_indices_and_bad_magic() {
        let mut bad_hdr = MorphSectionHeader {
            magic: *b"XXXX",
            version: 1,
            target_count: 1,
            vertex_count: 2,
            flags: FLAG_MORPH_SPARSE,
            max_displacement: 1.0,
            ontogenetic_bind_mask: 0,
            reserved: [0u8; 8],
        };
        let mut buf = [0u8; 64];
        assert!(matches!(
            encode_morph_section(&bad_hdr, &[], &mut buf),
            Err(MorphSectionError::BadMagic(_))
        ));

        bad_hdr.magic = MAGIC_MRP1;
        let bad_delta = quantize_morph_delta([0.1, 0.0, 0.0], [0.0; 3], [0.0; 3], 1.0, 0, 99);
        assert!(matches!(
            encode_morph_section(&bad_hdr, &[bad_delta], &mut buf),
            Err(MorphSectionError::DeltaVertexOutOfRange { .. })
        ));
    }
}
