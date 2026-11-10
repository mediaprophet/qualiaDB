//! Native pixel contracts for portal MAT3/MAT4 PNG ingest and metal-rough factors.

use super::*;
use crate::container_10d::{
    MaterialRecord, OpacityMode, SubmeshRange, TextureAddressMode, TextureFilterMode,
    TextureMinFilter, TextureSampler,
};
use crate::render::assets::Mesh;
use crate::render::portal_load_10d::{
    decode_load_10d, upload_load_10d_to_gpu, upload_load_10d_to_gpu_with_textures,
};
use crate::render::portal_load_textures::{compile_10d_pbr, pack_pbr_set};

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

fn require_gpu() -> bool {
    if crate::wgsl_forge::test_gpu_available() {
        true
    } else {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        false
    }
}

fn count_red_blue(rgba: &[u8]) -> (usize, usize) {
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
    (red_pixels, blue_pixels)
}

#[test]
#[serial_test::serial(gpu)]
fn native_portal_mat3_checker_png_is_sampled_with_uv0() {
    if !require_gpu() {
        return;
    }

    let png = checker_png();
    let packed = pack_pbr_set(Some(&png), None).expect("pack checker PNG");
    let mut material = MaterialRecord::legacy_default();
    material.opacity_mode = OpacityMode::Opaque;
    material.multiply_vertex_color = false;
    material.texture_samplers[0] = TextureSampler {
        wrap_s: TextureAddressMode::ClampToEdge,
        wrap_t: TextureAddressMode::ClampToEdge,
        mag_filter: TextureFilterMode::Nearest,
        min_filter: TextureMinFilter::Nearest,
    };
    packed.apply_to(&mut material);
    let range = SubmeshRange {
        first_index: 0,
        index_count: 6,
        material_id: material.id,
        semantic_id: 0,
    };
    let uv = [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
    let bytes = compile_10d_pbr(&quad_mesh(), &[material], &[range], Some(&uv), None)
        .expect("compile_10d_pbr checker");
    let decoded = decode_load_10d(&bytes).expect("decode MAT checker");
    assert_eq!(
        decoded.materials[0].base_color_texture,
        packed.albedo.as_ref().unwrap().digest
    );

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);
    renderer.set_screen_space_ao_enabled(false);
    upload_load_10d_to_gpu_with_textures(&mut renderer, &decoded, &[&png])
        .expect("portal PNG ingest + MAT upload");
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("draw checker");
    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer.read_rgba8_into(&mut rgba).expect("read checker");
    let (red_pixels, blue_pixels) = count_red_blue(&rgba);
    assert!(
        red_pixels > 10 && blue_pixels > 10,
        "portal MAT3 albedo PNG must sample red and blue checker texels (red={red_pixels} blue={blue_pixels})"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_portal_metal_rough_factors_apply_without_texture() {
    if !require_gpu() {
        return;
    }

    let mut metal = MaterialRecord::legacy_default();
    metal.id = 1;
    metal.metallic = 1.0;
    metal.roughness = 0.2;
    metal.multiply_vertex_color = false;
    metal.base_color = [0.9, 0.15, 0.12, 1.0];
    let mut dielectric = metal;
    dielectric.id = 2;
    dielectric.metallic = 0.0;
    dielectric.roughness = 1.0;
    let mesh = Mesh {
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
    };
    let ranges = [
        SubmeshRange {
            first_index: 0,
            index_count: 3,
            material_id: metal.id,
            semantic_id: 0,
        },
        SubmeshRange {
            first_index: 3,
            index_count: 3,
            material_id: dielectric.id,
            semantic_id: 0,
        },
    ];
    let bytes = compile_10d_pbr(&mesh, &[metal, dielectric], &ranges, None, None)
        .expect("compile metal-rough pair");
    let decoded = decode_load_10d(&bytes).expect("decode metal-rough pair");
    assert_eq!(decoded.materials.len(), 2);
    let metal_rec = decoded
        .materials
        .iter()
        .find(|material| material.id == 1)
        .expect("metal record");
    let dielectric_rec = decoded
        .materials
        .iter()
        .find(|material| material.id == 2)
        .expect("dielectric record");
    assert!((metal_rec.metallic - 1.0).abs() < 1e-6);
    assert!((metal_rec.roughness - 0.2).abs() < 1e-6);
    assert!((dielectric_rec.metallic - 0.0).abs() < 1e-6);
    assert!((dielectric_rec.roughness - 1.0).abs() < 1e-6);

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);
    renderer.set_screen_space_ao_enabled(false);
    upload_load_10d_to_gpu(&mut renderer, &decoded).expect("upload metal-rough MAT");
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("draw metal-rough");
    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("read metal-rough");
    let shaded = rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8)
        .count();
    assert!(
        shaded > 10,
        "metal-rough MAT factors must still shade the mesh without a texture, shaded={shaded}"
    );
}
