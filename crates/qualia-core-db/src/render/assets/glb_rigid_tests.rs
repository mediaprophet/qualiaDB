//! Verbatim canonical fixture construction plus native-playback regression cases.
use super::*;
use crate::container_10d::skeletal_section::SkeletalSection;
use crate::render::skeletal::{compose_palette_into, sample_pose_into, skin_vertices_into, Clip};
const GLB_MAGIC: u32 = 0x46546c67;
const CHUNK_JSON: u32 = 0x4e4f534a;
const CHUNK_BIN: u32 = 0x004e4942;
pub fn living_fixture() -> Vec<u8> {
    let mut bin = Vec::new();
    // bv 0: 3 positions (36 B)
    for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    // bv 1: 3 JOINTS_0 u8x4 (12 B)
    for _ in 0..3 {
        bin.extend_from_slice(&[0u8, 1, 0, 0]);
    }
    // bv 2: 3 WEIGHTS_0 f32x4 (48 B)
    for _ in 0..3 {
        for w in [0.75f32, 0.25, 0.0, 0.0] {
            bin.extend_from_slice(&w.to_le_bytes());
        }
    }
    // bv 3: 2 IBM MAT4 (128 B)
    let ident_mat: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    for _ in 0..2 {
        for f in ident_mat {
            bin.extend_from_slice(&f.to_le_bytes());
        }
    }
    // bv 4: anim times [0.0, 0.5] (8 B)
    for t in [0.0f32, 0.5] {
        bin.extend_from_slice(&t.to_le_bytes());
    }
    // bv 5: anim translations [[0,0,0], [0,2,0]] (24 B)
    for tr in [[0.0f32, 0.0, 0.0], [0.0, 2.0, 0.0]] {
        for c in tr {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    // bv 6: morph target POSITION deltas (36 B)
    for md in [[0.0f32, 0.5, 0.0], [0.0, 0.0, 0.0], [0.0, -0.25, 0.0]] {
        for c in md {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let json = serde_json::json!({
        "asset": { "version": "2.0", "extras": { "qualia": { "taxon_iri": "urn:qualia:flora:corymbia" } } },
        "buffers": [{ "byteLength": bin.len() }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 12 },
            { "buffer": 0, "byteOffset": 48, "byteLength": 48 },
            { "buffer": 0, "byteOffset": 96, "byteLength": 128 },
            { "buffer": 0, "byteOffset": 224, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 232, "byteLength": 24 },
            { "buffer": 0, "byteOffset": 256, "byteLength": 36 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
            { "bufferView": 1, "componentType": 5121, "count": 3, "type": "VEC4" },
            { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC4" },
            { "bufferView": 3, "componentType": 5126, "count": 2, "type": "MAT4" },
            { "bufferView": 4, "componentType": 5126, "count": 2, "type": "SCALAR" },
            { "bufferView": 5, "componentType": 5126, "count": 2, "type": "VEC3" },
            { "bufferView": 6, "componentType": 5126, "count": 3, "type": "VEC3" }
        ],
        "materials": [{
            "extensions": {
                "KHR_materials_clearcoat": { "clearcoatFactor": 0.8, "clearcoatRoughnessFactor": 0.2 },
                "KHR_materials_ior": { "ior": 1.333 },
                "KHR_materials_emissive_strength": { "emissiveStrength": 4.5 }
            }
        }],
        "extensions": {
            "KHR_lights_punctual": {
                "lights": [{ "name": "SunSpot", "type": "spot", "color": [1.0, 0.9, 0.8], "intensity": 12.0 }]
            }
        },
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0, "JOINTS_0": 1, "WEIGHTS_0": 2 },
                "targets": [{ "POSITION": 6 }],
                "material": 0
            }]
        }],
        "nodes": [
            { "name": "RootJoint", "children": [1], "translation": [0.0, 1.0, 0.0] },
            { "name": "ChildJoint", "translation": [0.0, 0.5, 0.0], "extensions": { "KHR_lights_punctual": { "light": 0 } } },
            { "name": "SkinnedMeshNode", "mesh": 0, "skin": 0 }
        ],
        "scenes": [{ "nodes": [0, 2] }],
        "skins": [{ "joints": [0, 1], "skeleton": 0, "inverseBindMatrices": 3 }],
        "animations": [{
            "name": "Sway",
            "samplers": [{ "input": 4, "output": 5, "interpolation": "LINEAR" }],
            "channels": [{ "sampler": 0, "target": { "node": 1, "path": "translation" } }]
        }]
    });
    let mut jb = serde_json::to_vec(&json).unwrap();
    while jb.len() % 4 != 0 {
        jb.push(b' ');
    }
    let total = 12 + 8 + jb.len() + 8 + bin.len();
    let mut glb = Vec::with_capacity(total);
    glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(jb.len() as u32).to_le_bytes());
    glb.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    glb.extend_from_slice(&jb);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    glb.extend_from_slice(&bin);

    glb
}

fn mutate_fixture(edit: impl FnOnce(&mut Value, &mut Vec<u8>)) -> Vec<u8> {
    let source = living_fixture();
    let json_len = u32::from_le_bytes(source[12..16].try_into().unwrap()) as usize;
    let mut json: Value = serde_json::from_slice(&source[20..20 + json_len]).unwrap();
    let mut bin = source[28 + json_len..].to_vec();
    edit(&mut json, &mut bin);
    let mut json = serde_json::to_vec(&json).unwrap();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::new();
    for word in [GLB_MAGIC, 2, total as u32, json.len() as u32, CHUNK_JSON] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}
fn skinned_first_y(scene: &GlbLivingScene, time: f64) -> f32 {
    let count = scene.joints.len();
    let mut pose = vec![Motor::identity(); count];
    let record = scene.clips[0];
    let clip = Clip {
        duration: record.duration,
        looping: record.looping,
        tracks: &scene.tracks
            [record.first_track as usize..(record.first_track + record.track_count) as usize],
        keys: &scene.keys,
    };
    sample_pose_into(&scene.joints, clip, time, None, 1.0, &mut pose).unwrap();
    let (mut globals, mut palette) = (
        vec![Motor::identity(); count],
        vec![Motor::identity(); count],
    );
    compose_palette_into(
        &scene.joints,
        &pose,
        Motor::identity(),
        &mut globals,
        &mut palette,
    )
    .unwrap();
    let positions = &scene.imported_mesh.mesh.positions;
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let mut output = vec![[0.0; 3]; positions.len()];
    let mut output_normals = output.clone();
    skin_vertices_into(
        positions,
        &normals,
        &scene.skin_weights,
        &palette,
        &mut output,
        &mut output_normals,
    )
    .unwrap();
    output[0][1]
}
#[test]
fn posed_gltf_preserves_inverse_bind_data_default_root_and_nonlooping_motion() {
    let scene = parse_glb_living_scene(&living_fixture()).unwrap();
    let bytes = scene
        .skl1_section
        .as_ref()
        .expect("valid source cannot silently lose SKL1");
    let section = SkeletalSection::parse(bytes, 3).unwrap();
    assert_eq!(section.counts().joints, 2);
    assert!(!scene.clips[0].looping);
    assert_eq!(scene.tracks.len(), 2);
    assert_eq!(
        scene.nodes[0].local_motor.to_rotation_translation().1,
        [0.0, 1.0, 0.0]
    );
    for joint in &scene.joints {
        assert_eq!(joint.inverse_bind.to_rotation_translation().1, [0.0; 3]);
        assert_eq!(joint.rest_local.to_rotation_translation().1, [0.0; 3]);
    }
    // Independent glTF linear skinning equation: root translates by 1; the child
    // channel translates 0→2 over .5s, with .75/.25 influence weights.
    for (time, y) in [(0.0, 1.0), (0.25, 1.25), (0.5, 1.5), (1.0, 1.5)] {
        assert!((skinned_first_y(&scene, time) - y).abs() < 1e-6);
    }
    assert!(scene.mrp1_section.is_some());
    assert_eq!(scene.morph_deltas.len(), 2);
    assert!((scene.material_extensions[0].clearcoat_factor - 0.8).abs() < 1e-5);
    assert_eq!(scene.punctual_lights.len(), 1);
}
#[test]
fn unordered_source_joint_indices_are_remapped_without_changing_deformation() {
    let bytes = mutate_fixture(|json, bin| {
        json["skins"][0]["joints"] = serde_json::json!([1, 0]);
        for vertex in 0..3 {
            bin[36 + vertex * 4] = 1;
            bin[37 + vertex * 4] = 0;
        }
    });
    let scene = parse_glb_living_scene(&bytes).unwrap();
    assert_eq!(scene.joints[1].parent, Some(0));
    assert!(scene.skin_weights.iter().all(|w| w.joints[..2] == [0, 1]));
    assert!((skinned_first_y(&scene, 0.25) - 1.25).abs() < 1e-6);
    SkeletalSection::parse(scene.skl1_section.as_ref().unwrap(), 3).unwrap();
}
#[test]
fn default_posed_skin_without_animation_stays_visible_as_authored() {
    let bytes = mutate_fixture(|json, _| {
        json.as_object_mut().unwrap().remove("animations");
        json["skins"][0]
            .as_object_mut()
            .unwrap()
            .remove("inverseBindMatrices");
    });
    let scene = parse_glb_living_scene(&bytes).unwrap();
    assert_eq!(scene.clips.len(), 1);
    assert!(!scene.clips[0].looping);
    for time in [0.0, 0.5, 1.0] {
        assert!((skinned_first_y(&scene, time) - 1.125).abs() < 1e-6);
    }
}
#[test]
fn unsupported_interpolation_and_nonrigid_binds_fail_instead_of_silent_section_loss() {
    for interpolation in ["STEP", "CUBICSPLINE"] {
        let bytes = mutate_fixture(|json, _| {
            json["animations"][0]["samplers"][0]["interpolation"] = serde_json::json!(interpolation)
        });
        assert!(
            matches!(parse_glb_living_scene(&bytes),Err(AssetError::Parse(message))if message.contains("interpolation"))
        );
    }
    let bytes = mutate_fixture(|_, bin| bin[96..100].copy_from_slice(&2.0f32.to_le_bytes()));
    assert!(
        matches!(parse_glb_living_scene(&bytes),Err(AssetError::Parse(message))if message.contains("nonrigid"))
    );
    let bytes = mutate_fixture(|json, _| json["accessors"][3]["count"] = serde_json::json!(1));
    assert!(
        matches!(parse_glb_living_scene(&bytes),Err(AssetError::Parse(message))if message.contains("count"))
    );
}

#[test]
fn malformed_weight_hierarchy_and_animation_ranges_are_rejected() {
    for value in [f32::NAN, -0.25] {
        let bytes = mutate_fixture(|_, bin| bin[48..52].copy_from_slice(&value.to_le_bytes()));
        assert!(
            matches!(parse_glb_living_scene(&bytes),Err(AssetError::Parse(message))if message.contains("weights"))
        );
    }
    let bytes = mutate_fixture(|_, bin| bin[48..64].fill(0));
    assert!(
        matches!(parse_glb_living_scene(&bytes),Err(AssetError::Parse(message))if message.contains("weight sum"))
    );
    let bytes = mutate_fixture(|json, _| json["nodes"][1]["children"] = serde_json::json!([0]));
    assert!(
        matches!(parse_glb_living_scene(&bytes),Err(AssetError::Parse(message))if message.contains("cyclic"))
    );
    let bytes = mutate_fixture(|json, _| json["nodes"][2]["children"] = serde_json::json!([1]));
    assert!(
        matches!(parse_glb_living_scene(&bytes),Err(AssetError::Parse(message))if message.contains("multiple parents"))
    );
    let bytes =
        mutate_fixture(|json, _| json["bufferViews"][5]["byteLength"] = serde_json::json!(12));
    assert!(
        matches!(parse_glb_living_scene(&bytes),Err(AssetError::Parse(message))if message.contains("buffer view"))
    );
}

#[test]
fn authored_nonidentity_inverse_binds_remain_native_bind_data() {
    let bytes = mutate_fixture(|_, bin| {
        // glTF inverse bind translations root=-1, child=-1.5.
        bin[148..152].copy_from_slice(&(-1.0f32).to_le_bytes());
        bin[212..216].copy_from_slice(&(-1.5f32).to_le_bytes());
    });
    let scene = parse_glb_living_scene(&bytes).unwrap();
    SkeletalSection::parse(scene.skl1_section.as_ref().unwrap(), 3).unwrap();
    assert_eq!(
        scene.joints[0].inverse_bind.to_rotation_translation().1,
        [0.0, -1.0, 0.0]
    );
    assert_eq!(
        scene.joints[1].inverse_bind.to_rotation_translation().1,
        [0.0, -1.5, 0.0]
    );
    assert_eq!(
        scene.joints[0].rest_local.to_rotation_translation().1,
        [0.0, 1.0, 0.0]
    );
    assert_eq!(
        scene.joints[1].rest_local.to_rotation_translation().1,
        [0.0, 0.5, 0.0]
    );
    // At .25s child world=2, and palette child=2-1.5; weighted y=.125.
    assert!((skinned_first_y(&scene, 0.25) - 0.125).abs() < 1e-6);
}
