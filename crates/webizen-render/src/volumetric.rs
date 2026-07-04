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
}
