//! Bounded native/WASM GPU material binding and transparent ordering for versioned submeshes.
//!
//! Material factors and resident texture maps share one per-material bind group, keeping mesh
//! pipelines portable on four-group WebGPU adapters. Blended draws use cold-computed bounds and
//! in-place back-to-front sorting.

use super::material_draws::build_material_draws;
pub(super) use super::material_draws::{update_draw_bounds, MaterialDraw};
use crate::container_10d::{
    validate_material_bindings, MaterialRecord, OpacityMode, ShadingModel, SubmeshRange,
};
use bytemuck::{Pod, Zeroable};
use std::num::NonZeroU64;
use wgpu::util::DeviceExt;

pub(super) const MATERIAL_UNIFORM_STRIDE: u64 = 512;
pub(super) const MAX_MATERIAL_DRAWS: usize = super::MAX_GPU_MATERIAL_DRAWS;

/// One per-material group contains the dynamic factors and all fixed texture/sampler slots. This
/// saves a pipeline group on WebGPU implementations that expose only the four-group baseline.
pub(super) fn create_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let mut entries = Vec::with_capacity(13);
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: NonZeroU64::new(MATERIAL_UNIFORM_STRIDE),
        },
        count: None,
    });
    entries.extend(super::material_textures::layout_entries(1));
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("portal-mesh-material-textures-layout"),
        entries: &entries,
    })
}

/// WGSL consumes the factors and UV transforms; padding provides WebGPU's portable 256-byte
/// dynamic-uniform offset alignment.
#[repr(C, align(256))]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct MaterialUniform {
    pub base_color: [f32; 4],
    pub emissive: [f32; 4],
    pub factors: [f32; 4],
    pub flags: [u32; 4],
    pub texture_flags: [u32; 4],
    pub texture_flags_extra: [u32; 4],
    /// Opacity mode, nearest-capable sampler mask, reserved, nearest magnification mask.
    pub coverage: [u32; 4],
    /// Six packed 7-bit sampler records: minification mode and U/V address modes.
    pub sampler_modes: [u32; 4],
    /// Six pairs of vec4: offset/scale, then cos/sin rotation and reserved lanes.
    pub uv_transform: [[f32; 4]; 12],
    _alignment: [u32; 48],
}

const _: [(); MATERIAL_UNIFORM_STRIDE as usize] = [(); std::mem::size_of::<MaterialUniform>()];

impl From<&MaterialRecord> for MaterialUniform {
    fn from(material: &MaterialRecord) -> Self {
        let uv_transform = std::array::from_fn(|index| {
            let transform = &material.texture_transforms[index / 2];
            if index % 2 == 0 {
                [
                    transform.offset[0],
                    transform.offset[1],
                    transform.scale[0],
                    transform.scale[1],
                ]
            } else {
                [transform.rotation.cos(), transform.rotation.sin(), 0.0, 0.0]
            }
        });
        Self {
            base_color: material.base_color,
            emissive: [
                material.emissive[0],
                material.emissive[1],
                material.emissive[2],
                1.0,
            ],
            factors: [
                material.metallic,
                material.roughness,
                material.specular,
                material.alpha_cutoff,
            ],
            flags: [
                match material.shading_model {
                    ShadingModel::MetallicRoughness => 0,
                    ShadingModel::Stylized => 1,
                },
                u32::from(material.multiply_vertex_color),
                u32::from(material.double_sided),
                u32::from(material.receives_shadow),
            ],
            texture_flags: [
                u32::from(material.base_color_texture != [0; 32]),
                u32::from(material.normal_texture != [0; 32]),
                u32::from(material.metallic_roughness_texture != [0; 32]),
                u32::from(material.occlusion_texture != [0; 32]),
            ],
            texture_flags_extra: [
                u32::from(material.emissive_texture != [0; 32]),
                u32::from(material.stylized_ramp_texture != [0; 32]),
                material.occlusion_strength.to_bits(),
                material.normal_scale.to_bits(),
            ],
            coverage: [
                match material.opacity_mode {
                    OpacityMode::Opaque => 0,
                    OpacityMode::Mask => 1,
                    OpacityMode::Blend => 2,
                },
                material
                    .texture_samplers
                    .iter()
                    .enumerate()
                    .fold(0, |mask, (slot, sampler)| {
                        let nearest_min = matches!(
                            sampler.min_filter,
                            crate::container_10d::TextureMinFilter::Nearest
                                | crate::container_10d::TextureMinFilter::NearestMipmapNearest
                        );
                        mask | (u32::from(
                            nearest_min
                                || sampler.mag_filter
                                    == crate::container_10d::TextureFilterMode::Nearest,
                        ) << slot)
                    }),
                0,
                material
                    .texture_samplers
                    .iter()
                    .enumerate()
                    .fold(0, |mask, (slot, sampler)| {
                        mask | (u32::from(
                            sampler.mag_filter == crate::container_10d::TextureFilterMode::Nearest,
                        ) << slot)
                    }),
            ],
            sampler_modes: material.texture_samplers.iter().enumerate().fold(
                [0u32; 4],
                |mut packed, (slot, sampler)| {
                    let mode = sampler.min_filter as u32
                        | ((sampler.wrap_s as u32) << 3)
                        | ((sampler.wrap_t as u32) << 5);
                    let (word, shift) = if slot < 4 {
                        (0, slot * 7)
                    } else {
                        (1, (slot - 4) * 7)
                    };
                    packed[word] |= mode << shift;
                    packed
                },
            ),
            uv_transform,
            _alignment: [0; 48],
        }
    }
}

pub(super) struct MaterialGpu {
    pub _uniform_buffer: wgpu::Buffer,
    pub material_bind_groups: Vec<wgpu::BindGroup>,
    pub draws: Vec<MaterialDraw>,
    /// Draw indices sorted in place each frame; capacity is fixed at mesh upload.
    pub transparent_draw_order: Vec<usize>,
}

/// Build one padded uniform array and merge adjacent ranges that use the same material.
pub(super) fn create_material_gpu(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    _texture_layout: &wgpu::BindGroupLayout,
    texture_defaults: &super::material_textures::MaterialTextureDefaults,
    resident_textures: &super::texture_residency::ResidentTextureMap,
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
    positions: &[[f32; 3]],
    indices: &[u32],
    mesh_index_count: u32,
) -> Result<MaterialGpu, String> {
    validate_material_bindings(materials, ranges, mesh_index_count)
        .map_err(|error| format!("MAT1 material/range validation: {error}"))?;
    let draws = build_material_draws(materials, ranges, positions, indices)?;
    let transparent_draw_order = draws
        .iter()
        .enumerate()
        .filter_map(|(index, draw)| {
            (materials[draw.material_index].opacity_mode == OpacityMode::Blend).then_some(index)
        })
        .collect();

    let uniforms = materials
        .iter()
        .map(MaterialUniform::from)
        .collect::<Vec<_>>();
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("portal-mesh-material-uniforms"),
        contents: bytemuck::cast_slice(&uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let material_bind_groups = materials
        .iter()
        .enumerate()
        .map(|(material_index, material)| {
            let base = resolve_map(
                resident_textures,
                &material.base_color_texture,
                super::texture_residency::TextureColorSpace::Srgb,
                if material.opacity_mode == OpacityMode::Mask {
                    super::texture_mips::TextureMipSemantic::alpha_mask(material.alpha_cutoff)
                        .unwrap_or(super::texture_mips::TextureMipSemantic::Color)
                } else {
                    super::texture_mips::TextureMipSemantic::Color
                },
                texture_defaults.view(0),
                material_index,
                "base colour",
            )?;
            let normal = resolve_map(
                resident_textures,
                &material.normal_texture,
                super::texture_residency::TextureColorSpace::Linear,
                super::texture_mips::TextureMipSemantic::Normal,
                texture_defaults.view(1),
                material_index,
                "normal",
            )?;
            let metallic_roughness = resolve_map(
                resident_textures,
                &material.metallic_roughness_texture,
                super::texture_residency::TextureColorSpace::Linear,
                super::texture_mips::TextureMipSemantic::LinearData,
                texture_defaults.view(2),
                material_index,
                "metallic-roughness",
            )?;
            let occlusion = resolve_map(
                resident_textures,
                &material.occlusion_texture,
                super::texture_residency::TextureColorSpace::Linear,
                super::texture_mips::TextureMipSemantic::LinearData,
                texture_defaults.view(3),
                material_index,
                "occlusion",
            )?;
            let emissive = resolve_map(
                resident_textures,
                &material.emissive_texture,
                super::texture_residency::TextureColorSpace::Srgb,
                super::texture_mips::TextureMipSemantic::Color,
                texture_defaults.view(4),
                material_index,
                "emissive",
            )?;
            let stylized = resolve_map(
                resident_textures,
                &material.stylized_ramp_texture,
                super::texture_residency::TextureColorSpace::Srgb,
                super::texture_mips::TextureMipSemantic::Color,
                texture_defaults.view(5),
                material_index,
                "stylized ramp",
            )?;
            let sampler_entries = material
                .texture_samplers
                .iter()
                .enumerate()
                .map(|(slot, sampler)| wgpu::BindGroupEntry {
                    binding: 7 + slot as u32,
                    resource: wgpu::BindingResource::Sampler(
                        texture_defaults.filtering_sampler(*sampler),
                    ),
                })
                .collect::<Vec<_>>();
            let mut entries = Vec::with_capacity(13);
            entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: NonZeroU64::new(MATERIAL_UNIFORM_STRIDE),
                }),
            });
            entries.extend([
                texture_entry(1, base),
                texture_entry(2, normal),
                texture_entry(3, metallic_roughness),
                texture_entry(4, occlusion),
                texture_entry(5, emissive),
                texture_entry(6, stylized),
            ]);
            entries.extend(sampler_entries);
            Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("portal-mesh-material-textures-and-factors"),
                layout,
                entries: &entries,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(MaterialGpu {
        _uniform_buffer: uniform_buffer,
        material_bind_groups,
        draws,
        transparent_draw_order,
    })
}

fn resolve_map<'a>(
    resident_textures: &'a super::texture_residency::ResidentTextureMap,
    digest: &[u8; 32],
    color_space: super::texture_residency::TextureColorSpace,
    mip_semantic: super::texture_mips::TextureMipSemantic,
    fallback: &'a wgpu::TextureView,
    material_index: usize,
    slot: &str,
) -> Result<&'a wgpu::TextureView, String> {
    if digest == &[0; 32] {
        return Ok(fallback);
    }
    super::texture_residency::resident_view(resident_textures, digest, color_space, mip_semantic)
        .ok_or_else(|| format!("MAT1 material {material_index} {slot} texture is not resident"))
}

fn texture_entry(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_abi_is_aligned_and_maps_core_material_factors() {
        assert_eq!(std::mem::size_of::<MaterialUniform>(), 512);
        assert_eq!(std::mem::align_of::<MaterialUniform>(), 256);
        let source = MaterialRecord {
            shading_model: ShadingModel::Stylized,
            base_color: [0.2, 0.4, 0.6, 0.8],
            metallic: 0.25,
            roughness: 0.75,
            multiply_vertex_color: false,
            ..MaterialRecord::legacy_default()
        };
        let uniform = MaterialUniform::from(&source);
        assert_eq!(uniform.base_color, source.base_color);
        assert_eq!(uniform.factors[..2], [0.25, 0.75]);
        assert_eq!(uniform.flags[..2], [1, 0]);
        assert_eq!(uniform.flags[3], u32::from(source.receives_shadow));
        assert!(uniform._alignment.iter().all(|word| *word == 0));
    }

    #[test]
    fn sampler_uniform_preserves_nearest_minification_with_linear_magnification() {
        let mut material = MaterialRecord::legacy_default();
        material.texture_samplers[0].min_filter =
            crate::container_10d::TextureMinFilter::NearestMipmapNearest;
        let uniform = MaterialUniform::from(&material);
        assert_eq!(uniform.coverage[1] & 1, 1);
        assert_eq!(uniform.coverage[3] & 1, 0);
        assert_eq!(uniform.sampler_modes[0] & 0x7f, 2 | (2 << 3) | (2 << 5));

        material.texture_samplers[0].mag_filter = crate::container_10d::TextureFilterMode::Nearest;
        let uniform = MaterialUniform::from(&material);
        assert_eq!(uniform.coverage[1] & 1, 1);
        assert_eq!(uniform.coverage[3] & 1, 1);
    }

    #[test]
    fn adjacent_equal_material_ranges_merge_into_one_draw() {
        let materials = [
            MaterialRecord {
                id: 11,
                ..MaterialRecord::legacy_default()
            },
            MaterialRecord {
                id: 22,
                ..MaterialRecord::legacy_default()
            },
        ];
        let ranges = [
            SubmeshRange {
                first_index: 0,
                index_count: 3,
                material_id: 11,
                semantic_id: 1,
            },
            SubmeshRange {
                first_index: 3,
                index_count: 6,
                material_id: 11,
                semantic_id: 2,
            },
            SubmeshRange {
                first_index: 9,
                index_count: 3,
                material_id: 22,
                semantic_id: 3,
            },
        ];
        let positions = [[0.0, 0.0, 0.0]; 4];
        let indices = [0, 1, 2, 0, 2, 3, 0, 1, 2, 0, 2, 3];
        let draws = build_material_draws(&materials, &ranges, &positions, &indices).unwrap();
        assert_eq!(draws.len(), 2);
        assert_eq!((draws[0].first_index, draws[0].index_count), (0, 9));
        assert_eq!(draws[0].material_offset, 0);
        assert_eq!(draws[0].bounds_min, [0.0; 3]);
        assert_eq!(draws[0].bounds_max, [0.0; 3]);
        assert_eq!((draws[1].first_index, draws[1].index_count), (9, 3));
        assert_eq!(draws[1].material_offset, 512);
    }

    #[test]
    fn animated_positions_refresh_bounds_for_every_draw_class() {
        let mut opaque = MaterialRecord::legacy_default();
        opaque.id = 1;
        let mut blend = MaterialRecord::legacy_default();
        blend.id = 2;
        blend.opacity_mode = OpacityMode::Blend;
        let ranges = [
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
            },
        ];
        let indices = [0, 1, 2, 3, 4, 5];
        let mut positions = [
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [-1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let mut draws = build_material_draws(&[opaque, blend], &ranges, &positions, &indices)
            .expect("valid submesh bounds");
        positions[0][0] = 4.0;
        positions[1][0] = 5.0;
        positions[2][0] = 6.0;
        positions[3][1] = 7.0;
        update_draw_bounds(&mut draws, &positions, &indices);

        assert_eq!(draws[0].bounds_min, [4.0, 0.0, 0.0]);
        assert_eq!(draws[0].bounds_max, [6.0, 0.0, 0.0]);
        assert_eq!(draws[1].bounds_min, [-1.0, 1.0, 0.0]);
        assert_eq!(draws[1].bounds_max, [1.0, 7.0, 0.0]);
    }

    #[test]
    fn uv_transform_uniform_packs_offset_scale_and_rotation_without_heap() {
        let mut material = MaterialRecord::legacy_default();
        material.texture_transforms[0] = crate::container_10d::TextureTransform {
            offset: [0.25, -0.5],
            scale: [2.0, 0.5],
            rotation: std::f32::consts::FRAC_PI_2,
            tex_coord: 0,
        };
        let uniform = MaterialUniform::from(&material);
        assert_eq!(uniform.uv_transform[0], [0.25, -0.5, 2.0, 0.5]);
        assert!(uniform.uv_transform[1][0].abs() < 1e-6);
        assert!((uniform.uv_transform[1][1] - 1.0).abs() < 1e-6);
        assert_eq!(
            uniform.uv_transform[2..],
            [
                [0.0, 0.0, 1.0, 1.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 1.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 1.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 1.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 1.0],
                [1.0, 0.0, 0.0, 0.0],
            ]
        );
    }
}
