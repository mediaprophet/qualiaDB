//! Native pixel contracts for material coverage and texture interpretation.

use super::*;

fn read_ao_depth_code(renderer: &PortalGpu, x: u32, y: u32) -> u16 {
    let ao = renderer
        .ao_targets
        .as_ref()
        .expect("AO target admitted for pixel test");
    let (width, height) = ao.extent();
    assert!(x < width && y < height, "AO probe outside {width}x{height}");

    let staging = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("alpha-mask-ao-test-readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("alpha-mask-ao-test-copy"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: ao.normal_texture_for_test(),
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    renderer.queue.submit(std::iter::once(encoder.finish()));
    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = renderer.device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv()
        .expect("AO readback callback")
        .expect("AO readback mapping");
    let mapped = slice.get_mapped_range().expect("mapped AO pixel");
    let code = u16::from_be_bytes([mapped[2], mapped[3]]);
    drop(mapped);
    staging.unmap();
    code
}

#[test]
#[serial_test::serial(gpu)]
fn native_mask_material_discards_below_cutoff_and_keeps_opaque_samples() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(8.0 / 255.0, 13.0 / 255.0, 20.0 / 255.0, 1.0);
    renderer.set_lighting(0.3, 1.0, 0.4, 0.25, 1.0);

    let digest = [0x6D; 32];
    renderer
        .upload_resident_texture_rgba8_with_mips(
            digest,
            TextureColorSpace::Srgb,
            TextureMipSemantic::alpha_mask(0.5).unwrap(),
            2,
            1,
            &[255, 255, 255, 0, 255, 255, 255, 255],
        )
        .expect("resident alpha-mask texture");
    let mut material = crate::container_10d::MaterialRecord::legacy_default();
    material.opacity_mode = crate::container_10d::OpacityMode::Mask;
    material.alpha_cutoff = 0.5;
    material.base_color_texture = digest;
    let range = crate::container_10d::SubmeshRange {
        first_index: 0,
        index_count: 12,
        material_id: material.id,
        semantic_id: 0,
    };

    // Two quads share a material: the left samples the zero-alpha texel, while the right
    // samples the fully opaque texel. Separate constant UVs make coverage unambiguous.
    let positions = [
        [-0.9, -0.7, 0.0],
        [-0.1, -0.7, 0.0],
        [-0.1, 0.7, 0.0],
        [-0.9, 0.7, 0.0],
        [0.1, -0.7, 0.0],
        [0.9, -0.7, 0.0],
        [0.9, 0.7, 0.0],
        [0.1, 0.7, 0.0],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 8];
    let uv0 = [
        [0.25, 0.5],
        [0.25, 0.5],
        [0.25, 0.5],
        [0.25, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
    ];
    let indices = [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    assert_eq!(
        renderer
            .upload_mesh_colored_with_frames_and_materials_and_uv0(
                &positions,
                &colors,
                None,
                None,
                Some(&uv0),
                &indices,
                &[material],
                &[range],
            )
            .expect("upload alpha-mask MAT1 mesh"),
        4
    );
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("draw alpha-mask mesh");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("read alpha-mask pixels");
    let pixel = |x: usize| &rgba[((32 * 64 + x) * 4)..((32 * 64 + x) * 4 + 4)];
    let discarded = pixel(16);
    let clear_rgb =
        crate::render::output::pbr_neutral_v1_srgb([8.0 / 255.0, 13.0 / 255.0, 20.0 / 255.0])
            .map(|channel| (channel * 255.0).round() as u8);
    let expected_clear = [clear_rgb[0], clear_rgb[1], clear_rgb[2], 255];
    assert!(
        discarded
            .iter()
            .zip(expected_clear)
            .all(|(actual, expected)| actual.abs_diff(expected) <= 3),
        "sub-cutoff alpha texel must discard and expose the clear colour, got {discarded:?}"
    );
    assert!(
        pixel(48)[0] > 100 && pixel(48)[1] > 100 && pixel(48)[2] > 100 && pixel(48)[3] == 255,
        "above-cutoff sample must retain opaque material shading, got {:?}",
        pixel(48)
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_alpha_coverage_mips_keep_minified_mask_visible() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);

    // Every 2x2 block has 75% base coverage. At cutoff 0.8, ordinary box-filtered
    // mips would have alpha 0.75 and disappear; correction raises the nearest
    // attainable mip coverage, with the known +25% quantization residual.
    let mut rgba = vec![255u8; 64 * 64 * 4];
    for y in 0..64usize {
        for x in 0..64usize {
            if x % 2 == 1 && y % 2 == 1 {
                rgba[(y * 64 + x) * 4 + 3] = 0;
            }
        }
    }
    let digest = [0xA7; 32];
    let coverage = renderer
        .upload_resident_texture_rgba8_with_mips_and_diagnostics(
            digest,
            TextureColorSpace::Srgb,
            TextureMipSemantic::alpha_mask(0.8).unwrap(),
            64,
            64,
            &rgba,
        )
        .expect("resident coverage-preserved mask chain");
    let coverage = coverage.expect("alpha-mask coverage diagnostics");
    assert_eq!(coverage.level_count, 7);
    assert!((coverage.base_coverage - 0.75).abs() < 0.001);
    assert!((coverage.signed_residuals[2] - 0.25).abs() < 0.001);

    let resident = renderer
        .resident_textures
        .iter()
        .find(|(identity, _)| identity.source_digest == digest)
        .map(|(_, resident)| resident)
        .expect("resident alpha texture");
    let mip_readback = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("alpha-coverage-mip-readback"),
        size: 3 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("alpha-coverage-mip-readback"),
        });
    for (index, mip_level) in [1u32, 2, 3].into_iter().enumerate() {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &resident._texture,
                mip_level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &mip_readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: (index * 256) as u64,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }
    renderer.queue.submit(std::iter::once(encoder.finish()));
    let slice = mip_readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = renderer.device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv()
        .expect("alpha mip readback callback")
        .expect("alpha mip readback mapping");
    let mapped = slice.get_mapped_range().expect("alpha mip mapped range");
    let gpu_mip_alpha = [mapped[3], mapped[259], mapped[515]];
    drop(mapped);
    mip_readback.unmap();
    assert!(
        gpu_mip_alpha
            .iter()
            .all(|&alpha| f32::from(alpha) / 255.0 >= 0.8),
        "GPU-generated alpha mips must preserve cutoff coverage, got {gpu_mip_alpha:?}"
    );

    let mut material = crate::container_10d::MaterialRecord::legacy_default();
    material.opacity_mode = crate::container_10d::OpacityMode::Mask;
    material.alpha_cutoff = 0.8;
    material.base_color_texture = digest;
    material.texture_samplers[0].min_filter =
        crate::container_10d::TextureMinFilter::NearestMipmapNearest;
    let positions = [
        [-2.0, -2.0, 0.0],
        [2.0, -2.0, 0.0],
        [2.0, 2.0, 0.0],
        [-2.0, 2.0, 0.0],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 4];
    let uv0 = [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
    let indices = [0, 1, 2, 0, 2, 3];
    let range = crate::container_10d::SubmeshRange {
        first_index: 0,
        index_count: 6,
        material_id: material.id,
        semantic_id: 0,
    };
    renderer
        .upload_mesh_colored_with_frames_and_materials_and_uv0(
            &positions,
            &colors,
            None,
            None,
            Some(&uv0),
            &indices,
            &[material],
            &[range],
        )
        .expect("upload minified alpha-mask quad");
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render minified alpha-mask quad");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("read coverage-preserved mip pixels");
    let visible = rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 24 || pixel[1] > 24 || pixel[2] > 24)
        .count();
    assert!(
        visible > 512,
        "cutoff-corrected mips should keep the minified mask visible; only {visible} pixels were shaded"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_mask_hole_preserves_receiver_depth_in_ao_prepass() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);
    renderer.set_screen_space_ao_enabled(true);
    renderer.set_screen_space_ao(0.5, 1.0, 0.0);
    renderer.set_screen_space_ao_sample_count(4);

    let positions = [
        [-1.5, -1.0, -0.35],
        [1.5, -1.0, -0.35],
        [1.5, 1.0, -0.35],
        [-1.5, 1.0, -0.35],
        [-0.5, -0.5, 0.35],
        [0.0, -0.5, 0.35],
        [0.0, 0.5, 0.35],
        [-0.5, 0.5, 0.35],
        [0.0, -0.5, 0.35],
        [0.5, -0.5, 0.35],
        [0.5, 0.5, 0.35],
        [0.0, 0.5, 0.35],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 12];
    let uv0 = [
        [0.0, 0.0],
        [0.0, 0.0],
        [0.0, 0.0],
        [0.0, 0.0],
        [0.25, 0.5],
        [0.25, 0.5],
        [0.25, 0.5],
        [0.25, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
    ];
    let indices = [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11];
    let mut receiver = crate::container_10d::MaterialRecord::legacy_default();
    receiver.id = 1;
    let mut mask = crate::container_10d::MaterialRecord::legacy_default();
    mask.id = 2;
    mask.opacity_mode = crate::container_10d::OpacityMode::Mask;
    mask.alpha_cutoff = 0.5;

    let render_depth = |renderer: &mut PortalGpu, digest_byte: u8, alpha: [u8; 2]| {
        let digest = [digest_byte; 32];
        let rgba = [255, 255, 255, alpha[0], 255, 255, 255, alpha[1]];
        renderer
            .upload_resident_texture_rgba8_with_mips(
                digest,
                TextureColorSpace::Srgb,
                TextureMipSemantic::alpha_mask(0.5).unwrap(),
                2,
                1,
                &rgba,
            )
            .expect("resident AO mask map");
        let mut current_mask = mask;
        current_mask.base_color_texture = digest;
        let ranges = [
            crate::container_10d::SubmeshRange {
                first_index: 0,
                index_count: 6,
                material_id: receiver.id,
                semantic_id: 0,
            },
            crate::container_10d::SubmeshRange {
                first_index: 6,
                index_count: 12,
                material_id: current_mask.id,
                semantic_id: 0,
            },
        ];
        renderer
            .upload_mesh_colored_with_frames_and_materials_and_uv0(
                &positions,
                &colors,
                None,
                None,
                Some(&uv0),
                &indices,
                &[receiver, current_mask],
                &ranges,
            )
            .expect("upload AO receiver and cutout");
        renderer
            .render(0.0, &SystemTelemetry::default())
            .expect("render AO mask comparison");
        // Pixel (13,16) in the half-resolution AO target lies inside the left
        // front half, away from its diagonal and outer edges.
        read_ao_depth_code(renderer, 13, 16)
    };

    let hole_depth = render_depth(&mut renderer, 0x81, [0, 255]);
    let solid_depth = render_depth(&mut renderer, 0x82, [255, 255]);
    assert!(
        hole_depth > solid_depth.saturating_add(100),
        "transparent cutout must leave the rear receiver in the AO surface, not the front caster: hole={hole_depth}, solid={solid_depth}"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_mask_hole_preserves_direct_sun_on_receiver() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.7, 0.0, 1.0, 1.0, 0.0);
    renderer.set_screen_space_ao_enabled(false);

    let positions = [
        // Receiver behind the mask plane.
        [-1.5, -1.0, -0.35],
        [1.5, -1.0, -0.35],
        [1.5, 1.0, -0.35],
        [-1.5, 1.0, -0.35],
        // Left half of the front mask quad samples transparent texel zero.
        [-0.5, -0.5, 0.35],
        [0.0, -0.5, 0.35],
        [0.0, 0.5, 0.35],
        [-0.5, 0.5, 0.35],
        // Right half samples opaque texel one.
        [0.0, -0.5, 0.35],
        [0.5, -0.5, 0.35],
        [0.5, 0.5, 0.35],
        [0.0, 0.5, 0.35],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 12];
    let uv0 = [
        [0.0, 0.0],
        [0.0, 0.0],
        [0.0, 0.0],
        [0.0, 0.0],
        [0.25, 0.5],
        [0.25, 0.5],
        [0.25, 0.5],
        [0.25, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
        [0.75, 0.5],
    ];
    let indices = [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11];

    let upload = |renderer: &mut PortalGpu, digest_byte: u8, alpha: [u8; 2]| {
        let digest = [digest_byte; 32];
        let rgba = [255, 255, 255, alpha[0], 255, 255, 255, alpha[1]];
        renderer
            .upload_resident_texture_rgba8_with_mips(
                digest,
                TextureColorSpace::Srgb,
                TextureMipSemantic::alpha_mask(0.5).unwrap(),
                2,
                1,
                &rgba,
            )
            .expect("resident mask map");

        let mut receiver = crate::container_10d::MaterialRecord::legacy_default();
        receiver.id = 1;
        let mut mask = crate::container_10d::MaterialRecord::legacy_default();
        mask.id = 2;
        mask.opacity_mode = crate::container_10d::OpacityMode::Mask;
        mask.alpha_cutoff = 0.5;
        mask.base_color_texture = digest;
        let ranges = [
            crate::container_10d::SubmeshRange {
                first_index: 0,
                index_count: 6,
                material_id: receiver.id,
                semantic_id: 0,
            },
            crate::container_10d::SubmeshRange {
                first_index: 6,
                index_count: 12,
                material_id: mask.id,
                semantic_id: 0,
            },
        ];
        renderer
            .upload_mesh_colored_with_frames_and_materials_and_uv0(
                &positions,
                &colors,
                None,
                None,
                Some(&uv0),
                &indices,
                &[receiver, mask],
                &ranges,
            )
            .expect("upload receiver and cutout casters");
        renderer
            .render(0.0, &SystemTelemetry::default())
            .expect("render shadow comparison scene");
        let mut pixels = vec![0u8; renderer.required_rgba8_bytes()];
        renderer
            .read_rgba8_into(&mut pixels)
            .expect("read shadow comparison scene");
        pixels
    };

    let hole_pixels = upload(&mut renderer, 0x71, [0, 255]);
    let solid_pixels = upload(&mut renderer, 0x72, [255, 255]);
    let receiver_probe = |pixels: &[u8]| {
        let offset = (32 * 64 + 16) * 4;
        [
            pixels[offset],
            pixels[offset + 1],
            pixels[offset + 2],
            pixels[offset + 3],
        ]
    };
    let through_hole = receiver_probe(&hole_pixels);
    let behind_solid_caster = receiver_probe(&solid_pixels);
    assert!(
        through_hole[0] > behind_solid_caster[0].saturating_add(12),
        "cutout should let direct sun reach the receiver outside the front quad; hole={through_hole:?}, solid={behind_solid_caster:?}"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_blend_draws_sort_back_to_front_and_composite() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_camera(0.0, 0.0, 3.5);
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);
    renderer.set_screen_space_ao_enabled(false);
    renderer.set_shadows_enabled(false);

    let mut far = crate::container_10d::MaterialRecord {
        id: 101,
        ..crate::container_10d::MaterialRecord::legacy_default()
    };
    far.opacity_mode = crate::container_10d::OpacityMode::Blend;
    far.base_color = [1.0, 1.0, 1.0, 0.5];
    far.multiply_vertex_color = true;
    let mut near = far.clone();
    near.id = 202;
    let materials = [far, near];

    // The near (red) draw intentionally comes first in the index stream. Correct compositing
    // therefore depends on the renderer's depth-aware transparent ordering, not upload order.
    let positions = [
        [-0.8, -0.8, -0.45],
        [0.8, -0.8, -0.45],
        [0.8, 0.8, -0.45],
        [-0.8, 0.8, -0.45],
        [-0.8, -0.8, 0.45],
        [0.8, -0.8, 0.45],
        [0.8, 0.8, 0.45],
        [-0.8, 0.8, 0.45],
    ];
    let colors = [
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
    ];
    let indices = [4, 5, 6, 4, 6, 7, 0, 1, 2, 0, 2, 3];
    let ranges = [
        crate::container_10d::SubmeshRange {
            first_index: 0,
            index_count: 6,
            material_id: 202,
            semantic_id: 2,
        },
        crate::container_10d::SubmeshRange {
            first_index: 6,
            index_count: 6,
            material_id: 101,
            semantic_id: 1,
        },
    ];
    renderer
        .upload_mesh_colored_with_frames_and_materials(
            &positions, &colors, None, None, &indices, &materials, &ranges,
        )
        .expect("upload overlapping blend submeshes");
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("draw sorted blended surfaces");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("read blended pixel oracle");
    let pixel = &rgba[((32 * 64 + 32) * 4)..((32 * 64 + 32) * 4 + 4)];
    assert!(
        pixel[0] > pixel[2].saturating_add(24),
        "far blue then near red should composite red-dominantly, got {pixel:?}"
    );
    assert!(
        (64..=215).contains(&pixel[0]) && (32..=180).contains(&pixel[2]),
        "both 50% layers should contribute a visible channel at the center, got {pixel:?}"
    );
    assert_eq!(pixel[3], 255, "opaque clear alpha remains fully covered");
}
