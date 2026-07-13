//! SDK facade over QualiaDB's canonical wgpu 29 volumetric renderer.
//!
//! This is the native bridge used by desktop/studio embedders. It deliberately delegates the
//! projection, depth, standpoint, bloom, picking, tensor ABI, and shared-device ownership to
//! `qualia-core-db`; this crate only adapts the serde scene contract.

use crate::scene_contract::{RenderScene, ScenePoint};
use qualia_core_db::render::gpu::PortalGpu;
use qualia_core_db::render::telemetry::SystemTelemetry as CoreTelemetry;
use qualia_core_db::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
use qualia_core_db::tensor::Tensor10D;

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
    pub fn new_surface(hwnd: isize, width: u32, height: u32, particle_cap: usize) -> Result<Self, String> {
        Ok(Self {
            inner: PortalGpu::new_surface(hwnd, width, height, particle_cap)?,
        })
    }

    pub fn upload_tensor_buffer(&mut self, bytes: &[u8]) -> Result<u32, String> {
        self.inner.upload_tensor_buffer(bytes)
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
        let mesh = qualia_core_db::container_10d::decode_mesh_section(bytes)
            .map_err(|e| e.to_string())?;
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
        self.inner.resize(width, height);
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

    /// P9.3 — CPU picking oracle: returns the nearest projected tensor node
    /// index at canvas pixel `(pick_x, pick_y)`, or `None` if no node is within
    /// hit radius. This is the deterministic fallback / differential oracle for
    /// the GPU picking pass.
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
            tensor,
            canvas_w,
            canvas_h,
            pick_x,
            pick_y,
            yaw,
            standpoint,
        )
    }

    /// P9.3 — Load a full `.10d` container asset (not just a mesh section).
    /// Parses the section table, extracts the QuantizedMesh, uploads it to the
    /// GPU, and returns `(vertex_count, triangle_count, provenance_mu)`.
    pub fn load_10d_asset(&mut self, bytes: &[u8]) -> Result<(u32, u32, f32), String> {
        use qualia_core_db::container_10d::{
            self, header::Container10dHeader,
        };

        let mut bytes_mut = bytes.to_vec();
        let header = Container10dHeader::parse(&bytes_mut)
            .map_err(|e| format!("10d header: {e}"))?;
        container_10d::verify_whole_file_crc32c(&mut bytes_mut)
            .map_err(|e| format!("10d CRC: {e}"))?;
        let descs = container_10d::parse_section_table(&bytes_mut, &header)
            .map_err(|e| format!("10d section table: {e}"))?;

        let mut mesh = None;
        let mut provenance_mu: f32 = 0.0;

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
                    if let Ok(t) = container_10d::read_node(payload, 0) {
                        provenance_mu = t.mu;
                    }
                }
                _ => {}
            }
        }

        let mesh = mesh.ok_or_else(|| "10d: no mesh section".to_string())?;
        let tri_count = mesh.triangles.len() as u32;
        let vert_count = mesh.positions.len() as u32;

        let mut indices = Vec::with_capacity(mesh.triangles.len() * 3);
        for triangle in &mesh.triangles {
            indices.extend_from_slice(triangle);
        }
        self.inner.upload_mesh(&mesh.positions, &indices);

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
    pub fn temporal_scrub(
        tensor: &[u8],
        t_slice: f32,
        t_window: f32,
    ) -> Result<Vec<u32>, String> {
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
        assert!(result.is_empty(), "zero-width window at t=5 should match no nodes");
    }

    #[test]
    fn temporal_scrub_matches_linear_scan_oracle() {
        use qualia_core_db::tensor::buffer_export::{read_tensor_at, tensor_node_count, write_tensor_buffer, TensorBufferHeader};
        use qualia_core_db::tensor::Tensor10D;

        let tensors: Vec<Tensor10D> = (0..20).map(|i| {
            Tensor10D::new(0.0, 0.0, 0.0, i as f32 * 0.1, 0.0, 0.0, i as f32 * 0.3, 1.0, 0.0, 0.0)
        }).collect();
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
        assert_eq!(result, oracle, "temporal_scrub must match linear-scan oracle");
    }
}
