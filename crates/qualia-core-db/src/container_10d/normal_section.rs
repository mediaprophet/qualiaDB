//! Compact, optional vertex-normal field sidecar for compiled mesh assets.
//!
//! The existing QuantizedMesh payload remains byte-for-byte stable. This field sidecar stores
//! unit normals as two signed normalized 16-bit octahedral coordinates (4 bytes per vertex).

use bytemuck::{bytes_of, Pod, Zeroable};

mod frame_decode;
pub use frame_decode::{
    decode_mesh_frames_into, decode_mesh_frames_with_limits, inspect_mesh_frames,
    MeshFrameDecodeLimits, MeshFrameLayout,
};

pub const NORMAL_SECTION_HEADER_SIZE: usize = 16;
pub const NORMAL_SECTION_VERSION: u16 = 1;
pub const NORMAL_ENCODING_OCT16: u16 = 1;
pub const MAX_NORMAL_COUNT: usize = 4_194_304;
const NORMAL_SECTION_MAGIC: [u8; 4] = *b"NRM1";
const FRAME_SECTION_MAGIC: [u8; 4] = *b"FRM1";
const FRAME_FLAG_NORMALS: u16 = 1 << 0;
const FRAME_FLAG_TANGENTS: u16 = 1 << 1;

/// Whether a FieldSidecar payload uses the per-vertex normal codec.
#[inline]
pub fn is_normal_section(bytes: &[u8]) -> bool {
    bytes.starts_with(&NORMAL_SECTION_MAGIC) || bytes.starts_with(&FRAME_SECTION_MAGIC)
}

/// Normal and tangent data decoded from one vertex-aligned FieldSidecar.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshFrameStreams {
    pub normals: Option<Vec<[f32; 3]>>,
    pub tangents: Option<Vec<[f32; 4]>>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
struct NormalSectionHeader {
    magic: [u8; 4],
    version: u16,
    encoding: u16,
    count: u32,
    reserved: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalSectionError {
    HeaderTooShort,
    BadMagic,
    UnsupportedVersion(u16),
    UnsupportedEncoding(u16),
    NonZeroReserved,
    CountTooLarge { count: u32, max: usize },
    PayloadLength { expected: usize, got: usize },
    OutputTooSmall { needed: usize, got: usize },
    InvalidNormal { index: usize },
    InvalidTangent { index: usize },
    StreamCountMismatch { expected: usize, got: usize },
    UnknownFlags(u16),
    EmptyFrame,
    PayloadTooLarge { needed: usize, max: usize },
    DecodedTooLarge { needed: usize, max: usize },
    AllocationFailed,
}

impl std::fmt::Display for NormalSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "10d normal field: {self:?}")
    }
}

impl std::error::Error for NormalSectionError {}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
struct FrameSectionHeader {
    magic: [u8; 4],
    version: u16,
    flags: u16,
    count: u32,
    reserved: u32,
}

/// Byte size of a normals-only (NRM1) section for `count` vertices
/// (header + 4 bytes per vertex); `None` on overflow.
#[inline]
pub const fn encoded_len(count: usize) -> Option<usize> {
    match count.checked_mul(4) {
        Some(body) => NORMAL_SECTION_HEADER_SIZE.checked_add(body),
        None => None,
    }
}

/// Byte size of an interleaved FRM1 frame section for `count` vertices,
/// with 4-byte normals and/or 8-byte tangents per vertex. `None` when
/// neither stream is present or on overflow.
pub const fn frame_encoded_len(
    count: usize,
    has_normals: bool,
    has_tangents: bool,
) -> Option<usize> {
    if !has_normals && !has_tangents {
        return None;
    }
    let stride = (if has_normals { 4usize } else { 0 }) + (if has_tangents { 8usize } else { 0 });
    match count.checked_mul(stride) {
        Some(body) => NORMAL_SECTION_HEADER_SIZE.checked_add(body),
        None => None,
    }
}

/// Encode normal and tangent streams in an interleaved, vertex-aligned FRM1 sidecar. Normals-only
/// data retains the existing NRM1 representation for byte-stable containers.
pub fn encode_mesh_frames(
    normals: Option<&[[f32; 3]]>,
    tangents: Option<&[[f32; 4]]>,
    out: &mut [u8],
) -> Result<usize, NormalSectionError> {
    let count = match (normals, tangents) {
        (None, None) => return Err(NormalSectionError::EmptyFrame),
        (Some(values), None) => return encode_normals(values, out),
        (None, Some(values)) => values.len(),
        (Some(values), Some(tangents)) => {
            if values.len() != tangents.len() {
                return Err(NormalSectionError::StreamCountMismatch {
                    expected: values.len(),
                    got: tangents.len(),
                });
            }
            values.len()
        }
    };
    if count > MAX_NORMAL_COUNT || count > u32::MAX as usize {
        return Err(NormalSectionError::CountTooLarge {
            count: count.min(u32::MAX as usize) as u32,
            max: MAX_NORMAL_COUNT,
        });
    }
    let has_normals = normals.is_some();
    let has_tangents = tangents.is_some();
    let needed = frame_encoded_len(count, has_normals, has_tangents)
        .ok_or(NormalSectionError::EmptyFrame)?;
    if out.len() < needed {
        return Err(NormalSectionError::OutputTooSmall {
            needed,
            got: out.len(),
        });
    }
    let header = FrameSectionHeader {
        magic: FRAME_SECTION_MAGIC,
        version: NORMAL_SECTION_VERSION,
        flags: (if has_normals { FRAME_FLAG_NORMALS } else { 0 })
            | (if has_tangents { FRAME_FLAG_TANGENTS } else { 0 }),
        count: count as u32,
        reserved: 0,
    };
    out[..NORMAL_SECTION_HEADER_SIZE].copy_from_slice(bytes_of(&header));
    let mut offset = NORMAL_SECTION_HEADER_SIZE;
    for index in 0..count {
        if let Some(values) = normals {
            let packed =
                oct_encode(values[index]).ok_or(NormalSectionError::InvalidNormal { index })?;
            out[offset..offset + 2].copy_from_slice(&packed[0].to_le_bytes());
            out[offset + 2..offset + 4].copy_from_slice(&packed[1].to_le_bytes());
            offset += 4;
        }
        if let Some(values) = tangents {
            let packed = tangent_encode(values[index])
                .ok_or(NormalSectionError::InvalidTangent { index })?;
            for component in packed {
                out[offset..offset + 2].copy_from_slice(&component.to_le_bytes());
                offset += 2;
            }
        }
    }
    Ok(offset)
}

/// Decode either the legacy NRM1 stream or the combined FRM1 normal/tangent stream.
pub fn decode_mesh_frames(bytes: &[u8]) -> Result<MeshFrameStreams, NormalSectionError> {
    decode_mesh_frames_with_limits(bytes, MeshFrameDecodeLimits::COMPATIBILITY)
}
/// Encode normalized vertex normals into a caller-owned payload buffer.
pub fn encode_normals(normals: &[[f32; 3]], out: &mut [u8]) -> Result<usize, NormalSectionError> {
    if normals.len() > MAX_NORMAL_COUNT || normals.len() > u32::MAX as usize {
        return Err(NormalSectionError::CountTooLarge {
            count: normals.len().min(u32::MAX as usize) as u32,
            max: MAX_NORMAL_COUNT,
        });
    }
    let needed = encoded_len(normals.len()).ok_or(NormalSectionError::CountTooLarge {
        count: u32::MAX,
        max: MAX_NORMAL_COUNT,
    })?;
    if out.len() < needed {
        return Err(NormalSectionError::OutputTooSmall {
            needed,
            got: out.len(),
        });
    }
    let header = NormalSectionHeader {
        magic: NORMAL_SECTION_MAGIC,
        version: NORMAL_SECTION_VERSION,
        encoding: NORMAL_ENCODING_OCT16,
        count: normals.len() as u32,
        reserved: 0,
    };
    out[..NORMAL_SECTION_HEADER_SIZE].copy_from_slice(bytes_of(&header));
    let mut offset = NORMAL_SECTION_HEADER_SIZE;
    for (index, normal) in normals.iter().enumerate() {
        let encoded = oct_encode(*normal).ok_or(NormalSectionError::InvalidNormal { index })?;
        out[offset..offset + 2].copy_from_slice(&encoded[0].to_le_bytes());
        out[offset + 2..offset + 4].copy_from_slice(&encoded[1].to_le_bytes());
        offset += 4;
    }
    Ok(offset)
}

/// Decode vertex normals from a normal-only or combined mesh-frame FieldSidecar.
pub fn decode_normals(bytes: &[u8]) -> Result<Vec<[f32; 3]>, NormalSectionError> {
    decode_mesh_frames(bytes)?
        .normals
        .ok_or(NormalSectionError::EmptyFrame)
}

/// Decode vertex tangents and handedness from a combined mesh-frame FieldSidecar.
pub fn decode_tangents(bytes: &[u8]) -> Result<Vec<[f32; 4]>, NormalSectionError> {
    decode_mesh_frames(bytes)?
        .tangents
        .ok_or(NormalSectionError::EmptyFrame)
}

fn oct_encode(normal: [f32; 3]) -> Option<[i16; 2]> {
    if normal.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        return None;
    }
    let mut x = normal[0] / length;
    let mut y = normal[1] / length;
    let z = normal[2] / length;
    let l1 = x.abs() + y.abs() + z.abs();
    x /= l1;
    y /= l1;
    let mut ox = x;
    let mut oy = y;
    if z < 0.0 {
        ox = (1.0 - y.abs()) * sign_not_zero(x);
        oy = (1.0 - x.abs()) * sign_not_zero(y);
    }
    Some([
        (ox.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16,
        (oy.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16,
    ])
}

fn oct_decode(encoded: [i16; 2]) -> [f32; 3] {
    let mut x = (encoded[0] as f32 / i16::MAX as f32).max(-1.0);
    let mut y = (encoded[1] as f32 / i16::MAX as f32).max(-1.0);
    let mut z = 1.0 - x.abs() - y.abs();
    if z < 0.0 {
        let old_x = x;
        x = (1.0 - y.abs()) * sign_not_zero(old_x);
        y = (1.0 - old_x.abs()) * sign_not_zero(y);
    }
    let length = (x * x + y * y + z * z).sqrt();
    if length > f32::EPSILON {
        x /= length;
        y /= length;
        z /= length;
    }
    [x, y, z]
}

fn tangent_encode(tangent: [f32; 4]) -> Option<[i16; 4]> {
    if tangent.iter().any(|value| !value.is_finite()) || (tangent[3].abs() - 1.0).abs() > 1e-3 {
        return None;
    }
    let length =
        (tangent[0] * tangent[0] + tangent[1] * tangent[1] + tangent[2] * tangent[2]).sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        return None;
    }
    Some([
        ((tangent[0] / length).clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16,
        ((tangent[1] / length).clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16,
        ((tangent[2] / length).clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16,
        if tangent[3] < 0.0 {
            -i16::MAX
        } else {
            i16::MAX
        },
    ])
}

fn tangent_decode(encoded: [i16; 4]) -> Option<[f32; 4]> {
    if encoded[3] != i16::MAX && encoded[3] != -i16::MAX {
        return None;
    }
    let mut tangent = [
        (encoded[0] as f32 / i16::MAX as f32).max(-1.0),
        (encoded[1] as f32 / i16::MAX as f32).max(-1.0),
        (encoded[2] as f32 / i16::MAX as f32).max(-1.0),
    ];
    let length =
        (tangent[0] * tangent[0] + tangent[1] * tangent[1] + tangent[2] * tangent[2]).sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        return None;
    }
    for component in &mut tangent {
        *component /= length;
    }
    Some([
        tangent[0],
        tangent[1],
        tangent[2],
        if encoded[3] < 0 { -1.0 } else { 1.0 },
    ])
}

#[inline]
fn sign_not_zero(value: f32) -> f32 {
    if value < 0.0 {
        -1.0
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oct16_round_trip_stays_within_angular_tolerance() {
        let input = [[0.0, 0.0, 1.0], [0.0, 0.0, -1.0], [1.0, 2.0, 3.0]];
        let mut bytes = vec![0; encoded_len(input.len()).unwrap()];
        let written = encode_normals(&input, &mut bytes).unwrap();
        let output = decode_normals(&bytes[..written]).unwrap();
        assert_eq!(output.len(), input.len());
        for (a, b) in input.iter().zip(output) {
            let length = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
            let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / length;
            assert!(dot > 0.9999, "angular dot product {dot}");
        }
    }

    #[test]
    fn encoder_rejects_zero_or_non_finite_normals() {
        for normal in [[0.0, 0.0, 0.0], [f32::NAN, 0.0, 1.0]] {
            let mut bytes = vec![0; encoded_len(1).unwrap()];
            assert!(matches!(
                encode_normals(&[normal], &mut bytes),
                Err(NormalSectionError::InvalidNormal { index: 0 })
            ));
        }
    }

    #[test]
    fn decoder_rejects_trailing_bytes() {
        let mut bytes = vec![0; encoded_len(1).unwrap()];
        encode_normals(&[[0.0, 0.0, 1.0]], &mut bytes).unwrap();
        bytes.push(0);
        assert!(matches!(
            decode_normals(&bytes),
            Err(NormalSectionError::PayloadLength { .. })
        ));
    }

    #[test]
    fn field_kind_detection_leaves_other_sidecar_families_available() {
        assert!(!is_normal_section(b"WAVE"));
        assert!(is_normal_section(b"NRM1"));
        assert!(is_normal_section(b"FRM1"));
    }

    #[test]
    fn frm1_round_trips_normal_tangent_and_mirrored_handedness() {
        let normals = [[0.0, 0.0, 1.0], [0.0, 0.0, -1.0]];
        let tangents = [[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, -1.0]];
        let mut bytes = vec![0; frame_encoded_len(2, true, true).unwrap()];
        let written = encode_mesh_frames(Some(&normals), Some(&tangents), &mut bytes).unwrap();
        let decoded = decode_mesh_frames(&bytes[..written]).unwrap();
        assert_eq!(decoded.normals.unwrap(), normals);
        assert_eq!(decoded.tangents.unwrap(), tangents);
    }

    #[test]
    fn frm1_rejects_tangent_count_mismatch_and_bad_handedness() {
        let mut bytes = vec![0; frame_encoded_len(2, true, true).unwrap()];
        assert!(matches!(
            encode_mesh_frames(
                Some(&[[0.0, 0.0, 1.0]]),
                Some(&[[1.0, 0.0, 0.0, 1.0]; 2]),
                &mut bytes
            ),
            Err(NormalSectionError::StreamCountMismatch { .. })
        ));
        assert!(matches!(
            encode_mesh_frames(None, Some(&[[1.0, 0.0, 0.0, 0.0]]), &mut bytes),
            Err(NormalSectionError::InvalidTangent { index: 0 })
        ));
    }

    #[test]
    fn frm1_rejects_corrupt_serialized_handedness_and_zero_direction() {
        let mut bytes = vec![0; frame_encoded_len(1, false, true).unwrap()];
        encode_mesh_frames(None, Some(&[[1.0, 0.0, 0.0, 1.0]]), &mut bytes).unwrap();
        bytes[NORMAL_SECTION_HEADER_SIZE + 6..NORMAL_SECTION_HEADER_SIZE + 8]
            .copy_from_slice(&0i16.to_le_bytes());
        assert!(matches!(
            decode_mesh_frames(&bytes),
            Err(NormalSectionError::InvalidTangent { index: 0 })
        ));

        let mut bytes = vec![0; frame_encoded_len(1, false, true).unwrap()];
        encode_mesh_frames(None, Some(&[[1.0, 0.0, 0.0, 1.0]]), &mut bytes).unwrap();
        bytes[NORMAL_SECTION_HEADER_SIZE..NORMAL_SECTION_HEADER_SIZE + 6].fill(0);
        assert!(matches!(
            decode_mesh_frames(&bytes),
            Err(NormalSectionError::InvalidTangent { index: 0 })
        ));
    }
}
