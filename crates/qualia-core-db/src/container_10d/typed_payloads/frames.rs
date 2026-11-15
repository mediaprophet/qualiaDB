//! Shared typed/legacy selection over the canonical bounded frame codec.
use super::{TypedPayloadError, TypedPayloadResolver};
use crate::container_10d::{
    decode_mesh_frames_into, decode_mesh_frames_with_limits, inspect_mesh_frames,
    MeshFrameDecodeLimits, MeshFrameLayout, MeshFrameStreams, NormalSectionError, TypedFieldRecord,
    MANIFOLD_IDENTITY_V2_MAX_FIELDS, NORMAL_SECTION_HEADER_SIZE,
};

/// Compatibility cold reader, preserving its original public signature and
/// MID2/legacy selection. New consumers select explicit limits or use `_into`.
pub fn read_container_mesh_frames(
    data: &[u8],
    expected_vertices: usize,
) -> Result<Option<MeshFrameStreams>, TypedPayloadError> {
    read_container_mesh_frames_with_limits(
        data,
        expected_vertices,
        MeshFrameDecodeLimits::COMPATIBILITY,
    )
}

/// Full payload preflight without allocation or output mutation.
/// With MID2 present, absent SurfaceFrames means absence; never guess a codec.
pub fn inspect_container_mesh_frames(
    data: &[u8],
    expected_vertices: usize,
    limits: MeshFrameDecodeLimits,
) -> Result<Option<MeshFrameLayout>, TypedPayloadError> {
    select(data, expected_vertices)?
        .map(|bytes| inspect_mesh_frames(bytes, limits).map_err(TypedPayloadError::Frames))
        .transpose()
}

/// All envelope/manifest/codec checks and both capacities validate before either
/// output changes. On absence neither buffer changes. No heap allocation.
pub fn read_container_mesh_frames_into(
    data: &[u8],
    expected_vertices: usize,
    limits: MeshFrameDecodeLimits,
    out_normals: &mut [[f32; 3]],
    out_tangents: &mut [[f32; 4]],
) -> Result<Option<MeshFrameLayout>, TypedPayloadError> {
    select(data, expected_vertices)?
        .map(|bytes| {
            decode_mesh_frames_into(bytes, limits, out_normals, out_tangents)
                .map_err(TypedPayloadError::Frames)
        })
        .transpose()
}

/// Cold owned output with explicit vertex, encoded-byte and decoded-byte caps.
/// Budget covers active frame streams, not all other resources in the container.
pub fn read_container_mesh_frames_with_limits(
    data: &[u8],
    expected_vertices: usize,
    limits: MeshFrameDecodeLimits,
) -> Result<Option<MeshFrameStreams>, TypedPayloadError> {
    select(data, expected_vertices)?
        .map(|bytes| {
            decode_mesh_frames_with_limits(bytes, limits).map_err(TypedPayloadError::Frames)
        })
        .transpose()
}

fn select(data: &[u8], expected_vertices: usize) -> Result<Option<&[u8]>, TypedPayloadError> {
    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let resolver = TypedPayloadResolver::new(data, &[], &mut scratch, &mut fields)?;
    let payload = if resolver.has_manifest() {
        resolver.surface_frames()?.map(|p| p.bytes)
    } else {
        resolver.legacy_frames()
    };
    let Some(payload) = payload else {
        return Ok(None);
    };
    // Retain the existing public expected-vertex error projection before decode.
    if payload.len() < NORMAL_SECTION_HEADER_SIZE {
        return Err(TypedPayloadError::Frames(
            NormalSectionError::HeaderTooShort,
        ));
    }
    if !crate::container_10d::is_normal_section(payload) {
        return Err(TypedPayloadError::Frames(NormalSectionError::BadMagic));
    }
    let declared = u32::from_le_bytes(payload[8..12].try_into().expect("header checked")) as usize;
    if declared != expected_vertices {
        return Err(TypedPayloadError::VertexCount {
            expected: expected_vertices,
            got: declared,
        });
    }
    Ok(Some(payload))
}
