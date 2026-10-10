use super::*;
use crate::scene_contract::{SceneEdge, SceneFace};
use crate::volumetric_texture::coarse_texture_level;
use qualia_core_db::render::gpu::TextureMipSemantic;

#[test]
fn coarse_texture_level_reduces_cpu_base_to_requested_physical_extent() {
    let source = [
        0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255,
        255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
    ];
    let (coarse, width, height) =
        coarse_texture_level(&source, 4, 2, 1, TextureMipSemantic::Color).unwrap();
    assert_eq!((width, height), (2, 1));
    assert_eq!(coarse.len(), 8);
    assert!(
        coarse[0] > 128,
        "sRGB reduction should average in linear light"
    );
    assert_eq!(coarse[3], 255);
}

#[test]
fn coarse_alpha_mask_preserves_authored_coverage() {
    let mut source = [255u8; 4 * 4 * 4];
    for pixel in 8..16 {
        source[pixel * 4 + 3] = 0;
    }
    let (coarse, width, height) = coarse_texture_level(
        &source,
        4,
        4,
        1,
        TextureMipSemantic::alpha_mask(0.5).unwrap(),
    )
    .unwrap();
    assert_eq!((width, height), (2, 2));
    let covered = coarse
        .chunks_exact(4)
        .filter(|pixel| pixel[3] >= 128)
        .count();
    assert_eq!(
        covered, 2,
        "coarse alpha coverage should match 8/16 base pixels"
    );
}

#[test]
fn scene_mesh_preserves_css_color_and_alpha() {
    let mut scene = RenderScene::new();
    scene.add_face(SceneFace {
        vertices: vec![
            ScenePoint {
                x: 0.1,
                y: 0.1,
                z: 0.0,
            },
            ScenePoint {
                x: 0.9,
                y: 0.1,
                z: 0.0,
            },
            ScenePoint {
                x: 0.5,
                y: 0.9,
                z: 0.0,
            },
        ],
        color: "#ff0000".to_string(),
        alpha: 0.5,
    });
    scene.add_edge(SceneEdge {
        from: ScenePoint {
            x: 0.0,
            y: 0.0,
            z: 0.1,
        },
        to: ScenePoint {
            x: 1.0,
            y: 1.0,
            z: 0.1,
        },
        color: "#00ff00".to_string(),
        width: 2.0,
        alpha: 0.75,
    });

    let (positions, colors, indices) = scene_mesh(&scene, 100, 100);
    assert_eq!(positions.len(), 7);
    assert_eq!(colors.len(), positions.len());
    assert_eq!(indices.len(), 9);
    assert_eq!(colors[0], [1.0, 0.0, 0.0, 0.5]);
    assert_eq!(colors[3], [0.0, 1.0, 0.0, 0.75]);
}

// ── P9.3 tests ────────────────────────────────────────────────────────

#[test]
fn colour_by_field_is_deterministic() {
    let a = VolumetricRenderer::colour_by_field(0.5, 0.0, 1.0);
    let b = VolumetricRenderer::colour_by_field(0.5, 0.0, 1.0);
    assert_eq!(a, b);
}

#[test]
fn colour_by_field_endpoints() {
    let blue = VolumetricRenderer::colour_by_field(0.0, 0.0, 1.0);
    assert_eq!(blue, [0.0, 0.0, 1.0]);
    let red = VolumetricRenderer::colour_by_field(1.0, 0.0, 1.0);
    assert_eq!(red, [1.0, 0.0, 0.0]);
}

#[test]
fn colour_by_field_midpoint_is_green() {
    let green = VolumetricRenderer::colour_by_field(0.5, 0.0, 1.0);
    assert!((green[0] - 0.0).abs() < 1e-6);
    assert!((green[1] - 1.0).abs() < 1e-6);
    assert!((green[2] - 0.0).abs() < 1e-6);
}

#[test]
fn colour_by_field_degenerate_range() {
    let c = VolumetricRenderer::colour_by_field(42.0, 42.0, 42.0);
    // Degenerate range → t=0.5 → green
    assert!((c[1] - 1.0).abs() < 1e-6);
}

#[test]
fn temporal_scrub_returns_in_window_nodes() {
    use qualia_core_db::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
    use qualia_core_db::tensor::Tensor10D;

    let tensors = [
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 1.0, 0.0, 0.0),
    ];
    let mut buf = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
    write_tensor_buffer(&tensors, &mut buf).unwrap();

    // Window around t=0.5 with width 0.6 → [0.2, 0.8] → only node 1 (t=0.5)
    let result = VolumetricRenderer::temporal_scrub(&buf, 0.5, 0.6).unwrap();
    assert_eq!(result, vec![1]);
}

#[test]
fn temporal_scrub_empty_window() {
    use qualia_core_db::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
    use qualia_core_db::tensor::Tensor10D;

    let tensors = [
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 10.0, 1.0, 0.0, 0.0),
    ];
    let mut buf = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
    write_tensor_buffer(&tensors, &mut buf).unwrap();

    let result = VolumetricRenderer::temporal_scrub(&buf, 5.0, 0.0).unwrap();
    assert!(
        result.is_empty(),
        "zero-width window at t=5 should match no nodes"
    );
}

#[test]
fn temporal_scrub_matches_linear_scan_oracle() {
    use qualia_core_db::tensor::buffer_export::{
        read_tensor_at, tensor_node_count, write_tensor_buffer, TensorBufferHeader,
    };
    use qualia_core_db::tensor::Tensor10D;

    let tensors: Vec<Tensor10D> = (0..20)
        .map(|i| {
            Tensor10D::new(
                0.0,
                0.0,
                0.0,
                i as f32 * 0.1,
                0.0,
                0.0,
                i as f32 * 0.3,
                1.0,
                0.0,
                0.0,
            )
        })
        .collect();
    let mut buf = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
    write_tensor_buffer(&tensors, &mut buf).unwrap();

    let t_slice = 3.0f32;
    let t_window = 2.0f32;
    let half = t_window * 0.5;
    let lo = t_slice - half;
    let hi = t_slice + half;

    // Linear-scan oracle
    let count = tensor_node_count(&buf).unwrap();
    let mut oracle = Vec::new();
    for i in 0..count {
        let t = read_tensor_at(&buf, i).unwrap();
        if t.t >= lo && t.t <= hi {
            oracle.push(i as u32);
        }
    }

    let result = VolumetricRenderer::temporal_scrub(&buf, t_slice, t_window).unwrap();
    assert_eq!(
        result, oracle,
        "temporal_scrub must match linear-scan oracle"
    );
}

// ── HMC Texture Streaming & Mip Selection tests ──────────────────────────

fn ktx2_fixture(vk_format: u32, base_w: u32, base_h: u32, levels: &[&[u8]]) -> Vec<u8> {
    use qualia_core_db::render::texture_ktx2::KTX2_IDENTIFIER;
    let level_count = levels.len();
    let index_bytes = level_count * 24;
    let dfd_offset = 80 + index_bytes;
    let dfd_len = 28usize;
    let metadata_end = dfd_offset + dfd_len;

    let mut current_offset = metadata_end;
    let mut level_ranges = Vec::new();
    for payload in levels {
        let start = current_offset;
        let end = start + payload.len();
        level_ranges.push((start, payload.len()));
        current_offset = end;
    }

    let mut bytes = vec![0u8; current_offset];
    bytes[..12].copy_from_slice(&KTX2_IDENTIFIER);
    bytes[12..16].copy_from_slice(&vk_format.to_le_bytes());
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes()); // type_size
    bytes[20..24].copy_from_slice(&base_w.to_le_bytes());
    bytes[24..28].copy_from_slice(&base_h.to_le_bytes());
    bytes[28..32].copy_from_slice(&0u32.to_le_bytes()); // depth
    bytes[32..36].copy_from_slice(&0u32.to_le_bytes()); // layer_count
    bytes[36..40].copy_from_slice(&1u32.to_le_bytes()); // face_count
    bytes[40..44].copy_from_slice(&(level_count as u32).to_le_bytes());
    bytes[44..48].copy_from_slice(&0u32.to_le_bytes()); // supercompression
    bytes[48..52].copy_from_slice(&(dfd_offset as u32).to_le_bytes());
    bytes[52..56].copy_from_slice(&(dfd_len as u32).to_le_bytes());

    bytes[dfd_offset..dfd_offset + 4].copy_from_slice(&28u32.to_le_bytes());
    bytes[dfd_offset + 8..dfd_offset + 12].copy_from_slice(&((24u32 << 16) | 2).to_le_bytes());

    for (i, &(offset, len)) in level_ranges.iter().enumerate() {
        let entry_offset = 80 + i * 24;
        bytes[entry_offset..entry_offset + 8].copy_from_slice(&(offset as u64).to_le_bytes());
        bytes[entry_offset + 8..entry_offset + 16].copy_from_slice(&(len as u64).to_le_bytes());
        bytes[entry_offset + 16..entry_offset + 24].copy_from_slice(&(len as u64).to_le_bytes());
        bytes[offset..offset + len].copy_from_slice(levels[i]);
    }
    bytes
}

fn build_test_hmc_bundle(
    texture_bytes: &[u8],
    texture_mime: &str,
    is_normal: bool,
) -> (Vec<u8>, [u8; 32]) {
    use qualia_core_db::bundle::BundleWriter;
    use qualia_core_db::container_10d::{attach_material_section, MaterialRecord, SubmeshRange};
    use qualia_core_db::render::asset_package::texture_hmc_key;

    let digest = qualia_core_db::q42::asset_envelope::sha256_of(texture_bytes);

    let asset = qualia_core_db::render::compile_10d::compile_asset(
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        Some("obj"),
        "urn:test:hmc-stream",
        "obj",
    )
    .unwrap();

    let mut mat = MaterialRecord::legacy_default();
    mat.id = 1;
    if is_normal {
        mat.normal_texture = digest;
    } else {
        mat.base_color_texture = digest;
    }

    let range = SubmeshRange {
        first_index: 0,
        index_count: 3,
        material_id: 1,
        semantic_id: 42,
    };

    let container_with_mat =
        attach_material_section(&asset.container_10d, &[mat], &[range], 3).unwrap();

    let mut writer = BundleWriter::new();
    writer
        .add_file("asset.10d", "10d", container_with_mat, None)
        .unwrap();
    writer
        .add_file(
            &texture_hmc_key(&digest),
            texture_mime,
            texture_bytes.to_vec(),
            None,
        )
        .unwrap();

    (writer.build().unwrap(), digest)
}

#[test]
fn hmc_multilevel_ktx2_selects_and_decodes_coarse_level() {
    let mut renderer = match VolumetricRenderer::new_offscreen(64, 64, 64) {
        Ok(r) => r,
        Err(_) => return, // Headless environment without adapter
    };

    // Keep level zero structurally present but invalid for RGBA8 (4B instead of 64B). A
    // successful load therefore proves the HMC path selected and decoded the authored 2x2 level
    // directly rather than falling back to a base-level decode.
    let lvl0 = [0xAAu8; 4];
    let lvl1 = [0xBBu8; 16];
    let lvl2 = [0xCCu8; 4];
    let ktx2 = ktx2_fixture(43, 4, 4, &[&lvl0, &lvl1, &lvl2]); // SRGB

    let (hmc_bytes, _) = build_test_hmc_bundle(&ktx2, "image/ktx2", false);

    // Request with projected footprint 2x2: should select mip 1
    let request = HmcTextureStreamRequest {
        budget: TextureStreamBudget {
            max_resident_bytes: 1_000_000,
            max_upload_bytes: 1_000_000,
        },
        projected_width: 2,
        projected_height: 2,
    };

    let (loaded, report) = renderer
        .load_hmc_asset_with_texture_request(&hmc_bytes, "asset.10d", request)
        .unwrap();

    assert_eq!(loaded.1, 1); // triangle count
    assert_eq!(report.requested_interpretations, 1);
    assert_eq!(report.resident_interpretations, 1);
    assert_eq!(report.deferred_interpretations, 0);
    assert_eq!(report.admitted_mips, 1);
    assert_eq!(report.admitted_upload_bytes, 16);
}

#[test]
fn texture_stream_generation_refines_rebinds_and_evicts_deterministically() {
    let mut renderer = match VolumetricRenderer::new_offscreen(64, 64, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };
    let levels = [vec![0x10u8; 64], vec![0x20u8; 16], vec![0x30u8; 4]];
    let level_refs = levels.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let ktx2 = ktx2_fixture(43, 4, 4, &level_refs);
    let (hmc_bytes, digest) = build_test_hmc_bundle(&ktx2, "image/ktx2", false);
    renderer
        .load_hmc_asset_with_texture_request(
            &hmc_bytes,
            "asset.10d",
            HmcTextureStreamRequest {
                budget: TextureStreamBudget {
                    max_resident_bytes: 1024,
                    max_upload_bytes: 1024,
                },
                projected_width: 2,
                projected_height: 2,
            },
        )
        .unwrap();

    let refine_request = [HmcTextureResidencyRequest {
        digest,
        color_space: qualia_core_db::render::gpu::TextureColorSpace::Srgb,
        mip_semantic: TextureMipSemantic::Color,
        projected_width: 4,
        projected_height: 4,
        importance: 100,
        distance_key: 0,
        last_used_frame: 1,
        pinned: true,
        visible: true,
    }];
    let refine = renderer
        .request_texture_stream(HmcTextureStreamApplyRequest {
            budget: TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 1024,
            },
            requests: &refine_request,
        })
        .unwrap();
    assert_eq!(refine.generation, 0);
    assert_eq!(refine.admitted_count, 1);
    let stale = refine.clone();
    renderer.apply_texture_stream(refine).unwrap();
    assert!(renderer.apply_texture_stream(stale).is_err());

    let evict = renderer
        .request_texture_stream(HmcTextureStreamApplyRequest {
            budget: TextureStreamBudget {
                max_resident_bytes: 1024,
                max_upload_bytes: 1024,
            },
            requests: &[],
        })
        .unwrap();
    assert_eq!(evict.generation, 1);
    assert_eq!(evict.admitted_count, 0);
    assert_eq!(evict.action_count, 1);
    renderer.apply_texture_stream(evict).unwrap();
}

#[test]
fn hmc_constrained_budget_defers_texture_while_mesh_loads() {
    let mut renderer = match VolumetricRenderer::new_offscreen(64, 64, 64) {
        Ok(r) => r,
        Err(_) => return,
    };

    let lvl0 = [0xAAu8; 64];
    let ktx2 = ktx2_fixture(43, 4, 4, &[&lvl0]);

    let (hmc_bytes, _) = build_test_hmc_bundle(&ktx2, "image/ktx2", false);

    // Constrained budget: 0 upload bytes allowed
    let request = HmcTextureStreamRequest {
        budget: TextureStreamBudget {
            max_resident_bytes: 0,
            max_upload_bytes: 0,
        },
        projected_width: 0,
        projected_height: 0,
    };

    let (loaded, report) = renderer
        .load_hmc_asset_with_texture_request(&hmc_bytes, "asset.10d", request)
        .unwrap();

    assert_eq!(loaded.1, 1);
    assert_eq!(report.requested_interpretations, 1);
    assert_eq!(report.resident_interpretations, 0);
    assert_eq!(report.deferred_interpretations, 1);
}

#[test]
fn hmc_color_space_mismatch_is_deferred_while_mesh_loads() {
    let mut renderer = match VolumetricRenderer::new_offscreen(64, 64, 64) {
        Ok(r) => r,
        Err(_) => return,
    };

    let lvl0 = [0x80u8; 16];
    // Normal map requires UNORM, but fixture specifies SRGB (43)
    let ktx2 = ktx2_fixture(43, 2, 2, &[&lvl0]);

    let (hmc_bytes, _) = build_test_hmc_bundle(&ktx2, "image/ktx2", true);

    let request = HmcTextureStreamRequest::default();
    let (loaded, report) = renderer
        .load_hmc_asset_with_texture_request(&hmc_bytes, "asset.10d", request)
        .unwrap();

    assert_eq!(loaded.1, 1);
    assert_eq!(report.requested_interpretations, 1);
    assert_eq!(report.resident_interpretations, 0);
    assert_eq!(report.deferred_interpretations, 1);
}

#[test]
fn hmc_unsupported_ktx2_format_uses_typed_deferred_fallback() {
    let mut renderer = match VolumetricRenderer::new_offscreen(64, 64, 64) {
        Ok(r) => r,
        Err(_) => return,
    };

    // VK_FORMAT_BC1_RGB_UNORM_BLOCK is structurally valid KTX2, but the existing HMC path only
    // admits authored uncompressed RGBA8 levels. The mesh must still load while the texture is
    // deferred to the renderer's typed material fallback.
    let level = [0x5Au8; 16];
    let ktx2 = ktx2_fixture(131, 2, 2, &[&level]);
    let (hmc_bytes, _) = build_test_hmc_bundle(&ktx2, "image/ktx2", false);

    let (loaded, report) = renderer
        .load_hmc_asset_with_texture_request(
            &hmc_bytes,
            "asset.10d",
            HmcTextureStreamRequest::default(),
        )
        .unwrap();

    assert_eq!(loaded.1, 1);
    assert_eq!(report.requested_interpretations, 1);
    assert_eq!(report.resident_interpretations, 0);
    assert_eq!(report.deferred_interpretations, 1);
    assert_eq!(report.admitted_mips, 0);
    assert_eq!(report.capability_refusals, 1);
}

#[test]
fn hmc_corrupted_texture_is_deferred_while_mesh_loads() {
    let mut renderer = match VolumetricRenderer::new_offscreen(64, 64, 64) {
        Ok(r) => r,
        Err(_) => return,
    };

    // Corrupted payload that cannot be parsed as KTX2
    let corrupted = [0xFFu8; 32];
    let (hmc_bytes, _) = build_test_hmc_bundle(&corrupted, "image/ktx2", false);

    let request = HmcTextureStreamRequest::default();
    let (loaded, report) = renderer
        .load_hmc_asset_with_texture_request(&hmc_bytes, "asset.10d", request)
        .unwrap();

    assert_eq!(loaded.1, 1);
    assert_eq!(report.requested_interpretations, 1);
    assert_eq!(report.resident_interpretations, 0);
    assert_eq!(report.deferred_interpretations, 1);
}

#[test]
fn frame_graph_scheduler_integration_and_pass_sequence() {
    use crate::pipeline::WebizenFrameScheduler;
    use qualia_core_db::render::frame_graph::PassId;
    use qualia_core_db::render::quality_profiles::{QualityTier, RenderQualityProfile};

    let mut scheduler = WebizenFrameScheduler::new();
    let profile = RenderQualityProfile {
        tier: QualityTier::High,
        render_scale_bps: 10_000,
        shadows_enabled: true,
        shadow_map_dimension: 2048,
        shadow_cascade_count: 2,
        ambient_occlusion_enabled: true,
        ao_scale_bps: 5_000,
        bloom_enabled: true,
        bloom_levels: 3,
        texture_residency_bytes: 1024 * 1024 * 1024,
        hdr_enabled: true,
        volume_projection_enabled: false,
    };

    let schedule = scheduler
        .plan_frame(&profile, 1920, 1080, None)
        .expect("plan frame");

    assert!(schedule.pass_count >= 5);
    let passes = &schedule.passes[..schedule.pass_count];
    assert!(passes.iter().any(|&p| p == Some(PassId::ForwardLighting)));
    assert!(passes.iter().any(|&p| p == Some(PassId::DepthPrepass)));
    assert!(passes.iter().any(|&p| p == Some(PassId::AoCompute)));
}

#[test]
fn lighting_and_ao_quality_evaluation_integration() {
    use qualia_core_db::render::ao_quality::*;
    use qualia_core_db::render::lighting::*;

    let surface = SurfaceParameters {
        base_color: [0.8, 0.8, 0.8],
        metallic: 0.0,
        roughness: 0.2,
        reflectance_f0: 0.04,
        emissive: [0.0; 3],
        ambient_occlusion: 0.9,
    };
    let sun = DirectionalLight {
        direction: [0.0, -1.0, 0.0],
        illuminance_lux: 100.0,
        color_linear: [1.0, 1.0, 1.0],
        cast_shadows: true,
    };
    let stylized = StylizedLightingParams {
        diffuse_bands: 2,
        band_softness: 0.05,
        rim_exponent: 3.0,
        rim_intensity: 1.0,
        rim_tint: [1.0, 1.0, 1.0],
        ..Default::default()
    };

    let color = evaluate_fragment_radiance(
        &surface,
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        Some(&sun),
        1.0,
        [0.1, 0.1, 0.1],
        Some(&stylized),
    );
    assert!(color[0] > 0.0);

    let weights = bilateral_upsample_quad_weights(
        10.0,
        [0.0, 1.0, 0.0],
        [10.0, 10.01, 9.99, 10.02],
        [[0.0, 1.0, 0.0]; 4],
        [0.25; 4],
        0.1,
        4.0,
    );
    let sum: f32 = weights.iter().sum();
    assert!((sum - 1.0).abs() < 1e-4);
}

#[test]
fn hmc_water_geometry_admission_rejects_non_finite_and_budget_overflow() {
    use crate::volumetric_hmc::validate_hmc_water_geometry;

    let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(validate_hmc_water_geometry(&positions, &[[0, 1, 2]], 1024), Ok(72));
    assert!(validate_hmc_water_geometry(&positions, &[[0, 1, 2]], 71).is_err());
    assert!(validate_hmc_water_geometry(
        &[[0.0, f32::NAN, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        &[[0, 1, 2]],
        1024,
    )
    .is_err());
    assert!(validate_hmc_water_geometry(&positions, &[[0, 1, 3]], 1024).is_err());
}

#[test]
fn hmc_water_quality_falls_back_without_changing_geometry_admission() {
    use crate::volumetric_hmc::{select_hmc_water_quality, HmcWaterLoadPolicy};

    assert_eq!(HmcWaterLoadPolicy::default().preferred_quality, 2);
    assert_eq!(select_hmc_water_quality(10, 1024, 2), 2);
    assert_eq!(select_hmc_water_quality(300_001, 1024, 2), 1);
    assert_eq!(select_hmc_water_quality(750_001, 1024, 2), 0);
    assert_eq!(select_hmc_water_quality(10, 1024, 0), 0);
}
