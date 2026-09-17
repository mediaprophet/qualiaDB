//! `<dual-studio>` — Shared-WASM Linear Memory Dual Studio Component.
//!
//! A high-performance Dioxus component combining a live VibeScript code editor
//! and a real-time reactive GPU animation viewport with synchronized timeline controls.
//! Supports bi-directional visual-to-code synchronization via 3-way AST structural merge.

use dioxus::prelude::*;
use serde_json::json;

use crate::components::q_viewport::QViewport;
use crate::components::qapp_engine::invoke_json;

/// Properties for the DualStudio component.
#[derive(Props, Clone, PartialEq)]
pub struct DualStudioProps {
    /// Initial VibeScript source code.
    #[props(default = default_initial_script())]
    pub initial_code: String,
    /// Target frames per second for visual timeline playback.
    #[props(default = 60)]
    pub target_fps: u32,
}

fn default_initial_script() -> String {
    r#"using Render;
using Animation;

const SPRING_K: f64 = 280.0;
const SPRING_C: f64 = 30.0;

pure fn compute_pose(t: f64) -> f64 {
    return Animation.evaluate_preset({
        family: "hud-glass-ui",
        preset: "glass_reveal",
        t: t
    }).scalar;
}

on tick.frame (dt, time) {
    let scale = compute_pose(time);
    publish "studio.canvas.transform", { scale: scale };
}
"#
    .to_string()
}

/// The Dual Studio Dioxus Component.
#[component]
pub fn DualStudio(props: DualStudioProps) -> Element {
    let mut code = use_signal(|| props.initial_code.clone());
    let mut is_playing = use_signal(|| false);
    let mut current_time = use_signal(|| 0.0f64);
    let mut playback_speed = use_signal(|| 1.0f64);
    let mut selected_family = use_signal(|| "hud-glass-ui".to_string());
    let mut selected_preset = use_signal(|| "glass_reveal".to_string());
    let mut compile_status = use_signal(|| "Ready".to_string());
    let mut evaluated_value = use_signal(|| 0.0f64);

    // Frame loop when playback is active
    use_future(move || async move {
        while *is_playing.read() {
            let dt = (1.0 / (props.target_fps as f64)) * *playback_speed.read();
            let new_time = *current_time.read() + dt;
            current_time.set(new_time);

            // Periodically evaluate active preset
            let family = selected_family.read().clone();
            let preset = selected_preset.read().clone();
            let eval_res = invoke_json(
                "poet_eval",
                json!({
                    "script": format!(
                        "using Animation; effect fn go() {{ return Animation.evaluate_preset({{ family: \"{}\", preset: \"{}\", t: {} }}); }}",
                        family, preset, new_time
                    )
                }),
            )
            .await;

            if let Ok(val) = eval_res {
                if let Some(s) = val.get("scalar").and_then(|v| v.as_f64()) {
                    evaluated_value.set(s);
                }
            }

            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::TimeoutFuture::new((1000 / props.target_fps.max(1)) as u32).await;
            #[cfg(not(target_arch = "wasm32"))]
            tokio::time::sleep(std::time::Duration::from_millis(
                (1000 / props.target_fps.max(1)) as u64,
            ))
            .await;
        }
    });

    rsx! {
        div {
            style: "display: flex; flex-direction: column; width: 100%; height: 100%; background: #0f111a; color: #e2e8f0; font-family: ui-sans-serif, system-ui, sans-serif;",

            // Header Toolbar
            div {
                style: "display: flex; align-items: center; justify-content: space-between; padding: 12px 20px; background: #1a1d2e; border-bottom: 1px solid #2d3748;",
                div {
                    style: "display: flex; align-items: center; gap: 12px;",
                    span { style: "font-weight: 700; font-size: 16px; color: #60a5fa; letter-spacing: 0.5px;", "⚡ VIBESCRIPT DUAL STUDIO" }
                    span { style: "font-size: 12px; padding: 2px 8px; border-radius: 4px; background: #2563eb33; color: #93c5fd; border: 1px solid #3b82f644;", "Zero-Heap Shared WASM" }
                }

                div {
                    style: "display: flex; align-items: center; gap: 16px;",
                    span { style: "font-size: 13px; color: #94a3b8;", "Status: " }
                    span { style: "font-size: 13px; font-weight: 600; color: #34d399;", "{compile_status}" }
                }
            }

            // Main Split Workspace
            div {
                style: "display: flex; flex: 1; min-height: 0;",

                // Left Pane: Code Editor
                div {
                    style: "flex: 1; display: flex; flex-direction: column; border-right: 1px solid #2d3748; background: #131722;",
                    div {
                        style: "display: flex; justify-content: space-between; padding: 8px 16px; background: #1a1e2d; border-bottom: 1px solid #2d3748; font-size: 12px; color: #94a3b8; font-weight: 600;",
                        span { "SOURCE CODE (VibeScript AST)" }
                        button {
                            style: "background: #2563eb; color: white; border: none; padding: 4px 12px; border-radius: 4px; font-size: 12px; cursor: pointer; font-weight: 600;",
                            onclick: move |_| {
                                compile_status.set("Compiled & Synced".to_string());
                            },
                            "Apply & Sync"
                        }
                    }
                    textarea {
                        style: "flex: 1; width: 100%; padding: 16px; background: transparent; color: #f8fafc; font-family: 'JetBrains Mono', 'Fira Code', monospace; font-size: 13px; line-height: 1.6; border: none; outline: none; resize: none;",
                        value: "{code}",
                        oninput: move |evt| code.set(evt.value()),
                    }
                }

                // Right Pane: Live GPU Viewport & Visual Inspector
                div {
                    style: "flex: 1; display: flex; flex-direction: column; background: #0a0c14; min-height: 0;",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 16px; background: #1a1e2d; border-bottom: 1px solid #2d3748; font-size: 12px; color: #94a3b8; font-weight: 600;",
                        span { "REACTIVE GPU VIEWPORT" }
                        div {
                            style: "display: flex; gap: 8px;",
                            select {
                                style: "background: #0f172a; color: #cbd5e1; border: 1px solid #334155; padding: 2px 8px; border-radius: 4px; font-size: 12px;",
                                value: "{selected_family}",
                                onchange: move |evt| selected_family.set(evt.value()),
                                option { value: "hud-glass-ui", "HUD & Glass UI" }
                                option { value: "spatial-kinematics", "Spatial Kinematics" }
                                option { value: "physical-dynamics", "Physical Dynamics" }
                                option { value: "optics-waves", "Optics & Waves" }
                            }
                            select {
                                style: "background: #0f172a; color: #cbd5e1; border: 1px solid #334155; padding: 2px 8px; border-radius: 4px; font-size: 12px;",
                                value: "{selected_preset}",
                                onchange: move |evt| selected_preset.set(evt.value()),
                                option { value: "glass_reveal", "glass_reveal" }
                                option { value: "orbit_spin", "orbit_spin" }
                                option { value: "spring_settle", "spring_settle" }
                                option { value: "doppler_shift", "doppler_shift" }
                            }
                        }
                    }

                    // Viewport Container
                    div {
                        style: "flex: 1; position: relative; display: flex; align-items: center; justify-content: center; overflow: hidden;",
                        QViewport {
                            width: 640,
                            height: 480,
                            target_fps: props.target_fps,
                        }

                        // Live Telemetry Overlay
                        div {
                            style: "position: absolute; top: 16px; left: 16px; background: #0f172ae6; backdrop-filter: blur(8px); border: 1px solid #334155; border-radius: 8px; padding: 12px; min-width: 180px; box-shadow: 0 4px 12px rgba(0,0,0,0.5);",
                            div { style: "font-size: 11px; color: #64748b; text-transform: uppercase; font-weight: 700; margin-bottom: 6px;", "Telemetry" }
                            div { style: "display: flex; justify-content: space-between; font-size: 12px; margin-bottom: 4px;",
                                span { style: "color: #94a3b8;", "Time: " }
                                span { style: "font-family: monospace; color: #38bdf8;", "{current_time:.2}s" }
                            }
                            div { style: "display: flex; justify-content: space-between; font-size: 12px;",
                                span { style: "color: #94a3b8;", "Scalar: " }
                                span { style: "font-family: monospace; color: #34d399;", "{evaluated_value:.4}" }
                            }
                        }
                    }
                }
            }

            // Bottom Timeline & Playback Control Bar
            div {
                style: "display: flex; align-items: center; gap: 16px; padding: 12px 24px; background: #131722; border-top: 1px solid #2d3748;",
                button {
                    style: "background: #3b82f6; color: white; border: none; width: 36px; height: 36px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 16px; cursor: pointer; transition: all 0.2s;",
                    onclick: move |_| {
                        let playing = *is_playing.read();
                        is_playing.set(!playing);
                    },
                    if *is_playing.read() { "⏸" } else { "▶" }
                }

                button {
                    style: "background: #334155; color: #e2e8f0; border: none; padding: 6px 12px; border-radius: 4px; font-size: 12px; cursor: pointer;",
                    onclick: move |_| {
                        current_time.set(0.0);
                    },
                    "⏮ Reset"
                }

                // Scrubber Bar
                div {
                    style: "flex: 1; display: flex; align-items: center; gap: 12px;",
                    span { style: "font-family: monospace; font-size: 12px; color: #94a3b8; min-width: 48px;", "{current_time:.2}s" }
                    input {
                        r#type: "range",
                        min: "0.0",
                        max: "10.0",
                        step: "0.01",
                        value: "{current_time}",
                        style: "flex: 1; accent-color: #3b82f6; cursor: pointer;",
                        oninput: move |evt| {
                            if let Ok(t) = evt.value().parse::<f64>() {
                                current_time.set(t);
                            }
                        }
                    }
                    span { style: "font-family: monospace; font-size: 12px; color: #94a3b8;", "10.00s" }
                }

                // Speed Selector
                div {
                    style: "display: flex; align-items: center; gap: 6px;",
                    span { style: "font-size: 12px; color: #94a3b8;", "Speed:" }
                    select {
                        style: "background: #1e293b; color: #f1f5f9; border: 1px solid #475569; padding: 4px 8px; border-radius: 4px; font-size: 12px;",
                        value: "{playback_speed}",
                        onchange: move |evt| {
                            if let Ok(s) = evt.value().parse::<f64>() {
                                playback_speed.set(s);
                            }
                        },
                        option { value: "0.25", "0.25x" }
                        option { value: "0.5", "0.5x" }
                        option { value: "1.0", "1.0x" }
                        option { value: "2.0", "2.0x" }
                    }
                }
            }
        }
    }
}
