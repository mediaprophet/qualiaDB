//! Public native consumer regression: exact texture union, not model acceptance.
use qualia_core_db::{
    bundle::{BundleReader, BundleWriter},
    poet_host::invoke::studio_store::{
        self,
        checkpoint::{self, Checkpoint, CheckpointLimits, ResourceData, ResourceEdit},
        StudioArtifact,
    },
    render::{
        asset_package::texture_hmc_key,
        compile_10d::{compile_asset, CompiledAsset},
    },
};
use sha2::{Digest, Sha256};

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
// A native-supported GLB source with one synthetic 2x2 RGBA8 KTX2 base-colour
// image. The public importer/compiler produces its material/dependency/graphs.
fn textured_asset(identifier: &str) -> CompiledAsset {
    let mut image = vec![0; 152];
    image[..12].copy_from_slice(&qualia_core_db::render::texture_ktx2::KTX2_IDENTIFIER);
    for (at, n) in [
        (12, 43u32),
        (16, 1),
        (20, 2),
        (24, 2),
        (36, 1),
        (40, 1),
        (48, 104),
        (52, 28),
        (104, 28),
        (112, (24 << 16) | 2),
    ] {
        image[at..at + 4].copy_from_slice(&n.to_le_bytes());
    }
    for (at, n) in [(80, 136u64), (88, 16), (96, 16)] {
        image[at..at + 8].copy_from_slice(&n.to_le_bytes());
    }
    image[136..].copy_from_slice(&[
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ]);
    let mut binary = Vec::new();
    for p in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for n in p {
            binary.extend_from_slice(&n.to_le_bytes());
        }
    }
    for p in [[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0]] {
        for n in p {
            binary.extend_from_slice(&n.to_le_bytes());
        }
    }
    for i in [0u16, 1, 2] {
        binary.extend_from_slice(&i.to_le_bytes());
    }
    while binary.len() % 4 != 0 {
        binary.push(0);
    }
    let image_offset = binary.len();
    binary.extend_from_slice(&image);
    let json = serde_json::json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":binary.len()}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":24},{"buffer":0,"byteOffset":60,"byteLength":6},{"buffer":0,"byteOffset":image_offset,"byteLength":image.len()}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},{"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"},{"bufferView":2,"componentType":5123,"count":3,"type":"SCALAR"}],
        "images":[{"bufferView":3,"mimeType":"image/ktx2"}],"textures":[{"source":0}],
        "materials":[{"pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0.0,"roughnessFactor":0.6}}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"indices":2,"material":0}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    let mut json = serde_json::to_vec(&json).unwrap();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    while binary.len() % 4 != 0 {
        binary.push(0);
    }
    let mut glb = Vec::new();
    for n in [
        0x46546c67u32,
        2,
        (12 + 8 + json.len() + 8 + binary.len()) as u32,
        json.len() as u32,
        0x4e4f534a,
    ] {
        glb.extend_from_slice(&n.to_le_bytes());
    }
    glb.extend_from_slice(&json);
    glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004e4942u32.to_le_bytes());
    glb.extend_from_slice(&binary);
    let asset = compile_asset(&glb, Some("glb"), identifier, "glb").unwrap();
    assert_eq!(asset.texture_dependencies.len(), 1);
    assert_eq!(asset.texture_dependencies[0].digest, digest(&image));
    asset
}
fn source() -> Checkpoint {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = format!(
        "urn:public:hmc-texture-union:{}:{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let mut source = StudioArtifact::new(&id, "Geometry", "Temporary canonical HMC texture union");
    source.properties.insert(
        "source.attribution".into(),
        "Synthetic software-agent fixture; unpaid and unreviewed".into(),
    );
    let (_, stamp) = studio_store::create_artifact_if_absent(source).unwrap();
    checkpoint::capture(&id, &stamp, Default::default())
        .unwrap()
        .with_parent_digest([9; 32])
}
fn retained(base: &Checkpoint, asset: &CompiledAsset) -> Checkpoint {
    let d = &asset.texture_dependencies[0];
    base.apply_resource_edits(
        vec![
            ResourceEdit::Add(ResourceData {
                key: texture_hmc_key(&d.digest),
                kind: d.mime_type.clone(),
                bytes: d.bytes.clone(),
                meta: Some(vec![0xa2, 0x61, b'p', 1, 0x61, b'x', 0x82, 7, 8]),
            }),
            ResourceEdit::Add(ResourceData {
                key: "masters/opaque-retained".into(),
                kind: "application/vnd.future.master".into(),
                bytes: vec![0, 255, 9],
                meta: Some(vec![0xff, 0, 0x80]),
            }),
        ],
        Default::default(),
    )
    .unwrap()
}
#[test]
fn retained_texture_union_preserves_exact_metadata_lineage_and_unique_entry_budget() {
    let a = textured_asset("urn:public:union:a");
    let b = textured_asset("urn:public:union:b");
    let before = retained(&source(), &a);
    let original = before.resources().to_vec();
    let limits = CheckpointLimits {
        entries: 7,
        ..Default::default()
    }; // source + 2 retained + 2 assets and their graphs
    let bytes = checkpoint::encode(
        &before,
        vec![("derived/a".into(), a.clone()), ("derived/b".into(), b)],
        limits,
    )
    .unwrap();
    let reopened = checkpoint::decode(&bytes, limits).unwrap();
    assert_eq!(reopened.artifact(), before.artifact());
    assert_eq!(reopened.parent_digest(), Some([9; 32]));
    for old in original {
        assert_eq!(
            reopened.resources().iter().find(|r| r.key == old.key),
            Some(&old)
        );
    }
    let reader = BundleReader::parse(&bytes).unwrap();
    assert_eq!(reader.entries().len(), 7);
    assert_eq!(
        reader
            .entries()
            .iter()
            .filter(|r| r.key.starts_with("textures/sha256/"))
            .count(),
        1
    );
    let texture = &a.texture_dependencies[0];
    assert_eq!(
        qualia_core_db::render::asset_package::resolve_hmc_texture_resource(
            &reader,
            &texture.digest
        )
        .unwrap()
        .bytes,
        texture.bytes
    );
    let tiny = CheckpointLimits {
        entries: 6,
        ..limits
    };
    assert!(checkpoint::encode(
        &before,
        vec![("derived/a".into(), a.clone()), ("derived/b".into(), a)],
        tiny
    )
    .is_err());
}
#[test]
fn new_shared_texture_is_charged_and_packaged_once_across_native_assets() {
    let a = textured_asset("urn:public:new-union:a");
    let b = textured_asset("urn:public:new-union:b");
    let bytes = checkpoint::encode(
        &source(),
        vec![("a".into(), a), ("b".into(), b)],
        CheckpointLimits {
            entries: 6,
            ..Default::default()
        },
    )
    .unwrap();
    let reader = BundleReader::parse(&bytes).unwrap();
    assert_eq!(reader.entries().len(), 6);
    assert_eq!(
        reader
            .entries()
            .iter()
            .filter(|r| r.key.starts_with("textures/sha256/"))
            .count(),
        1
    );
}
// Decode remains inert. Composition checks known dependencies only when they
// participate in a union; unrelated opaque/future entries must survive unchanged.
fn malformed(base: &Checkpoint, kind: &str, key: &str, bytes: Vec<u8>) -> Checkpoint {
    let original = checkpoint::encode(base, vec![], Default::default()).unwrap();
    let reader = BundleReader::parse(&original).unwrap();
    let mut writer = BundleWriter::new();
    for entry in reader.entries() {
        writer
            .add_file(
                &entry.key,
                &entry.kind,
                reader.get(&entry.key).unwrap().to_vec(),
                entry.meta.clone(),
            )
            .unwrap();
    }
    writer
        .add_file(key, kind, bytes, Some(vec![0xa1, 0x61, b'p', 7]))
        .unwrap();
    let decoded = checkpoint::decode(&writer.build().unwrap(), Default::default()).unwrap();
    assert!(decoded.resources().iter().any(|r| r.key == key));
    decoded
}
#[test]
fn conflicting_mime_digest_alias_cross_kind_and_reserved_keys_refuse_without_mutation() {
    let asset = textured_asset("urn:public:union:refusal");
    let base = source();
    let d = &asset.texture_dependencies[0];
    let key = texture_hmc_key(&d.digest);
    for (kind, key, payload) in [
        ("image/jpeg", key.clone(), d.bytes.clone()),
        ("application/json", key.clone(), d.bytes.clone()),
        ("image/png", key.to_ascii_uppercase(), d.bytes.clone()),
        ("image/png", key.clone(), vec![1, 2, 3]),
    ] {
        let bad = malformed(&base, kind, &key, payload);
        let original = bad.resources().to_vec();
        let file_digest = bad.file_digest();
        assert!(checkpoint::encode(
            &bad,
            vec![("new".into(), asset.clone())],
            Default::default()
        )
        .is_err());
        assert_eq!(bad.resources(), original);
        assert_eq!(bad.file_digest(), file_digest);
    }
    let before = retained(&base, &asset);
    let old = before.resources().to_vec();
    let mut conflict = asset.clone();
    conflict.texture_dependencies[0].mime_type = "image/jpeg".into();
    assert!(checkpoint::encode(
        &base,
        vec![("a".into(), asset.clone()), ("b".into(), conflict)],
        Default::default()
    )
    .is_err());
    let mut wrong = asset.clone();
    wrong.texture_dependencies[0].digest = [1; 32];
    assert!(checkpoint::encode(&before, vec![("new".into(), wrong)], Default::default()).is_err());
    for reserved in [
        "authoring/native-source.json".to_owned(),
        key,
        "masters/opaque-retained".into(),
    ] {
        assert!(
            checkpoint::encode(&before, vec![(reserved, asset.clone())], Default::default())
                .is_err()
        );
    }
    assert_eq!(before.resources(), old);
}
#[test]
fn source_only_encode_retains_unrelated_future_texture_namespace_bytes_and_metadata() {
    let base = source();
    let inert = malformed(
        &base,
        "application/vnd.future.texture",
        "textures/sha256/future-inert",
        vec![0, 255, 1],
    );
    let original = inert.resources().to_vec();
    let bytes = checkpoint::encode(&inert, vec![], Default::default()).unwrap();
    let reopened = checkpoint::decode(&bytes, Default::default()).unwrap();
    assert_eq!(reopened.resources(), original);
    assert_eq!(reopened.artifact(), inert.artifact());
    assert_eq!(reopened.parent_digest(), inert.parent_digest());
    assert_eq!(Some(digest(&bytes)), inert.file_digest());
}
#[test]
fn caller_entry_lengths_refuse_before_digest_or_cloned_key_work() {
    let asset = textured_asset("urn:public:union:budget-preflight");
    let base = source();
    let limits = CheckpointLimits {
        file_bytes: 2048,
        ..Default::default()
    };
    let mut oversized = asset.clone();
    oversized.texture_dependencies[0].bytes = vec![0; 4096];
    // A digest-first implementation would report invalid digest for this data.
    let error = checkpoint::encode(&base, vec![("a".into(), oversized)], limits).unwrap_err();
    assert!(error.contains("budget"), "{error}");
    let error =
        checkpoint::encode(&base, vec![("k".repeat(4096), asset.clone())], limits).unwrap_err();
    assert!(error.contains("budget"), "{error}");
    let mut oversized = asset;
    oversized.texture_dependencies[0].mime_type = "image/".to_owned() + &"x".repeat(4096);
    let error = checkpoint::encode(&base, vec![("a".into(), oversized)], limits).unwrap_err();
    assert!(error.contains("budget"), "{error}");
    assert!(base.resources().is_empty());
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_save_reopen_keeps_metadata_and_conflicting_union_preserves_destination_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("temporary-union.hmc");
    let asset = textured_asset("urn:public:union:native-save");
    let retained = retained(&source(), &asset);
    let bytes = checkpoint::encode(
        &retained,
        vec![("derived".into(), asset.clone())],
        Default::default(),
    )
    .unwrap();
    let saved = checkpoint::save(
        &path,
        &bytes,
        checkpoint::ExpectedFile::Absent,
        Default::default(),
    )
    .unwrap();
    let reopened = checkpoint::load(&path, Default::default()).unwrap();
    for resource in retained.resources() {
        assert_eq!(
            reopened.resources().iter().find(|r| r.key == resource.key),
            Some(resource)
        );
    }
    let mut bad = asset;
    bad.texture_dependencies[0].mime_type = "image/jpeg".into();
    let directory_before = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    let result = checkpoint::encode(&retained, vec![("another".into(), bad)], Default::default())
        .and_then(|candidate| {
            checkpoint::save(
                &path,
                &candidate,
                checkpoint::ExpectedFile::Sha256(saved.digest),
                Default::default(),
            )
        });
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<std::collections::BTreeSet<_>>(),
        directory_before
    );
}
