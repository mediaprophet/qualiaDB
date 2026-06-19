//! Webizen N-Dimensional Renderer
//!
//! A zero-heap, N-dimensional semantic renderer that projects QualiaDB's
//! multi-modal logic graph onto 2D/3D surfaces using Projective Geometric Algebra (PGA).
//!
//! # Features
//! - PGA-based transformations (Motors unify rotation and translation)
//! - Zero-heap render loop (512-byte stack buffer discipline)
//! - Cross-platform support (WASM/Windows/Apple Silicon)
//! - Semantic culling (Deontic/Temporal passes)
//! - Epistemic Level of Detail (DoXastic LoD)

pub mod audio_contract;
pub mod math;
pub mod pipeline;
pub mod scene_contract;
pub mod shaders;
pub mod telemetry;
pub mod wgpu_renderer;

// Re-export main types for convenience
pub use math::{AlignedBufferF32, Motor, MotorEncoder, RenderQuin};
pub use pipeline::{BindGroupManager, RenderBindGroups};
pub use scene_contract::{RenderScene, SceneCamera, SceneEdge, SceneFace, SceneNode, ScenePoint};
pub use shaders::{EPISTEMIC_WGSL, PROJECTOR_WGSL};
pub use telemetry::SystemTelemetry;
#[cfg(not(target_arch = "wasm32"))]
pub use wgpu_renderer::{
    render_preview_data_uri, render_preview_png, render_scene_data_uri, render_scene_png,
    render_scene_png_with_time,
};
pub use wgpu_renderer::{Camera, ScreenPoint, Vec3, WgpuRenderer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motor_size() {
        assert_eq!(std::mem::size_of::<Motor>(), 64);
    }

    #[test]
    fn test_render_quin_size() {
        assert_eq!(std::mem::size_of::<RenderQuin>(), 64);
    }

    #[test]
    fn test_motor_encoder() {
        let mut encoder = MotorEncoder::new();
        encoder.add_motor(Motor::identity());
        assert_eq!(encoder.len(), 1);

        let bytes = encoder.encode();
        assert_eq!(bytes.len(), 64); // 1 motor * 64 bytes
    }

    #[test]
    fn test_render_quin_creation() {
        let quin = RenderQuin::new(
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            123,
            0.5,
            0.9,
            1000.0,
        );
        assert_eq!(quin.semantic_id, 123);
        assert_eq!(quin.intensity, 0.5);
    }

    #[test]
    fn test_shaders_exist() {
        // Verify shaders are included
        assert!(!PROJECTOR_WGSL.is_empty());
        assert!(!EPISTEMIC_WGSL.is_empty());
    }

    /// End-to-end headless render: build an offscreen renderer, clear to red,
    /// and read the pixels back. Skips gracefully when no GPU adapter is present
    /// (e.g. a headless CI box without a software fallback).
    #[test]
    fn offscreen_clear_reads_back_solid_color() {
        let renderer = match pollster::block_on(WgpuRenderer::new_offscreen(64, 64)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("skipping offscreen render test: no GPU adapter ({e})");
                return;
            }
        };

        renderer.clear("#ff0000");
        let pixels = renderer
            .read_pixels()
            .expect("offscreen target yields pixels");

        assert_eq!(pixels.len(), 64 * 64 * 4, "tightly-packed RGBA8");
        // sRGB endpoints are invariant, so a pure-red clear reads back as (255,0,0,255).
        assert!(
            pixels[0] > 200,
            "expected red channel high, got {}",
            pixels[0]
        );
        assert!(
            pixels[1] < 80,
            "expected green channel low, got {}",
            pixels[1]
        );
        assert!(
            pixels[2] < 80,
            "expected blue channel low, got {}",
            pixels[2]
        );
        assert_eq!(pixels[3], 255, "expected opaque alpha");
    }

    /// Locks in the linear (non-sRGB) offscreen target: a mid-tone CSS color must
    /// read back byte-for-byte. An sRGB target would re-encode 128 (~0.50) up to
    /// ~188, so this test fails if the format regresses to `Rgba8UnormSrgb`.
    #[test]
    fn offscreen_midtone_passthrough_is_linear() {
        let renderer = match pollster::block_on(WgpuRenderer::new_offscreen(64, 64)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("skipping offscreen midtone test: no GPU adapter ({e})");
                return;
            }
        };

        renderer.clear("#808080"); // byte 128 per channel
        let pixels = renderer
            .read_pixels()
            .expect("offscreen target yields pixels");

        // Allow ±2 for rounding; sRGB re-encoding would land near 188 and fail.
        for (i, label) in ["R", "G", "B"].iter().enumerate() {
            assert!(
                (126..=130).contains(&pixels[i]),
                "{label} expected ~128 (linear passthrough), got {}",
                pixels[i]
            );
        }
    }

    /// Render real content (a triangle on a dark clear) and confirm we produce a
    /// valid PNG headlessly — the byte stream a webview can display directly.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn offscreen_encodes_valid_png() {
        let renderer = match pollster::block_on(WgpuRenderer::new_offscreen(96, 96)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("skipping png test: no GPU adapter ({e})");
                return;
            }
        };

        renderer.clear("#101820");
        renderer.fill_polygon(
            &[
                ScreenPoint { x: 48.0, y: 12.0 },
                ScreenPoint { x: 12.0, y: 82.0 },
                ScreenPoint { x: 84.0, y: 82.0 },
            ],
            "#67e8f9",
            1.0,
        );

        let png = renderer.read_png().expect("offscreen yields a PNG");

        // PNG 8-byte signature.
        assert_eq!(
            &png[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "expected PNG magic bytes"
        );
        assert!(
            png.len() > 100,
            "PNG should be non-trivial, got {} bytes",
            png.len()
        );
        eprintln!("encoded headless PNG: {} bytes", png.len());
    }

    /// The one-call bridge the native webview uses: render → PNG → base64 data-URI.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn preview_data_uri_is_well_formed() {
        let uri = match render_preview_data_uri(80, 80) {
            Some(u) => u,
            None => {
                eprintln!("skipping data-uri test: no GPU adapter");
                return;
            }
        };
        assert!(
            uri.starts_with("data:image/png;base64,"),
            "expected PNG data-URI, got: {}",
            &uri[..uri.len().min(40)]
        );
        assert!(uri.len() > 200, "data-URI should carry real image data");
        eprintln!("preview data-URI: {} chars", uri.len());
    }

    /// Test RenderScene contract with nodes (points)
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn render_scene_with_nodes() {
        let mut scene = RenderScene::new();
        scene.add_node(SceneNode {
            position: ScenePoint {
                x: 0.5,
                y: 0.5,
                z: 0.0,
            },
            color: "#ff0000".to_string(),
            radius: 10.0,
            alpha: 1.0,
        });
        scene.add_node(SceneNode {
            position: ScenePoint {
                x: 0.2,
                y: 0.3,
                z: 0.0,
            },
            color: "#00ff00".to_string(),
            radius: 5.0,
            alpha: 0.8,
        });

        let png = match render_scene_png(&scene, 64, 64) {
            Some(p) => p,
            None => {
                eprintln!("skipping render_scene test: no GPU adapter");
                return;
            }
        };

        assert_eq!(
            &png[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "expected PNG magic bytes"
        );
        eprintln!("rendered scene with nodes: {} bytes", png.len());
    }

    /// Test RenderScene contract with edges (lines)
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn render_scene_with_edges() {
        let mut scene = RenderScene::new();
        scene.add_edge(SceneEdge {
            from: ScenePoint {
                x: 0.1,
                y: 0.1,
                z: 0.0,
            },
            to: ScenePoint {
                x: 0.9,
                y: 0.9,
                z: 0.0,
            },
            color: "#67e8f9".to_string(),
            width: 2.0,
            alpha: 1.0,
        });

        let png = match render_scene_png(&scene, 64, 64) {
            Some(p) => p,
            None => {
                eprintln!("skipping render_scene test: no GPU adapter");
                return;
            }
        };

        assert_eq!(
            &png[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "expected PNG magic bytes"
        );
        eprintln!("rendered scene with edges: {} bytes", png.len());
    }

    /// Test RenderScene contract with faces (filled polygons)
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn render_scene_with_faces() {
        let mut scene = RenderScene::new();
        scene.add_face(SceneFace {
            vertices: vec![
                ScenePoint {
                    x: 0.5,
                    y: 0.1,
                    z: 0.0,
                },
                ScenePoint {
                    x: 0.1,
                    y: 0.9,
                    z: 0.0,
                },
                ScenePoint {
                    x: 0.9,
                    y: 0.9,
                    z: 0.0,
                },
            ],
            color: "#f59e0b".to_string(),
            alpha: 1.0,
        });

        let png = match render_scene_png(&scene, 64, 64) {
            Some(p) => p,
            None => {
                eprintln!("skipping render_scene test: no GPU adapter");
                return;
            }
        };

        assert_eq!(
            &png[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "expected PNG magic bytes"
        );
        eprintln!("rendered scene with faces: {} bytes", png.len());
    }

    /// Test RenderScene contract with mixed content (nodes, edges, faces)
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn render_scene_mixed() {
        let mut scene = RenderScene::new();
        scene.set_background("#101820");

        // Add a face
        scene.add_face(SceneFace {
            vertices: vec![
                ScenePoint {
                    x: 0.5,
                    y: 0.2,
                    z: 0.0,
                },
                ScenePoint {
                    x: 0.2,
                    y: 0.8,
                    z: 0.0,
                },
                ScenePoint {
                    x: 0.8,
                    y: 0.8,
                    z: 0.0,
                },
            ],
            color: "#3b82f6".to_string(),
            alpha: 0.5,
        });

        // Add edges
        scene.add_edge(SceneEdge {
            from: ScenePoint {
                x: 0.2,
                y: 0.2,
                z: 0.0,
            },
            to: ScenePoint {
                x: 0.8,
                y: 0.2,
                z: 0.0,
            },
            color: "#ffffff".to_string(),
            width: 1.0,
            alpha: 1.0,
        });

        // Add nodes
        scene.add_node(SceneNode {
            position: ScenePoint {
                x: 0.5,
                y: 0.5,
                z: 0.0,
            },
            color: "#ef4444".to_string(),
            radius: 8.0,
            alpha: 1.0,
        });

        let png = match render_scene_png(&scene, 96, 96) {
            Some(p) => p,
            None => {
                eprintln!("skipping render_scene test: no GPU adapter");
                return;
            }
        };

        assert_eq!(
            &png[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "expected PNG magic bytes"
        );
        assert!(png.len() > 100, "PNG should be non-trivial");
        eprintln!("rendered mixed scene: {} bytes", png.len());
    }

    /// Test RenderScene data URI output
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn render_scene_data_uri_is_well_formed() {
        let scene = RenderScene::new();
        let uri = match render_scene_data_uri(&scene, 64, 64) {
            Some(u) => u,
            None => {
                eprintln!("skipping data-uri test: no GPU adapter");
                return;
            }
        };
        assert!(
            uri.starts_with("data:image/png;base64,"),
            "expected PNG data-URI, got: {}",
            &uri[..uri.len().min(40)]
        );
        assert!(uri.len() > 200, "data-URI should carry real image data");
        eprintln!("scene data-URI: {} chars", uri.len());
    }
}
