//! glTF material decoding and focused import conformance fixtures.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::{AssetError, TextureDependency};
use crate::container_10d::{
    MaterialRecord, OpacityMode, TextureAddressMode, TextureFilterMode, TextureMinFilter,
    TextureSampler,
};

pub(super) fn gltf_default_material() -> MaterialRecord {
    let mut material = MaterialRecord::legacy_default();
    // glTF's implicit metallic-roughness material defaults both factors to 1.
    material.metallic = 1.0;
    material.roughness = 1.0;
    material
}

pub(super) fn parse_gltf_materials(
    gltf: &Value,
    buffer_views: &[Value],
    bin: &[u8],
) -> Result<(Vec<MaterialRecord>, Vec<TextureDependency>), AssetError> {
    let empty: Vec<Value> = Vec::new();
    let source_materials = gltf["materials"].as_array().unwrap_or(&empty);
    if source_materials.len() > u32::MAX as usize - 1 {
        return Err(AssetError::Parse("too many glTF materials".into()));
    }
    let mut materials = Vec::with_capacity(source_materials.len());
    // Content-addressed, ordered deduplication gives repeatable HMC packaging while allowing
    // multiple material slots to share one original image payload.
    let mut dependencies = BTreeMap::<[u8; 32], TextureDependency>::new();
    let mut dependency_bytes = 0usize;
    for (index, source) in source_materials.iter().enumerate() {
        let pbr = &source["pbrMetallicRoughness"];
        let mut material = gltf_default_material();
        material.id = index as u64 + 1;
        material.base_color = float_array::<4>(
            &pbr["baseColorFactor"],
            [1.0, 1.0, 1.0, 1.0],
            "baseColorFactor",
        )?;
        material.metallic = finite_factor(&pbr["metallicFactor"], 1.0, "metallicFactor")?;
        material.roughness = finite_factor(&pbr["roughnessFactor"], 1.0, "roughnessFactor")?;
        material.emissive =
            float_array::<3>(&source["emissiveFactor"], [0.0; 3], "emissiveFactor")?;
        material.double_sided = source["doubleSided"].as_bool().unwrap_or(false);
        material.opacity_mode = match source["alphaMode"].as_str().unwrap_or("OPAQUE") {
            "OPAQUE" => OpacityMode::Opaque,
            "MASK" => OpacityMode::Mask,
            "BLEND" => OpacityMode::Blend,
            mode => {
                return Err(AssetError::Parse(format!(
                    "unsupported glTF alphaMode {mode}"
                )));
            }
        };
        material.alpha_cutoff = finite_factor(&source["alphaCutoff"], 0.5, "alphaCutoff")?;
        material.normal_scale = finite_factor(
            &source["normalTexture"]["scale"],
            1.0,
            "normalTexture.scale",
        )?;
        material.occlusion_strength = finite_factor(
            &source["occlusionTexture"]["strength"],
            1.0,
            "occlusionTexture.strength",
        )?;
        material.base_color_texture = texture_digest(
            gltf,
            &pbr["baseColorTexture"],
            buffer_views,
            bin,
            "baseColorTexture",
            &mut dependencies,
            &mut dependency_bytes,
        )?;
        material.texture_transforms[0] =
            super::glb_texture_transform::parse(&pbr["baseColorTexture"], "baseColorTexture")?;
        material.texture_samplers[0] =
            parse_sampler(gltf, &pbr["baseColorTexture"], "baseColorTexture")?;
        material.normal_texture = texture_digest(
            gltf,
            &source["normalTexture"],
            buffer_views,
            bin,
            "normalTexture",
            &mut dependencies,
            &mut dependency_bytes,
        )?;
        material.texture_transforms[1] =
            super::glb_texture_transform::parse(&source["normalTexture"], "normalTexture")?;
        material.texture_samplers[1] =
            parse_sampler(gltf, &source["normalTexture"], "normalTexture")?;
        material.metallic_roughness_texture = texture_digest(
            gltf,
            &pbr["metallicRoughnessTexture"],
            buffer_views,
            bin,
            "metallicRoughnessTexture",
            &mut dependencies,
            &mut dependency_bytes,
        )?;
        material.texture_transforms[2] = super::glb_texture_transform::parse(
            &pbr["metallicRoughnessTexture"],
            "metallicRoughnessTexture",
        )?;
        material.texture_samplers[2] = parse_sampler(
            gltf,
            &pbr["metallicRoughnessTexture"],
            "metallicRoughnessTexture",
        )?;
        material.occlusion_texture = texture_digest(
            gltf,
            &source["occlusionTexture"],
            buffer_views,
            bin,
            "occlusionTexture",
            &mut dependencies,
            &mut dependency_bytes,
        )?;
        material.texture_transforms[3] =
            super::glb_texture_transform::parse(&source["occlusionTexture"], "occlusionTexture")?;
        material.texture_samplers[3] =
            parse_sampler(gltf, &source["occlusionTexture"], "occlusionTexture")?;
        material.emissive_texture = texture_digest(
            gltf,
            &source["emissiveTexture"],
            buffer_views,
            bin,
            "emissiveTexture",
            &mut dependencies,
            &mut dependency_bytes,
        )?;
        material.texture_transforms[4] =
            super::glb_texture_transform::parse(&source["emissiveTexture"], "emissiveTexture")?;
        material.texture_samplers[4] =
            parse_sampler(gltf, &source["emissiveTexture"], "emissiveTexture")?;
        materials.push(material);
    }
    Ok((materials, dependencies.into_values().collect()))
}

fn parse_sampler(
    gltf: &Value,
    texture_info: &Value,
    semantic: &str,
) -> Result<TextureSampler, AssetError> {
    if texture_info["index"].is_null() {
        return Ok(TextureSampler::GLTF_DEFAULT);
    }
    let texture_index = texture_info["index"].as_u64().ok_or_else(|| {
        AssetError::Parse(format!(
            "{semantic} texture index must be an unsigned integer"
        ))
    })?;
    let textures = gltf["textures"].as_array().ok_or_else(|| {
        AssetError::Parse(format!("{semantic} references missing textures table"))
    })?;
    let texture = textures
        .get(texture_index as usize)
        .ok_or_else(|| AssetError::Parse(format!("{semantic} texture index out of range")))?;
    if texture["sampler"].is_null() {
        return Ok(TextureSampler::GLTF_DEFAULT);
    }
    let sampler_index = texture["sampler"].as_u64().ok_or_else(|| {
        AssetError::Parse(format!(
            "{semantic} sampler index must be an unsigned integer"
        ))
    })?;
    let samplers = gltf["samplers"].as_array().ok_or_else(|| {
        AssetError::Parse(format!("{semantic} references missing samplers table"))
    })?;
    let sampler = samplers
        .get(sampler_index as usize)
        .ok_or_else(|| AssetError::Parse(format!("{semantic} sampler index out of range")))?;
    let numeric_field = |name: &str, default: u64| -> Result<u64, AssetError> {
        if sampler[name].is_null() {
            Ok(default)
        } else {
            sampler[name].as_u64().ok_or_else(|| {
                AssetError::Parse(format!(
                    "{semantic} sampler {name} must be an unsigned integer"
                ))
            })
        }
    };
    let address = |name: &str| -> Result<TextureAddressMode, AssetError> {
        match numeric_field(name, 10497)? {
            33071 => Ok(TextureAddressMode::ClampToEdge),
            33648 => Ok(TextureAddressMode::MirroredRepeat),
            10497 => Ok(TextureAddressMode::Repeat),
            value => Err(AssetError::Parse(format!(
                "{semantic} sampler {name} value {value} is invalid"
            ))),
        }
    };
    let mag_filter = match numeric_field("magFilter", 9729)? {
        9728 => TextureFilterMode::Nearest,
        9729 => TextureFilterMode::Linear,
        value => {
            return Err(AssetError::Parse(format!(
                "{semantic} sampler magFilter value {value} is invalid"
            )))
        }
    };
    let min_filter = match numeric_field("minFilter", 9987)? {
        9728 => TextureMinFilter::Nearest,
        9729 => TextureMinFilter::Linear,
        9984 => TextureMinFilter::NearestMipmapNearest,
        9985 => TextureMinFilter::LinearMipmapNearest,
        9986 => TextureMinFilter::NearestMipmapLinear,
        9987 => TextureMinFilter::LinearMipmapLinear,
        value => {
            return Err(AssetError::Parse(format!(
                "{semantic} sampler minFilter value {value} is invalid"
            )))
        }
    };
    Ok(TextureSampler {
        wrap_s: address("wrapS")?,
        wrap_t: address("wrapT")?,
        mag_filter,
        min_filter,
    })
}

fn finite_factor(value: &Value, default: f32, name: &str) -> Result<f32, AssetError> {
    match value.as_f64() {
        Some(value) if value.is_finite() && (value as f32).is_finite() => Ok(value as f32),
        Some(_) => Err(AssetError::Parse(format!("{name} must be finite"))),
        None => Ok(default),
    }
}

fn float_array<const N: usize>(
    value: &Value,
    default: [f32; N],
    name: &str,
) -> Result<[f32; N], AssetError> {
    let Some(values) = value.as_array() else {
        return Ok(default);
    };
    if values.len() != N {
        return Err(AssetError::Parse(format!("{name} must contain {N} values")));
    }
    let mut out = [0.0; N];
    for (index, value) in values.iter().enumerate() {
        out[index] = finite_factor(value, 0.0, name)?;
    }
    Ok(out)
}

fn texture_digest(
    gltf: &Value,
    texture_info: &Value,
    buffer_views: &[Value],
    bin: &[u8],
    semantic: &str,
    dependencies: &mut BTreeMap<[u8; 32], TextureDependency>,
    dependency_bytes: &mut usize,
) -> Result<[u8; 32], AssetError> {
    let Some(texture_index) = texture_info["index"].as_u64() else {
        return Ok([0; 32]);
    };
    let textures = gltf["textures"].as_array().ok_or_else(|| {
        AssetError::Parse(format!("{semantic} references missing textures table"))
    })?;
    let texture = textures
        .get(texture_index as usize)
        .ok_or_else(|| AssetError::Parse(format!("{semantic} texture index out of range")))?;
    // KHR_texture_basisu replaces the core source with its KTX2 image when present. Preserve
    // those encoded bytes for platform-specific transcoding instead of forcing CPU decompression.
    let image_index = texture["extensions"]["KHR_texture_basisu"]["source"]
        .as_u64()
        .or_else(|| texture["extensions"]["EXT_texture_webp"]["source"].as_u64())
        .or_else(|| texture["source"].as_u64())
        .ok_or_else(|| AssetError::Parse(format!("{semantic} texture has no image source")))?;
    let images = gltf["images"]
        .as_array()
        .ok_or_else(|| AssetError::Parse(format!("{semantic} references missing images table")))?;
    let image = images
        .get(image_index as usize)
        .ok_or_else(|| AssetError::Parse(format!("{semantic} image index out of range")))?;
    let mime_type = image["mimeType"].as_str().ok_or_else(|| {
        AssetError::Parse(format!("{semantic} embedded image must declare mimeType"))
    })?;
    if !matches!(
        mime_type,
        "image/png" | "image/jpeg" | "image/webp" | "image/ktx2"
    ) {
        return Err(AssetError::Parse(format!(
            "{semantic} embedded image MIME type {mime_type:?} is unsupported"
        )));
    }
    let view_index = image["bufferView"].as_u64().ok_or_else(|| {
        AssetError::Parse(format!(
            "{semantic} image must be embedded in GLB BIN (URI images are not supported yet)"
        ))
    })? as usize;
    let view = buffer_views
        .get(view_index)
        .ok_or_else(|| AssetError::Parse(format!("{semantic} image bufferView out of range")))?;
    let start = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let length = view["byteLength"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse(format!("{semantic} image byteLength missing")))?
        as usize;
    let end = start
        .checked_add(length)
        .ok_or_else(|| AssetError::Parse(format!("{semantic} image byte range overflow")))?;
    let bytes = bin
        .get(start..end)
        .ok_or_else(|| AssetError::Parse(format!("{semantic} image exceeds GLB BIN")))?;
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    if let Some(existing) = dependencies.get(&digest) {
        if existing.bytes.as_slice() != bytes || existing.mime_type != mime_type {
            return Err(AssetError::Parse(format!(
                "{semantic} image digest collision has inconsistent payload metadata"
            )));
        }
    } else {
        let new_total = dependency_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| AssetError::Parse("texture dependency byte count overflow".into()))?;
        // Extraction must not amplify the source GLB's binary payload through overlapping views.
        if new_total > bin.len() {
            return Err(AssetError::Parse(
                "unique texture dependencies exceed the source GLB BIN byte budget".into(),
            ));
        }
        *dependency_bytes = new_total;
        dependencies.insert(
            digest,
            TextureDependency {
                digest,
                mime_type: mime_type.to_owned(),
                bytes: bytes.to_vec(),
            },
        );
    }
    Ok(digest)
}

#[cfg(test)]
mod material_import_tests {
    use super::*;
    use crate::container_10d::SubmeshRange;
    use crate::render::assets::{import_glb_with_normals, CHUNK_BIN, CHUNK_JSON, GLB_MAGIC};

    fn glb_with_two_material_primitives() -> Vec<u8> {
        let mut bin = Vec::new();
        for position in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for value in position {
                bin.extend_from_slice(&value.to_le_bytes());
            }
        }
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":36}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}],"materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0.2,0.4,0.6,1.0],"metallicFactor":0.25,"roughnessFactor":0.75}},{"alphaMode":"BLEND","doubleSided":true}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"material":0},{"attributes":{"POSITION":0},"material":1}]}]}"#;
        let mut json = json.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let total_len = 12 + 8 + json.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total_len as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
        out.extend_from_slice(&json);
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
        out.extend_from_slice(&bin);
        out
    }

    #[test]
    fn gltf_sampler_import_preserves_wrap_and_filter_modes() {
        let gltf = serde_json::json!({
            "textures": [{ "sampler": 0 }],
            "samplers": [{
                "wrapS": 33071,
                "wrapT": 33648,
                "magFilter": 9728,
                "minFilter": 9986
            }]
        });
        let sampler = parse_sampler(
            &gltf,
            &serde_json::json!({ "index": 0 }),
            "baseColorTexture",
        )
        .unwrap();
        assert_eq!(
            sampler,
            TextureSampler {
                wrap_s: TextureAddressMode::ClampToEdge,
                wrap_t: TextureAddressMode::MirroredRepeat,
                mag_filter: TextureFilterMode::Nearest,
                min_filter: TextureMinFilter::NearestMipmapLinear,
            }
        );
        assert_eq!(
            parse_sampler(&gltf, &serde_json::json!({}), "normalTexture").unwrap(),
            TextureSampler::GLTF_DEFAULT
        );
        let invalid = serde_json::json!({
            "textures": [{ "sampler": 0 }],
            "samplers": [{ "wrapS": 1 }]
        });
        assert!(parse_sampler(
            &invalid,
            &serde_json::json!({ "index": 0 }),
            "normalTexture"
        )
        .is_err());
    }

    fn glb_with_embedded_ktx2() -> Vec<u8> {
        let mut bin = Vec::new();
        for position in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for value in position {
                bin.extend_from_slice(&value.to_le_bytes());
            }
        }
        let image_bytes = [0xAB, 0xCD, 0xEF, 0x42];
        bin.extend_from_slice(&image_bytes);
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":40}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":4}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}],"images":[{"bufferView":1,"mimeType":"image/ktx2"}],"textures":[{"extensions":{"KHR_texture_basisu":{"source":0}}}],"materials":[{"pbrMetallicRoughness":{"baseColorTexture":{"index":0}},"normalTexture":{"index":0}}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"material":0}]}]}"#;
        let mut json = json.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let total_len = 12 + 8 + json.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total_len as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
        out.extend_from_slice(&json);
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
        out.extend_from_slice(&bin);
        out
    }

    fn glb_with_uv0() -> Vec<u8> {
        let mut bin = Vec::new();
        for position in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for value in position {
                bin.extend_from_slice(&value.to_le_bytes());
            }
        }
        for coordinate in [[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0]] {
            for value in coordinate {
                bin.extend_from_slice(&value.to_le_bytes());
            }
        }
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":60}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":24}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1}}]}]}"#;
        let mut json = json.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let total_len = 12 + 8 + json.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total_len as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
        out.extend_from_slice(&json);
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
        out.extend_from_slice(&bin);
        out
    }

    #[test]
    fn glb_import_preserves_material_factors_and_primitive_ranges() {
        let imported = import_glb_with_normals(&glb_with_two_material_primitives()).unwrap();
        assert_eq!(imported.materials.len(), 2);
        assert_eq!(imported.materials[0].base_color, [0.2, 0.4, 0.6, 1.0]);
        assert_eq!(imported.materials[0].metallic, 0.25);
        assert_eq!(imported.materials[0].roughness, 0.75);
        assert_eq!(imported.materials[1].opacity_mode, OpacityMode::Blend);
        assert!(imported.materials[1].double_sided);
        assert_eq!(
            imported.submeshes,
            [
                SubmeshRange {
                    first_index: 0,
                    index_count: 3,
                    material_id: 1,
                    semantic_id: 0,
                },
                SubmeshRange {
                    first_index: 3,
                    index_count: 3,
                    material_id: 2,
                    semantic_id: 0,
                }
            ]
        );

        let compiled = crate::render::compile_10d::compile_asset(
            &glb_with_two_material_primitives(),
            Some("glb"),
            "urn:test:two-material-glb",
            "glb",
        )
        .unwrap();
        let mut materials = [MaterialRecord::legacy_default(); 2];
        let mut ranges = [SubmeshRange {
            first_index: 0,
            index_count: 3,
            material_id: 1,
            semantic_id: 0,
        }; 2];
        crate::render::material_compile::decode_10d_materials_into(
            &compiled.container_10d,
            6,
            &mut materials,
            &mut ranges,
        )
        .unwrap();
        assert_eq!(ranges[0].first_index, 0);
        assert_eq!(ranges[1].first_index, 3);
        assert_ne!(ranges[0].material_id, ranges[1].material_id);
        assert!(materials
            .iter()
            .all(|material| material.id != 1 && material.id != 2));
    }

    #[test]
    fn glb_uv0_survives_tangent_seam_processing_and_compiled_sidecar() {
        let source = glb_with_uv0();
        let imported = import_glb_with_normals(&source).unwrap();
        let imported_uv = imported.texture_coordinates_0.as_ref().unwrap();
        assert_eq!(imported_uv.len(), imported.mesh.vertex_count());
        assert_eq!(imported_uv, &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);

        let compiled = crate::render::compile_10d::compile_asset(
            &source,
            Some("glb"),
            "urn:test:uv0-glb",
            "glb",
        )
        .unwrap();
        let mut decoded = vec![[0.0; 2]; compiled.mesh.vertex_count()];
        assert_eq!(
            crate::render::compile_10d::decode_10d_texture_coordinates_into(
                &compiled.container_10d,
                compiled.mesh.vertex_count(),
                &mut decoded,
            )
            .unwrap(),
            Some(compiled.mesh.vertex_count())
        );
        assert_eq!(decoded, *imported_uv);
    }

    #[test]
    fn glb_rejects_out_of_range_material_index() {
        let mut glb = glb_with_two_material_primitives();
        let json_chunk = glb
            .windows(4)
            .position(|window| window == CHUNK_JSON.to_le_bytes())
            .unwrap()
            + 4;
        let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let json = std::str::from_utf8(&glb[json_chunk..json_chunk + json_len]).unwrap();
        let updated = json.replace("\"material\":1", "\"material\":9");
        assert_eq!(updated.len(), json.len());
        glb[json_chunk..json_chunk + json_len].copy_from_slice(updated.as_bytes());
        assert!(matches!(
            import_glb_with_normals(&glb),
            Err(AssetError::Parse(_))
        ));
    }

    #[test]
    fn embedded_texture_reference_hashes_only_its_buffer_view_payload() {
        let gltf = serde_json::json!({
            "textures": [{"source": 0}],
            "images": [{"bufferView": 0, "mimeType":"image/png"}]
        });
        let mut dependencies = BTreeMap::new();
        let mut dependency_bytes = 0;
        let digest = texture_digest(
            &gltf,
            &serde_json::json!({"index": 0}),
            &[serde_json::json!({"byteOffset": 1, "byteLength": 2})],
            &[0, 11, 22, 33],
            "baseColorTexture",
            &mut dependencies,
            &mut dependency_bytes,
        )
        .unwrap();
        let expected: [u8; 32] = Sha256::digest([11, 22]).into();
        assert_eq!(digest, expected);
        let dependency = dependencies.get(&expected).unwrap();
        assert_eq!(dependency.bytes, [11, 22]);
        assert_eq!(dependency.mime_type, "image/png");

        let webp = serde_json::json!({
            "textures": [{"extensions": {"EXT_texture_webp": {"source": 0}}}],
            "images": [{"bufferView": 0, "mimeType": "image/webp"}]
        });
        let mut webp_dependencies = BTreeMap::new();
        let mut webp_dependency_bytes = 0;
        let webp_digest = texture_digest(
            &webp,
            &serde_json::json!({"index": 0}),
            &[serde_json::json!({"byteLength": 1})],
            &[0x52],
            "baseColorTexture",
            &mut webp_dependencies,
            &mut webp_dependency_bytes,
        )
        .unwrap();
        assert_eq!(webp_dependencies[&webp_digest].mime_type, "image/webp");
    }

    #[test]
    fn ktx2_image_payload_survives_glb_import_and_asset_compilation() {
        let glb = glb_with_embedded_ktx2();
        let imported = import_glb_with_normals(&glb).unwrap();
        let image_bytes = [0xAB, 0xCD, 0xEF, 0x42];
        let digest: [u8; 32] = Sha256::digest(image_bytes).into();
        assert_eq!(imported.texture_dependencies.len(), 1);
        assert_eq!(imported.texture_dependencies[0].digest, digest);
        assert_eq!(imported.texture_dependencies[0].mime_type, "image/ktx2");
        assert_eq!(imported.texture_dependencies[0].bytes, image_bytes);
        assert_eq!(imported.materials[0].base_color_texture, digest);
        assert_eq!(imported.materials[0].normal_texture, digest);

        let compiled = crate::render::compile_10d::compile_asset(
            &glb,
            Some("glb"),
            "urn:test:ktx2-texture",
            "glb",
        )
        .unwrap();
        assert_eq!(compiled.texture_dependencies, imported.texture_dependencies);
        assert_eq!(compiled.texture_dependencies.len(), 1);
        let expected_container = compiled.container_10d.clone();
        let package = compiled
            .build_hmc_bundle("assets/ktx2-material.10d")
            .unwrap();
        let bundle = crate::bundle::BundleReader::parse(&package).unwrap();
        let texture_key = crate::render::asset_package::texture_hmc_key(&digest);
        assert_eq!(
            bundle.get("assets/ktx2-material.10d").unwrap(),
            expected_container
        );
        assert_eq!(bundle.get(&texture_key).unwrap(), image_bytes);
        assert!(bundle.verify_entry(&texture_key));
        assert_eq!(bundle.entry(&texture_key).unwrap().kind, "image/ktx2");
        let resolved =
            crate::render::asset_package::resolve_hmc_texture_resource(&bundle, &digest).unwrap();
        assert_eq!(resolved.digest, digest);
        assert_eq!(resolved.mime_type, "image/ktx2");
        assert_eq!(resolved.bytes, image_bytes);

        let developmental = crate::render::compile_10d::compile_developmental_asset(
            &glb,
            Some("glb"),
            "urn:test:ktx2-developmental",
            "glb",
            42,
            12,
        )
        .unwrap();
        assert_eq!(developmental.texture_dependencies.len(), 1);
        let mut decoded_materials = [MaterialRecord::legacy_default(); 1];
        let mut decoded_ranges = [SubmeshRange {
            first_index: 0,
            index_count: 3,
            material_id: 1,
            semantic_id: 0,
        }];
        crate::render::material_compile::decode_10d_materials_into(
            &developmental.container_10d,
            3,
            &mut decoded_materials,
            &mut decoded_ranges,
        )
        .unwrap();
        assert_eq!(decoded_materials[0].base_color_texture, digest);
    }
}
