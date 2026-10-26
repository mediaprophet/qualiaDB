//! Canonical allocation-free frame admission and caller-buffered decoding.
use super::{
    frame_encoded_len, oct_decode, tangent_decode, MeshFrameStreams, NormalSectionError,
    FRAME_FLAG_NORMALS, FRAME_FLAG_TANGENTS, FRAME_SECTION_MAGIC, MAX_NORMAL_COUNT,
    NORMAL_ENCODING_OCT16, NORMAL_SECTION_HEADER_SIZE, NORMAL_SECTION_MAGIC,
    NORMAL_SECTION_VERSION,
};

/// Admission limits apply equally to borrowed decoding and cold owned output.
/// Decoded bytes count only active streams (12 bytes/normal, 16 bytes/tangent).
/// They bound logical output storage, not allocator bookkeeping or the envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshFrameDecodeLimits {
    pub max_vertices: usize,
    pub max_payload_bytes: usize,
    pub max_decoded_bytes: usize,
}

impl MeshFrameDecodeLimits {
    /// Compatibility cold policy admits every historically valid frame count.
    /// New callers should select their own limits before crossing the SDK.
    pub const COMPATIBILITY: Self = Self {
        max_vertices: MAX_NORMAL_COUNT,
        max_payload_bytes: NORMAL_SECTION_HEADER_SIZE + MAX_NORMAL_COUNT * 12,
        max_decoded_bytes: MAX_NORMAL_COUNT * 28,
    };
}

/// Fully validated active stream layout; no borrowed typed cast is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshFrameLayout {
    pub count: usize,
    pub has_normals: bool,
    pub has_tangents: bool,
    pub decoded_bytes: usize,
}

/// Validate the entire payload, including every late tangent, without mutation
/// or allocation. Unaligned bytes and LE headers share the native/WASM path.
pub fn inspect_mesh_frames(
    bytes: &[u8],
    limits: MeshFrameDecodeLimits,
) -> Result<MeshFrameLayout, NormalSectionError> {
    if bytes.len() > limits.max_payload_bytes {
        return Err(NormalSectionError::PayloadTooLarge {
            needed: bytes.len(),
            max: limits.max_payload_bytes,
        });
    }
    if bytes.len() < NORMAL_SECTION_HEADER_SIZE {
        return Err(NormalSectionError::HeaderTooShort);
    }
    let magic = &bytes[..4];
    if magic != NORMAL_SECTION_MAGIC && magic != FRAME_SECTION_MAGIC {
        return Err(NormalSectionError::BadMagic);
    }
    let version = u16_at(bytes, 4);
    if version != NORMAL_SECTION_VERSION {
        return Err(NormalSectionError::UnsupportedVersion(version));
    }
    let flags = u16_at(bytes, 6);
    let (has_normals, has_tangents) = if magic == NORMAL_SECTION_MAGIC {
        if flags != NORMAL_ENCODING_OCT16 {
            return Err(NormalSectionError::UnsupportedEncoding(flags));
        }
        (true, false)
    } else {
        if flags & !(FRAME_FLAG_NORMALS | FRAME_FLAG_TANGENTS) != 0 {
            return Err(NormalSectionError::UnknownFlags(flags));
        }
        if flags == 0 {
            return Err(NormalSectionError::EmptyFrame);
        }
        (
            flags & FRAME_FLAG_NORMALS != 0,
            flags & FRAME_FLAG_TANGENTS != 0,
        )
    };
    if bytes[12..16] != [0; 4] {
        return Err(NormalSectionError::NonZeroReserved);
    }
    let declared = u32::from_le_bytes(bytes[8..12].try_into().expect("header checked"));
    let count = declared as usize;
    let max = MAX_NORMAL_COUNT.min(limits.max_vertices);
    if count > max {
        return Err(NormalSectionError::CountTooLarge {
            count: declared,
            max,
        });
    }
    // MAX_NORMAL_COUNT makes these products safe even on wasm32.
    let expected = frame_encoded_len(count, has_normals, has_tangents).ok_or(
        NormalSectionError::CountTooLarge {
            count: declared,
            max,
        },
    )?;
    if bytes.len() != expected {
        return Err(NormalSectionError::PayloadLength {
            expected,
            got: bytes.len(),
        });
    }
    let decoded_bytes = count * (usize::from(has_normals) * 12 + usize::from(has_tangents) * 16);
    if decoded_bytes > limits.max_decoded_bytes {
        return Err(NormalSectionError::DecodedTooLarge {
            needed: decoded_bytes,
            max: limits.max_decoded_bytes,
        });
    }
    let layout = MeshFrameLayout {
        count,
        has_normals,
        has_tangents,
        decoded_bytes,
    };
    if has_tangents {
        let stride = usize::from(has_normals) * 4 + 8;
        let start = NORMAL_SECTION_HEADER_SIZE + usize::from(has_normals) * 4;
        for index in 0..count {
            if tangent_decode(tangent_at(bytes, start + index * stride)).is_none() {
                return Err(NormalSectionError::InvalidTangent { index });
            }
        }
    }
    Ok(layout)
}

/// Decode all present streams into caller storage. Absent streams and buffer
/// tails remain untouched. A present stream requires at least `layout.count`
/// slots, even if the caller only plans to consume the other stream. Every
/// validation and capacity error leaves BOTH outputs unchanged.
pub fn decode_mesh_frames_into(
    bytes: &[u8],
    limits: MeshFrameDecodeLimits,
    out_normals: &mut [[f32; 3]],
    out_tangents: &mut [[f32; 4]],
) -> Result<MeshFrameLayout, NormalSectionError> {
    let layout = inspect_mesh_frames(bytes, limits)?;
    check_capacity(layout.has_normals, layout.count, out_normals.len())?;
    check_capacity(layout.has_tangents, layout.count, out_tangents.len())?;
    write_validated(bytes, layout, out_normals, out_tangents);
    Ok(layout)
}

/// Cold convenience API. Full admission precedes any allocation; requested
/// vector capacities fit the explicit decoded-byte budget. Allocation refusal
/// is an error, not an unchecked `with_capacity` panic. The hot API is `_into`.
pub fn decode_mesh_frames_with_limits(
    bytes: &[u8],
    limits: MeshFrameDecodeLimits,
) -> Result<MeshFrameStreams, NormalSectionError> {
    let layout = inspect_mesh_frames(bytes, limits)?;
    let mut normals = allocate_stream::<3>(layout.has_normals, layout.count)?;
    let mut tangents = allocate_stream::<4>(layout.has_tangents, layout.count)?;
    write_validated(
        bytes,
        layout,
        normals.as_deref_mut().unwrap_or(&mut []),
        tangents.as_deref_mut().unwrap_or(&mut []),
    );
    Ok(MeshFrameStreams { normals, tangents })
}

fn allocate_stream<const N: usize>(
    present: bool,
    count: usize,
) -> Result<Option<Vec<[f32; N]>>, NormalSectionError> {
    if !present {
        return Ok(None);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| NormalSectionError::AllocationFailed)?;
    out.resize(count, [0.0; N]);
    Ok(Some(out))
}

fn check_capacity(present: bool, needed: usize, got: usize) -> Result<(), NormalSectionError> {
    if present && got < needed {
        return Err(NormalSectionError::OutputTooSmall { needed, got });
    }
    Ok(())
}

fn write_validated(
    bytes: &[u8],
    layout: MeshFrameLayout,
    out_normals: &mut [[f32; 3]],
    out_tangents: &mut [[f32; 4]],
) {
    let mut offset = NORMAL_SECTION_HEADER_SIZE;
    for index in 0..layout.count {
        if layout.has_normals {
            out_normals[index] = oct_decode([i16_at(bytes, offset), i16_at(bytes, offset + 2)]);
            offset += 4;
        }
        if layout.has_tangents {
            out_tangents[index] = tangent_decode(tangent_at(bytes, offset))
                .expect("immutable payload fully preflighted");
            offset += 8;
        }
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn i16_at(bytes: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn tangent_at(bytes: &[u8], offset: usize) -> [i16; 4] {
    [
        i16_at(bytes, offset),
        i16_at(bytes, offset + 2),
        i16_at(bytes, offset + 4),
        i16_at(bytes, offset + 6),
    ]
}
