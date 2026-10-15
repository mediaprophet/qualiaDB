use super::*;

#[test]
fn rig_admission_and_orbit_transform_preserve_native_motion() {
    use crate::container_10d::skeletal_section::*;
    use crate::render::skeletal::{Joint, Key, Motor, SkinWeight, Track};
    let mesh = crate::render::assets::Mesh {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![[0, 1, 2]],
        min: [0.0; 3],
        max: [1.0, 1.0, 0.0],
    };
    let base = crate::render::compile_10d::compile_mesh_to_10d(&mesh).unwrap();
    let identity = Motor::identity();
    let joints = [Joint {
        parent: None,
        rest_local: identity,
        inverse_bind: identity,
    }];
    let ids = [crate::q_hash("urn:test:actor:root")];
    let skin = [SkinWeight {
        joints: [0; 4],
        weights: [1.0, 0.0, 0.0, 0.0],
    }; 3];
    let clips = [ClipRecord {
        id: crate::q_hash("urn:test:actor:move"),
        duration: 1.0,
        first_track: 0,
        track_count: 1,
        looping: false,
    }];
    let tracks = [Track {
        joint: 0,
        first_key: 0,
        key_count: 2,
    }];
    let keys = [
        Key {
            time: 0.0,
            local: identity,
        },
        Key {
            time: 1.0,
            local: Motor::from_translation(0.25, 0.0, 0.0),
        },
    ];
    let data = SkeletalData {
        joints: &joints,
        joint_ids: &ids,
        skin: &skin,
        clips: &clips,
        tracks: &tracks,
        keys: &keys,
    };
    let mut payload = [0; 432];
    encode_skeletal_section_into(&data, &mut payload).unwrap();
    let mut bytes = [0; 4096];
    let len = crate::container_10d::skeletal_container::attach_skeletal_section_into(
        &base, &payload, &mut bytes,
    )
    .unwrap();
    let mut scene = SceneMeshAccum::new();
    scene.append_asset_10d(&bytes[..len], [1.0; 4]);
    assert_eq!(scene.assets_loaded, 1);
    assert_eq!(scene.animation.actors.len(), 1);
    scene.normalise_to_orbit_frame().unwrap();
    let bind = scene.positions[0];
    let actor = &mut scene.animation.actors[0];
    actor.player.advance(1.0, false).unwrap();
    assert!((actor.player.positions()[0][0] - bind[0] - 0.425).abs() < 1e-5);
}

fn material_asset(id: u64, roughness: f32, offset: f32) -> Vec<u8> {
    let mesh = crate::render::assets::Mesh {
        positions: vec![
            [offset, 0.0, 0.0],
            [offset + 1.0, 0.0, 0.0],
            [offset, 1.0, 0.0],
        ],
        triangles: vec![[0, 1, 2]],
        min: [offset, 0.0, 0.0],
        max: [offset + 1.0, 1.0, 0.0],
    };
    let bytes = crate::render::compile_10d::compile_mesh_to_10d(&mesh).unwrap();
    let mut material = MaterialRecord::legacy_default();
    material.id = id;
    material.roughness = roughness;
    let ranges = [SubmeshRange {
        first_index: 0,
        index_count: 3,
        material_id: id,
        semantic_id: id,
    }];
    container_10d::attach_material_section(&bytes, &[material], &ranges, 3).unwrap()
}

#[test]
fn merged_scene_keeps_and_deduplicates_material_bindings() {
    let mut scene = SceneMeshAccum::new();
    scene.append_asset_10d(&material_asset(100, 0.34, 0.0), [0.8, 0.6, 0.3, 1.0]);
    scene.append_asset_10d(&material_asset(100, 0.34, 2.0), [0.3, 0.6, 0.8, 1.0]);
    scene.append_asset_10d(&material_asset(101, 0.86, 4.0), [0.3, 0.5, 0.2, 1.0]);

    assert_eq!(scene.assets_loaded, 3);
    assert_eq!(scene.assets_refused, 0);
    assert_eq!(scene.materials.len(), 2);
    assert_eq!(scene.material_ranges.len(), 3);
    assert_eq!(scene.material_ranges[0].first_index, 0);
    assert_eq!(scene.material_ranges[1].first_index, 3);
    assert_eq!(scene.material_ranges[2].first_index, 6);
    assert_eq!(scene.material_ranges[2].material_id, 101);
}
#[test]
fn merged_uvs_preserve_source_coordinates_and_fill_only_legacy_vertices() {
    let bytes = material_asset(100, 0.34, 0.0);
    let uv = [[0.1, 0.2], [0.7, 0.2], [0.1, 0.9]];
    let textured = container_10d::attach_texture_coordinate_section(&bytes, &uv).unwrap();
    let mut scene = SceneMeshAccum::new();
    scene.append_asset_10d(&textured, [1.0; 4]);
    scene.append_asset_10d(&material_asset(101, 0.86, 2.0), [1.0; 4]);
    assert_eq!(scene.uv0, [uv.as_slice(), &[[0.0; 2]; 3]].concat());
    assert_eq!(scene.uv0.len(), scene.positions.len());
    scene.normalise_to_orbit_frame().unwrap();
    assert_eq!(&scene.uv0[..3], &uv);
}
#[test]
fn authored_normal_survives_merge_and_uniform_orbit_normalisation() {
    let bytes = material_asset(100, 0.34, 0.0);
    let mesh = crate::render::compile_10d::decode_10d_mesh(&bytes).unwrap();
    let bytes =
        crate::render::compile_10d::compile_mesh_to_10d_with_normals(&mesh, &[[0.6, 0.8, 0.0]; 3])
            .unwrap();
    let mut scene = SceneMeshAccum::new();
    scene.append_asset_10d(&bytes, [1.0; 4]);
    assert_eq!(scene.assets_loaded, 1);
    let original = scene.normals.clone();
    scene.normalise_to_orbit_frame().unwrap();
    assert_eq!(scene.normals, original);
    for normal in original {
        assert!((normal[0] - 0.6).abs() < 0.0001);
        assert!((normal[1] - 0.8).abs() < 0.0001);
    }
}
