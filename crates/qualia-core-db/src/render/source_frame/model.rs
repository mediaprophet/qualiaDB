//! Cold integrity/count admission using the canonical .10d mesh owners only.
use super::{SourceFrameBindLimits, SourceFrameError};
use crate::{container_10d, render::assets::Mesh};
use sha2::{Digest, Sha256};

pub fn decode_bound_model(
    bytes: &[u8],
    limits: SourceFrameBindLimits,
) -> Result<(Mesh, [u8; 32]), SourceFrameError> {
    if bytes.is_empty() || bytes.len() > limits.max_model_bytes {
        return Err(SourceFrameError::Budget);
    }
    let header =
        container_10d::Container10dHeader::parse(bytes).map_err(|_| SourceFrameError::Model)?;
    // Include borrowed input + owned CRC copy + descriptor vector before parsing.
    let base = bytes
        .len()
        .checked_mul(2)
        .and_then(|n| {
            (header.section_count as usize)
                .checked_mul(core::mem::size_of::<container_10d::SectionDescriptor>())
                .and_then(|d| n.checked_add(d))
        })
        .ok_or(SourceFrameError::Budget)?;
    if base > limits.max_cold_bytes {
        return Err(SourceFrameError::Budget);
    }
    let mut checked = bytes.to_vec();
    container_10d::verify_whole_file_crc32c(&mut checked)
        .map_err(|_| SourceFrameError::Integrity)?;
    let sections = container_10d::parse_section_table(&checked, &header)
        .map_err(|_| SourceFrameError::Integrity)?;
    let mut meshes = sections
        .iter()
        .filter(|s| s.typ() == Some(container_10d::SectionType::QuantizedMesh));
    let section = meshes.next().ok_or(SourceFrameError::Model)?;
    if meshes.next().is_some() {
        return Err(SourceFrameError::Model);
    }
    let start = usize::try_from(section.byte_offset).map_err(|_| SourceFrameError::Budget)?;
    let len = usize::try_from(section.byte_length).map_err(|_| SourceFrameError::Budget)?;
    let end = start.checked_add(len).ok_or(SourceFrameError::Budget)?;
    let payload = checked.get(start..end).ok_or(SourceFrameError::Model)?;
    let (mini, exact_len) = container_10d::mesh_section::parse_mesh_header(payload)
        .map_err(|_| SourceFrameError::Model)?;
    let vertices = mini.vertex_count as usize;
    let indices = (mini.triangle_count as usize)
        .checked_mul(3)
        .ok_or(SourceFrameError::Budget)?;
    if exact_len != payload.len()
        || vertices == 0
        || indices == 0
        || vertices > limits.max_vertices
        || indices > limits.max_indices
    {
        return Err(SourceFrameError::Budget);
    }
    let cold = vertices
        .checked_mul(12)
        .and_then(|n| indices.checked_mul(4).and_then(|i| n.checked_add(i)))
        .and_then(|n| base.checked_add(n))
        .ok_or(SourceFrameError::Budget)?;
    if cold > limits.max_cold_bytes {
        return Err(SourceFrameError::Budget);
    }
    let mesh = container_10d::decode_mesh_section(payload).map_err(|_| SourceFrameError::Model)?;
    if mesh.positions.iter().flatten().any(|v| !v.is_finite())
        || mesh
            .triangles
            .iter()
            .flatten()
            .any(|&i| i as usize >= vertices)
    {
        return Err(SourceFrameError::Model);
    }
    Ok((mesh, Sha256::digest(bytes).into()))
}

/// Bit-identical authored geometry and topology, not equal counts/nearest weld.
/// Materials, texture rights and source authority remain the admitted host's job.
pub fn validate_resident_mesh(
    mesh: &Mesh,
    positions: &[[f32; 3]],
    indices: &[u32],
) -> Result<(), SourceFrameError> {
    if mesh.positions.len() != positions.len()
        || mesh.triangles.len().checked_mul(3) != Some(indices.len())
        || !mesh
            .positions
            .iter()
            .flatten()
            .zip(positions.iter().flatten())
            .all(|(a, b)| a.to_bits() == b.to_bits())
        || !mesh
            .triangles
            .iter()
            .flatten()
            .copied()
            .eq(indices.iter().copied())
    {
        return Err(SourceFrameError::ResidentMismatch);
    }
    Ok(())
}
