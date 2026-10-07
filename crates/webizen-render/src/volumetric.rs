//! SDK facade over QualiaDB's canonical wgpu 29 volumetric renderer.
//!
//! This is the native bridge used by desktop/studio embedders. It deliberately delegates the
//! projection, depth, standpoint, bloom, picking, tensor ABI, and shared-device ownership to
//! `qualia-core-db`; this crate only adapts the serde scene contract.

use crate::scene_contract::{RenderScene, ScenePoint};
use qualia_core_db::render::gpu::PortalGpu;
use qualia_core_db::render::gpu::{TextureColorSpace, TextureMipSemantic};
use qualia_core_db::render::telemetry::SystemTelemetry as CoreTelemetry;
use qualia_core_db::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
use qualia_core_db::tensor::Tensor10D;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TextureUse {
    color_space: TextureColorSpace,
    mip_semantic: TextureMipSemantic,
}

/// Cross-platform volumetric renderer SDK. Native instances render offscreen on the same physical
/// wgpu device as QualiaDB inference and expose caller-buffered RGBA8 readback.
pub struct VolumetricRenderer {
    inner: PortalGpu,
}

impl VolumetricRenderer {
    pub fn new_offscreen(width: u32, height: u32, particle_cap: usize) -> Result<Self, String> {
        Ok(Self {
            inner: PortalGpu::new_offscreen(width, height, particle_cap)?,
        })
    }

    /// Create a **surface** renderer that draws directly to a window's GPU swapchain.
    ///
    /// This is the native desktop path — no PNG round-trip, no webview `<img>`.
    /// The surface is created from a raw window handle (HWND on Windows).
    /// Call `render()` to draw a frame; the swapchain present is automatic.
    #[cfg(all(not(target_arch = "wasm32"), feature = "qualia"))]
    pub fn new_surface(
        hwnd: isize,
        width: u32,
        height: u32,
        particle_cap: usize,
    ) -> Result<Self, String> {
        Ok(Self {
            inner: PortalGpu::new_surface(hwnd, width, height, particle_cap)?,
        })
    }

    pub fn upload_tensor_buffer(&mut self, bytes: &[u8]) -> Result<u32, String> {
        self.inner.upload_tensor_buffer(bytes)
    }

    /// Load one `.10d` scene asset and its digest-verified HMC texture dependencies.
    /// Texture decode and GPU residency remain cold, bounded asset-boundary work.
    pub fn load_hmc_asset(
        &mut self,
        hmc_bytes: &[u8],
        asset_key: &str,
    ) -> Result<(u32, u32, f32), String> {
        use std::collections::{BTreeMap, BTreeSet};

        let bundle = qualia_core_db::bundle::BundleReader::parse(hmc_bytes)
            .map_err(|error| format!("HMC bundle: {error}"))?;
        let asset_bytes = bundle
            .get(asset_key)
            .ok_or_else(|| format!("HMC asset is missing: {asset_key}"))?;
        if !bundle.verify_entry(asset_key) {
            return Err(format!("HMC asset digest check failed: {asset_key}"));
        }
        let entry = bundle
            .entry(asset_key)
            .ok_or_else(|| format!("HMC asset index entry is missing: {asset_key}"))?;
        if entry.kind != "10d" {
            return Err(format!(
                "HMC asset {asset_key} has kind {:?}, expected 10d",
                entry.kind
            ));
        }

        let header = qualia_core_db::container_10d::Container10dHeader::parse(asset_bytes)
            .map_err(|error| format!("10d header: {error}"))?;
        if qualia_core_db::container_10d::compute_whole_file_crc32c(asset_bytes)
            != header.header_crc32c
        {
            return Err("10d whole-file integrity check failed".to_string());
        }
        let descs = qualia_core_db::container_10d::parse_section_table(asset_bytes, &header)
            .map_err(|error| format!("10d section table: {error}"))?;
        let mut material_payload = None;
        for descriptor in descs {
            if descriptor.typ() == Some(qualia_core_db::container_10d::SectionType::Materials) {
                let start = descriptor.byte_offset as usize;
                let end = start
                    .checked_add(descriptor.byte_length as usize)
                    .filter(|&end| end <= asset_bytes.len())
                    .ok_or_else(|| "10d MAT1 section is outside asset bytes".to_string())?;
                material_payload = Some(&asset_bytes[start..end]);
                break;
            }
        }
        let mut texture_uses = BTreeMap::<[u8; 32], BTreeSet<TextureUse>>::new();
        if let Some(payload) = material_payload {
            use qualia_core_db::container_10d::{MaterialRecord, OpacityMode};
            let (material_count, range_count) =
                qualia_core_db::container_10d::material_section_counts(payload)
                    .map_err(|error| format!("10d MAT1 header: {error}"))?;
            let mut materials = vec![MaterialRecord::legacy_default(); material_count];
            let first_material_id = materials
                .first()
                .ok_or_else(|| "10d MAT1 section has no materials".to_string())?
                .id;
            let mut ranges = vec![
                qualia_core_db::container_10d::SubmeshRange {
                    first_index: 0,
                    index_count: 3,
                    material_id: first_material_id,
                    semantic_id: 0,
                };
                range_count
            ];
            let mesh_index_count = descs
                .iter()
                .find(|descriptor| {
                    descriptor.typ()
                        == Some(qualia_core_db::container_10d::SectionType::QuantizedMesh)
                })
                .map(|descriptor| {
                    let start = descriptor.byte_offset as usize;
                    let end = start + descriptor.byte_length as usize;
                    qualia_core_db::container_10d::decode_mesh_section(&asset_bytes[start..end])
                        .map(|mesh| mesh.triangles.len().saturating_mul(3) as u32)
                        .map_err(|error| format!("10d mesh decode: {error}"))
                })
                .transpose()?
                .ok_or_else(|| "10d asset has no QuantizedMesh section".to_string())?;
            qualia_core_db::container_10d::decode_material_section_into(
                payload,
                mesh_index_count,
                &mut materials,
                &mut ranges,
            )
            .map_err(|error| format!("10d MAT1 decode: {error}"))?;
            for material in &materials {
                if material.base_color_texture != [0; 32] {
                    texture_uses
                        .entry(material.base_color_texture)
                        .or_default()
                        .insert(TextureUse {
                            color_space: TextureColorSpace::Srgb,
                            mip_semantic: if material.opacity_mode == OpacityMode::Mask {
                                TextureMipSemantic::alpha_mask(material.alpha_cutoff)
                                    .unwrap_or(TextureMipSemantic::Color)
                            } else {
                                TextureMipSemantic::Color
                            },
                        });
                }
                if material.emissive_texture != [0; 32] {
                    texture_uses
                        .entry(material.emissive_texture)
                        .or_default()
                        .insert(TextureUse {
                            color_space: TextureColorSpace::Srgb,
                            mip_semantic: TextureMipSemantic::Color,
                        });
                }
                if material.normal_texture != [0; 32] {
                    texture_uses
                        .entry(material.normal_texture)
                        .or_default()
                        .insert(TextureUse {
                            color_space: TextureColorSpace::Linear,
                            mip_semantic: TextureMipSemantic::Normal,
                        });
                }
                for digest in [
                    &material.metallic_roughness_texture,
                    &material.occlusion_texture,
                ] {
                    if *digest != [0; 32] {
                        texture_uses.entry(*digest).or_default().insert(TextureUse {
                            color_space: TextureColorSpace::Linear,
                            mip_semantic: TextureMipSemantic::LinearData,
                        });
                    }
                }
                if material.stylized_ramp_texture != [0; 32] {
                    texture_uses
                        .entry(material.stylized_ramp_texture)
                        .or_default()
                        .insert(TextureUse {
                            color_space: TextureColorSpace::Srgb,
                            mip_semantic: TextureMipSemantic::Color,
                        });
                }
            }
        }

        let mut newly_resident = Vec::new();
        let upload_result = (|| -> Result<(), String> {
            for (digest, uses) in texture_uses {
                let resource = qualia_core_db::render::asset_package::resolve_hmc_texture_resource(
                    &bundle, &digest,
                )
                .map_err(|error| format!("HMC texture resolution: {error}"))?;
                let (rgba8, info) =
                    qualia_core_db::render::texture_decode::decode_hmc_texture_rgba8(
                        &resource,
                        qualia_core_db::render::texture_decode::TextureDecodeLimits::default(),
                    )
                    .map_err(|error| format!("HMC texture decode: {error}"))?;
                for texture_use in uses {
                    let was_resident = self
                        .inner
                        .resident_texture_binding_with_mips(
                            &digest,
                            texture_use.color_space,
                            texture_use.mip_semantic,
                        )
                        .is_some();
                    self.inner
                        .upload_resident_texture_rgba8_with_mips(
                            digest,
                            texture_use.color_space,
                            texture_use.mip_semantic,
                            info.width,
                            info.height,
                            &rgba8,
                        )
                        .map_err(|error| format!("HMC texture upload: {error}"))?;
                    if !was_resident {
                        newly_resident.push((digest, texture_use));
                    }
                }
            }
            Ok(())
        })();
        if let Err(error) = upload_result {
            for (digest, texture_use) in newly_resident {
                self.inner.evict_resident_texture_with_mips(
                    &digest,
                    texture_use.color_space,
                    texture_use.mip_semantic,
                );
            }
            return Err(error);
        }
        match self.load_10d_asset(asset_bytes) {
            Ok(result) => Ok(result),
            Err(error) => {
                for (digest, texture_use) in newly_resident {
                    self.inner.evict_resident_texture_with_mips(
                        &digest,
                        texture_use.color_space,
                        texture_use.mip_semantic,
                    );
                }
                Err(error)
            }
        }
    }

    pub fn upload_mesh(&mut self, positions: &[[f32; 3]], indices: &[u32]) -> u32 {
        self.inner.upload_mesh(positions, indices)
    }

    /// Upload a `.10d` QuantizedMesh section (QualiaDB's compact native
    /// geometry format — u16-quantized vertices within the mesh's bounding
    /// box, u16/u32 triangle indices).
    ///
    /// Decode/allocation occurs once at this explicit asset boundary; the
    /// resulting vertex/index buffers use the normal zero-copy GPU draw path.
    pub fn upload_10d_mesh(&mut self, bytes: &[u8]) -> Result<u32, String> {
        let mesh =
            qualia_core_db::container_10d::decode_mesh_section(bytes).map_err(|e| e.to_string())?;
        let mut indices = Vec::with_capacity(mesh.triangles.len() * 3);
        for triangle in &mesh.triangles {
            indices.extend_from_slice(triangle);
        }
        Ok(self.inner.upload_mesh(&mesh.positions, &indices))
    }

    pub fn upload_mesh_colored(
        &mut self,
        positions: &[[f32; 3]],
        colors: &[[f32; 4]],
        indices: &[u32],
    ) -> u32 {
        self.inner.upload_mesh_colored(positions, colors, indices)
    }

    pub fn set_camera(&mut self, yaw: f32, pitch: f32, zoom: f32) {
        self.inner.set_camera(yaw, pitch, zoom);
    }

    /// Toggle the bounded portable directional shadow pass. When its VRAM reservation is refused,
    /// the renderer keeps the unshadowed material path available.
    pub fn set_sun_shadows_enabled(&mut self, enabled: bool) {
        self.inner.set_shadows_enabled(enabled);
    }

    pub fn sun_shadow_resolution(&self) -> Option<u32> {
        self.inner.shadow_resolution()
    }

    /// Enable the optional half-resolution depth/normal ambient-visibility pass when budget permits.
    pub fn set_screen_space_ao_enabled(&mut self, enabled: bool) {
        self.inner.set_screen_space_ao_enabled(enabled);
    }

    /// Set the world-space AO radius, indirect visibility strength, and normal-offset bias.
    pub fn set_screen_space_ao(&mut self, radius: f32, strength: f32, bias: f32) {
        self.inner.set_screen_space_ao(radius, strength, bias);
    }

    pub fn set_screen_space_ao_sample_count(&mut self, samples: u32) {
        self.inner.set_screen_space_ao_sample_count(samples);
    }

    pub fn screen_space_ao_available(&self) -> bool {
        self.inner.screen_space_ao_available()
    }

    pub fn screen_space_ao_resolution(&self) -> Option<(u32, u32)> {
        self.inner.screen_space_ao_resolution()
    }

    pub fn render(
        &mut self,
        time_seconds: f32,
        telemetry: &crate::telemetry::SystemTelemetry,
    ) -> Result<(), String> {
        self.inner.render(time_seconds, &core_telemetry(telemetry))
    }

    pub fn required_rgba8_bytes(&self) -> usize {
        self.inner.required_rgba8_bytes()
    }

    pub fn read_rgba8_into(&self, out: &mut [u8]) -> Result<usize, String> {
        self.inner.read_rgba8_into(out)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Err(error) = self.inner.resize(width, height) {
            log::warn!("volumetric renderer resize retained previous targets: {error}");
        }
    }

    /// P9.3 — Queue an integer pick at pixel `(x, y)`. The result is available
    /// after the next `render()` call via `poll_pick_readback()`.
    pub fn queue_pick(&mut self, x: f32, y: f32) {
        self.inner.queue_pick(x, y);
    }

    /// P9.3 — Poll for a completed GPU pick readback. Returns `Some(node_index)`
    /// if the picking pass has completed, or `None` if still pending.
    pub fn poll_pick_readback(&mut self) -> Option<u32> {
        self.inner.poll_pick_readback()
    }

    /// Legacy planar CPU picker retained for callers using the Canvas2D
    /// projection. The camera-aware WebGPU-equivalent oracle is exposed by
    /// [`Self::cpu_pick_node_at_camera`].
    pub fn cpu_pick_node_at(
        tensor: &[u8],
        canvas_w: f64,
        canvas_h: f64,
        pick_x: f64,
        pick_y: f64,
        yaw: f32,
        standpoint: &qualia_core_db::render::telemetry::ObserverStandpoint,
    ) -> Option<u32> {
        qualia_core_db::render::navigation::cpu_pick_node_at(
            tensor, canvas_w, canvas_h, pick_x, pick_y, yaw, standpoint,
        )
    }

    /// CPU oracle for the Tensor10D WebGPU picking pass. Uses the same camera,
    /// frame time, observer/PGA transform, clip range, pixel footprint and
    /// depth ordering as the GPU projector.
    pub fn cpu_pick_node_at_camera(
        tensor: &[u8],
        canvas_w: u32,
        canvas_h: u32,
        pick_x: f64,
        pick_y: f64,
        frame_time: f32,
        camera: qualia_core_db::render::camera::CameraState,
        standpoint: &qualia_core_db::render::telemetry::ObserverStandpoint,
    ) -> Option<u32> {
        qualia_core_db::render::navigation::cpu_pick_node_at_camera(
            tensor, canvas_w, canvas_h, pick_x, pick_y, frame_time, camera, standpoint,
        )
    }

    /// P9.3 — Load a full `.10d` container asset (not just a mesh section).
    /// Parses the section table, extracts the QuantizedMesh, uploads it to the
    /// GPU, and returns `(vertex_count, triangle_count, provenance_mu)`.
    ///
    /// D3: when Tensor10DNodes are present, vertices are coloured by nearest-node
    /// σ via `sigma_to_display_rgb` (vision recon / EMF paint fuel).
    pub fn load_10d_asset(&mut self, bytes: &[u8]) -> Result<(u32, u32, f32), String> {
        use qualia_core_db::container_10d::{
            self, header::Container10dHeader, node_section::parse_node_header,
        };
        use qualia_core_db::render::spectral::sigma_to_display_rgb;
        use qualia_core_db::tensor::Tensor10D;

        let mut bytes_mut = bytes.to_vec();
        let header =
            Container10dHeader::parse(&bytes_mut).map_err(|e| format!("10d header: {e}"))?;
        container_10d::verify_whole_file_crc32c(&mut bytes_mut)
            .map_err(|e| format!("10d CRC: {e}"))?;
        let descs = container_10d::parse_section_table(&bytes_mut, &header)
            .map_err(|e| format!("10d section table: {e}"))?;

        let mut mesh = None;
        let mut material_payload = None;
        let mut texture_coordinates_payload = None;
        let mut provenance_mu: f32 = 0.0;
        let mut nodes: Vec<Tensor10D> = Vec::new();

        for desc in descs.iter() {
            let st = container_10d::SectionType::from_u8(desc.section_type)
                .ok_or_else(|| format!("10d: unknown section type {}", desc.section_type))?;
            let off = desc.byte_offset as usize;
            let len = desc.byte_length as usize;
            let payload = &bytes_mut[off..off + len];

            match st {
                container_10d::SectionType::QuantizedMesh => {
                    mesh = Some(
                        container_10d::decode_mesh_section(payload)
                            .map_err(|e| format!("10d mesh decode: {e}"))?,
                    );
                }
                container_10d::SectionType::Tensor10DNodes => {
                    if let Ok((nh, _)) = parse_node_header(payload) {
                        let count = nh.node_count as usize;
                        for i in 0..count {
                            if let Ok(t) = container_10d::read_node(payload, i) {
                                if i == 0 {
                                    provenance_mu = t.mu;
                                }
                                nodes.push(t);
                            }
                        }
                    } else if let Ok(t) = container_10d::read_node(payload, 0) {
                        provenance_mu = t.mu;
                        nodes.push(t);
                    }
                }
                container_10d::SectionType::Materials => {
                    material_payload = Some(payload);
                }
                container_10d::SectionType::TextureCoordinates => {
                    texture_coordinates_payload = Some(payload);
                }
                _ => {}
            }
        }

        let mesh = mesh.ok_or_else(|| "10d: no mesh section".to_string())?;
        let tri_count = mesh.triangles.len() as u32;
        let vert_count = mesh.positions.len() as u32;
        let texture_coordinates_0 = if let Some(payload) = texture_coordinates_payload {
            let mut coordinates = vec![[0.0f32; 2]; mesh.positions.len()];
            let count = container_10d::decode_texture_coordinates_into(
                payload,
                mesh.positions.len(),
                &mut coordinates,
            )
            .map_err(|error| format!("10d UV01 decode: {error}"))?;
            if count != mesh.positions.len() {
                return Err("10d UV01 vertex count does not match mesh".to_string());
            }
            Some(coordinates)
        } else {
            None
        };

        let mut indices = Vec::with_capacity(mesh.triangles.len() * 3);
        for triangle in &mesh.triangles {
            indices.extend_from_slice(triangle);
        }

        let colors: Vec<[f32; 4]> = if nodes.is_empty() {
            Vec::new()
        } else {
            // Colour vertices by nearest node σ as an explicit observer projection. MAT1's
            // multiplyVertexColor flag controls whether that projection tints the surface.
            mesh.positions
                .iter()
                .map(|p| {
                    let mut best = 0usize;
                    let mut best_d = f32::INFINITY;
                    for (i, n) in nodes.iter().enumerate() {
                        let dx = p[0] - n.x;
                        let dy = p[1] - n.y;
                        let dz = p[2] - n.z;
                        let d = dx * dx + dy * dy + dz * dz;
                        if d < best_d {
                            best_d = d;
                            best = i;
                        }
                    }
                    let (r, g, b) = sigma_to_display_rgb(nodes[best].sigma);
                    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
                })
                .collect()
        };

        if let Some(payload) = material_payload {
            let (material_count, range_count) = container_10d::material_section_counts(payload)
                .map_err(|error| format!("10d MAT1 header: {error}"))?;
            if range_count > qualia_core_db::render::gpu::MAX_GPU_MATERIAL_DRAWS {
                return Err("10d MAT1 draw count exceeds the active renderer cap".to_string());
            }
            let mut materials =
                vec![container_10d::MaterialRecord::legacy_default(); material_count];
            let mut ranges = vec![
                container_10d::SubmeshRange {
                    first_index: 0,
                    index_count: 3,
                    material_id: materials.first().map(|material| material.id).unwrap_or(1),
                    semantic_id: 0,
                };
                range_count
            ];
            let index_count = u32::try_from(indices.len())
                .map_err(|_| "10d mesh index count exceeds u32".to_string())?;
            container_10d::decode_material_section_into(
                payload,
                index_count,
                &mut materials,
                &mut ranges,
            )
            .map_err(|error| format!("10d MAT1 decode: {error}"))?;
            self.inner
                .upload_mesh_colored_with_frames_and_materials_and_uv0(
                    &mesh.positions,
                    &colors,
                    None,
                    None,
                    texture_coordinates_0.as_deref(),
                    &indices,
                    &materials,
                    &ranges,
                )
                .map_err(|error| format!("10d MAT1 GPU upload: {error}"))?;
        } else if let Some(coordinates) = texture_coordinates_0.as_deref() {
            let material = container_10d::MaterialRecord::legacy_default();
            let range = container_10d::SubmeshRange {
                first_index: 0,
                index_count: u32::try_from(indices.len())
                    .map_err(|_| "10d mesh index count exceeds u32".to_string())?,
                material_id: material.id,
                semantic_id: 0,
            };
            self.inner
                .upload_mesh_colored_with_frames_and_materials_and_uv0(
                    &mesh.positions,
                    &colors,
                    None,
                    None,
                    Some(coordinates),
                    &indices,
                    &[material],
                    &[range],
                )
                .map_err(|error| format!("10d UV0 GPU upload: {error}"))?;
        } else if colors.is_empty() {
            self.inner.upload_mesh(&mesh.positions, &indices);
        } else {
            self.inner
                .upload_mesh_colored(&mesh.positions, &colors, &indices);
        }

        Ok((vert_count, tri_count, provenance_mu))
    }

    /// P9.3 — Colour-by-field: map a scalar field value to a deterministic RGB
    /// colour. The mapping is a simple linear interpolation across a fixed
    /// colour ramp, ensuring the same field value produces the same colour on
    /// both CPU and GPU paths.
    pub fn colour_by_field(value: f32, min: f32, max: f32) -> [f32; 3] {
        let t = if (max - min).abs() < f32::EPSILON {
            0.5
        } else {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        };
        // 5-stop ramp: blue → cyan → green → yellow → red
        let stops: [(f32, [f32; 3]); 5] = [
            (0.00, [0.0, 0.0, 1.0]),
            (0.25, [0.0, 1.0, 1.0]),
            (0.50, [0.0, 1.0, 0.0]),
            (0.75, [1.0, 1.0, 0.0]),
            (1.00, [1.0, 0.0, 0.0]),
        ];
        for i in 0..4 {
            if t <= stops[i + 1].0 {
                let local = (t - stops[i].0) / (stops[i + 1].0 - stops[i].0);
                let local = local.clamp(0.0, 1.0);
                return [
                    stops[i].1[0] + (stops[i + 1].1[0] - stops[i].1[0]) * local,
                    stops[i].1[1] + (stops[i + 1].1[1] - stops[i].1[1]) * local,
                    stops[i].1[2] + (stops[i + 1].1[2] - stops[i].1[2]) * local,
                ];
            }
        }
        stops[4].1
    }

    /// P9.3 — Temporal-scrub: filter tensor nodes to those within the
    /// `[t_slice - t_window/2, t_slice + t_window/2]` time window. Returns
    /// the indices of nodes in the window, byte-identical to a linear-scan
    /// oracle.
    pub fn temporal_scrub(tensor: &[u8], t_slice: f32, t_window: f32) -> Result<Vec<u32>, String> {
        let count = qualia_core_db::tensor::buffer_export::tensor_node_count(tensor)
            .map_err(|e| e.to_string())?;
        let half = t_window * 0.5;
        let lo = t_slice - half;
        let hi = t_slice + half;
        let mut result = Vec::new();
        for i in 0..count {
            let t = qualia_core_db::tensor::buffer_export::read_tensor_at(tensor, i)
                .map_err(|e| e.to_string())?;
            if t.t >= lo && t.t <= hi {
                result.push(i as u32);
            }
        }
        Ok(result)
    }
}

/// Render the neutral SDK scene through the canonical depth-buffered projector and mesh pipeline.
///
/// Nodes become Tensor10D projector instances. Faces and edges become a triangulated depth-tested
/// mesh. Conversion allocates only on this cold serde/IPC boundary; the renderer draw/readback path
/// remains caller-buffered.
pub fn render_scene_rgba8_into(
    scene: &RenderScene,
    width: u32,
    height: u32,
    time_seconds: f32,
    telemetry: &crate::telemetry::SystemTelemetry,
    out: &mut [u8],
) -> Result<usize, String> {
    let mut renderer = VolumetricRenderer::new_offscreen(width, height, 50_000)?;

    let tensors: Vec<Tensor10D> = scene.nodes.iter().map(node_tensor).collect();
    if !tensors.is_empty() {
        let mut bytes = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
        write_tensor_buffer(&tensors, &mut bytes).map_err(str::to_owned)?;
        renderer.upload_tensor_buffer(&bytes)?;
    }

    let (positions, colors, indices) = scene_mesh(scene, width, height);
    if !indices.is_empty() {
        renderer.upload_mesh_colored(&positions, &colors, &indices);
    }

    let eye = scene.camera.position;
    let target = scene.camera.target;
    let dx = (eye[0] - target[0]) as f32;
    let dy = (eye[1] - target[1]) as f32;
    let dz = (eye[2] - target[2]) as f32;
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    if distance.is_finite() && (0.35..=48.0).contains(&distance) {
        renderer.set_camera(dx.atan2(dz), (dy / distance).asin(), distance);
    }

    renderer.render(time_seconds, telemetry)?;
    renderer.read_rgba8_into(out)
}

/// PNG convenience bridge for native webviews. The render itself still uses caller-buffered core
/// APIs; allocation here belongs to the explicit image-codec boundary.
pub fn render_scene_png(
    scene: &RenderScene,
    width: u32,
    height: u32,
    time_seconds: f32,
    telemetry: &crate::telemetry::SystemTelemetry,
) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;

    let mut rgba = vec![0u8; width.max(1) as usize * height.max(1) as usize * 4];
    render_scene_rgba8_into(
        scene,
        width.max(1),
        height.max(1),
        time_seconds,
        telemetry,
        &mut rgba,
    )?;
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            &rgba,
            width.max(1),
            height.max(1),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("PNG encode failed: {e}"))?;
    Ok(png)
}

fn node_tensor(node: &crate::scene_contract::SceneNode) -> Tensor10D {
    let t = node.tensor;
    let tensor_has_position = t.x != 0.0 || t.y != 0.0 || t.z != 0.0;
    let [x, y, z] = if tensor_has_position {
        [t.x as f32, t.y as f32, t.z as f32]
    } else {
        scene_point_world(node.position)
    };
    Tensor10D::new(
        t.q as f32,
        t.v as f32,
        t.w as f32,
        x,
        y,
        z,
        if t.t == 0.0 {
            node.version as f32
        } else {
            t.t as f32
        },
        (t.alpha * node.alpha).clamp(0.0, 1.0) as f32,
        t.mu as f32,
        t.sigma as f32,
    )
}

fn scene_mesh(
    scene: &RenderScene,
    width: u32,
    height: u32,
) -> (Vec<[f32; 3]>, Vec<[f32; 4]>, Vec<u32>) {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for face in &scene.faces {
        if face.vertices.len() < 3 {
            continue;
        }
        let base = positions.len() as u32;
        positions.extend(face.vertices.iter().copied().map(scene_point_world));
        colors.extend(
            std::iter::repeat(css_color_linear(&face.color, face.alpha)).take(face.vertices.len()),
        );
        for i in 1..face.vertices.len() - 1 {
            indices.extend_from_slice(&[base, base + i as u32, base + i as u32 + 1]);
        }
    }

    for edge in &scene.edges {
        let from = scene_point_world(edge.from);
        let to = scene_point_world(edge.to);
        let dx = to[0] - from[0];
        let dy = to[1] - from[1];
        let length = (dx * dx + dy * dy).sqrt();
        if length <= f32::EPSILON {
            continue;
        }
        let pixel_scale = 2.0 / width.max(height).max(1) as f32;
        let half = edge.width.max(1.0) as f32 * pixel_scale * 0.5;
        let ox = -dy / length * half;
        let oy = dx / length * half;
        let base = positions.len() as u32;
        positions.extend_from_slice(&[
            [from[0] + ox, from[1] + oy, from[2]],
            [from[0] - ox, from[1] - oy, from[2]],
            [to[0] - ox, to[1] - oy, to[2]],
            [to[0] + ox, to[1] + oy, to[2]],
        ]);
        colors.extend_from_slice(&[css_color_linear(&edge.color, edge.alpha); 4]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (positions, colors, indices)
}

#[inline]
fn scene_point_world(point: ScenePoint) -> [f32; 3] {
    [
        (point.x as f32 - 0.5) * 2.0,
        (0.5 - point.y as f32) * 2.0,
        point.z as f32,
    ]
}

fn css_color_linear(color: &str, alpha: f64) -> [f32; 4] {
    let hex = color.strip_prefix('#').unwrap_or("");
    let (r, g, b) = match hex.len() {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).unwrap_or(128),
            u8::from_str_radix(&hex[2..4], 16).unwrap_or(153),
            u8::from_str_radix(&hex[4..6], 16).unwrap_or(209),
        ),
        3 => (
            u8::from_str_radix(&hex[0..1], 16).unwrap_or(8) * 17,
            u8::from_str_radix(&hex[1..2], 16).unwrap_or(9) * 17,
            u8::from_str_radix(&hex[2..3], 16).unwrap_or(12) * 17,
        ),
        _ => (128, 153, 209),
    };
    let decode = |value: u8| {
        let s = value as f32 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    [
        decode(r),
        decode(g),
        decode(b),
        alpha.clamp(0.0, 1.0) as f32,
    ]
}

fn core_telemetry(value: &crate::telemetry::SystemTelemetry) -> CoreTelemetry {
    CoreTelemetry {
        memory_pressure: value.memory_pressure,
        network_ripple: value.network_ripple,
        baking_crystallization: value.baking_crystallization,
        logic_flashes: value.logic_flashes,
        llm_heat: value.llm_heat,
        quantum_activity: value.quantum_activity,
        spectral_shift: value.spectral_shift,
        temporal_pulse: value.temporal_pulse,
        epistemic_density: value.epistemic_density,
        manifold_pressure: value.manifold_pressure,
        _padding: value._padding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_contract::{SceneEdge, SceneFace};

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
}
