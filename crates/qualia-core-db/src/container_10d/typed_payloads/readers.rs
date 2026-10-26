//! Caller-buffered morph codec adapter over typed selection.
use super::{TypedPayloadError, TypedPayloadResolver};
use crate::container_10d::{
    MANIFOLD_IDENTITY_V2_MAX_FIELDS, MORPH_SECTION_HEADER_SIZE,
    MorphSectionHeader, QuantizedMorphDelta, TypedFieldRecord, decode_morph_section_into,
};
/// Decode a MID2 MorphTargets field, or a recognized whole MRP1 sidecar when
/// MID2 is absent. With MID2 present, never guess from an unrelated field.
/// MRP1 validates every record before publishing any caller output.
pub fn read_container_morph_targets_into(
    data: &[u8],
    expected_vertices: usize,
    out_deltas: &mut [QuantizedMorphDelta],
) -> Result<Option<(MorphSectionHeader, usize)>, TypedPayloadError> {
    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let resolver = TypedPayloadResolver::new(data, &[], &mut scratch, &mut fields)?;
    let payload = if resolver.has_manifest() {
        resolver.morph_targets()?.map(|payload| payload.bytes)
    } else {
        resolver.legacy_morph_targets()
    };
    let Some(payload) = payload else {
        return Ok(None);
    };
    if payload.len() >= MORPH_SECTION_HEADER_SIZE {
        let header: MorphSectionHeader =
            bytemuck::pod_read_unaligned(&payload[..MORPH_SECTION_HEADER_SIZE]);
        if header.vertex_count as usize != expected_vertices {
            return Err(TypedPayloadError::VertexCount {
                expected: expected_vertices,
                got: header.vertex_count as usize,
            });
        }
    }
    decode_morph_section_into(payload, out_deltas)
        .map(Some)
        .map_err(TypedPayloadError::Morph)
}
