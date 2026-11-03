//! Native `.10d` review projection on the existing QualiaDB PortalGpu path.
#![cfg(all(feature = "qualia", not(target_arch = "wasm32")))]

use super::{SystemTelemetry, VolumetricRenderer};
use image::ImageEncoder;
use qualia_core_db::container_10d::{
    self as c, header::Container10dHeader, MeshFrameDecodeLimits, SectionType,
};
use std::io::{self, Write};

pub const MAX_REVIEW_10D_BYTES: usize = 42 * 1024 * 1024;
pub const MAX_REVIEW_SIDE: u32 = 2048;
pub const MAX_REVIEW_PIXELS: usize = 2048 * 2048;
const MAX_REVIEW_VERTICES: usize = 262_144;
const MAX_REVIEW_TRIANGLES: usize = 524_288;
const MAX_REVIEW_FRAME_BYTES: usize = 8 * 1024 * 1024;
const MAX_REVIEW_FRAME_DECODED_BYTES: usize = MAX_REVIEW_VERTICES * 28;
const MAX_REVIEW_SIGMA_COMPARISONS: usize = 20_000_000;
const REVIEW_CAMERA_FIT_MARGIN: f32 = 1.08;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct ReviewLighting {
    /// Unit directional key light passed to PortalGpu.
    pub sun_direction: [f32; 3],
    pub direct_intensity: f32,
    pub ambient_intensity: f32,
    /// Additive final RGB contribution when no environment probe is bound.
    /// This is not diffuse irradiance and must not add a grey veil to albedo.
    pub environment_additive_fallback_rgb: [f32; 3],
    /// Neutral opaque clear color; this does not alter rendered subject pixels.
    pub clear_color: [f32; 4],
}

const REVIEW_LIGHTING: ReviewLighting = ReviewLighting {
    sun_direction: [0.408_248_3, 0.816_496_6, 0.408_248_3],
    direct_intensity: 1.8,
    ambient_intensity: 0.45,
    environment_additive_fallback_rgb: [0.0, 0.0, 0.0],
    clear_color: [0.34, 0.34, 0.34, 1.0],
};

#[derive(Clone, Copy)]
struct LoadProfile {
    max_input_bytes: Option<usize>,
    max_vertices: usize,
    max_triangles: usize,
    require_renderable_mesh: bool,
    frame_limits: MeshFrameDecodeLimits,
    max_sigma_comparisons: Option<usize>,
}

impl LoadProfile {
    const fn compatibility() -> Self {
        Self {
            max_input_bytes: None,
            max_vertices: c::MAX_VERTEX_COUNT,
            max_triangles: c::MAX_TRIANGLE_COUNT,
            require_renderable_mesh: false,
            frame_limits: MeshFrameDecodeLimits::COMPATIBILITY,
            max_sigma_comparisons: None,
        }
    }
    const fn review() -> Self {
        Self {
            max_input_bytes: Some(MAX_REVIEW_10D_BYTES),
            max_vertices: MAX_REVIEW_VERTICES,
            max_triangles: MAX_REVIEW_TRIANGLES,
            require_renderable_mesh: true,
            frame_limits: MeshFrameDecodeLimits {
                max_vertices: MAX_REVIEW_VERTICES,
                max_payload_bytes: MAX_REVIEW_FRAME_BYTES,
                max_decoded_bytes: MAX_REVIEW_FRAME_DECODED_BYTES,
            },
            max_sigma_comparisons: Some(MAX_REVIEW_SIGMA_COMPARISONS),
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct ReviewView {
    pub name: &'static str,
    pub yaw: f32,
    pub pitch: f32,
}

pub const REVIEW_VIEWS: [ReviewView; 3] = [
    ReviewView { name: "three-quarter-view-assumed", yaw: 0.72, pitch: 0.28 },
    ReviewView { name: "side-view-assumed", yaw: 1.5707964, pitch: 0.12 },
    ReviewView { name: "front-view-assumed", yaw: 0.0, pitch: 0.12 },
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReviewGeometry {
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub camera_target: [f32; 3],
    pub camera_aspect: f32,
    pub vertical_tan_half_fov: f32,
    pub camera_fit_margin: f32,
    pub requested_camera_distance: f32,
    /// Exact zoom/distance submitted after applying Core's public clamp.
    pub camera_distance: f32,
    pub colors: &'static str,
    pub normals: &'static str,
    pub tangents: &'static str,
    pub texture_coordinates: &'static str,
    pub materials: &'static str,
    pub lighting: ReviewLighting,
    pub screen_space_ao_available: bool,
    pub screen_space_ao_radius_m: f32,
    pub screen_space_ao_strength: f32,
    pub screen_space_ao_samples: u32,
    pub shadows_available: bool,
}

fn apply_review_camera_fit(geometry: &mut ReviewGeometry, aspect: f32) -> Result<(), String> {
    let half_extent = [
        (geometry.bounds_max[0] - geometry.bounds_min[0]).abs() * 0.5,
        (geometry.bounds_max[1] - geometry.bounds_min[1]).abs() * 0.5,
        (geometry.bounds_max[2] - geometry.bounds_min[2]).abs() * 0.5,
    ];
    let radius = (half_extent[0] * half_extent[0]
        + half_extent[1] * half_extent[1]
        + half_extent[2] * half_extent[2])
        .sqrt();
    let tan_vertical = qualia_core_db::render::camera::CAMERA_TAN_HALF_FOV;
    let tan_horizontal = tan_vertical * aspect;
    let limiting_half_angle = tan_vertical.min(tan_horizontal).atan();
    if !aspect.is_finite() || aspect <= 0.0 || !radius.is_finite() || radius <= 0.0
        || !limiting_half_angle.is_finite() || limiting_half_angle <= 0.0
    {
        return Err("10d bounds or preview aspect cannot produce a finite camera fit".into());
    }
    // A bounding sphere fits every yaw/pitch. tan_half_fov is Core's vertical
    // projection constant; horizontal FOV scales with the render target aspect.
    let fit_distance = (radius / limiting_half_angle.sin()) * REVIEW_CAMERA_FIT_MARGIN;
    if !fit_distance.is_finite() {
        return Err("10d bounds produce a non-finite sphere-fit camera distance".into());
    }
    geometry.requested_camera_distance = fit_distance;
    geometry.camera_distance = fit_distance.clamp(
        qualia_core_db::render::camera::CAMERA_MIN_ZOOM,
        qualia_core_db::render::camera::CAMERA_MAX_ZOOM,
    );
    geometry.camera_aspect = aspect;
    geometry.vertical_tan_half_fov = tan_vertical;
    geometry.camera_fit_margin = REVIEW_CAMERA_FIT_MARGIN;
    Ok(())
}

struct NodeAdmission<'a> {
    payload: &'a [u8],
    count: usize,
    first_mu: Option<f32>,
}

struct CappedPngWriter {
    bytes: Vec<u8>,
    cap: usize,
}

impl CappedPngWriter {
    fn new(cap: usize) -> Self { Self { bytes: Vec::new(), cap } }
    fn into_inner(self) -> Vec<u8> { self.bytes }
}

impl Write for CappedPngWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let _end = self.bytes.len().checked_add(bytes.len())
            .filter(|end| *end <= self.cap)
            .ok_or_else(|| io::Error::new(io::ErrorKind::WriteZero, "PNG output cap exceeded"))?;
        self.bytes.try_reserve(bytes.len())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("PNG buffer allocation: {e}")))?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl VolumetricRenderer {
    /// Shared loader entry for the existing native general-purpose API. It
    /// retains Core's compatibility limits rather than the review profile.
    pub(super) fn load_10d_compat_asset(&mut self, bytes: &[u8]) -> Result<(u32, u32, f32), String> {
        let (geometry, provenance_mu) = self.load_10d_with_profile(bytes, LoadProfile::compatibility())?;
        Ok((geometry.vertex_count, geometry.triangle_count, provenance_mu))
    }

    pub fn load_10d_review_asset(&mut self, bytes: &[u8]) -> Result<(ReviewGeometry, f32), String> {
        self.load_10d_with_profile(bytes, LoadProfile::review())
    }

    fn load_10d_with_profile(
        &mut self,
        bytes: &[u8],
        profile: LoadProfile,
    ) -> Result<(ReviewGeometry, f32), String> {
        if bytes.is_empty() || profile.max_input_bytes.is_some_and(|max| bytes.len() > max) {
            return Err("10d input is empty or exceeds the selected loader profile".into());
        }
        let mut verified = bytes.to_vec();
        let header = Container10dHeader::parse(&verified).map_err(|e| format!("10d header: {e}"))?;
        c::verify_whole_file_crc32c(&mut verified).map_err(|e| format!("10d CRC: {e}"))?;
        let sections = c::parse_section_table(&verified, &header)
            .map_err(|e| format!("10d section table: {e}"))?;
        let mut mesh_payload = None;
        let mut material_payload = None;
        let mut texture_coordinates_payload = None;
        let mut surface_payload = None;
        let mut node_admission = None;
        for section in sections {
            let offset = section.byte_offset as usize;
            let end = offset.checked_add(section.byte_length as usize)
                .filter(|end| *end <= verified.len()).ok_or("10d section range is invalid")?;
            let payload = &verified[offset..end];
            match section.typ().ok_or("10d section type is invalid")? {
                SectionType::QuantizedMesh => mesh_payload = Some(payload),
                SectionType::Materials => material_payload = Some(payload),
                SectionType::TextureCoordinates => texture_coordinates_payload = Some(payload),
                SectionType::SurfaceReading => surface_payload = Some(payload),
                SectionType::Tensor10DNodes => {
                    let (node_header, _) = c::node_section::parse_node_header(payload)
                        .map_err(|e| format!("10d node preflight: {e:?}"))?;
                    let count = node_header.node_count as usize;
                    let first_mu = if count == 0 { None } else {
                        Some(c::read_node(payload, 0)
                            .map_err(|e| format!("10d first-node preflight: {e:?}"))?.mu)
                    };
                    let admission = NodeAdmission { payload, count, first_mu };
                    node_admission = Some(admission);
                }
                _ => {}
            }
        }

        // Core's canonical mesh header admits counts and payload extent before
        // decode_mesh_section allocates vertex and triangle vectors.
        let mesh_payload = mesh_payload.ok_or("10d has no QuantizedMesh")?;
        let (mesh_header, _) = c::parse_mesh_header(mesh_payload)
            .map_err(|e| format!("10d mesh preflight: {e:?}"))?;
        let vertex_count = mesh_header.vertex_count as usize;
        let triangle_count = mesh_header.triangle_count as usize;
        if vertex_count > profile.max_vertices || (profile.require_renderable_mesh && vertex_count == 0) {
            return Err("10d vertex count exceeds the selected loader profile".into());
        }
        if triangle_count > profile.max_triangles || (profile.require_renderable_mesh && triangle_count == 0) {
            return Err("10d triangle count exceeds the selected loader profile".into());
        }
        let mesh = c::decode_mesh_section(mesh_payload).map_err(|e| format!("10d mesh decode: {e:?}"))?;
        if mesh.positions.len() != vertex_count || mesh.triangles.len() != triangle_count {
            return Err("decoded 10d mesh counts differ from canonical header".into());
        }

        let frames = c::read_container_mesh_frames_with_limits(
            &verified, vertex_count, profile.frame_limits,
        ).map_err(|e| format!("10d authored normals/tangents: {e:?}"))?;
        let (normals, tangents, normals_label, tangents_label) = match frames {
            Some(frame) => {
                let normals = frame.normals;
                let tangents = frame.tangents;
                let normal_label = if normals.is_some() { "authored" } else { "absent-renderer-derived" };
                let tangent_label = if tangents.is_some() { "authored" } else { "absent-renderer-derived" };
                (normals, tangents, normal_label, tangent_label)
            }
            None => (None, None, "absent-renderer-derived", "absent-renderer-derived"),
        };
        let texture_coordinates = if let Some(payload) = texture_coordinates_payload {
            let mut uv = Vec::new();
            uv.try_reserve_exact(vertex_count).map_err(|_| "UV01 allocation refused")?;
            uv.resize(vertex_count, [0.0f32; 2]);
            let decoded = c::decode_texture_coordinates_into(payload, vertex_count, &mut uv)
                .map_err(|e| format!("10d UV01 decode: {e:?}"))?;
            if decoded != vertex_count { return Err("10d UV01 count differs from mesh".into()); }
            Some(uv)
        } else { None };

        let colors = if let Some(payload) = surface_payload {
            if payload.len() < 8 || &payload[..4] != c::surface_reading::SURFACE_READING_MAGIC {
                return Err("present SurfaceReading section has invalid SRD1 header".into());
            }
            let count = u32::from_le_bytes(payload[4..8].try_into()
                .map_err(|_| "SRD1 count header invalid")?) as usize;
            if count != vertex_count { return Err("SRD1 vertex count differs from mesh".into()); }
            let expected = 8usize.checked_add(count.checked_mul(4).ok_or("SRD1 length overflow")?)
                .ok_or("SRD1 length overflow")?;
            if payload.len() != expected { return Err("present SRD1 section has noncanonical length".into()); }
            Some(c::surface_reading::decode_surface_reading(payload)
                .ok_or("present SRD1 section failed canonical decoder")?)
        } else if let Some(admission) = node_admission.as_ref() {
            let comparisons = vertex_count.checked_mul(admission.count)
                .ok_or("legacy sigma projection work overflow")?;
            if profile.max_sigma_comparisons.is_some_and(|max| comparisons > max) {
                return Err("legacy sigma fallback exceeds the selected work budget".into());
            }
            if admission.count == 0 { None } else {
                let mut nodes = Vec::new();
                nodes.try_reserve_exact(admission.count).map_err(|_| "node allocation refused")?;
                for index in 0..admission.count {
                    nodes.push(c::read_node(admission.payload, index)
                        .map_err(|e| format!("10d node {index} decode: {e:?}"))?);
                }
                Some(mesh.positions.iter().map(|p| {
                    let mut best = 0usize;
                    let mut best_d = f32::INFINITY;
                    for (i, node) in nodes.iter().enumerate() {
                        let dx = p[0] - node.x; let dy = p[1] - node.y; let dz = p[2] - node.z;
                        let distance = dx * dx + dy * dy + dz * dz;
                        if distance < best_d { best = i; best_d = distance; }
                    }
                    let (r, g, b) = qualia_core_db::render::spectral::sigma_to_display_rgb(nodes[best].sigma);
                    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
                }).collect())
            }
        } else { None };
        let (colors_label, colors) = match colors {
            Some(colors) => (if surface_payload.is_some() { "canonical-SurfaceReading" } else { "legacy-sigma-projection" }, colors),
            None => ("absent-renderer-default", Vec::new()),
        };
        let provenance_mu = node_admission.as_ref()
            .and_then(|admission| admission.first_mu).unwrap_or(0.0);

        let mut indices = Vec::new();
        let index_count = triangle_count.checked_mul(3).ok_or("triangle index count overflow")?;
        indices.try_reserve_exact(index_count).map_err(|_| "mesh index allocation refused")?;
        for triangle in &mesh.triangles { indices.extend_from_slice(triangle); }
        let (materials, ranges, materials_label) = if let Some(payload) = material_payload {
            let (material_count, range_count) = c::material_section_counts(payload)
                .map_err(|e| format!("MAT1 header: {e:?}"))?;
            if material_count > c::MAX_MATERIALS
                || range_count > qualia_core_db::render::gpu::MAX_GPU_MATERIAL_DRAWS {
                return Err("MAT1 counts exceed renderer limits".into());
            }
            let mut materials = vec![c::MaterialRecord::legacy_default(); material_count];
            let mut ranges = vec![c::SubmeshRange { first_index: 0, index_count: 0,
                material_id: materials.first().map(|m| m.id).unwrap_or(1), semantic_id: 0 }; range_count];
            c::decode_material_section_into(payload, indices.len() as u32, &mut materials, &mut ranges)
                .map_err(|e| format!("MAT1 decode: {e:?}"))?;
            (materials, ranges, "canonical-MAT1")
        } else {
            let material = c::MaterialRecord::legacy_default();
            let range = c::SubmeshRange { first_index: 0,
                index_count: u32::try_from(indices.len()).map_err(|_| "mesh indices exceed u32")?,
                material_id: material.id, semantic_id: 0 };
            (vec![material], vec![range], "legacy-default-material")
        };
        let min = mesh.min;
        let max = mesh.max;
        let target = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let extent = [
            (max[0] - min[0]).abs(),
            (max[1] - min[1]).abs(),
            (max[2] - min[2]).abs(),
        ];
        if !target.iter().chain(extent.iter()).all(|value| value.is_finite()) {
            return Err("10d mesh bounds cannot produce a finite camera".into());
        }
        // This preliminary value is replaced by the review render's FOV/aspect-aware
        // bounding-sphere fit. Keeping a valid compatibility value here preserves the
        // general loader's previous camera metadata semantics.
        let requested_distance = extent[0].max(extent[1]).max(extent[2]).max(0.001) * 2.8;
        if !requested_distance.is_finite() {
            return Err("10d mesh bounds produce a non-finite camera distance".into());
        }
        let camera_distance = requested_distance.clamp(
            qualia_core_db::render::camera::CAMERA_MIN_ZOOM,
            qualia_core_db::render::camera::CAMERA_MAX_ZOOM,
        );
        let mut geometry = ReviewGeometry {
            vertex_count: vertex_count as u32,
            triangle_count: triangle_count as u32,
            bounds_min: min,
            bounds_max: max,
            camera_target: target,
            camera_aspect: 1.0,
            vertical_tan_half_fov: qualia_core_db::render::camera::CAMERA_TAN_HALF_FOV,
            camera_fit_margin: REVIEW_CAMERA_FIT_MARGIN,
            requested_camera_distance: requested_distance,
            camera_distance,
            colors: colors_label,
            normals: normals_label,
            tangents: tangents_label,
            texture_coordinates: if texture_coordinates.is_some() { "authored-UV01" } else { "absent" },
            materials: materials_label,
            lighting: REVIEW_LIGHTING,
            screen_space_ao_available: self.inner.screen_space_ao_available(),
            screen_space_ao_radius_m: 0.05,
            screen_space_ao_strength: 0.75,
            screen_space_ao_samples: 12,
            shadows_available: self.inner.shadows_enabled(),
        };
        if profile.require_renderable_mesh {
            apply_review_camera_fit(&mut geometry, 1.0)?;
        }
        let upload = self.inner.upload_mesh_colored_with_frames_and_materials_and_uv0(
            &mesh.positions, &colors, normals.as_deref(), tangents.as_deref(),
            texture_coordinates.as_deref(), &indices, &materials, &ranges,
        ).map_err(|e| format!("10d GPU upload: {e}"))?;
        if profile.require_renderable_mesh && upload == 0 {
            return Err("10d review mesh upload produced no resident triangles".into());
        }
        Ok((geometry, provenance_mu))
    }

    pub fn render_10d_review_pngs(
        &mut self,
        bytes: &[u8],
        side: u32,
    ) -> Result<(ReviewGeometry, Vec<(ReviewView, Vec<u8>)>), String> {
        let pixels = (side as usize).checked_mul(side as usize).ok_or("preview pixel overflow")?;
        if side == 0 || side > MAX_REVIEW_SIDE || pixels > MAX_REVIEW_PIXELS {
            return Err("preview dimensions exceed 2048x2048".into());
        }
        self.inner.set_tensor_projection_enabled(false);
        self.inner.set_ambient_enabled(false);
        let (mut geometry, _) = self.load_10d_review_asset(bytes)?;
        self.inner.set_screen_space_ao_enabled(true);
        self.inner.set_screen_space_ao(0.05,0.75,0.001);
        self.inner.set_screen_space_ao_sample_count(12);
        self.inner.set_shadows_enabled(true);
        geometry.shadows_available=self.inner.shadows_enabled();
        let aspect = side as f32 / side as f32;
        apply_review_camera_fit(&mut geometry, aspect)?;

        let lighting = geometry.lighting;
        self.inner.set_clear_color(
            lighting.clear_color[0] as f64,
            lighting.clear_color[1] as f64,
            lighting.clear_color[2] as f64,
            lighting.clear_color[3] as f64,
        );
        self.inner.set_lighting(
            lighting.sun_direction[0],
            lighting.sun_direction[1],
            lighting.sun_direction[2],
            lighting.direct_intensity,
            lighting.ambient_intensity,
        );
        self.inner.set_environment_probes(
            None,
            lighting.environment_additive_fallback_rgb,
        );
        let byte_len = pixels.checked_mul(4).ok_or("RGBA readback size overflow")?;
        if self.required_rgba8_bytes() != byte_len {
            return Err("renderer target extent differs from requested PNG dimensions".into());
        }
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(byte_len).map_err(|_| "RGBA buffer allocation refused")?;
        rgba.resize(byte_len, 0);
        let png_cap = pixels.checked_mul(5).ok_or("PNG cap overflow")?;
        let mut outputs = Vec::new();
        outputs.try_reserve_exact(REVIEW_VIEWS.len()).map_err(|_| "preview list allocation refused")?;
        for view in REVIEW_VIEWS {
            self.inner.set_camera_target(view.yaw, view.pitch, geometry.camera_distance,
                geometry.camera_target[0], geometry.camera_target[1], geometry.camera_target[2]);
            self.render(0.0, &SystemTelemetry::default())?;
            let written = self.read_rgba8_into(&mut rgba)?;
            if written != byte_len { return Err("GPU readback length differs from requested RGBA extent".into()); }
            let mut png = CappedPngWriter::new(png_cap);
            image::codecs::png::PngEncoder::new(&mut png).write_image(
                &rgba, side, side, image::ExtendedColorType::Rgba8,
            ).map_err(|e| format!("bounded PNG encoding failed: {e}"))?;
            outputs.push((view, png.into_inner()));
        }
        Ok((geometry, outputs))
    }
}
