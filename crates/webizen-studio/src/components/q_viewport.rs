//! `<q-viewport>` — declarative GPU viewport component (plan §7.3 W5).
//!
//! A Dioxus component that mounts a canvas, detects the GPU backend,
//! initializes the GPU portal, runs a reactive frame loop driving
//! `Render.gpu_render_frame`, handles resize/camera events, and cleans up
//! on unmount.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::components::q_viewport::QViewport;
//!
//! rsx! {
//!     QViewport {
//!         width: 1024,
//!         height: 768,
//!         target_fps: 60,
//!     }
//! }
//! ```
//!
//! ## Backend selection
//!
//! 1. Calls `Render.gpu_backend_info` to detect WebGPU/WebGL2 availability.
//! 2. If WebGPU is available, uses `Render.gpu_init` for full GPU rendering.
//! 3. If only WebGL2 is available, uses the WebGL2 fallback path.
//! 4. If no GPU backend is available, shows a fallback message.
//!
//! ## Reactive frame loop
//!
//! The frame loop runs via `spawn` + a timing loop:
//! - On each tick, calls `Render.gpu_render_frame` with the current sim-time.
//! - The sim-time advances by `1/target_fps` per frame.
//! - The loop stops when the component unmounts.

use dioxus::prelude::*;
use serde_json::{json, Value};

use crate::components::qapp_engine::invoke_json;
use crate::render::q_viewport::{
    self, ViewportBackend, ViewportConfig, ViewportState,
};

// ── Async invoke helpers ──────────────────────────────────────────────────
// These call VibeScript `capability.invoke` through the `poet_eval` Tauri
// command, which evaluates the script and returns the result.

/// Detect the available GPU backend.
async fn detect_backend() -> Result<ViewportBackend, String> {
    let script = r#"requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Render.gpu_backend_info", null);
}"#;
    let result = invoke_json(
        "poet_eval",
        json!({ "source": script, "as_cell": false, "function": "go" }),
    )
    .await?;
    Ok(q_viewport::extract_backend(&result))
}

/// Initialize the GPU viewport.
async fn mount_gpu(config: &ViewportConfig) -> Result<(u64, ViewportBackend), String> {
    let backend = detect_backend().await.unwrap_or(ViewportBackend::None);
    let script = format!(
        r#"requires [ capability("capability.invoke") ];
effect fn go() {{
    return capability.invoke("Render.gpu_init", {{
        width: {width}, height: {height}, particle_cap: {particle_cap}
    }});
}}"#,
        width = config.width,
        height = config.height,
        particle_cap = config.particle_cap
    );
    let result = invoke_json(
        "poet_eval",
        json!({ "source": script, "as_cell": false, "function": "go" }),
    )
    .await?;
    let handle = q_viewport::extract_handle(&result)?;
    Ok((handle, backend))
}

/// Render one frame.
async fn render_frame(handle: u64, time: f32) -> Result<(), String> {
    let script = format!(
        r#"requires [ capability("capability.invoke") ];
effect fn go() {{
    return capability.invoke("Render.gpu_render_frame", {{
        handle: {handle}, time: {time}
    }});
}}"#,
        handle = handle,
        time = time
    );
    let _ = invoke_json(
        "poet_eval",
        json!({ "source": script, "as_cell": false, "function": "go" }),
    )
    .await?;
    Ok(())
}

/// Resize the GPU viewport.
async fn resize_gpu(handle: u64, width: u32, height: u32) -> Result<(), String> {
    let script = format!(
        r#"requires [ capability("capability.invoke") ];
effect fn go() {{
    return capability.invoke("Render.gpu_resize", {{
        handle: {handle}, width: {width}, height: {height}
    }});
}}"#,
        handle = handle,
        width = width,
        height = height
    );
    let _ = invoke_json(
        "poet_eval",
        json!({ "source": script, "as_cell": false, "function": "go" }),
    )
    .await?;
    Ok(())
}

/// Set the camera.
async fn set_camera(handle: u64, yaw: f32, pitch: f32, zoom: f32) -> Result<(), String> {
    let script = format!(
        r#"requires [ capability("capability.invoke") ];
effect fn go() {{
    return capability.invoke("Render.gpu_set_camera", {{
        handle: {handle}, yaw: {yaw}, pitch: {pitch}, zoom: {zoom}
    }});
}}"#,
        handle = handle,
        yaw = yaw,
        pitch = pitch,
        zoom = zoom
    );
    let _ = invoke_json(
        "poet_eval",
        json!({ "source": script, "as_cell": false, "function": "go" }),
    )
    .await?;
    Ok(())
}

/// Destroy the GPU viewport.
async fn unmount_gpu(handle: u64) -> Result<(), String> {
    let script = format!(
        r#"requires [ capability("capability.invoke") ];
effect fn go() {{
    return capability.invoke("Render.gpu_destroy", {{ handle: {handle} }});
}}"#,
        handle = handle
    );
    let _ = invoke_json(
        "poet_eval",
        json!({ "source": script, "as_cell": false, "function": "go" }),
    )
    .await?;
    Ok(())
}

/// `<q-viewport>` — declarative GPU viewport with reactive frame loop.
///
/// Mounts a canvas, initializes the GPU, runs a frame loop, and handles
/// camera/resize events. The viewport auto-mounts on creation unless
/// `auto_mount` is false.
#[component]
pub fn QViewport(
    /// Canvas width in physical pixels.
    #[props(default = 800u32)]
    width: u32,
    /// Canvas height in physical pixels.
    #[props(default = 600u32)]
    height: u32,
    /// Particle capacity for the ambient field.
    #[props(default = 4096u32)]
    particle_cap: u32,
    /// Target frame rate (frames per second). 0 = uncapped.
    #[props(default = 60u32)]
    target_fps: u32,
    /// Whether to auto-mount on creation.
    #[props(default = true)]
    auto_mount: bool,
) -> Element {
    let mut state = use_signal(|| ViewportState::default());
    let mut camera_yaw = use_signal(|| 0.0_f32);
    let mut camera_pitch = use_signal(|| -0.3_f32);
    let mut camera_zoom = use_signal(|| 1.0_f32);
    let mut dragging = use_signal(|| false);
    let mut last_x = use_signal(|| 0.0_f64);
    let mut last_y = use_signal(|| 0.0_f64);
    let mut status_msg = use_signal(|| "Initializing…".to_string());

    // Mount the GPU viewport on component creation.
    use_effect(move || {
        if !auto_mount {
            return;
        }
        let config = ViewportConfig {
            width,
            height,
            particle_cap,
            target_fps,
            auto_mount,
            ..Default::default()
        };
        spawn(async move {
            status_msg.set("Detecting backend…".to_string());

            // Mount the GPU.
            match mount_gpu(&config).await {
                Ok((handle, backend)) => {
                    state.set(ViewportState {
                        handle: Some(handle),
                        backend,
                        frame_count: 0,
                        last_time: 0.0,
                        running: true,
                        last_error: None,
                    });
                    status_msg.set(format!(
                        "Mounted: {:?} (handle={})",
                        backend, handle
                    ));

                    // Send initial camera state.
                    let _ = set_camera(
                        handle,
                        *camera_yaw.read(),
                        *camera_pitch.read(),
                        *camera_zoom.read(),
                    )
                    .await;

                    // Run the frame loop.
                    let frame_dt = if target_fps > 0 {
                        1.0 / target_fps as f32
                    } else {
                        1.0 / 60.0
                    };
                    let mut sim_time = 0.0_f32;
                    let mut frame_count = 0u64;

                    loop {
                        let s = state.read();
                        if !s.running || s.handle.is_none() {
                            break;
                        }
                        let handle = s.handle.unwrap();
                        drop(s);

                        // Render one frame.
                        if let Err(e) = render_frame(handle, sim_time).await {
                            let mut s = state.write();
                            s.last_error = Some(e.clone());
                            s.running = false;
                            status_msg.set(format!("Frame error: {e}"));
                            break;
                        }

                        frame_count += 1;
                        sim_time += frame_dt;

                        {
                            let mut s = state.write();
                            s.frame_count = frame_count;
                            s.last_time = sim_time;
                        }

                        // Yield to the scheduler — the render_frame await point
                        // already yields; this tiny pause prevents a busy loop
                        // when render_frame returns instantly (e.g. on error).
                        // On WASM, use setTimeout(0); on native, tokio::task::yield_now.
                        #[cfg(target_arch = "wasm32")]
                        {
                            let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                                if let Some(window) = web_sys::window() {
                                    let cb = wasm_bindgen::closure::Closure::once(move || {
                                        let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
                                    });
                                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                                        cb.as_ref().unchecked_ref(),
                                        0,
                                    );
                                    cb.forget();
                                } else {
                                    let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
                                }
                            });
                            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            // On native, yield to the tokio runtime.
                            tokio::task::yield_now().await;
                        }
                    }
                }
                Err(e) => {
                    status_msg.set(format!("Mount failed: {e}"));
                    state.set(ViewportState {
                        last_error: Some(e),
                        ..Default::default()
                    });
                }
            }
        });
    });

    // Send camera updates when camera state changes.
    use_effect(move || {
        let s = state.read();
        if !s.is_mounted() {
            return;
        }
        let handle = s.handle.unwrap();
        drop(s);
        let yaw = *camera_yaw.read();
        let pitch = *camera_pitch.read();
        let zoom = *camera_zoom.read();
        spawn(async move {
            let _ = set_camera(handle, yaw, pitch, zoom).await;
        });
    });

    // Mouse handlers for camera orbit.
    let onmousedown = move |e: MouseEvent| {
        dragging.set(true);
        let coords = e.data().client_coordinates();
        last_x.set(coords.x);
        last_y.set(coords.y);
    };

    let onmousemove = move |e: MouseEvent| {
        if !*dragging.read() {
            return;
        }
        let coords = e.data().client_coordinates();
        let dx = coords.x - *last_x.read();
        let dy = coords.y - *last_y.read();
        last_x.set(coords.x);
        last_y.set(coords.y);

        let mut yaw = camera_yaw.write();
        *yaw += dx as f32 * 0.01;
        let mut pitch = camera_pitch.write();
        *pitch += dy as f32 * 0.01;
        *pitch = pitch.clamp(-1.5, 1.5);
    };

    let onmouseup = move |_e: MouseEvent| {
        dragging.set(false);
    };

    let onwheel = move |e: WheelEvent| {
        let delta = match e.data().delta() {
            dioxus::html::geometry::WheelDelta::Pixels(p) => p.y,
            dioxus::html::geometry::WheelDelta::Lines(l) => l.y * 100.0,
            dioxus::html::geometry::WheelDelta::Pages(p) => p.y * 800.0,
        };
        let mut zoom = camera_zoom.write();
        *zoom *= if delta > 0.0 { 0.9 } else { 1.1 };
        *zoom = zoom.clamp(0.1, 10.0);
    };

    // Unmount handler.
    let on_unmount = move |_| {
        let handle = {
            let s = state.read();
            s.handle
        };
        if let Some(handle) = handle {
            {
                let mut s = state.write();
                s.running = false;
            }
            spawn(async move {
                let _ = unmount_gpu(handle).await;
                state.set(ViewportState::default());
                status_msg.set("Unmounted".to_string());
            });
        }
    };

    let s = state.read();
    let camera_info = format!(
        "Yaw: {:.1}° Pitch: {:.1}° Zoom: {:.2} · Frame: {}",
        *camera_yaw.read() * 57.2958,
        *camera_pitch.read() * 57.2958,
        *camera_zoom.read(),
        s.frame_count,
    );

    rsx! {
        div {
            style: "position:relative;width:100%;height:100%;overflow:hidden;background:#000;",

            // Error/fallback display.
            if let Some(err) = &s.last_error {
                div {
                    style: "position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);color:#e74c3c;font-size:0.85rem;text-align:center;padding:1rem;max-width:400px;",
                    "{err}"
                }
            } else if !s.is_mounted() && *status_msg.read() != "Initializing…" {
                div {
                    style: "position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);color:#94a3b8;font-size:0.85rem;text-align:center;padding:1rem;max-width:400px;",
                    "{status_msg}"
                }
            }

            // The viewport container — the GPU renders into this area.
            // On native, the GPU surface is a child HWND positioned over this div.
            // On WASM, a canvas element would be inserted here.
            div {
                id: "q-viewport-container",
                style: "position:absolute;inset:0;cursor:grab;",
                onmousedown: onmousedown,
                onmousemove: onmousemove,
                onmouseup: onmouseup,
                onmouseleave: move |_| dragging.set(false),
                onwheel: onwheel,
            }

            // Status overlay (top-left).
            div {
                style: "position:absolute;top:0.5rem;left:0.5rem;padding:0.25rem 0.5rem;background:rgba(0,0,0,0.6);border-radius:4px;font-size:0.75rem;color:#aaa;pointer-events:none;",
                if s.is_mounted() {
                    "q-viewport: {status_msg} · Backend: {s.backend:?}"
                } else {
                    "q-viewport: {status_msg}"
                }
            }

            // Controls overlay (top-right).
            div {
                style: "position:absolute;top:0.5rem;right:0.5rem;display:flex;gap:0.25rem;",
                button {
                    r#type: "button",
                    onclick: on_unmount,
                    style: "padding:0.2rem 0.5rem;border:1px solid #333;border-radius:4px;background:rgba(0,0,0,0.6);color:#aaa;cursor:pointer;font-size:0.75rem;",
                    "Unmount"
                }
            }

            // Camera info overlay (bottom-left).
            div {
                style: "position:absolute;bottom:0.5rem;left:0.5rem;padding:0.25rem 0.5rem;background:rgba(0,0,0,0.6);border-radius:4px;font-size:0.7rem;color:#888;pointer-events:none;",
                "{camera_info}"
            }
        }
    }
}

/// Convenience component: a full-page `<q-viewport>` with a header.
#[component]
pub fn QViewportPage(
    #[props(default = 1200u32)]
    width: u32,
    #[props(default = 800u32)]
    height: u32,
) -> Element {
    rsx! {
        div {
            style: "display:flex;flex-direction:column;height:calc(100vh - 60px);",

            // Header.
            div {
                style: "padding:0.75rem 1rem;border-bottom:1px solid var(--qualia-border,#333);display:flex;align-items:center;justify-content:space-between;",
                div {
                    h1 { style: "margin:0;font-size:1.1rem;", "q-viewport" }
                    p {
                        style: "margin:0;font-size:0.78rem;color:var(--qualia-text-muted,#888);",
                        "Declarative GPU viewport — WebGPU first, WebGL2 fallback"
                    }
                }
            }

            // Viewport fills the rest.
            div {
                style: "flex:1;position:relative;",
                QViewport { width: width, height: height, target_fps: 60 }
            }
        }
    }
}
