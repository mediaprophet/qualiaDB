//! GPU render preview component.
//!
//! Triggers a headless `wgpu` render in the Tauri host (`update_render_preview`)
//! and displays the result via the `webizen://localhost/render/preview.png`
//! custom protocol. The PNG bytes are fetched by the `<img>` directly from the
//! backend and never cross the Dioxus Virtual DOM — mirroring the diffusion
//! visualizer's zero-VDOM frame path, but using `<img>` + the host's PNG encoder
//! rather than manual canvas blitting.

use crate::components::camera_controls::{CameraControlState, CameraControls};
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(
        event: &str,
        handler: &js_sys::Function,
    ) -> Result<js_sys::Function, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_tauri_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

/// Renders a headless GPU frame in the native host and shows it via `<img>`.
#[component]
pub fn RenderPreview(width: u32, height: u32) -> Element {
    // Bumped each time the host signals a new frame, to bust the webview cache.
    let epoch = use_signal(|| 0u64);
    let status = use_signal(|| "Initializing GPU preview…".to_string());
    #[cfg(target_arch = "wasm32")]
    let started = use_signal(|| false);

    // Camera state - zero-heap compliant (all Copy types)
    let mut camera_state = use_signal(|| RenderCameraState::default_camera());

    // Camera control handlers - zero-heap compliant (stack-allocated parameters)
    let handle_orbit = move |(yaw, pitch): (f64, f64)| {
        let mut cam = camera_state.write();
        // Calculate orbit using stack-allocated math
        let dx = cam.eye_x - cam.target_x;
        let dy = cam.eye_y - cam.target_y;
        let dz = cam.eye_z - cam.target_z;
        let radius = (dx * dx + dy * dy + dz * dz).sqrt();
        let current_yaw = dz.atan2(dx);
        let current_pitch = (dy / radius).asin();
        let new_yaw = current_yaw + yaw;
        let new_pitch = (current_pitch + pitch).clamp(
            -std::f64::consts::FRAC_PI_2 + 0.01,
            std::f64::consts::FRAC_PI_2 - 0.01,
        );
        let cos_pitch = new_pitch.cos();
        let sin_pitch = new_pitch.sin();
        let cos_yaw = new_yaw.cos();
        let sin_yaw = new_yaw.sin();
        cam.eye_x = cam.target_x + radius * cos_pitch * cos_yaw;
        cam.eye_y = cam.target_y + radius * sin_pitch;
        cam.eye_z = cam.target_z + radius * cos_pitch * sin_yaw;
    };

    let handle_zoom = move |delta: f64| {
        let mut cam = camera_state.write();
        let dx = cam.eye_x - cam.target_x;
        let dy = cam.eye_y - cam.target_y;
        let dz = cam.eye_z - cam.target_z;
        let current_distance = (dx * dx + dy * dy + dz * dz).sqrt();
        let zoom_factor = (-delta * 0.1).exp();
        let new_distance = (current_distance * zoom_factor).max(0.5);
        let scale = new_distance / current_distance;
        cam.eye_x = cam.target_x + dx * scale;
        cam.eye_y = cam.target_y + dy * scale;
        cam.eye_z = cam.target_z + dz * scale;
    };

    let handle_pan = move |(dx, dy): (f64, f64)| {
        let mut cam = camera_state.write();
        let forward_x = cam.target_x - cam.eye_x;
        let forward_y = cam.target_y - cam.eye_y;
        let forward_z = cam.target_z - cam.eye_z;
        let forward_len =
            (forward_x * forward_x + forward_y * forward_y + forward_z * forward_z).sqrt();
        let forward_x = forward_x / forward_len;
        let forward_y = forward_y / forward_len;
        let forward_z = forward_z / forward_len;
        let right_x = forward_y * cam.up_z - forward_z * cam.up_y;
        let right_y = forward_z * cam.up_x - forward_x * cam.up_z;
        let right_z = forward_x * cam.up_y - forward_y * cam.up_x;
        let right_len = (right_x * right_x + right_y * right_y + right_z * right_z).sqrt();
        let right_x = right_x / right_len;
        let right_y = right_y / right_len;
        let right_z = right_z / right_len;
        let up_x = right_y * forward_z - right_z * forward_y;
        let up_y = right_z * forward_x - right_x * forward_z;
        let up_z = right_x * forward_y - right_y * forward_x;
        let pan_scale = forward_len * 0.001;
        let pan_x = (right_x * dx + up_x * dy) * pan_scale;
        let pan_y = (right_y * dx + up_y * dy) * pan_scale;
        let pan_z = (right_z * dx + up_z * dy) * pan_scale;
        cam.eye_x += pan_x;
        cam.eye_y += pan_y;
        cam.eye_z += pan_z;
        cam.target_x += pan_x;
        cam.target_y += pan_y;
        cam.target_z += pan_z;
    };

    // Lifecycle bridge: start daemon on mount (native desktop)
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Note: Desktop-specific lifecycle handled by host application
        // The daemon is started via Tauri commands from the desktop host
    }

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let mut started = started;
            if started() {
                return;
            }
            started.set(true);

            let epoch = epoch;
            let mut status = status;

            wasm_bindgen_futures::spawn_local(async move {
                // Bump the cache-busting epoch whenever a new frame is ready.
                let callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event: JsValue| {
                        let mut epoch = epoch;
                        epoch.set(epoch() + 1);
                    }));
                if tauri_listen("render-preview-ready", callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    callback.forget();
                }

                // Kick off the first render in the host.
                match invoke_tauri_json::<()>(
                    "update_render_preview",
                    json!({ "width": width, "height": height }),
                )
                .await
                {
                    Ok(_) => status.set("Streaming GPU frames".to_string()),
                    Err(err) => status.set(format!("GPU preview unavailable: {err}")),
                }
            });
        }
    });

    // Native event listener for render-preview-ready
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Note: Desktop-specific event handling handled by host application
        // The desktop host manages the render loop and epoch updates
    }

    let current = epoch();
    let status_text = status();
    let src = format!("webizen://localhost/render/preview.png?t={current}");

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
                "Headless wgpu frame served over the webizen:// protocol. No image bytes pass through the Dioxus Virtual DOM."
            }

            div {
                style: "display: flex; gap: 16px; align-items: flex-start;",

                // Render preview image
                div {
                    style: "flex: 1;",
                    if current > 0 {
                        img {
                            src: "{src}",
                            width: "{width}",
                            height: "{height}",
                            style: "display: block; max-width: 100%; border-radius: 10px; background: rgba(0,0,0,0.2);",
                        }
                    } else {
                        div {
                            style: "display: flex; align-items: center; justify-content: center; min-height: 160px; font-size: 0.82rem; color: var(--qualia-text-muted); border-radius: 10px; background: rgba(0,0,0,0.12);",
                            "{status_text}"
                        }
                    }
                }

                // Camera controls sidebar
                CameraControls {
                    on_orbit: handle_orbit,
                    on_zoom: handle_zoom,
                    on_pan: handle_pan,
                    initial_state: CameraControlState::new(),
                }
            }
        }
    }
}
