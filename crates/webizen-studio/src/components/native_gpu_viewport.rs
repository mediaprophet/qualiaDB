//! Native GPU surface viewport — renders directly to a child HWND via wgpu,
//! bypassing the PNG round-trip entirely.
//!
//! This component calls `mount_gpu_surface` on mount, which creates a child
//! HWND inside the Tauri window and a wgpu::Surface that presents directly
//! to the GPU swapchain. The render loop runs on a background thread in the
//! native host.

use dioxus::prelude::*;
use serde_json::json;

use crate::components::qapp_engine::invoke_json;

/// Native GPU surface viewport — renders directly to a child HWND.
///
/// On mount, calls `mount_gpu_surface` to create a child window + wgpu surface.
/// The render loop runs in the native host thread. Camera updates are sent
/// via `set_gpu_camera`. On unmount, calls `unmount_gpu_surface`.
#[component]
pub fn NativeGpuViewport(
    #[props(default = 1200u32)] width: u32,
    #[props(default = 800u32)] height: u32,
    #[props(default = true)] auto_mount: bool,
) -> Element {
    let mut mounted = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let mut camera_yaw = use_signal(|| 0.0_f32);
    let mut camera_pitch = use_signal(|| -0.3_f32);
    let mut camera_zoom = use_signal(|| 1.0_f32);
    let mut dragging = use_signal(|| false);
    let mut last_x = use_signal(|| 0.0_f64);
    let mut last_y = use_signal(|| 0.0_f64);

    // Mount the GPU surface on component creation
    use_effect(move || {
        if !auto_mount || *mounted.read() {
            return;
        }

        spawn(async move {
            // Position the child HWND below the nav bar
            match invoke_json(
                "mount_gpu_surface",
                json!({ "x": 0, "y": 60, "width": width, "height": height }),
            )
            .await
            {
                Ok(_) => {
                    mounted.set(true);
                    // Send initial camera state
                    let _ = invoke_json(
                        "set_gpu_camera",
                        json!({
                            "yaw": *camera_yaw.read(),
                            "pitch": *camera_pitch.read(),
                            "zoom": *camera_zoom.read(),
                        }),
                    )
                    .await;
                }
                Err(e) => {
                    error_msg.set(Some(format!("mount_gpu_surface failed: {}", e)));
                }
            }
        });
    });

    // Send camera updates when camera state changes
    use_effect(move || {
        if !*mounted.read() {
            return;
        }
        let yaw = *camera_yaw.read();
        let pitch = *camera_pitch.read();
        let zoom = *camera_zoom.read();
        spawn(async move {
            let _ = invoke_json(
                "set_gpu_camera",
                json!({ "yaw": yaw, "pitch": pitch, "zoom": zoom }),
            )
            .await;
        });
    });

    // Mouse handlers for camera orbit
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

    // Unmount button
    let on_unmount = move |_| {
        spawn(async move {
            let _ = invoke_json("unmount_gpu_surface", json!({})).await;
            mounted.set(false);
        });
    };

    let camera_info = format!(
        "Yaw: {:.1}° Pitch: {:.1}° Zoom: {:.2}",
        *camera_yaw.read(),
        *camera_pitch.read(),
        *camera_zoom.read()
    );

    rsx! {
        div {
            style: "position:relative;width:100%;height:100%;overflow:hidden;background:#000;",

            // Error display
            if let Some(err) = error_msg.read().as_ref() {
                div {
                    style: "position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);color:#e74c3c;font-size:0.85rem;text-align:center;padding:1rem;max-width:400px;",
                    "{err}"
                }
            }

            // The container div — the child HWND will be positioned over this area.
            div {
                id: "gpu-viewport-container",
                style: "position:absolute;inset:0;cursor:grab;",
                onmousedown: onmousedown,
                onmousemove: onmousemove,
                onmouseup: onmouseup,
                onmouseleave: move |_| dragging.set(false),
                onwheel: onwheel,
            }

            // Status overlay (top-left)
            div {
                style: "position:absolute;top:0.5rem;left:0.5rem;padding:0.25rem 0.5rem;background:rgba(0,0,0,0.6);border-radius:4px;font-size:0.75rem;color:#aaa;pointer-events:none;",
                if *mounted.read() {
                    "GPU Surface: Active (direct-to-swapchain)"
                } else {
                    "GPU Surface: Initializing..."
                }
            }

            // Controls overlay (top-right)
            div {
                style: "position:absolute;top:0.5rem;right:0.5rem;display:flex;gap:0.25rem;",
                button {
                    r#type: "button",
                    onclick: on_unmount,
                    style: "padding:0.2rem 0.5rem;border:1px solid #333;border-radius:4px;background:rgba(0,0,0,0.6);color:#aaa;cursor:pointer;font-size:0.75rem;",
                    "Unmount"
                }
            }

            // Camera info overlay (bottom-left)
            div {
                style: "position:absolute;bottom:0.5rem;left:0.5rem;padding:0.25rem 0.5rem;background:rgba(0,0,0,0.6);border-radius:4px;font-size:0.7rem;color:#888;pointer-events:none;",
                "{camera_info}"
            }
        }
    }
}

/// Convenience component: a full-page native GPU viewport with a header.
#[component]
pub fn NativeGpuViewportPage() -> Element {
    rsx! {
        div {
            style: "display:flex;flex-direction:column;height:calc(100vh - 60px);",

            // Header
            div {
                style: "padding:0.75rem 1rem;border-bottom:1px solid var(--qualia-border,#333);display:flex;align-items:center;justify-content:space-between;",
                div {
                    h1 { style: "margin:0;font-size:1.1rem;", "Native GPU Viewport" }
                    p {
                        style: "margin:0;font-size:0.78rem;color:var(--qualia-text-muted,#888);",
                        "Direct-to-swapchain rendering — no PNG round-trip"
                    }
                }
            }

            // Viewport fills the rest
            div {
                style: "flex:1;position:relative;",
                NativeGpuViewport { width: 1200, height: 800, auto_mount: true }
            }
        }
    }
}
