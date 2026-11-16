//! Cold, per-asset surface-frame preservation for scene assembly.
//! Existing tangent scratch cap stays per asset; merging must not discard frames
//! and then regenerate them against an artificial whole-world vertex ceiling.
use crate::{container_10d as codec, render::assets::Mesh};
pub(super) const MAX_TANGENT_VERTICES: usize = 1_048_576;
const MAX_REFRAME_BYTES: usize = 256 * 1024 * 1024;

pub(super) fn decode_asset_frames(
    bytes: &[u8],
    mesh: &Mesh,
    uv: &[[f32; 2]],
) -> Result<(Vec<[f32; 3]>, Vec<[f32; 4]>), &'static str> {
    let frames = codec::read_container_mesh_frames_with_limits(
        bytes,
        mesh.positions.len(),
        codec::MeshFrameDecodeLimits {
            max_vertices: codec::MAX_NORMAL_COUNT,
            max_payload_bytes: 64 * 1024 * 1024,
            max_decoded_bytes: 256 * 1024 * 1024,
        },
    )
    .map_err(|_| "invalid authored scene frames")?;
    let (normals, tangents) = frames.map(|f| (f.normals, f.tangents)).unwrap_or_default();
    if tangents.is_none() && mesh.positions.len() > MAX_TANGENT_VERTICES {
        return Err("per-asset tangent scratch exceeds bounded mesh budget");
    }
    let normals = match normals {
        Some(n) => n,
        None => {
            let mut n = vec![[0.; 3]; mesh.positions.len()];
            crate::render::mesh_surface_frames::vertex_normals_into(
                &mesh.positions,
                &mesh.triangles,
                &mut n,
            )
            .map_err(|_| "invalid scene geometry for normals")?;
            n
        }
    };
    let tangents = match tangents {
        Some(t) => t,
        None => generate(
            &mesh.positions,
            uv,
            &normals,
            bytemuck::cast_slice(&mesh.triangles),
        )?,
    };
    Ok((normals, tangents))
}

fn generate(
    positions: &[[f32; 3]],
    uv: &[[f32; 2]],
    normals: &[[f32; 3]],
    indices: &[u32],
) -> Result<Vec<[f32; 4]>, &'static str> {
    if positions.len() > MAX_TANGENT_VERTICES {
        return Err("per-asset tangent scratch exceeds bounded mesh budget");
    }
    let mut frames = vec![[0.; 4]; positions.len()];
    let mut scratch = vec![[0.; 6]; positions.len()];
    crate::render::mesh_tangent_frames::tangents_from_uvs_into(
        positions,
        uv,
        normals,
        indices,
        &mut frames,
        &mut scratch,
    )
    .map_err(|_| "invalid scene tangent frames")?;
    Ok(frames)
}

/// Nonuniform body fitting changes surface directions. Reframe each original
/// asset with local indices, bounded scratch and the original UV seam topology.
pub(super) fn reframe_fitted_assets(
    positions: &[[f32; 3]],
    uv: &[[f32; 2]],
    indices: &[u32],
    vertex_spans: &[(usize, usize)],
    index_spans: &[(usize, usize)],
    normals: &mut [[f32; 3]],
    tangents: &mut [[f32; 4]],
) -> Result<(), &'static str> {
    if uv.len() != positions.len()
        || normals.len() != positions.len()
        || tangents.len() != positions.len()
        || vertex_spans.len() != index_spans.len()
    {
        return Err("scene frame streams are not aligned");
    }
    for (&(begin, end), &(ib, ie)) in vertex_spans.iter().zip(index_spans) {
        if begin > end
            || end > positions.len()
            || ib > ie
            || ie > indices.len()
            || end - begin > MAX_TANGENT_VERTICES
        {
            return Err("invalid fitted asset frame span");
        }
        let bytes = (end - begin)
            .checked_mul(40)
            .and_then(|n| (ie - ib).checked_mul(4).and_then(|i| n.checked_add(i)))
            .ok_or("fitted frame budget overflow")?;
        if bytes > MAX_REFRAME_BYTES {
            return Err("fitted frame scratch budget exceeded");
        }
        if indices[ib..ie]
            .iter()
            .any(|i| (*i as usize) < begin || (*i as usize) >= end)
        {
            return Err("fitted asset index crosses vertex span");
        }
    }
    for (&(begin, end), &(ib, ie)) in vertex_spans.iter().zip(index_spans) {
        let local: Vec<u32> = indices[ib..ie].iter().map(|i| *i - begin as u32).collect();
        crate::render::mesh_surface_frames::vertex_normals_from_indices_into(
            &positions[begin..end],
            &local,
            &mut normals[begin..end],
        )
        .map_err(|_| "invalid fitted normals")?;
        tangents[begin..end].copy_from_slice(&generate(
            &positions[begin..end],
            &uv[begin..end],
            &normals[begin..end],
            &local,
        )?);
    }
    Ok(())
}
#[cfg(test)]
#[path = "scene_frames_tests.rs"]
mod tests;
