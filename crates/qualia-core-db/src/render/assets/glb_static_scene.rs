//! Static glTF scene selection and affine instance transforms.
use serde_json::Value;
use super::AssetError;
use crate::specialized_libs::computational_geometry::{authoring::{identity, mat_mul, Mat4}, math_geometry::{quaternion_to_matrix, Quaternion}};

fn bad(s: &str) -> AssetError { AssetError::Parse(format!("GLB scene: {s}")) }
fn array<const N: usize>(v: &Value, default: [f64; N]) -> Result<[f64; N], AssetError> {
    if v.is_null() { return Ok(default); }
    let a = v.as_array().ok_or_else(|| bad("transform must be an array"))?;
    if a.len() != N { return Err(bad("invalid transform component count")); }
    let mut out = default;
    for (i, value) in a.iter().enumerate() {
        out[i] = value.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad("non-finite transform"))?;
    }
    Ok(out)
}
fn local(node: &Value) -> Result<Mat4, AssetError> {
    if !node["matrix"].is_null() {
        if ["translation", "rotation", "scale"].iter().any(|key| !node[*key].is_null()) { return Err(bad("matrix and TRS cannot be combined")); }
        let values = array::<16>(&node["matrix"], [0.0; 16])?;
        let mut m = identity();
        for col in 0..4 { for row in 0..4 { m[col][row] = values[col * 4 + row]; } }
        if m[0][3] != 0.0 || m[1][3] != 0.0 || m[2][3] != 0.0 || m[3][3] != 1.0 { return Err(bad("matrix must be affine")); }
        return Ok(m);
    }
    let t = array(&node["translation"], [0.0; 3])?;
    let s = array(&node["scale"], [1.0; 3])?;
    let q = array(&node["rotation"], [0.0, 0.0, 0.0, 1.0])?;
    let norm = q.iter().map(|x| x*x).sum::<f64>();
    if !norm.is_finite() || norm < 1e-20 { return Err(bad("invalid rotation quaternion")); }
    let r = quaternion_to_matrix(Quaternion { x:q[0], y:q[1], z:q[2], w:q[3] });
    let mut m = identity();
    for col in 0..3 { for row in 0..3 { m[col][row] = r[row][col] * s[col]; } }
    for row in 0..3 { m[3][row] = t[row]; }
    Ok(m)
}

/// Mesh indices and world matrices in selected scene traversal order. Old files without
/// nodes retain the importer's historical flat-mesh behavior.
pub(super) fn instances(gltf: &Value, meshes: usize) -> Result<Vec<(usize, Mat4)>, AssetError> {
    let Some(nodes) = gltf["nodes"].as_array() else { return Ok((0..meshes).map(|i| (i, identity())).collect()); };
    let mut parents = vec![0usize; nodes.len()];
    for node in nodes { for child in node["children"].as_array().into_iter().flatten() {
        let i = child.as_u64().and_then(|v| usize::try_from(v).ok()).filter(|&i| i < nodes.len()).ok_or_else(|| bad("invalid child index"))?;
        parents[i] += 1;
        if parents[i] > 1 { return Err(bad("node has multiple parents")); }
    }}
    let roots: Vec<usize> = if let Some(scenes) = gltf["scenes"].as_array() {
        let selected = gltf["scene"].as_u64().unwrap_or(0) as usize;
        let scene = scenes.get(selected).ok_or_else(|| bad("invalid scene index"))?;
        scene["nodes"].as_array().into_iter().flatten().map(|n| n.as_u64().and_then(|v| usize::try_from(v).ok()).filter(|&i| i < nodes.len()).ok_or_else(|| bad("invalid root index"))).collect::<Result<_,_>>()?
    } else { parents.iter().enumerate().filter_map(|(i, &p)| (p == 0).then_some(i)).collect() };
    let mut stack: Vec<_> = roots.into_iter().rev().map(|i| (i, identity())).collect();
    let mut visited = vec![false; nodes.len()];
    let mut result = Vec::new();
    while let Some((index, parent)) = stack.pop() {
        if visited[index] { return Err(bad("cycle or repeated scene node")); }
        visited[index] = true;
        let node = &nodes[index];
        let world = mat_mul(&parent, &local(node)?);
        if !world.iter().flatten().all(|v| v.is_finite()) { return Err(bad("non-finite world transform")); }
        if let Some(mesh) = node["mesh"].as_u64() {
            let mesh = usize::try_from(mesh).map_err(|_| bad("invalid mesh index"))?;
            if mesh >= meshes { return Err(bad("invalid mesh index")); }
            result.push((mesh, world));
        }
        if let Some(children) = node["children"].as_array() { for child in children.iter().rev() {
            let index = child.as_u64().unwrap() as usize;
            stack.push((index, world));
        }}
    }
    if result.is_empty() { return Err(bad("selected scene has no mesh")); }
    Ok(result)
}

/// Transform a split primitive, preserving its vertex mapping and orientation.
pub(super) fn apply(m: &Mat4, positions: &mut [[f32;3]], normals: &mut [[f32;3]], tangents: &mut [[f32;4]], indices: &mut [u32]) -> Result<(), AssetError> {
    let a = [m[0][0],m[0][1],m[0][2]]; let b = [m[1][0],m[1][1],m[1][2]]; let c = [m[2][0],m[2][1],m[2][2]];
    let cross = |u: [f64;3], v: [f64;3]| [u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];
    let cof = [cross(b,c), cross(c,a), cross(a,b)];
    let det = a[0]*cof[0][0]+a[1]*cof[0][1]+a[2]*cof[0][2];
    if !det.is_finite() || det.abs() < 1e-18 { return Err(bad("singular transform")); }
    let unit = |v: [f64;3]| -> Result<[f32;3], AssetError> { let len = (v[0]*v[0]+v[1]*v[1]+v[2]*v[2]).sqrt(); if !len.is_finite() || len < 1e-20 { return Err(bad("invalid transformed frame")); } Ok([(v[0]/len) as f32,(v[1]/len) as f32,(v[2]/len) as f32]) };
    for p in positions { let old = *p; for i in 0..3 { let v = a[i]*old[0] as f64+b[i]*old[1] as f64+c[i]*old[2] as f64+m[3][i]; if !v.is_finite() || v.abs() > f32::MAX as f64 { return Err(bad("position outside finite range")); } p[i] = v as f32; } }
    for n in normals { let old = *n; let mut v = [0.0;3]; for i in 0..3 { v[i] = (cof[0][i]*old[0] as f64+cof[1][i]*old[1] as f64+cof[2][i]*old[2] as f64)/det; } *n = unit(v)?; }
    for t in tangents { let old = *t; let mut v = [0.0;3]; for i in 0..3 { v[i] = a[i]*old[0] as f64+b[i]*old[1] as f64+c[i]*old[2] as f64; } let xyz = unit(v)?; t[0]=xyz[0]; t[1]=xyz[1]; t[2]=xyz[2]; if det < 0.0 { t[3] = -t[3]; } }
    if det < 0.0 { for tri in indices.chunks_exact_mut(3) { tri.swap(1,2); } }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn selected_hierarchy_instances_bake_reflection_and_frames() {
        let scene = json!({"scene":0,"scenes":[{"nodes":[0]}],"nodes":[
            {"translation":[3,0,0],"children":[1,2]},
            {"mesh":0,"scale":[-2,1,1]},
            {"mesh":0,"translation":[0,4,0]}
        ]});
        let instances = instances(&scene, 1).unwrap();
        assert_eq!(instances.len(), 2);
        let mut p = vec![[1.0,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,0.0]];
        let mut n = vec![[0.0,0.0,1.0];3];
        let mut t = vec![[1.0,0.0,0.0,1.0];3];
        let mut i = vec![0,1,2];
        apply(&instances[0].1, &mut p, &mut n, &mut t, &mut i).unwrap();
        assert_eq!(p[0], [1.0,0.0,0.0]);
        assert_eq!(n[0], [0.0,0.0,1.0]);
        assert_eq!(t[0], [-1.0,0.0,0.0,-1.0]);
        assert_eq!(i, [0,2,1]);
        assert_eq!(instances[1].1[3][0..3], [3.0,4.0,0.0]);
    }

    #[test]
    fn singular_scene_transform_fails_closed() {
        let scene = json!({"nodes":[{"mesh":0,"scale":[1,0,1]}]});
        let m = instances(&scene, 1).unwrap()[0].1;
        assert!(apply(&m, &mut [[0.0;3]], &mut [[0.0,0.0,1.0]], &mut [[1.0,0.0,0.0,1.0]], &mut []).is_err());
    }
}

#[cfg(test)]
mod import_tests {
    use super::super::glb::import_glb_static_scene;
    #[test]
    fn static_glb_repeats_primitive_for_two_transformed_nodes() {
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":36}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0},{"mesh":0,"translation":[2,0,0]}],"scenes":[{"nodes":[0,1]}],"scene":0}"#;
        let mut json = json.as_bytes().to_vec();
        while json.len() % 4 != 0 { json.push(b' '); }
        let mut bin = Vec::new();
        for x in [0.0f32,0.0,0.0, 1.0,0.0,0.0, 0.0,1.0,0.0] { bin.extend_from_slice(&x.to_le_bytes()); }
        let mut glb = Vec::new();
        glb.extend_from_slice(b"glTF"); glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&((12+8+json.len()+8+bin.len()) as u32).to_le_bytes());
        glb.extend_from_slice(&(json.len() as u32).to_le_bytes()); glb.extend_from_slice(b"JSON"); glb.extend_from_slice(&json);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes()); glb.extend_from_slice(b"BIN\0"); glb.extend_from_slice(&bin);
        let imported = import_glb_static_scene(&glb, 6, 2).unwrap();
        assert_eq!(imported.mesh.positions.len(), 6);
        assert_eq!(imported.mesh.triangles.len(), 2);
        assert_eq!(imported.mesh.positions[3], [2.0,0.0,0.0]);
        assert_eq!(imported.submeshes.len(), 2);
        assert!(import_glb_static_scene(&glb, 5, 2).is_err());
    }
}

#[cfg(test)]
mod accessor_preflight_tests {
    use super::super::glb::{read_indices, read_texcoords_0, read_vec3, read_vec4};
    use serde_json::json;

    #[test]
    fn declared_counts_and_buffer_view_bounds_fail_before_allocation() {
        let view = vec![json!({"buffer":0,"byteOffset":0,"byteLength":12})];
        let bin = [0u8; 16];
        let huge = json!({"bufferView":0,"componentType":5126,"type":"VEC3","count":u64::MAX});
        assert!(read_vec3(&huge, &view, &bin, "POSITION").is_err());
        let vec4 = json!({"bufferView":0,"componentType":5126,"type":"VEC4","count":1});
        assert!(read_vec4(&vec4, &view, &bin, "TANGENT").is_err());
        let uv = json!({"bufferView":0,"componentType":5126,"type":"VEC2","count":2});
        assert!(read_texcoords_0(&uv, &view, &bin).is_err());
        let indices = json!({"bufferView":0,"componentType":5125,"type":"SCALAR","count":4});
        assert!(read_indices(&indices, &view, &bin).is_err());
        let invalid_stride = vec![json!({"buffer":0,"byteLength":16,"byteStride":4})];
        let pos = json!({"bufferView":0,"componentType":5126,"type":"VEC3","count":1});
        assert!(read_vec3(&pos, &invalid_stride, &bin, "POSITION").is_err());
    }
}
