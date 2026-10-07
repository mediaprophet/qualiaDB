//! Cold-path material/submesh compilation over the canonical `.10d` geometry compiler.

use crate::container_10d::provenance_section::ProvenanceSidecar;
use crate::container_10d::{
    attach_material_section, read_material_section_into, MaterialContainerError, MaterialRecord,
    SubmeshRange,
};
use crate::q_hash;
use crate::render::assets::Mesh;
use crate::render::compile_10d::{
    compile_mesh_to_10d_with_extras_and_frames, Compile10dError, Compile10dExtras,
};
use crate::tensor::Tensor10D;

#[derive(Debug, PartialEq, Eq)]
pub enum MaterialCompileError {
    Geometry(Compile10dError),
    Materials(MaterialContainerError),
    IndexCountOverflow,
    MaterialIdentityCollision,
}

/// Scope source-local imported material IDs to a stable asset URI and remap primitive ranges.
/// This is cold-path construction; deterministic IDs keep separately compiled assets isolated.
pub fn scope_material_bindings(
    asset_uri: &str,
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
) -> Result<(Vec<MaterialRecord>, Vec<SubmeshRange>), MaterialCompileError> {
    let mut scoped = Vec::with_capacity(materials.len());
    for source in materials {
        let key = format!("urn:qualia:material:{asset_uri}:{}", source.id);
        let mut material = *source;
        material.id = q_hash(&key);
        if material.id == 0 {
            material.id = q_hash(&format!("{key}:nonzero"));
        }
        scoped.push(material);
    }
    scoped.sort_by_key(|material| material.id);
    if scoped.windows(2).any(|pair| pair[0].id == pair[1].id) {
        return Err(MaterialCompileError::MaterialIdentityCollision);
    }
    let mut remapped = Vec::with_capacity(ranges.len());
    for range in ranges {
        let source = materials
            .iter()
            .find(|material| material.id == range.material_id)
            .ok_or(MaterialCompileError::MaterialIdentityCollision)?;
        let key = format!("urn:qualia:material:{asset_uri}:{}", source.id);
        let mut scoped_id = q_hash(&key);
        if scoped_id == 0 {
            scoped_id = q_hash(&format!("{key}:nonzero"));
        }
        remapped.push(SubmeshRange {
            material_id: scoped_id,
            ..*range
        });
    }
    Ok((scoped, remapped))
}

impl From<Compile10dError> for MaterialCompileError {
    fn from(value: Compile10dError) -> Self {
        Self::Geometry(value)
    }
}
impl From<MaterialContainerError> for MaterialCompileError {
    fn from(value: MaterialContainerError) -> Self {
        Self::Materials(value)
    }
}

/// Compile geometry and its optional streams, then attach validated material and submesh data.
/// The output is a sealed `.10d` container; old static assets continue to use their implicit
/// `MaterialRecord::legacy_default()` when no MAT1 section is present.
pub fn compile_mesh_with_materials(
    mesh: &Mesh,
    nodes: &[Tensor10D],
    provenance: Option<&ProvenanceSidecar>,
    extras: Compile10dExtras,
    surface_reading: Option<&[[f32; 4]]>,
    normals: Option<&[[f32; 3]]>,
    tangents: Option<&[[f32; 4]]>,
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
) -> Result<Vec<u8>, MaterialCompileError> {
    let geometry = compile_mesh_to_10d_with_extras_and_frames(
        mesh,
        nodes,
        provenance,
        extras,
        surface_reading,
        normals,
        tangents,
    )?;
    let mesh_index_count = u32::try_from(mesh.triangle_count().saturating_mul(3))
        .map_err(|_| MaterialCompileError::IndexCountOverflow)?;
    Ok(attach_material_section(
        &geometry,
        materials,
        ranges,
        mesh_index_count,
    )?)
}

/// Convenience compile for a static mesh with material/submesh data and legacy defaults for all
/// other optional streams.
pub fn compile_mesh_to_10d_with_materials(
    mesh: &Mesh,
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
) -> Result<Vec<u8>, MaterialCompileError> {
    compile_mesh_with_materials(
        mesh,
        &[],
        None,
        Compile10dExtras::default(),
        None,
        None,
        None,
        materials,
        ranges,
    )
}

/// Read the material records and submesh ranges from a compiled mesh container.
pub fn decode_10d_materials_into(
    container: &[u8],
    mesh_index_count: u32,
    materials_out: &mut [MaterialRecord],
    ranges_out: &mut [SubmeshRange],
) -> Result<(usize, usize), MaterialCompileError> {
    Ok(read_material_section_into(
        container,
        mesh_index_count,
        materials_out,
        ranges_out,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::assets::import_asset;

    #[test]
    fn material_compile_round_trips_mesh_and_stable_submesh_bindings() {
        let mesh = import_asset(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", Some("obj")).unwrap();
        let material = MaterialRecord::legacy_default();
        let ranges = [SubmeshRange {
            first_index: 0,
            index_count: 3,
            material_id: material.id,
            semantic_id: 0xabc,
        }];
        let bytes = compile_mesh_to_10d_with_materials(&mesh, &[material], &ranges).unwrap();
        let decoded_mesh = crate::render::compile_10d::decode_10d_mesh(&bytes).unwrap();
        assert_eq!(decoded_mesh.triangle_count(), 1);

        let mut materials_out = [MaterialRecord::legacy_default(); 1];
        let mut ranges_out = [ranges[0]; 1];
        assert_eq!(
            decode_10d_materials_into(&bytes, 3, &mut materials_out, &mut ranges_out),
            Ok((1, 1))
        );
        assert_eq!(materials_out, [material]);
        assert_eq!(ranges_out, ranges);

        let mut corrupted = bytes.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 1;
        assert!(matches!(
            decode_10d_materials_into(&corrupted, 3, &mut materials_out, &mut ranges_out),
            Err(MaterialCompileError::Materials(
                MaterialContainerError::Integrity(_)
            ))
        ));
    }
}
