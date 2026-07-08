//! GPU render preview component.
//!
//! Triggers a headless `wgpu` render in the Tauri host (`update_render_preview`)
//! and displays the result via the `webizen://localhost/render/preview.png`
//! custom protocol. The PNG bytes are fetched by the `<img>` directly from the
//! backend and never cross the Dioxus Virtual DOM — mirroring the diffusion
//! visualizer's zero-VDOM frame path, but using `<img>` + the host's PNG encoder
//! rather than manual canvas blitting.

use crate::components::camera_controls::{CameraControlState, CameraControls};
use crate::components::native_gpu_viewport::NativeGpuViewport;
use dioxus::prelude::*;

/// Camera state for zero-heap compliance (all Copy types)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct RenderCameraState {
    eye_x: f64,
    eye_y: f64,
    eye_z: f64,
    target_x: f64,
    target_y: f64,
    target_z: f64,
    up_x: f64,
    up_y: f64,
    up_z: f64,
    fov: f64,
}

impl RenderCameraState {
    fn default_camera() -> Self {
        Self {
            eye_x: 0.0,
            eye_y: 0.0,
            eye_z: 5.0,
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,
            up_x: 0.0,
            up_y: 1.0,
            up_z: 0.0,
            fov: 60.0,
        }
    }
}



/// Renders a headless GPU frame in the native host and shows it via `<img>`.
#[component]
pub fn RenderPreview(width: u32, height: u32) -> Element {
    rsx! {
        div {
            class: "panel-card",
            style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 18px; padding: 1.15rem 1.2rem 1.25rem; backdrop-filter: blur(24px); box-shadow: 0 8px 32px rgba(0,0,0,0.08);",

            h2 {
                style: "margin: 0 0 0.25rem 0; font-size: 0.98rem; font-weight: 700; color: var(--qualia-text);",
                "GPU Render Preview"
            }
            p {
                style: "margin: 0 0 0.9rem 0; font-size: 0.76rem; color: var(--qualia-text-muted); line-height: 1.45;",
                "Direct-to-swapchain native wgpu surface. No image bytes pass through the Dioxus Virtual DOM."
            }

            div {
                style: "display: flex; gap: 16px; align-items: flex-start;",

                div {
                    style: "flex: 1; position: relative;",
                    // We render NativeGpuViewport which instructs the Tauri backend to mount
                    // the child HWND over this specific div area.
                    NativeGpuViewport {
                        width: width,
                        height: height,
                        auto_mount: true,
                    }
                }

                // Camera controls sidebar
                CameraControls {
                    on_orbit: move |_| {},
                    on_zoom: move |_| {},
                    on_pan: move |_| {},
                    initial_state: CameraControlState::new(),
                }
            }
        }
    }
}
