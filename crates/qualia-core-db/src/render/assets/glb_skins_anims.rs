//! Full glTF 2.0 / GLB → `.10d` Living Container Compiler (`SKL1`, `MRP1`, `PROC1`, `EONT1`,
//! KHR Material Extensions & `KHR_lights_punctual`).

use serde_json::Value;

use super::glb::{import_glb_with_normals, read_vec3, read_vec4};
use super::{AssetError, ImportedMesh, CHUNK_BIN, CHUNK_JSON, GLB_MAGIC};
use crate::container_10d::morph_section::{
    encode_morph_section, morph_section_encoded_len, quantize_morph_delta, MorphSectionHeader,
    QuantizedMorphDelta, FLAG_MORPH_HAS_NORMALS, FLAG_MORPH_HAS_TANGENTS, FLAG_MORPH_ONTOGENETIC_BIND,
    FLAG_MORPH_SPARSE, MAGIC_MRP1, MORPH_SECTION_VERSION,
};
use crate::container_10d::procedural_synthesis::{
    ProceduralSynthesisHeader, FLAG_ALLOMETRIC_GROWTH, FLAG_CELLULAR_VORONOI,
    FLAG_PGA_MOTOR_DYNAMICS, MAGIC_PROC, PROCEDURAL_SYNTHESIS_SECTION_VERSION,
};
use crate::container_10d::skeletal_section::{
    encode_skeletal_section_into, ClipRecord, Counts, SkeletalData,
};
use crate::container_10d::{EntityOntologyMiniHeader, MAGIC_EONT};
use crate::q_hash;
use crate::render::skeletal::{Joint, Key, Motor, SkinWeight, Track};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlbInterpolation { Step, Linear, CubicSpline }

#[derive(Debug, Clone, PartialEq)]
pub struct GlbNodeRecord {
    pub name: Option<String>, pub parent: Option<u16>, pub children: Vec<u16>,
    pub local_motor: Motor, pub scale: [f32; 3], pub mesh_index: Option<u32>,
    pub skin_index: Option<u32>, pub light_index: Option<u32>, pub weights: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KhrMaterialExtensions {
    pub clearcoat_factor: f32, pub clearcoat_roughness: f32, pub sheen_color: [f32; 3], pub sheen_roughness: f32,
    pub transmission_factor: f32, pub volume_thickness: f32, pub volume_attenuation_distance: f32,
    pub volume_attenuation_color: [f32; 3], pub ior: f32, pub anisotropy_strength: f32,
    pub anisotropy_rotation: f32, pub emissive_strength: f32,
}

impl Default for KhrMaterialExtensions {
    fn default() -> Self {
        Self {
            clearcoat_factor: 0.0, clearcoat_roughness: 0.0, sheen_color: [0.0; 3], sheen_roughness: 0.0,
            transmission_factor: 0.0, volume_thickness: 0.0, volume_attenuation_distance: f32::INFINITY,
            volume_attenuation_color: [1.0, 1.0, 1.0], ior: 1.5, anisotropy_strength: 0.0, anisotropy_rotation: 0.0, emissive_strength: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KhrPunctualLight {
    pub name: Option<String>, pub kind: String, pub color: [f32; 3], pub intensity: f32,
    pub range: Option<f32>, pub inner_cone_angle: f32, pub outer_cone_angle: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlbMorphWeightTrack {
    pub node_index: u16, pub interpolation: GlbInterpolation, pub times: Vec<f32>, pub values: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct GlbLivingScene {
    pub imported_mesh: ImportedMesh, pub scene_roots: Vec<u16>, pub nodes: Vec<GlbNodeRecord>,
    pub joints: Vec<Joint>, pub joint_ids: Vec<u64>, pub skin_weights: Vec<SkinWeight>,
    pub clips: Vec<ClipRecord>, pub tracks: Vec<Track>, pub keys: Vec<Key>, pub skl1_section: Option<Vec<u8>>,
    pub morph_header: Option<MorphSectionHeader>, pub morph_deltas: Vec<QuantizedMorphDelta>,
    pub mrp1_section: Option<Vec<u8>>, pub morph_weight_tracks: Vec<GlbMorphWeightTrack>,
    pub material_extensions: Vec<KhrMaterialExtensions>, pub punctual_lights: Vec<KhrPunctualLight>,
    pub proc1_header: ProceduralSynthesisHeader, pub eont1_header: EntityOntologyMiniHeader,
}

pub type MotorAndScale = (Motor, [f32; 3]);

/// Decompose a column-major 4x4 affine matrix into `(Motor, scale: [f32; 3])`.
pub fn matrix4_to_motor_and_scale(m: &[f32; 16]) -> MotorAndScale {
    let (tx, ty, tz) = (m[12] as f64, m[13] as f64, m[14] as f64);
    let sx = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt().max(1e-6);
    let sy = (m[4] * m[4] + m[5] * m[5] + m[6] * m[6]).sqrt().max(1e-6);
    let sz = (m[8] * m[8] + m[9] * m[9] + m[10] * m[10]).sqrt().max(1e-6);
    let (r00, r10, r20) = ((m[0] / sx) as f64, (m[1] / sx) as f64, (m[2] / sx) as f64);
    let (r01, r11, r21) = ((m[4] / sy) as f64, (m[5] / sy) as f64, (m[6] / sy) as f64);
    let (r02, r12, r22) = ((m[8] / sz) as f64, (m[9] / sz) as f64, (m[10] / sz) as f64);
    let trace = r00 + r11 + r22;
    let (qw, qx, qy, qz) = if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        (0.25 * s, (r21 - r12) / s, (r02 - r20) / s, (r10 - r01) / s)
    } else if r00 > r11 && r00 > r22 {
        let s = (1.0 + r00 - r11 - r22).sqrt() * 2.0;
        ((r21 - r12) / s, 0.25 * s, (r01 + r10) / s, (r02 + r20) / s)
    } else if r11 > r22 {
        let s = (1.0 + r11 - r00 - r22).sqrt() * 2.0;
        ((r02 - r20) / s, (r01 + r10) / s, 0.25 * s, (r12 + r21) / s)
    } else {
        let s = (1.0 + r22 - r00 - r11).sqrt() * 2.0;
        ((r10 - r01) / s, (r02 + r20) / s, (r12 + r21) / s, 0.25 * s)
    };
    let len = (qw * qw + qx * qx + qy * qy + qz * qz).sqrt().max(1e-12);
    (Motor::from_rotation_translation([qw / len, qx / len, qy / len, qz / len], [tx, ty, tz]), [sx, sy, sz])
}

/// Parse a binary glTF (`.glb`) into a full `.10d` Living Scene (`SKL1`, `MRP1`, `PROC1`, `EONT1`,
/// KHR material extensions, and `KHR_lights_punctual`).
pub fn parse_glb_living_scene(bytes: &[u8]) -> Result<GlbLivingScene, AssetError> {
    let imported_mesh = import_glb_with_normals(bytes)?;
    if bytes.len() < 12 || u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) != GLB_MAGIC {
        return Err(AssetError::UnknownFormat);
    }
    let (mut json_chunk, mut bin_chunk, mut off) = (None, None, 12usize);
    while off + 8 <= bytes.len() {
        let clen = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) as usize;
        let ctype = u32::from_le_bytes(bytes[off + 4..off + 8].try_into().unwrap());
        let dend = off + 8 + clen;
        if ctype == CHUNK_JSON { json_chunk = Some(&bytes[off + 8..dend]); }
        else if ctype == CHUNK_BIN { bin_chunk = Some(&bytes[off + 8..dend]); }
        off = dend;
    }
    let gltf: Value = serde_json::from_slice(json_chunk.ok_or_else(|| AssetError::Parse("no JSON".into()))?)
        .map_err(|e| AssetError::Parse(format!("glTF JSON: {e}")))?;
    let (bin, empty) = (bin_chunk.unwrap_or(&[]), Vec::new());
    let (accessors, bvs) = (gltf["accessors"].as_array().unwrap_or(&empty), gltf["bufferViews"].as_array().unwrap_or(&empty));

    let (nodes, scene_roots) = parse_nodes_and_scenes(&gltf)?;
    let mut skin_weights = parse_primitive_skin_weights(&gltf, accessors, bvs, bin, imported_mesh.mesh.vertex_count())?;
    let (joints, joint_ids, node_to_joint) = parse_skins(&gltf, &nodes, accessors, bvs, bin, &mut skin_weights)?;
    let (clips, tracks, keys, morph_weight_tracks) =
        parse_animations(&gltf, &nodes, &joints, &node_to_joint, accessors, bvs, bin)?;

    let skl1_section = if !joints.is_empty() && !skin_weights.is_empty() {
        let (c_slice, t_slice, k_slice) = if clips.is_empty() {
            (
                vec![ClipRecord { id: q_hash("urn:qualia:clip:bind"), duration: 1.0, first_track: 0, track_count: 1, looping: true }],
                vec![Track { joint: 0, first_key: 0, key_count: 1 }],
                vec![Key { time: 0.0, local: joints[0].rest_local }],
            )
        } else {
            (clips.clone(), tracks.clone(), keys.clone())
        };
        let counts = Counts { joints: joints.len(), vertices: skin_weights.len(), clips: c_slice.len(), tracks: t_slice.len(), keys: k_slice.len() };
        counts.encoded_len().and_then(|len| {
            let mut buf = vec![0u8; len];
            let data = SkeletalData { joints: &joints, joint_ids: &joint_ids, skin: &skin_weights, clips: &c_slice, tracks: &t_slice, keys: &k_slice };
            encode_skeletal_section_into(&data, &mut buf).ok().map(|_| buf)
        })
    } else { None };

    let (morph_header, morph_deltas, mrp1_section) =
        parse_morph_targets(&gltf, accessors, bvs, bin, imported_mesh.mesh.vertex_count())?;
    let material_extensions = parse_khr_material_extensions(&gltf);
    let punctual_lights = parse_khr_lights_punctual(&gltf);
    let (proc1_header, eont1_header) = build_living_headers(&gltf, &imported_mesh);

    Ok(GlbLivingScene {
        imported_mesh, scene_roots, nodes, joints, joint_ids, skin_weights, clips, tracks, keys, skl1_section,
        morph_header, morph_deltas, mrp1_section, morph_weight_tracks, material_extensions, punctual_lights, proc1_header, eont1_header,
    })
}

fn parse_nodes_and_scenes(gltf: &Value) -> Result<(Vec<GlbNodeRecord>, Vec<u16>), AssetError> {
    let empty = Vec::new();
    let raw_nodes = gltf["nodes"].as_array().unwrap_or(&empty);
    let mut nodes = Vec::with_capacity(raw_nodes.len());
    for n in raw_nodes {
        let (local_motor, scale) = if let Some(mat_arr) = n["matrix"].as_array() {
            if mat_arr.len() != 16 { return Err(AssetError::Parse("node.matrix must have 16 floats".into())); }
            let mut m = [0.0f32; 16];
            for (i, v) in mat_arr.iter().enumerate() { m[i] = v.as_f64().unwrap_or(0.0) as f32; }
            matrix4_to_motor_and_scale(&m)
        } else {
            let t = n["translation"].as_array().map(|a| [
                a.first().and_then(|v| v.as_f64()).unwrap_or(0.0),
                a.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0),
                a.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0),
            ]).unwrap_or([0.0; 3]);
            let r = n["rotation"].as_array().map(|a| {
                let (x, y) = (a.first().and_then(|v| v.as_f64()).unwrap_or(0.0), a.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0));
                let (z, w) = (a.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0), a.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0));
                let l = (w * w + x * x + y * y + z * z).sqrt().max(1e-12);
                [w / l, x / l, y / l, z / l]
            }).unwrap_or([1.0, 0.0, 0.0, 0.0]);
            let s = n["scale"].as_array().map(|a| [
                a.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                a.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                a.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
            ]).unwrap_or([1.0; 3]);
            (Motor::from_rotation_translation(r, t), s)
        };
        let children = n["children"].as_array().into_iter().flatten().filter_map(|c| c.as_u64().map(|v| v as u16)).collect();
        let weights = n["weights"].as_array().into_iter().flatten().filter_map(|w| w.as_f64().map(|v| v as f32)).collect();
        let light_index = n["extensions"]["KHR_lights_punctual"]["light"].as_u64().map(|v| v as u32);
        nodes.push(GlbNodeRecord {
            name: n["name"].as_str().map(String::from), parent: None, children, local_motor, scale,
            mesh_index: n["mesh"].as_u64().map(|v| v as u32), skin_index: n["skin"].as_u64().map(|v| v as u32), light_index, weights,
        });
    }
    for parent_idx in 0..nodes.len() {
        let ch = nodes[parent_idx].children.clone();
        for c in ch { if let Some(child_node) = nodes.get_mut(c as usize) { child_node.parent = Some(parent_idx as u16); } }
    }
    let scene_idx = gltf["scene"].as_u64().unwrap_or(0) as usize;
    let scene_roots = gltf["scenes"].as_array().and_then(|s| s.get(scene_idx)).and_then(|s| s["nodes"].as_array())
        .into_iter().flatten().filter_map(|v| v.as_u64().map(|u| u as u16)).collect();
    Ok((nodes, scene_roots))
}

fn parse_primitive_skin_weights(
    gltf: &Value, accessors: &[Value], bvs: &[Value], bin: &[u8], total_vertices: usize,
) -> Result<Vec<SkinWeight>, AssetError> {
    let empty = Vec::new();
    let mut out = Vec::new();
    for mesh in gltf["meshes"].as_array().unwrap_or(&empty) {
        for prim in mesh["primitives"].as_array().into_iter().flatten() {
            if prim["mode"].as_u64().unwrap_or(4) != 4 { continue; }
            let (Some(j_idx), Some(w_idx)) = (prim["attributes"]["JOINTS_0"].as_u64(), prim["attributes"]["WEIGHTS_0"].as_u64()) else { continue };
            let j_acc = accessors.get(j_idx as usize).ok_or_else(|| AssetError::Parse("JOINTS_0 OOB".into()))?;
            let w_acc = accessors.get(w_idx as usize).ok_or_else(|| AssetError::Parse("WEIGHTS_0 OOB".into()))?;
            let (joints, weights) = (read_joints_0(j_acc, bvs, bin)?, read_weights_0(w_acc, bvs, bin)?);
            if joints.len() != weights.len() { return Err(AssetError::Parse("JOINTS_0/WEIGHTS_0 count mismatch".into())); }
            for (j, w) in joints.into_iter().zip(weights) {
                let sum: f32 = w.iter().map(|v| v.max(0.0)).sum();
                let norm_w = if sum > 1e-6 { [w[0].max(0.0) / sum, w[1].max(0.0) / sum, w[2].max(0.0) / sum, w[3].max(0.0) / sum] } else { [1.0, 0.0, 0.0, 0.0] };
                out.push(SkinWeight { joints: j, weights: norm_w });
            }
        }
    }
    if !out.is_empty() && out.len() < total_vertices {
        out.resize(total_vertices, SkinWeight { joints: [0; 4], weights: [1.0, 0.0, 0.0, 0.0] });
    }
    Ok(out)
}

fn read_joints_0(acc: &Value, bvs: &[Value], bin: &[u8]) -> Result<Vec<[u16; 4]>, AssetError> {
    if acc["type"].as_str() != Some("VEC4") { return Err(AssetError::Parse("JOINTS_0 VEC4".into())); }
    let (comp, count) = (acc["componentType"].as_u64().unwrap_or(0), acc["count"].as_u64().ok_or_else(|| AssetError::Parse("JOINTS_0 count".into()))? as usize);
    let bv = bvs.get(acc["bufferView"].as_u64().unwrap_or(0) as usize).ok_or_else(|| AssetError::Parse("JOINTS_0 bv".into()))?;
    let start = bv["byteOffset"].as_u64().unwrap_or(0) as usize + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let elem_bytes = match comp { 5121 => 4usize, 5123 => 8usize, _ => return Err(AssetError::Parse("JOINTS_0 comp".into())) };
    let stride = bv["byteStride"].as_u64().map(|s| s as usize).unwrap_or(elem_bytes);
    (0..count).map(|i| {
        let s = bin.get(start + i * stride..start + i * stride + elem_bytes).ok_or_else(|| AssetError::Parse("JOINTS_0 OOB".into()))?;
        Ok(if comp == 5121 { [s[0] as u16, s[1] as u16, s[2] as u16, s[3] as u16] } else {
            [u16::from_le_bytes([s[0], s[1]]), u16::from_le_bytes([s[2], s[3]]), u16::from_le_bytes([s[4], s[5]]), u16::from_le_bytes([s[6], s[7]])]
        })
    }).collect()
}

fn read_weights_0(acc: &Value, bvs: &[Value], bin: &[u8]) -> Result<Vec<[f32; 4]>, AssetError> {
    if acc["type"].as_str() != Some("VEC4") { return Err(AssetError::Parse("WEIGHTS_0 VEC4".into())); }
    let (comp, count) = (acc["componentType"].as_u64().unwrap_or(0), acc["count"].as_u64().ok_or_else(|| AssetError::Parse("WEIGHTS_0 count".into()))? as usize);
    let bv = bvs.get(acc["bufferView"].as_u64().unwrap_or(0) as usize).ok_or_else(|| AssetError::Parse("WEIGHTS_0 bv".into()))?;
    let start = bv["byteOffset"].as_u64().unwrap_or(0) as usize + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let elem_bytes = match comp { 5121 => 4usize, 5123 => 8usize, 5126 => 16usize, _ => return Err(AssetError::Parse("WEIGHTS_0 comp".into())) };
    let stride = bv["byteStride"].as_u64().map(|s| s as usize).unwrap_or(elem_bytes);
    (0..count).map(|i| {
        let s = bin.get(start + i * stride..start + i * stride + elem_bytes).ok_or_else(|| AssetError::Parse("WEIGHTS_0 OOB".into()))?;
        Ok(match comp {
            5121 => [s[0] as f32 / 255.0, s[1] as f32 / 255.0, s[2] as f32 / 255.0, s[3] as f32 / 255.0],
            5123 => [u16::from_le_bytes([s[0], s[1]]) as f32 / 65535.0, u16::from_le_bytes([s[2], s[3]]) as f32 / 65535.0, u16::from_le_bytes([s[4], s[5]]) as f32 / 65535.0, u16::from_le_bytes([s[6], s[7]]) as f32 / 65535.0],
            _ => [f32::from_le_bytes(s[0..4].try_into().unwrap()), f32::from_le_bytes(s[4..8].try_into().unwrap()), f32::from_le_bytes(s[8..12].try_into().unwrap()), f32::from_le_bytes(s[12..16].try_into().unwrap())],
        })
    }).collect()
}

fn parse_skins(
    gltf: &Value, nodes: &[GlbNodeRecord], accessors: &[Value], bvs: &[Value], bin: &[u8], _skin_weights: &mut [SkinWeight],
) -> Result<(Vec<Joint>, Vec<u64>, std::collections::HashMap<usize, u16>), AssetError> {
    let Some(skin) = gltf["skins"].as_array().and_then(|s| s.first()) else {
        return Ok((Vec::new(), Vec::new(), std::collections::HashMap::new()));
    };
    let joint_nodes: Vec<usize> = skin["joints"].as_array().ok_or_else(|| AssetError::Parse("skin.joints".into()))?
        .iter().filter_map(|v| v.as_u64().map(|u| u as usize)).collect();
    let ibm_motors = if let Some(ibm_idx) = skin["inverseBindMatrices"].as_u64() {
        read_mat4_motors(accessors.get(ibm_idx as usize).ok_or_else(|| AssetError::Parse("IBM OOB".into()))?, bvs, bin)?
    } else { vec![Motor::identity(); joint_nodes.len()] };
    let node_to_joint: std::collections::HashMap<usize, u16> = joint_nodes.iter().enumerate().map(|(ji, &ni)| (ni, ji as u16)).collect();
    let (mut joints, mut joint_ids) = (Vec::with_capacity(joint_nodes.len()), Vec::with_capacity(joint_nodes.len()));
    for (j_idx, &n_idx) in joint_nodes.iter().enumerate() {
        let node = nodes.get(n_idx).ok_or_else(|| AssetError::Parse("skin joint node OOB".into()))?;
        let parent = node.parent.and_then(|p| node_to_joint.get(&(p as usize)).copied()).filter(|&pj| (pj as usize) < j_idx);
        let name_str = node.name.as_deref().unwrap_or("joint");
        joints.push(Joint { parent, rest_local: node.local_motor, inverse_bind: ibm_motors.get(j_idx).copied().unwrap_or_else(Motor::identity) });
        joint_ids.push(q_hash(&format!("urn:qualia:joint:{j_idx}:{name_str}")));
    }
    Ok((joints, joint_ids, node_to_joint))
}

fn read_mat4_motors(acc: &Value, bvs: &[Value], bin: &[u8]) -> Result<Vec<Motor>, AssetError> {
    if acc["componentType"].as_u64() != Some(5126) || acc["type"].as_str() != Some("MAT4") { return Err(AssetError::Parse("IBM MAT4".into())); }
    let (count, bv) = (acc["count"].as_u64().unwrap_or(0) as usize, bvs.get(acc["bufferView"].as_u64().unwrap_or(0) as usize).ok_or_else(|| AssetError::Parse("IBM bv".into()))?);
    let (start, stride) = (bv["byteOffset"].as_u64().unwrap_or(0) as usize + acc["byteOffset"].as_u64().unwrap_or(0) as usize, bv["byteStride"].as_u64().map(|s| s as usize).unwrap_or(64));
    (0..count).map(|i| {
        let slice = bin.get(start + i * stride..start + i * stride + 64).ok_or_else(|| AssetError::Parse("IBM OOB".into()))?;
        let mut m = [0.0f32; 16];
        for k in 0..16 { m[k] = f32::from_le_bytes(slice[k * 4..k * 4 + 4].try_into().unwrap()); }
        Ok(matrix4_to_motor_and_scale(&m).0)
    }).collect()
}

fn read_f32_scalars(acc: &Value, bvs: &[Value], bin: &[u8]) -> Result<Vec<f32>, AssetError> {
    let (count, bv) = (acc["count"].as_u64().unwrap_or(0) as usize, bvs.get(acc["bufferView"].as_u64().unwrap_or(0) as usize).ok_or_else(|| AssetError::Parse("scalar bv".into()))?);
    let (start, stride) = (bv["byteOffset"].as_u64().unwrap_or(0) as usize + acc["byteOffset"].as_u64().unwrap_or(0) as usize, bv["byteStride"].as_u64().map(|s| s as usize).unwrap_or(4));
    (0..count).map(|i| {
        let s = bin.get(start + i * stride..start + i * stride + 4).ok_or_else(|| AssetError::Parse("scalar OOB".into()))?;
        Ok(f32::from_le_bytes(s.try_into().unwrap()))
    }).collect()
}

fn parse_animations(
    gltf: &Value, nodes: &[GlbNodeRecord], joints: &[Joint], node_to_joint: &std::collections::HashMap<usize, u16>,
    accessors: &[Value], bvs: &[Value], bin: &[u8],
) -> Result<(Vec<ClipRecord>, Vec<Track>, Vec<Key>, Vec<GlbMorphWeightTrack>), AssetError> {
    let empty = Vec::new();
    let (mut clips, mut tracks, mut keys, mut morph_tracks) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (anim_idx, anim) in gltf["animations"].as_array().unwrap_or(&empty).iter().enumerate() {
        let (samplers, first_track, mut max_time) = (anim["samplers"].as_array().unwrap_or(&empty), tracks.len() as u32, 0.0f64);
        let mut joint_channels: std::collections::BTreeMap<u16, (Vec<f32>, Vec<[f64; 3]>, Vec<[f64; 4]>, GlbInterpolation)> = std::collections::BTreeMap::new();
        for ch in anim["channels"].as_array().unwrap_or(&empty) {
            let (Some(s_idx), Some(n_idx)) = (ch["sampler"].as_u64(), ch["target"]["node"].as_u64()) else { continue };
            let (path, sampler) = (ch["target"]["path"].as_str().unwrap_or(""), samplers.get(s_idx as usize).ok_or_else(|| AssetError::Parse("sampler OOB".into()))?);
            let interp = match sampler["interpolation"].as_str().unwrap_or("LINEAR") {
                "STEP" => GlbInterpolation::Step, "CUBICSPLINE" => GlbInterpolation::CubicSpline, _ => GlbInterpolation::Linear,
            };
            let in_acc = accessors.get(sampler["input"].as_u64().unwrap_or(0) as usize).ok_or_else(|| AssetError::Parse("in OOB".into()))?;
            let out_acc = accessors.get(sampler["output"].as_u64().unwrap_or(0) as usize).ok_or_else(|| AssetError::Parse("out OOB".into()))?;
            let times = read_f32_scalars(in_acc, bvs, bin)?;
            for &t in &times { max_time = max_time.max(t as f64); }
            if path == "weights" {
                morph_tracks.push(GlbMorphWeightTrack { node_index: n_idx as u16, interpolation: interp, times, values: read_f32_scalars(out_acc, bvs, bin)? });
                continue;
            }
            let Some(&joint_idx) = node_to_joint.get(&(n_idx as usize)) else { continue };
            let rest = joints.get(joint_idx as usize).map(|j| j.rest_local).unwrap_or_else(|| nodes.get(n_idx as usize).map(|n| n.local_motor).unwrap_or_else(Motor::identity));
            let (rest_rot, rest_trans) = rest.to_rotation_translation();
            let entry = joint_channels.entry(joint_idx).or_insert_with(|| (times.clone(), Vec::new(), Vec::new(), interp));
            if entry.0.is_empty() { entry.0 = times.clone(); }
            entry.3 = interp;
            let (stride, offset) = if interp == GlbInterpolation::CubicSpline { (3, 1) } else { (1, 0) };
            if path == "translation" {
                let raw = read_vec3(out_acc, bvs, bin, "translation")?;
                entry.1 = (0..times.len()).filter_map(|i| raw.get(i * stride + offset)).map(|v| [v[0] as f64, v[1] as f64, v[2] as f64]).collect();
            } else if path == "rotation" {
                let raw = read_vec4(out_acc, bvs, bin, "rotation")?;
                entry.2 = (0..times.len()).filter_map(|i| raw.get(i * stride + offset)).map(|q| {
                    let (x, y, z, w) = (q[0] as f64, q[1] as f64, q[2] as f64, q[3] as f64);
                    let l = (w * w + x * x + y * y + z * z).sqrt().max(1e-12);
                    [w / l, x / l, y / l, z / l]
                }).collect();
            }
            if entry.1.is_empty() { entry.1 = vec![rest_trans; entry.0.len()]; }
            if entry.2.is_empty() { entry.2 = vec![rest_rot; entry.0.len()]; }
        }
        for (joint_idx, (times, trans, rots, _)) in joint_channels {
            if times.is_empty() { continue; }
            let (first_key, mut prev_t, mut key_count) = (keys.len() as u32, -1.0f64, 0u16);
            for i in 0..times.len() {
                let mut t = (times[i] as f64).max(0.0);
                if t <= prev_t { t = prev_t + 1e-4; }
                prev_t = t;
                max_time = max_time.max(t);
                let tr = trans.get(i).copied().or_else(|| trans.last().copied()).unwrap_or([0.0; 3]);
                let ro = rots.get(i).copied().or_else(|| rots.last().copied()).unwrap_or([1.0, 0.0, 0.0, 0.0]);
                keys.push(Key { time: t, local: Motor::from_rotation_translation(ro, tr) });
                key_count += 1;
            }
            if key_count > 0 { tracks.push(Track { joint: joint_idx, first_key, key_count }); }
        }
        let track_count = (tracks.len() as u32).saturating_sub(first_track);
        if track_count > 0 {
            let name = anim["name"].as_str().unwrap_or("clip");
            clips.push(ClipRecord { id: q_hash(&format!("urn:qualia:clip:{anim_idx}:{name}")), duration: max_time.max(1e-3), first_track, track_count, looping: true });
        }
    }
    Ok((clips, tracks, keys, morph_tracks))
}

fn parse_morph_targets(
    gltf: &Value, accessors: &[Value], bvs: &[Value], bin: &[u8], total_vertices: usize,
) -> Result<(Option<MorphSectionHeader>, Vec<QuantizedMorphDelta>, Option<Vec<u8>>), AssetError> {
    let empty = Vec::new();
    let mut raw_deltas: Vec<(u8, u32, [f32; 3], [f32; 3], [f32; 3])> = Vec::new();
    let (mut max_disp, mut target_count, mut has_normals, mut has_tangents, mut v_base) = (0.0f32, 0u16, false, false, 0u32);
    for mesh in gltf["meshes"].as_array().unwrap_or(&empty) {
        for prim in mesh["primitives"].as_array().into_iter().flatten() {
            if prim["mode"].as_u64().unwrap_or(4) != 4 { continue; }
            let pos_idx = prim["attributes"]["POSITION"].as_u64().unwrap_or(0) as usize;
            let prim_vcount = accessors.get(pos_idx).and_then(|a| a["count"].as_u64()).unwrap_or(0) as u32;
            if let Some(targets) = prim["targets"].as_array() {
                target_count = target_count.max(targets.len() as u16);
                for (t_idx, target) in targets.iter().enumerate() {
                    let Some(p_acc_i) = target["POSITION"].as_u64() else { continue };
                    let p_deltas = read_vec3(accessors.get(p_acc_i as usize).ok_or_else(|| AssetError::Parse("morph POS".into()))?, bvs, bin, "morph.POSITION")?;
                    let n_deltas = if let Some(n_i) = target["NORMAL"].as_u64() {
                        has_normals = true;
                        read_vec3(accessors.get(n_i as usize).ok_or_else(|| AssetError::Parse("morph NRM".into()))?, bvs, bin, "morph.NORMAL")?
                    } else { vec![[0.0; 3]; p_deltas.len()] };
                    let t_deltas = if let Some(t_i) = target["TANGENT"].as_u64() {
                        has_tangents = true;
                        read_vec3(accessors.get(t_i as usize).ok_or_else(|| AssetError::Parse("morph TAN".into()))?, bvs, bin, "morph.TANGENT")?
                    } else { vec![[0.0; 3]; p_deltas.len()] };
                    for (vi, &pd) in p_deltas.iter().enumerate() {
                        let (nd, td) = (n_deltas.get(vi).copied().unwrap_or([0.0; 3]), t_deltas.get(vi).copied().unwrap_or([0.0; 3]));
                        let mag = pd[0].abs().max(pd[1].abs()).max(pd[2].abs());
                        if mag > 1e-6 || nd != [0.0; 3] {
                            max_disp = max_disp.max(mag);
                            raw_deltas.push((t_idx as u8, (v_base + vi as u32).min(total_vertices.saturating_sub(1) as u32), pd, nd, td));
                        }
                    }
                }
            }
            v_base = v_base.saturating_add(prim_vcount);
        }
    }
    if target_count == 0 || raw_deltas.is_empty() { return Ok((None, Vec::new(), None)); }
    let max_displacement = max_disp.max(1e-4);
    let flags = FLAG_MORPH_SPARSE | FLAG_MORPH_ONTOGENETIC_BIND
        | if has_normals { FLAG_MORPH_HAS_NORMALS } else { 0 }
        | if has_tangents { FLAG_MORPH_HAS_TANGENTS } else { 0 };
    let header = MorphSectionHeader {
        magic: MAGIC_MRP1, version: MORPH_SECTION_VERSION, target_count, vertex_count: total_vertices as u32,
        flags, max_displacement, ontogenetic_bind_mask: (1u32 << target_count.min(16)) - 1, reserved: [0u8; 8],
    };
    let deltas: Vec<QuantizedMorphDelta> = raw_deltas.into_iter().map(|(ti, vi, pd, nd, td)| quantize_morph_delta(pd, nd, td, max_displacement, ti, vi)).collect();
    let mrp1_bytes = morph_section_encoded_len(deltas.len()).and_then(|len| {
        let mut buf = vec![0u8; len];
        encode_morph_section(&header, &deltas, &mut buf).ok().map(|_| buf)
    });
    Ok((Some(header), deltas, mrp1_bytes))
}

fn parse_khr_material_extensions(gltf: &Value) -> Vec<KhrMaterialExtensions> {
    let empty = Vec::new();
    gltf["materials"].as_array().unwrap_or(&empty).iter().map(|m| {
        let (ext, mut khr) = (&m["extensions"], KhrMaterialExtensions::default());
        if let Some(cc) = ext.get("KHR_materials_clearcoat") {
            khr.clearcoat_factor = cc["clearcoatFactor"].as_f64().unwrap_or(0.0) as f32;
            khr.clearcoat_roughness = cc["clearcoatRoughnessFactor"].as_f64().unwrap_or(0.0) as f32;
        }
        if let Some(sh) = ext.get("KHR_materials_sheen") {
            if let Some(c) = sh["sheenColorFactor"].as_array().filter(|c| c.len() == 3) {
                khr.sheen_color = [c[0].as_f64().unwrap_or(0.0) as f32, c[1].as_f64().unwrap_or(0.0) as f32, c[2].as_f64().unwrap_or(0.0) as f32];
            }
            khr.sheen_roughness = sh["sheenRoughnessFactor"].as_f64().unwrap_or(0.0) as f32;
        }
        if let Some(tr) = ext.get("KHR_materials_transmission") { khr.transmission_factor = tr["transmissionFactor"].as_f64().unwrap_or(0.0) as f32; }
        if let Some(vol) = ext.get("KHR_materials_volume") {
            khr.volume_thickness = vol["thicknessFactor"].as_f64().unwrap_or(0.0) as f32;
            khr.volume_attenuation_distance = vol["attenuationDistance"].as_f64().map(|v| v as f32).unwrap_or(f32::INFINITY);
            if let Some(c) = vol["attenuationColor"].as_array().filter(|c| c.len() == 3) {
                khr.volume_attenuation_color = [c[0].as_f64().unwrap_or(1.0) as f32, c[1].as_f64().unwrap_or(1.0) as f32, c[2].as_f64().unwrap_or(1.0) as f32];
            }
        }
        if let Some(ior) = ext.get("KHR_materials_ior") { khr.ior = ior["ior"].as_f64().unwrap_or(1.5) as f32; }
        if let Some(aniso) = ext.get("KHR_materials_anisotropy") {
            khr.anisotropy_strength = aniso["anisotropyStrength"].as_f64().unwrap_or(0.0) as f32;
            khr.anisotropy_rotation = aniso["anisotropyRotation"].as_f64().unwrap_or(0.0) as f32;
        }
        if let Some(em) = ext.get("KHR_materials_emissive_strength") { khr.emissive_strength = em["emissiveStrength"].as_f64().unwrap_or(1.0) as f32; }
        khr
    }).collect()
}

fn parse_khr_lights_punctual(gltf: &Value) -> Vec<KhrPunctualLight> {
    let empty = Vec::new();
    gltf["extensions"]["KHR_lights_punctual"]["lights"].as_array().unwrap_or(&empty).iter().map(|l| {
        let color = l["color"].as_array().filter(|c| c.len() == 3)
            .map(|c| [c[0].as_f64().unwrap_or(1.0) as f32, c[1].as_f64().unwrap_or(1.0) as f32, c[2].as_f64().unwrap_or(1.0) as f32]).unwrap_or([1.0, 1.0, 1.0]);
        KhrPunctualLight {
            name: l["name"].as_str().map(String::from), kind: l["type"].as_str().unwrap_or("point").to_string(),
            color, intensity: l["intensity"].as_f64().unwrap_or(1.0) as f32, range: l["range"].as_f64().map(|v| v as f32),
            inner_cone_angle: l["spot"]["innerConeAngle"].as_f64().unwrap_or(0.0) as f32,
            outer_cone_angle: l["spot"]["outerConeAngle"].as_f64().unwrap_or(core::f64::consts::FRAC_PI_4) as f32,
        }
    }).collect()
}

fn build_living_headers(gltf: &Value, imported: &ImportedMesh) -> (ProceduralSynthesisHeader, EntityOntologyMiniHeader) {
    let extras = &gltf["asset"]["extras"]["qualia"];
    let seed = extras["seed"].as_u64().unwrap_or(0x5155_414C_4941_3130);
    let proc1 = ProceduralSynthesisHeader {
        magic: MAGIC_PROC, version: PROCEDURAL_SYNTHESIS_SECTION_VERSION,
        flags: (FLAG_CELLULAR_VORONOI | FLAG_ALLOMETRIC_GROWTH | FLAG_PGA_MOTOR_DYNAMICS) as u16, seed,
        voronoi_frequency: extras["voronoi_frequency"].as_f64().unwrap_or(12.0) as f32, voronoi_jitter: 0.85,
        fillet_blend_radius: 0.08, fillet_threshold: 0.02,
        allometry_exponent_k: extras["allometry_k"].as_f64().unwrap_or(1.25) as f32, allometry_base_b: 1.0,
        spiral_growth_rate_k: 0.15, spiral_initial_radius_a: 0.1,
        motor_frequency_hz: extras["motor_frequency_hz"].as_f64().unwrap_or(1.8) as f32, motor_damping_ratio: 0.15,
        critical_stress_threshold: 50.0, reserved_f32: 0.0,
    };
    let taxon = extras["taxon_iri"].as_str().unwrap_or("urn:qualia:taxon:living_asset");
    let eont1 = EntityOntologyMiniHeader {
        magic: MAGIC_EONT, version: 1, stage: extras["stage"].as_u64().unwrap_or(2) as u8, reserved_u8: 0,
        entity_did_hash: q_hash(taxon), taxon_iri_hash: q_hash(taxon),
        energy_permill: 1000, damage_permill: 0, hydration_permill: 1000, wear_permill: 0,
        affordance_count: imported.submeshes.len() as u32, reserved_u32: 0,
    };
    (proc1, eont1)
}
