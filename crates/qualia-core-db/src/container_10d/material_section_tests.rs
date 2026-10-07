use super::*;

fn material(id: u64) -> MaterialRecord {
    MaterialRecord {
        id,
        ..MaterialRecord::legacy_default()
    }
}

#[test]
fn mat3_round_trip_preserves_materials_ranges_transforms_samplers_and_digests() {
    let materials = [
        material(10),
        MaterialRecord {
            id: 20,
            shading_model: ShadingModel::Stylized,
            opacity_mode: OpacityMode::Mask,
            base_color: [0.2, 0.4, 0.8, 0.75],
            emissive: [0.1, 0.0, 0.3],
            metallic: 0.0,
            roughness: 0.65,
            normal_texture: [0x12; 32],
            metallic_roughness_texture: [0x56; 32],
            occlusion_texture: [0x78; 32],
            stylized_ramp_texture: [0x34; 32],
            texture_transforms: [
                TextureTransform {
                    offset: [0.25, -0.5],
                    scale: [2.0, 0.5],
                    rotation: 0.75,
                    tex_coord: 0,
                },
                TextureTransform::IDENTITY,
                TextureTransform::IDENTITY,
                TextureTransform::IDENTITY,
                TextureTransform::IDENTITY,
                TextureTransform::IDENTITY,
            ],
            texture_samplers: [
                TextureSampler {
                    wrap_s: TextureAddressMode::ClampToEdge,
                    wrap_t: TextureAddressMode::MirroredRepeat,
                    mag_filter: TextureFilterMode::Nearest,
                    min_filter: TextureMinFilter::NearestMipmapLinear,
                },
                TextureSampler::GLTF_DEFAULT,
                TextureSampler::GLTF_DEFAULT,
                TextureSampler::GLTF_DEFAULT,
                TextureSampler::GLTF_DEFAULT,
                TextureSampler::GLTF_DEFAULT,
            ],
            ..MaterialRecord::legacy_default()
        },
    ];
    let ranges = [
        SubmeshRange {
            first_index: 0,
            index_count: 6,
            material_id: 10,
            semantic_id: 101,
        },
        SubmeshRange {
            first_index: 6,
            index_count: 3,
            material_id: 20,
            semantic_id: 202,
        },
    ];
    let mut bytes = vec![0; encoded_len(materials.len(), ranges.len()).unwrap()];
    encode_material_section(&materials, &ranges, 9, &mut bytes).unwrap();
    assert_eq!(&bytes[..4], b"MAT3");
    assert_eq!(
        bytes[MATERIAL_SECTION_HEADER_SIZE + MATERIAL_RECORD_SIZE + 96
            ..MATERIAL_SECTION_HEADER_SIZE + MATERIAL_RECORD_SIZE + 128],
        [0x12; 32]
    );
    let mut decoded = [MaterialRecord::legacy_default(); 2];
    let mut decoded_ranges = [ranges[0]; 2];
    assert_eq!(
        decode_material_section_into(&bytes, 9, &mut decoded, &mut decoded_ranges),
        Ok((2, 2))
    );
    assert_eq!(decoded, materials);
    assert_eq!(decoded_ranges, ranges);
}

#[test]
fn mat3_rejects_overlap_unknown_material_and_nonfinite_values() {
    let materials = [material(10)];
    let overlap = [
        SubmeshRange {
            first_index: 0,
            index_count: 6,
            material_id: 10,
            semantic_id: 0,
        },
        SubmeshRange {
            first_index: 3,
            index_count: 3,
            material_id: 10,
            semantic_id: 0,
        },
    ];
    let mut bytes = vec![0; encoded_len(1, 2).unwrap()];
    assert!(matches!(
        encode_material_section(&materials, &overlap, 9, &mut bytes),
        Err(MaterialSectionError::RangeOrder { .. })
    ));
    let unknown = [SubmeshRange {
        material_id: 99,
        ..overlap[0]
    }];
    let mut bytes = vec![0; encoded_len(1, 1).unwrap()];
    assert!(matches!(
        encode_material_section(&materials, &unknown, 6, &mut bytes),
        Err(MaterialSectionError::UnknownMaterial { .. })
    ));
    let bad_material = [MaterialRecord {
        roughness: f32::NAN,
        ..materials[0]
    }];
    assert!(matches!(
        encode_material_section(&bad_material, &unknown, 6, &mut bytes),
        Err(MaterialSectionError::InvalidMaterial { .. })
    ));
}

#[test]
fn mat3_rejects_unknown_versions_and_trailing_bytes() {
    let materials = [material(10)];
    let ranges = [SubmeshRange {
        first_index: 0,
        index_count: 3,
        material_id: 10,
        semantic_id: 0,
    }];
    let mut bytes = vec![0; encoded_len(1, 1).unwrap()];
    encode_material_section(&materials, &ranges, 3, &mut bytes).unwrap();
    let mut out_materials = [materials[0]; 1];
    let mut out_ranges = [ranges[0]; 1];
    bytes[4] = 4;
    assert!(matches!(
        decode_material_section_into(&bytes, 3, &mut out_materials, &mut out_ranges),
        Err(MaterialSectionError::Version(4))
    ));
    bytes[4] = MATERIAL_SECTION_VERSION as u8;
    bytes.push(0);
    assert!(matches!(
        decode_material_section_into(&bytes, 3, &mut out_materials, &mut out_ranges),
        Err(MaterialSectionError::PayloadLength { .. })
    ));
}

#[test]
fn mat3_rejects_unknown_sampler_enums_and_reserved_bits() {
    let materials = [material(10)];
    let ranges = [SubmeshRange {
        first_index: 0,
        index_count: 3,
        material_id: 10,
        semantic_id: 0,
    }];
    let mut bytes = vec![0; encoded_len(1, 1).unwrap()];
    encode_material_section(&materials, &ranges, 3, &mut bytes).unwrap();
    let sampler = MATERIAL_SECTION_HEADER_SIZE + MATERIAL_V1_RECORD_SIZE + 6 * 24;
    let mut out_materials = materials;
    let mut out_ranges = ranges;
    bytes[sampler] = 3;
    assert_eq!(
        decode_material_section_into(&bytes, 3, &mut out_materials, &mut out_ranges),
        Err(MaterialSectionError::Layout)
    );
    bytes[sampler] = TextureAddressMode::Repeat as u8;
    bytes[sampler + 4] = 1;
    assert_eq!(
        decode_material_section_into(&bytes, 3, &mut out_materials, &mut out_ranges),
        Err(MaterialSectionError::Layout)
    );
}

#[test]
fn mat2_v2_decodes_transforms_and_uses_gltf_default_samplers() {
    let material = MaterialRecord {
        id: 10,
        texture_transforms: [
            TextureTransform {
                offset: [0.2, 0.3],
                scale: [2.0, 0.5],
                rotation: 0.4,
                tex_coord: 0,
            },
            TextureTransform::IDENTITY,
            TextureTransform::IDENTITY,
            TextureTransform::IDENTITY,
            TextureTransform::IDENTITY,
            TextureTransform::IDENTITY,
        ],
        ..MaterialRecord::legacy_default()
    };
    let ranges = [SubmeshRange {
        first_index: 0,
        index_count: 3,
        material_id: 10,
        semantic_id: 0,
    }];
    let mut bytes = vec![0; encoded_len(1, 1).unwrap()];
    encode_material_section(&[material], &ranges, 3, &mut bytes).unwrap();
    bytes[..4].copy_from_slice(&PREVIOUS_MAGIC);
    bytes[4..6].copy_from_slice(&PREVIOUS_VERSION.to_le_bytes());
    bytes[MATERIAL_SECTION_HEADER_SIZE + MATERIAL_V1_RECORD_SIZE + 6 * 24
        ..MATERIAL_SECTION_HEADER_SIZE + MATERIAL_RECORD_SIZE]
        .fill(0);
    let mut decoded = [MaterialRecord::legacy_default(); 1];
    let mut decoded_ranges = ranges;
    assert_eq!(
        decode_material_section_into(&bytes, 3, &mut decoded, &mut decoded_ranges),
        Ok((1, 1))
    );
    assert_eq!(decoded[0].texture_transforms, material.texture_transforms);
    assert_eq!(
        decoded[0].texture_samplers,
        [TextureSampler::GLTF_DEFAULT; 6]
    );
}

#[test]
fn mat1_v1_decodes_with_identity_texture_transforms() {
    let material = material(10);
    let ranges = [SubmeshRange {
        first_index: 0,
        index_count: 3,
        material_id: 10,
        semantic_id: 0,
    }];
    let mut bytes = vec![0u8; MATERIAL_SECTION_HEADER_SIZE + MATERIAL_V1_RECORD_SIZE + 24];
    bytes[..4].copy_from_slice(&LEGACY_MAGIC);
    bytes[4..6].copy_from_slice(&LEGACY_VERSION.to_le_bytes());
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&(MATERIAL_V1_RECORD_SIZE as u32).to_le_bytes());
    bytes[20..24].copy_from_slice(&(SUBMESH_RANGE_SIZE as u32).to_le_bytes());
    encode_material(
        &material,
        &mut bytes
            [MATERIAL_SECTION_HEADER_SIZE..MATERIAL_SECTION_HEADER_SIZE + MATERIAL_V1_RECORD_SIZE],
    );
    let range_start = MATERIAL_SECTION_HEADER_SIZE + MATERIAL_V1_RECORD_SIZE;
    bytes[range_start..range_start + 4].copy_from_slice(&0u32.to_le_bytes());
    bytes[range_start + 4..range_start + 8].copy_from_slice(&3u32.to_le_bytes());
    bytes[range_start + 8..range_start + 16].copy_from_slice(&10u64.to_le_bytes());
    let mut decoded = [MaterialRecord::legacy_default(); 1];
    let mut decoded_ranges = [ranges[0]; 1];
    assert_eq!(
        decode_material_section_into(&bytes, 3, &mut decoded, &mut decoded_ranges),
        Ok((1, 1))
    );
    assert_eq!(decoded[0], material);
    assert_eq!(
        decoded[0].texture_transforms,
        [TextureTransform::IDENTITY; 6]
    );
    assert_eq!(
        decoded[0].texture_samplers,
        [TextureSampler::GLTF_DEFAULT; 6]
    );
}

#[test]
fn sampler_cache_indices_cover_every_supported_state_exactly_once() {
    let addresses = [
        TextureAddressMode::ClampToEdge,
        TextureAddressMode::MirroredRepeat,
        TextureAddressMode::Repeat,
    ];
    let filters = [TextureFilterMode::Nearest, TextureFilterMode::Linear];
    let min_filters = [
        TextureMinFilter::Nearest,
        TextureMinFilter::Linear,
        TextureMinFilter::NearestMipmapNearest,
        TextureMinFilter::LinearMipmapNearest,
        TextureMinFilter::NearestMipmapLinear,
        TextureMinFilter::LinearMipmapLinear,
    ];
    let mut seen = [false; 108];
    for wrap_s in addresses {
        for wrap_t in addresses {
            for mag_filter in filters {
                for min_filter in min_filters {
                    let index = TextureSampler {
                        wrap_s,
                        wrap_t,
                        mag_filter,
                        min_filter,
                    }
                    .cache_index();
                    assert!(index < seen.len());
                    assert!(!seen[index]);
                    seen[index] = true;
                }
            }
        }
    }
    assert!(seen.into_iter().all(|value| value));
}
