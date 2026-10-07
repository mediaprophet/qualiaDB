use serde_json::Value;

use super::glb_materials::{gltf_default_material, parse_gltf_materials};
use super::tangents::{
    default_tangent_for_normal, generate_import_normals, generate_mikktspace_tangents,
    normalize_imported_normal, orthogonalize_imported_tangent,
};
use super::{AssetError, ImportedMesh, Mesh, CHUNK_BIN, CHUNK_JSON, GLB_MAGIC};
use crate::container_10d::SubmeshRange;
pub(super) fn looks_like_glb(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == GLB_MAGIC
}

/// Parse a binary glTF (`.glb`): the 12-byte header, JSON chunk, and BIN chunk, then walk
/// `meshes[].primitives[]`, reading `POSITION` (FLOAT VEC3), optional `NORMAL` (FLOAT VEC3),
/// optional `TANGENT` (FLOAT VEC4), optional `TEXCOORD_0` VEC2, and optional index accessors
/// (u8/u16/u32 SCALAR) out of the BIN buffer. Triangle primitives only (mode 4); other
/// modes are skipped (a faithful first cut). Embedded/external-URI buffers are not handled here —
/// self-contained GLB binary only.
pub fn import_glb(bytes: &[u8]) -> Result<Mesh, AssetError> {
    import_glb_with_normals(bytes).map(|imported| imported.mesh)
}

/// GLB importer variant that retains NORMAL VEC3 and TANGENT VEC4 attributes, generates fallback
/// normals where needed, and uses MikkTSpace with deterministic corner splitting when UV0 is
/// available but authored tangents are absent. Authored tangent directions are orthogonalized
/// before returning GPU streams.
pub fn import_glb_with_normals(bytes: &[u8]) -> Result<ImportedMesh, AssetError> {
    if bytes.len() < 12 {
        return Err(AssetError::Parse("GLB shorter than 12-byte header".into()));
    }
    if u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) != GLB_MAGIC {
        return Err(AssetError::UnknownFormat);
    }
    // bytes[4..8] = version, bytes[8..12] = total length (not re-validated).

    let mut json: Option<&[u8]> = None;
    let mut bin: Option<&[u8]> = None;
    let mut off = 12usize;
    while off + 8 <= bytes.len() {
        let clen = u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
            as usize;
        let ctype = u32::from_le_bytes([
            bytes[off + 4],
            bytes[off + 5],
            bytes[off + 6],
            bytes[off + 7],
        ]);
        let dstart = off + 8;
        let dend = dstart
            .checked_add(clen)
            .ok_or_else(|| AssetError::Parse("GLB chunk length overflow".into()))?;
        if dend > bytes.len() {
            return Err(AssetError::Parse("GLB chunk exceeds file".into()));
        }
        match ctype {
            CHUNK_JSON => json = Some(&bytes[dstart..dend]),
            CHUNK_BIN => bin = Some(&bytes[dstart..dend]),
            _ => {}
        }
        off = dend;
    }

    let json = json.ok_or_else(|| AssetError::Parse("GLB has no JSON chunk".into()))?;
    let gltf: Value =
        serde_json::from_slice(json).map_err(|e| AssetError::Parse(format!("glTF JSON: {e}")))?;
    let bin = bin.unwrap_or(&[]);

    let empty: Vec<Value> = Vec::new();
    let accessors = gltf["accessors"].as_array().unwrap_or(&empty);
    let buffer_views = gltf["bufferViews"].as_array().unwrap_or(&empty);
    let meshes = gltf["meshes"].as_array().unwrap_or(&empty);
    let (mut materials, texture_dependencies) = parse_gltf_materials(&gltf, buffer_views, bin)?;
    let implicit_default_id = materials.len() as u64 + 1;
    let mut uses_implicit_default = false;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut saw_authored_normals = false;
    let mut tangents: Vec<[f32; 4]> = Vec::new();
    let mut saw_tangents = false;
    let mut texture_coordinates_0: Vec<[f32; 2]> = Vec::new();
    let mut saw_texture_coordinates_0 = false;
    let mut submeshes = Vec::new();

    for mesh in meshes {
        for prim in mesh["primitives"].as_array().into_iter().flatten() {
            if prim["mode"].as_u64().unwrap_or(4) != 4 {
                continue; // not TRIANGLES
            }
            let pos_idx = prim["attributes"]["POSITION"]
                .as_u64()
                .ok_or_else(|| AssetError::Parse("primitive has no POSITION".into()))?
                as usize;
            let pos_acc = accessors
                .get(pos_idx)
                .ok_or_else(|| AssetError::Parse("POSITION accessor index out of range".into()))?;
            let mut prim_pos = read_positions(pos_acc, buffer_views, bin)?;
            let vcount = prim_pos.len() as u32;
            let mut prim_indices = match prim["indices"].as_u64() {
                Some(idx_i) => {
                    let idx_acc = accessors
                        .get(idx_i as usize)
                        .ok_or_else(|| AssetError::Parse("index accessor out of range".into()))?;
                    read_indices(idx_acc, buffer_views, bin)?
                }
                None => (0..vcount).collect(),
            };
            if prim_indices.len() % 3 != 0 {
                return Err(AssetError::Parse(
                    "triangle index count must be divisible by 3".into(),
                ));
            }
            if prim_indices.iter().any(|&i| i >= vcount) {
                return Err(AssetError::Parse("primitive index out of range".into()));
            }
            let material_id = match prim["material"].as_u64() {
                Some(index) if index < materials.len() as u64 => index + 1,
                Some(_) => {
                    return Err(AssetError::Parse(
                        "primitive material index out of range".into(),
                    ));
                }
                None => {
                    uses_implicit_default = true;
                    implicit_default_id
                }
            };
            let mut prim_normals = match prim["attributes"]["NORMAL"].as_u64() {
                Some(normal_idx) => {
                    let accessor = accessors.get(normal_idx as usize).ok_or_else(|| {
                        AssetError::Parse("NORMAL accessor index out of range".into())
                    })?;
                    let mut values = read_vec3(accessor, buffer_views, bin, "NORMAL")?;
                    if values.len() != prim_pos.len() {
                        return Err(AssetError::Parse(
                            "NORMAL count must match POSITION count".into(),
                        ));
                    }
                    for normal in &mut values {
                        *normal = normalize_imported_normal(*normal)?;
                    }
                    saw_authored_normals = true;
                    values
                }
                None => generate_import_normals(&prim_pos, &prim_indices)?,
            };
            let mut prim_texcoords = match prim["attributes"]["TEXCOORD_0"].as_u64() {
                Some(texcoord_idx) => {
                    let accessor = accessors.get(texcoord_idx as usize).ok_or_else(|| {
                        AssetError::Parse("TEXCOORD_0 accessor index out of range".into())
                    })?;
                    let values = read_texcoords_0(accessor, buffer_views, bin)?;
                    if values.len() != prim_pos.len() {
                        return Err(AssetError::Parse(
                            "TEXCOORD_0 count must match POSITION count".into(),
                        ));
                    }
                    saw_texture_coordinates_0 = true;
                    values
                }
                None => vec![[0.0, 0.0]; prim_pos.len()],
            };
            let prim_tangents = match prim["attributes"]["TANGENT"].as_u64() {
                Some(tangent_idx) => {
                    let accessor = accessors.get(tangent_idx as usize).ok_or_else(|| {
                        AssetError::Parse("TANGENT accessor index out of range".into())
                    })?;
                    let values = read_vec4(accessor, buffer_views, bin, "TANGENT")?;
                    if values.len() != prim_pos.len() {
                        return Err(AssetError::Parse(
                            "TANGENT count must match POSITION count".into(),
                        ));
                    }
                    saw_tangents = true;
                    values
                        .into_iter()
                        .zip(&prim_normals)
                        .map(|(tangent, normal)| orthogonalize_imported_tangent(tangent, *normal))
                        .collect::<Result<Vec<_>, _>>()?
                }
                None => {
                    if prim["attributes"]["TEXCOORD_0"].as_u64().is_some() {
                        saw_tangents = true;
                        let split = generate_mikktspace_tangents(
                            &prim_pos,
                            &prim_normals,
                            &prim_texcoords,
                            &prim_indices,
                        )?;
                        prim_pos = split
                            .source_vertices
                            .iter()
                            .map(|&source| prim_pos[source as usize])
                            .collect();
                        prim_normals = split
                            .source_vertices
                            .iter()
                            .map(|&source| prim_normals[source as usize])
                            .collect();
                        prim_texcoords = split
                            .source_vertices
                            .iter()
                            .map(|&source| prim_texcoords[source as usize])
                            .collect();
                        prim_indices = split.indices;
                        split.tangents
                    } else {
                        prim_normals
                            .iter()
                            .copied()
                            .map(default_tangent_for_normal)
                            .collect()
                    }
                }
            };
            let base = u32::try_from(positions.len())
                .map_err(|_| AssetError::Parse("GLB mesh exceeds the u32 vertex limit".into()))?;
            let first_index = u32::try_from(triangles.len().saturating_mul(3))
                .map_err(|_| AssetError::Parse("GLB index stream exceeds the u32 limit".into()))?;
            positions.extend_from_slice(&prim_pos);
            normals.extend_from_slice(&prim_normals);
            tangents.extend_from_slice(&prim_tangents);
            texture_coordinates_0.extend_from_slice(&prim_texcoords);
            for c in prim_indices.chunks_exact(3) {
                triangles.push([base + c[0], base + c[1], base + c[2]]);
            }
            let index_count = u32::try_from(prim_indices.len()).map_err(|_| {
                AssetError::Parse("GLB primitive exceeds the u32 index limit".into())
            })?;
            if index_count != 0 {
                submeshes.push(SubmeshRange {
                    first_index,
                    index_count,
                    material_id,
                    semantic_id: 0,
                });
            }
        }
    }

    if uses_implicit_default {
        let mut default_material = gltf_default_material();
        default_material.id = implicit_default_id;
        materials.push(default_material);
    }
    let mesh = Mesh::build(positions, triangles)?;
    Ok(ImportedMesh {
        mesh,
        normals: saw_authored_normals.then_some(normals),
        tangents: saw_tangents.then_some(tangents),
        texture_coordinates_0: saw_texture_coordinates_0.then_some(texture_coordinates_0),
        materials,
        texture_dependencies,
        submeshes,
    })
}

/// Read a FLOAT VEC3 accessor (e.g. `POSITION`) out of the GLB binary buffer.
fn read_positions(
    accessor: &Value,
    bvs: &[Value],
    bin: &[u8],
) -> Result<Vec<[f32; 3]>, AssetError> {
    read_vec3(accessor, bvs, bin, "POSITION")
}

fn read_vec3(
    accessor: &Value,
    bvs: &[Value],
    bin: &[u8],
    semantic: &str,
) -> Result<Vec<[f32; 3]>, AssetError> {
    if accessor["componentType"].as_u64() != Some(5126) {
        return Err(AssetError::Parse(format!(
            "{semantic} componentType must be FLOAT (5126)"
        )));
    }
    if accessor["type"].as_str() != Some("VEC3") {
        return Err(AssetError::Parse(format!(
            "{semantic} accessor type must be VEC3"
        )));
    }
    let count = accessor["count"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.count missing".into()))?
        as usize;
    let acc_off = accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let bv_idx = accessor["bufferView"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.bufferView missing".into()))?
        as usize;
    let bv = bvs
        .get(bv_idx)
        .ok_or_else(|| AssetError::Parse("bufferView index out of range".into()))?;
    let bv_off = bv["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = match bv["byteStride"].as_u64().unwrap_or(0) {
        0 => 12, // tightly packed VEC3 f32
        s => s as usize,
    };
    let start = bv_off + acc_off;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * stride;
        if o + 12 > bin.len() {
            return Err(AssetError::Parse(format!(
                "{semantic} read past end of BIN"
            )));
        }
        let rd = |k: usize| {
            f32::from_le_bytes([bin[o + k], bin[o + k + 1], bin[o + k + 2], bin[o + k + 3]])
        };
        out.push([rd(0), rd(4), rd(8)]);
    }
    Ok(out)
}

fn read_vec4(
    accessor: &Value,
    bvs: &[Value],
    bin: &[u8],
    semantic: &str,
) -> Result<Vec<[f32; 4]>, AssetError> {
    if accessor["componentType"].as_u64() != Some(5126) {
        return Err(AssetError::Parse(format!(
            "{semantic} componentType must be FLOAT (5126)"
        )));
    }
    if accessor["type"].as_str() != Some("VEC4") {
        return Err(AssetError::Parse(format!(
            "{semantic} accessor type must be VEC4"
        )));
    }
    let count = accessor["count"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.count missing".into()))?
        as usize;
    let acc_off = accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let bv_idx = accessor["bufferView"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.bufferView missing".into()))?
        as usize;
    let bv = bvs
        .get(bv_idx)
        .ok_or_else(|| AssetError::Parse("bufferView index out of range".into()))?;
    let bv_off = bv["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = match bv["byteStride"].as_u64().unwrap_or(0) {
        0 => 16,
        s => s as usize,
    };
    let start = bv_off + acc_off;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * stride;
        if o + 16 > bin.len() {
            return Err(AssetError::Parse(format!(
                "{semantic} read past end of BIN"
            )));
        }
        let rd = |k: usize| {
            f32::from_le_bytes([bin[o + k], bin[o + k + 1], bin[o + k + 2], bin[o + k + 3]])
        };
        out.push([rd(0), rd(4), rd(8), rd(12)]);
    }
    Ok(out)
}

/// Read glTF `TEXCOORD_0` values from FLOAT or normalized unsigned-byte/unsigned-short VEC2
/// accessors. Sparse accessors are rejected until the importer applies their overlays.
pub(super) fn read_texcoords_0(
    accessor: &Value,
    bvs: &[Value],
    bin: &[u8],
) -> Result<Vec<[f32; 2]>, AssetError> {
    if accessor.get("sparse").is_some() {
        return Err(AssetError::Parse(
            "sparse TEXCOORD_0 accessors are not supported".into(),
        ));
    }
    if accessor["type"].as_str() != Some("VEC2") {
        return Err(AssetError::Parse(
            "TEXCOORD_0 accessor type must be VEC2".into(),
        ));
    }
    let component_type = accessor["componentType"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("TEXCOORD_0 componentType missing".into()))?;
    let normalized = accessor["normalized"].as_bool().unwrap_or(false);
    let (component_size, default_stride) = match component_type {
        5126 if !normalized => (4usize, 8usize),
        5121 if normalized => (1usize, 2usize),
        5123 if normalized => (2usize, 4usize),
        5121 | 5123 => {
            return Err(AssetError::Parse(
                "integer TEXCOORD_0 accessors must be normalized".into(),
            ));
        }
        _ => {
            return Err(AssetError::Parse(
                "TEXCOORD_0 must use FLOAT or normalized UNSIGNED_BYTE/UNSIGNED_SHORT".into(),
            ));
        }
    };
    let count = accessor["count"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("TEXCOORD_0 accessor.count missing".into()))?
        as usize;
    let accessor_offset = accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let view_index = accessor["bufferView"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("TEXCOORD_0 accessor.bufferView missing".into()))?
        as usize;
    let view = bvs
        .get(view_index)
        .ok_or_else(|| AssetError::Parse("TEXCOORD_0 bufferView index out of range".into()))?;
    let view_offset = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = view["byteStride"].as_u64().unwrap_or(default_stride as u64) as usize;
    let element_size = component_size * 2;
    if stride < element_size || stride % component_size != 0 {
        return Err(AssetError::Parse(
            "TEXCOORD_0 byteStride is invalid for its component type".into(),
        ));
    }
    let start = view_offset
        .checked_add(accessor_offset)
        .ok_or_else(|| AssetError::Parse("TEXCOORD_0 byte offset overflow".into()))?;
    let view_end =
        view_offset
            .checked_add(view["byteLength"].as_u64().ok_or_else(|| {
                AssetError::Parse("TEXCOORD_0 bufferView.byteLength missing".into())
            })? as usize)
            .ok_or_else(|| AssetError::Parse("TEXCOORD_0 bufferView range overflow".into()))?;
    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        let element_offset = index
            .checked_mul(stride)
            .and_then(|offset| start.checked_add(offset))
            .ok_or_else(|| AssetError::Parse("TEXCOORD_0 range overflow".into()))?;
        let end = element_offset
            .checked_add(element_size)
            .ok_or_else(|| AssetError::Parse("TEXCOORD_0 range overflow".into()))?;
        if end > view_end {
            return Err(AssetError::Parse(
                "TEXCOORD_0 accessor exceeds its bufferView".into(),
            ));
        }
        let bytes = bin
            .get(element_offset..end)
            .ok_or_else(|| AssetError::Parse("TEXCOORD_0 read past end of BIN".into()))?;
        let read_component = |component: usize| -> f32 {
            let offset = component * component_size;
            match component_type {
                5126 => f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
                5121 => bytes[offset] as f32 / u8::MAX as f32,
                5123 => {
                    u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as f32 / u16::MAX as f32
                }
                _ => unreachable!("component type validated above"),
            }
        };
        let uv = [read_component(0), read_component(1)];
        if uv.iter().any(|value| !value.is_finite()) {
            return Err(AssetError::Parse(
                "TEXCOORD_0 contains non-finite values".into(),
            ));
        }
        values.push(uv);
    }
    Ok(values)
}

/// Read a SCALAR index accessor (u8/u16/u32) out of the GLB binary buffer, widened to `u32`.
fn read_indices(accessor: &Value, bvs: &[Value], bin: &[u8]) -> Result<Vec<u32>, AssetError> {
    if accessor["type"].as_str() != Some("SCALAR") {
        return Err(AssetError::Parse(
            "index accessor type must be SCALAR".into(),
        ));
    }
    let comp = match accessor["componentType"].as_u64() {
        Some(5121) => 1usize,
        Some(5123) => 2,
        Some(5125) => 4,
        _ => {
            return Err(AssetError::Parse(
                "index componentType must be u8/u16/u32".into(),
            ));
        }
    };
    let count = accessor["count"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.count missing".into()))?
        as usize;
    let acc_off = accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let bv_idx = accessor["bufferView"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.bufferView missing".into()))?
        as usize;
    let bv = bvs
        .get(bv_idx)
        .ok_or_else(|| AssetError::Parse("bufferView index out of range".into()))?;
    let bv_off = bv["byteOffset"].as_u64().unwrap_or(0) as usize;
    let start = bv_off + acc_off;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * comp;
        if o + comp > bin.len() {
            return Err(AssetError::Parse("index read past end of BIN".into()));
        }
        let v = match comp {
            1 => bin[o] as u32,
            2 => u16::from_le_bytes([bin[o], bin[o + 1]]) as u32,
            _ => u32::from_le_bytes([bin[o], bin[o + 1], bin[o + 2], bin[o + 3]]),
        };
        out.push(v);
    }
    Ok(out)
}
