use super::*;
use crate::container_10d::{
    attach_texture_coordinate_section, MaterialRecord, OpacityMode, SubmeshRange, TextureFilterMode,
    TextureMinFilter, TextureSampler,
};
use crate::render::assets::Mesh;
use crate::render::compile_10d::{
    compile_mesh_to_10d, compile_mesh_to_10d_with_surface_reading,
};
use crate::render::portal_load_textures::{
    compile_10d_pbr, ingest_portal_texture, pack_pbr_set, PortalTextureIngest,
};
use crate::render::texture_ktx2::KTX2_IDENTIFIER;

const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
/// GPU fallback when `colors` is empty (`mesh_upload.rs`).
const GPU_GREY: [f32; 4] = [0.50, 0.60, 0.82, 1.0];

fn halves_mesh() -> Mesh {
    Mesh {
        positions: vec![
            [-1.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        triangles: vec![[0, 1, 2], [3, 4, 5]],
        min: [-1.0, -1.0, 0.0],
        max: [1.0, 1.0, 0.0],
    }
}

fn quad_mesh() -> Mesh {
    Mesh {
        positions: vec![
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
        min: [-1.0, -1.0, 0.0],
        max: [1.0, 1.0, 0.0],
    }
}

fn red_blue_reading() -> [[f32; 4]; 6] {
    [RED, RED, RED, BLUE, BLUE, BLUE]
}

fn is_grey(c: [f32; 4]) -> bool {
    (c[0] - GPU_GREY[0]).abs() < 0.02
        && (c[1] - GPU_GREY[1]).abs() < 0.02
        && (c[2] - GPU_GREY[2]).abs() < 0.02
}

fn is_red(c: [f32; 4]) -> bool {
    c[0] > 0.9 && c[1] < 0.1 && c[2] < 0.1
}

fn is_blue(c: [f32; 4]) -> bool {
    c[2] > 0.9 && c[0] < 0.1 && c[1] < 0.1
}

fn checker_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[
                255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 0, 0, 255,
            ])
            .unwrap();
    }
    bytes
}

fn ktx2_rgba8_fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; 144];
    bytes[..12].copy_from_slice(&KTX2_IDENTIFIER);
    bytes[12..16].copy_from_slice(&43u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&2u32.to_le_bytes());
    bytes[24..28].copy_from_slice(&1u32.to_le_bytes());
    bytes[36..40].copy_from_slice(&1u32.to_le_bytes());
    bytes[40..44].copy_from_slice(&1u32.to_le_bytes());
    bytes[48..52].copy_from_slice(&104u32.to_le_bytes());
    bytes[52..56].copy_from_slice(&28u32.to_le_bytes());
    bytes[104..108].copy_from_slice(&28u32.to_le_bytes());
    bytes[112..116].copy_from_slice(&((24u32 << 16) | 2).to_le_bytes());
    bytes[80..88].copy_from_slice(&136u64.to_le_bytes());
    bytes[88..96].copy_from_slice(&8u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&8u64.to_le_bytes());
    bytes[136..144].copy_from_slice(&[255, 0, 0, 255, 0, 0, 255, 255]);
    bytes
}

fn ktx2_basis_fixture() -> Vec<u8> {
    let mut bytes = ktx2_rgba8_fixture();
    bytes.resize(152, 0);
    bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    bytes[44..48].copy_from_slice(&1u32.to_le_bytes());
    bytes[64..72].copy_from_slice(&136u64.to_le_bytes());
    bytes[72..80].copy_from_slice(&8u64.to_le_bytes());
    bytes[80..88].copy_from_slice(&144u64.to_le_bytes());
    bytes[88..96].copy_from_slice(&4u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&0u64.to_le_bytes());
    bytes
}

fn nearest_sampler() -> TextureSampler {
    TextureSampler {
        wrap_s: crate::container_10d::TextureAddressMode::ClampToEdge,
        wrap_t: crate::container_10d::TextureAddressMode::ClampToEdge,
        mag_filter: TextureFilterMode::Nearest,
        min_filter: TextureMinFilter::Nearest,
    }
}

fn pbr_material(packed: &crate::render::portal_load_textures::PackedPbrSet) -> MaterialRecord {
    let mut material = MaterialRecord::legacy_default();
    material.opacity_mode = OpacityMode::Opaque;
    material.multiply_vertex_color = false;
    material.texture_samplers[0] = nearest_sampler();
    packed.apply_to(&mut material);
    material
}

fn pbr_range(material: &MaterialRecord, index_count: u32) -> SubmeshRange {
    SubmeshRange {
        first_index: 0,
        index_count,
        material_id: material.id,
        semantic_id: 0,
    }
}

#[test]
#[serial_test::serial(gpu)]
fn load_10d_surface_reading_red_blue_halves_are_not_grey() {
    let mesh = halves_mesh();
    let reading = red_blue_reading();
    let bytes = compile_mesh_to_10d_with_surface_reading(&mesh, None, &reading)
        .expect("seal .10d with SurfaceReading");
    let decoded = decode_load_10d(&bytes).expect("decode load_10d buffers");

    assert_eq!(
        decoded.colors.len(),
        6,
        "SurfaceReading must reach GPU upload"
    );
    assert!(decoded.uv0.is_none(), "fixture has no UV01 section");
    assert!(
        decoded.materials.is_empty(),
        "missing MAT is allowed; colour path stays"
    );
    assert!(
        decoded.colors.iter().copied().all(|c| !is_grey(c)),
        "load_10d must not pass grey/empty colours when SurfaceReading is present: {:?}",
        decoded.colors
    );
    assert!(decoded.colors[..3].iter().copied().all(is_red));
    assert!(decoded.colors[3..].iter().copied().all(is_blue));

    #[cfg(feature = "gpu-runtime")]
    upload_and_check_pixels(&decoded);
}

#[test]
fn load_10d_absent_surface_reading_keeps_empty_colors() {
    let bytes = compile_mesh_to_10d(&halves_mesh()).expect("seal mesh-only .10d");
    let decoded = decode_load_10d(&bytes).expect("decode mesh-only load_10d");
    assert!(
        decoded.colors.is_empty(),
        "grey/empty GPU colours only when SurfaceReading is absent"
    );
    assert!(decoded.materials.is_empty());
}

#[test]
fn load_10d_uv01_mismatch_is_fail_closed() {
    let mesh = halves_mesh();
    let reading = red_blue_reading();
    let bytes = compile_mesh_to_10d_with_surface_reading(&mesh, None, &reading).unwrap();
    let mismatched = attach_texture_coordinate_section(&bytes, &[[0.0, 0.0]; 3])
        .expect("attach wrong-count UV01");
    let err = decode_load_10d(&mismatched).expect_err("UV01 count mismatch");
    assert!(
        err.contains("UV01"),
        "expected UV01 fail-closed error, got {err}"
    );
}

#[test]
fn load_10d_uv01_is_uploaded_when_vertex_aligned() {
    let mesh = halves_mesh();
    let reading = red_blue_reading();
    let bytes = compile_mesh_to_10d_with_surface_reading(&mesh, None, &reading).unwrap();
    let uv = [
        [0.0, 0.0],
        [0.5, 0.0],
        [0.0, 1.0],
        [0.5, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
    ];
    let bytes = attach_texture_coordinate_section(&bytes, &uv).unwrap();
    let decoded = decode_load_10d(&bytes).expect("decode UV01 load_10d");
    assert_eq!(decoded.uv0.as_deref(), Some(uv.as_slice()));
    assert_eq!(decoded.colors.len(), 6);
}

#[test]
fn load_10d_mat3_checker_png_survives_portal_decode() {
    let png = checker_png();
    let packed = pack_pbr_set(Some(&png), None).expect("pack checker PNG");
    let albedo = packed.albedo.as_ref().expect("albedo slot");
    let mut material = pbr_material(&packed);
    material.metallic = 0.0;
    material.roughness = 1.0;
    let range = pbr_range(&material, 6);
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let bytes = compile_10d_pbr(&quad_mesh(), &[material], &[range], Some(&uv), None)
        .expect("compile_10d_pbr with MAT4 + UV01");
    let decoded = decode_load_10d(&bytes).expect("decode MAT3/MAT4 load_10d");

    assert_eq!(decoded.materials.len(), 1);
    assert_eq!(decoded.ranges.len(), 1);
    assert_eq!(decoded.materials[0].base_color_texture, albedo.digest);
    assert_eq!(decoded.materials[0].metallic, 0.0);
    assert_eq!(decoded.materials[0].roughness, 1.0);
    assert_eq!(decoded.uv0.as_deref(), Some(uv.as_slice()));
    assert!(
        decoded.colors.is_empty(),
        "vertex colour stays valid when SurfaceReading is absent"
    );

    match ingest_portal_texture(Some("image/png"), &png) {
        Ok(PortalTextureIngest::Rgba8(decoded_tex)) => {
            assert_eq!(decoded_tex.digest, albedo.digest);
            assert_eq!(decoded_tex.width, 2);
            assert_eq!(decoded_tex.height, 2);
            assert_eq!(
                &decoded_tex.rgba8,
                &[255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 0, 0, 255]
            );
        }
        other => panic!("checker PNG must decode to RGBA, got {other:?}"),
    }
}

#[test]
fn load_10d_mat3_metal_rough_factors_survive_without_texture() {
    let mut material = MaterialRecord::legacy_default();
    material.metallic = 1.0;
    material.roughness = 0.2;
    material.multiply_vertex_color = false;
    material.base_color = [0.8, 0.1, 0.1, 1.0];
    let range = pbr_range(&material, 6);
    let bytes = compile_10d_pbr(&quad_mesh(), &[material], &[range], None, None)
        .expect("compile metal-rough MAT");
    let decoded = decode_load_10d(&bytes).expect("decode metal-rough MAT");
    assert_eq!(decoded.materials.len(), 1);
    assert_eq!(decoded.materials[0].metallic, 1.0);
    assert!((decoded.materials[0].roughness - 0.2).abs() < 1e-6);
    assert_eq!(decoded.materials[0].base_color_texture, [0; 32]);
    assert!(decoded.uv0.is_none());
}

#[test]
fn load_10d_ktx2_rgba_ingests_and_basisu_is_skipped() {
    match ingest_portal_texture(Some("image/ktx2"), &ktx2_rgba8_fixture()) {
        Ok(PortalTextureIngest::Rgba8(decoded)) => {
            assert_eq!(decoded.width, 2);
            assert_eq!(decoded.height, 1);
            assert_eq!(&decoded.rgba8[..4], &[255, 0, 0, 255]);
        }
        other => panic!("uncompressed KTX2 RGBA must ingest, got {other:?}"),
    }
    assert_eq!(
        ingest_portal_texture(Some("image/ktx2"), &ktx2_basis_fixture()),
        Ok(PortalTextureIngest::SkippedTranscode)
    );
}

/// When a native adapter exists, upload the same buffers `load_10d` would and
/// assert the framebuffer is not the grey default. Without an adapter this
/// returns after the CPU colour-buffer checks above; that buffer is the
/// slice passed to GPU upload.
#[cfg(feature = "gpu-runtime")]
fn upload_and_check_pixels(decoded: &Load10dGpuBuffers) {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = crate::render::gpu::PortalGpu::new_offscreen(64, 64, 0)
        .expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);
    renderer.set_screen_space_ao_enabled(false);
    let tris = upload_load_10d_to_gpu(&mut renderer, decoded).expect("upload load_10d colours");
    assert_eq!(tris, decoded.triangle_count);
    renderer
        .render(0.0, &crate::render::telemetry::SystemTelemetry::default())
        .expect("draw SurfaceReading mesh");
    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("read SurfaceReading pixels");

    let mut red_pixels = 0usize;
    let mut blue_pixels = 0usize;
    for pixel in rgba.chunks_exact(4) {
        if pixel[0] > 40
            && pixel[0] > pixel[1].saturating_add(20)
            && pixel[0] > pixel[2].saturating_add(20)
        {
            red_pixels += 1;
        }
        if pixel[2] > 40
            && pixel[2] > pixel[1].saturating_add(20)
            && pixel[2] > pixel[0].saturating_add(20)
        {
            blue_pixels += 1;
        }
    }
    assert!(
        red_pixels > 10 && blue_pixels > 10,
        "GPU frame must show red and blue SurfaceReading halves, not grey (red={red_pixels} blue={blue_pixels})"
    );
}

#[cfg(feature = "gpu-runtime")]
#[test]
#[serial_test::serial(gpu)]
fn load_10d_checker_png_survives_portal_upload() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let png = checker_png();
    let packed = pack_pbr_set(Some(&png), None).expect("pack checker PNG");
    let material = pbr_material(&packed);
    let range = pbr_range(&material, 6);
    let uv = [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
    let bytes = compile_10d_pbr(&quad_mesh(), &[material], &[range], Some(&uv), None)
        .expect("compile checker MAT");
    let decoded = decode_load_10d(&bytes).expect("decode checker MAT");

    let mut renderer = crate::render::gpu::PortalGpu::new_offscreen(64, 64, 0)
        .expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);
    renderer.set_screen_space_ao_enabled(false);
    let tris = upload_load_10d_to_gpu_with_textures(&mut renderer, &decoded, &[&png])
        .expect("upload checker MAT + PNG");
    assert_eq!(tris, decoded.triangle_count);
    renderer
        .render(0.0, &crate::render::telemetry::SystemTelemetry::default())
        .expect("draw checker mesh");
    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("read checker pixels");

    let mut red_pixels = 0usize;
    let mut blue_pixels = 0usize;
    for pixel in rgba.chunks_exact(4) {
        if pixel[0] > 40
            && pixel[0] > pixel[1].saturating_add(20)
            && pixel[0] > pixel[2].saturating_add(20)
        {
            red_pixels += 1;
        }
        if pixel[2] > 40
            && pixel[2] > pixel[1].saturating_add(20)
            && pixel[2] > pixel[0].saturating_add(20)
        {
            blue_pixels += 1;
        }
    }
    assert!(
        red_pixels > 10 && blue_pixels > 10,
        "GPU frame must sample the 2x2 red/blue checker (red={red_pixels} blue={blue_pixels})"
    );
}
