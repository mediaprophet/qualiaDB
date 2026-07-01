use dioxus::prelude::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use std::cell::{Cell, RefCell};
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

// 3D math, camera, projection, and the Canvas 2D draw backend now live in the
// shared, backend-agnostic `render` module so a future GPU (WebGPU) backend can
// implement the same `Renderer` trait.
#[cfg(target_arch = "wasm32")]
use crate::render::{Camera, Canvas2dRenderer, Renderer, ScreenPoint, Vec3};

const CANVAS_ID: &str = "physics-engine-surface";
const SURFACE_WIDTH: u32 = 960;
const SURFACE_HEIGHT: u32 = 540;

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct ForgePhysicsCertification {
    engine_version: String,
    forge_schema_version: u32,
    kernel: String,
    backend: String,
    particle_count: usize,
    certified: bool,
    max_abs_error: f32,
    momentum_drift: f32,
    elapsed_ms: f64,
    q42_provenance: String,
    sample_positions: Vec<[f32; 3]>,
    note: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: wasm_bindgen::JsValue,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_forge_physics() -> Result<ForgePhysicsCertification, String> {
    let args =
        serde_wasm_bindgen::to_value(&serde_json::json!({})).map_err(|err| err.to_string())?;
    let value = tauri_invoke("certify_forge_physics", args)
        .await
        .map_err(|err| format!("{err:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|err| err.to_string())
}

#[cfg(target_arch = "wasm32")]
fn get_canvas_context() -> Result<(HtmlCanvasElement, CanvasRenderingContext2d), String> {
    let document = web_sys::window()
        .ok_or_else(|| "window unavailable".to_string())?
        .document()
        .ok_or_else(|| "document unavailable".to_string())?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id(CANVAS_ID)
        .ok_or_else(|| "physics canvas not mounted".to_string())?
        .dyn_into()
        .map_err(|_| "failed to cast canvas element".to_string())?;
    let context: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(|_| "failed to fetch 2d context".to_string())?
        .ok_or_else(|| "2d context unavailable".to_string())?
        .dyn_into()
        .map_err(|_| "failed to cast 2d context".to_string())?;
    Ok((canvas, context))
}

#[cfg(target_arch = "wasm32")]
fn sample_height(x: f64, z: f64, phase: f64, amplitude: f64, pulse: f64) -> f64 {
    let wave_a = (x * 0.68 + phase * (0.6 + pulse * 0.4)).sin();
    let wave_b = (z * 0.55 - phase * 0.42).cos();
    let wave_c = ((x + z) * 0.21 + phase * 0.18).sin();
    amplitude * (wave_a * 0.55 + wave_b * 0.3 + wave_c * 0.15)
}

#[cfg(target_arch = "wasm32")]
fn draw_scene(
    canvas: &HtmlCanvasElement,
    context: &CanvasRenderingContext2d,
    phase: f64,
    orbit_speed: f64,
    amplitude: f64,
    grid_density: u32,
    wireframe: bool,
    pulse: f64,
) {
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    let viewport = (width, height);

    // The Canvas 2D backend of the shared `Renderer` trait. Swapping in a GPU
    // backend later means changing only this constructor.
    let mut renderer = Canvas2dRenderer::new(context.clone(), viewport);
    renderer.clear("#040712");

    // Backend-specific atmosphere wash (kept on the raw context).
    context.set_fill_style(&JsValue::from_str("rgba(24, 76, 123, 0.18)"));
    context.fill_rect(0.0, height * 0.48, width, height * 0.52);
    context.set_fill_style(&JsValue::from_str("rgba(255, 214, 102, 0.035)"));
    context.fill_rect(0.0, height * 0.18, width, height * 0.32);

    let orbit_angle = phase * orbit_speed * 0.55 + 0.25;
    renderer.set_camera(Camera {
        eye: Vec3::new(
            orbit_angle.cos() * 19.0,
            9.5 + pulse * 2.4,
            orbit_angle.sin() * 19.0,
        ),
        target: Vec3::new(0.0, 0.5, 0.0),
        world_up: Vec3::new(0.0, 1.0, 0.0),
        focal_scale: 0.88,
    });
    let span = 14.0;
    let steps = grid_density.max(8) as usize;
    let step = span * 2.0 / steps as f64;

    let mut grid = vec![ScreenPoint::default(); (steps + 1) * (steps + 1)];
    let mut visible = vec![false; (steps + 1) * (steps + 1)];
    let index = |x: usize, z: usize| z * (steps + 1) + x;

    for z in 0..=steps {
        let world_z = -span + z as f64 * step;
        for x in 0..=steps {
            let world_x = -span + x as f64 * step;
            let crest = sample_height(world_x, world_z, phase, amplitude, pulse);
            let world = Vec3::new(world_x, crest, world_z);
            let idx = index(x, z);
            if let Some(projected) = renderer.project(world) {
                grid[idx] = projected;
                visible[idx] = true;
            }
        }
    }

    if !wireframe {
        for z in 0..steps {
            for x in 0..steps {
                let a_idx = index(x, z);
                let b_idx = index(x + 1, z);
                let c_idx = index(x + 1, z + 1);
                let d_idx = index(x, z + 1);
                if !(visible[a_idx] && visible[b_idx] && visible[c_idx] && visible[d_idx]) {
                    continue;
                }

                let a = grid[a_idx];
                let b = grid[b_idx];
                let c = grid[c_idx];
                let d = grid[d_idx];
                let avg_depth = (a.depth + b.depth + c.depth + d.depth) * 0.25;
                let alpha = (0.24 - avg_depth * 0.008).clamp(0.03, 0.16);
                renderer.fill_polygon(&[a, b, c, d], "rgba(66, 153, 225, 0.16)", alpha);
            }
        }
    }

    for z in 0..=steps {
        for x in 0..steps {
            let a_idx = index(x, z);
            let b_idx = index(x + 1, z);
            if visible[a_idx] && visible[b_idx] {
                let a = grid[a_idx];
                let b = grid[b_idx];
                let alpha = (0.85 - ((a.depth + b.depth) * 0.5) * 0.03).clamp(0.08, 0.6);
                renderer.line(a, b, "#67e8f9", alpha, 1.1);
            }
        }
    }

    for x in 0..=steps {
        for z in 0..steps {
            let a_idx = index(x, z);
            let b_idx = index(x, z + 1);
            if visible[a_idx] && visible[b_idx] {
                let a = grid[a_idx];
                let b = grid[b_idx];
                let alpha = (0.72 - ((a.depth + b.depth) * 0.5) * 0.026).clamp(0.06, 0.42);
                renderer.line(a, b, "#60a5fa", alpha, 0.9);
            }
        }
    }

    for z in (0..=steps).step_by(3) {
        for x in (0..=steps).step_by(3) {
            let world_x = -span + x as f64 * step;
            let world_z = -span + z as f64 * step;
            let base_y = sample_height(world_x, world_z, phase, amplitude, pulse);
            let height_scale = 1.6 + ((world_x * 0.33 + phase).cos() + 1.0) * 1.1;
            let base = renderer.project(Vec3::new(world_x, base_y, world_z));
            let tip = renderer.project(Vec3::new(world_x, base_y + height_scale, world_z));
            if let (Some(base), Some(tip)) = (base, tip) {
                let alpha = (0.95 - tip.depth * 0.035).clamp(0.09, 0.65);
                renderer.line(base, tip, "#f59e0b", alpha, 1.4);
                renderer.point(
                    tip,
                    (4.8 - tip.depth * 0.08).clamp(1.3, 3.4),
                    "#fde68a",
                    alpha,
                );
            }
        }
    }

    for orbit in 0..10 {
        let angle = phase * (0.45 + pulse * 0.15) + orbit as f64 * 0.628;
        let world = Vec3::new(
            angle.cos() * (7.0 + orbit as f64 * 0.15),
            3.5 + (angle * 1.7).sin() * 1.2,
            angle.sin() * (7.0 + orbit as f64 * 0.15),
        );
        if let Some(point) = renderer.project(world) {
            let alpha = (0.92 - point.depth * 0.03).clamp(0.16, 0.85);
            let radius = (5.2 - point.depth * 0.06).clamp(1.6, 3.8);
            renderer.point(point, radius, "#f472b6", alpha);
        }
    }

    context.set_global_alpha(0.9);
    context.set_fill_style(&JsValue::from_str("rgba(226, 232, 240, 0.9)"));
    context.set_font("600 14px 'Segoe UI', sans-serif");
    let _ = context.fill_text("Webizen Spatial Physics Surface", 18.0, 28.0);
    context.set_global_alpha(0.55);
    context.set_font("12px 'Segoe UI', sans-serif");
    let _ = context.fill_text(
        "Procedural 3D mesh | orbit camera | direct canvas raster",
        18.0,
        48.0,
    );
    context.set_global_alpha(1.0);
}

#[component]
pub fn PhysicsSimulator() -> Element {
    let mut orbit_speed = use_signal(|| 1.15_f64);
    let mut wave_amplitude = use_signal(|| 2.8_f64);
    let mut pulse_strength = use_signal(|| 0.65_f64);
    let mut grid_density = use_signal(|| 22_u32);
    let mut wireframe = use_signal(|| true);
    let mut paused = use_signal(|| false);
    let status = use_signal(|| "Booting spatial viewport...".to_string());
    let mut forge_status =
        use_signal(|| "Run the bounded native probe to certify the Forge kernel.".to_string());
    let forge_running = use_signal(|| false);
    let forge_result = use_signal(|| None::<ForgePhysicsCertification>);
    #[cfg(target_arch = "wasm32")]
    let animation_started = use_signal(|| false);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let mut animation_started = animation_started;
            if animation_started() {
                return;
            }
            animation_started.set(true);

            let mut status = status;
            let orbit_speed_signal = orbit_speed;
            let amplitude_signal = wave_amplitude;
            let pulse_signal = pulse_strength;
            let density_signal = grid_density;
            let wireframe_signal = wireframe;
            let paused_signal = paused;

            match get_canvas_context() {
                Ok((canvas, context)) => {
                    canvas.set_width(SURFACE_WIDTH);
                    canvas.set_height(SURFACE_HEIGHT);
                    status.set("Spatial engine online".to_string());

                    let phase = Rc::new(Cell::new(0.0_f64));
                    let last_timestamp = Rc::new(Cell::new(None::<f64>));
                    let raf_loop = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
                    let raf_loop_handle = raf_loop.clone();

                    *raf_loop.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp: f64| {
                        let dt = last_timestamp
                            .get()
                            .map(|previous| ((timestamp - previous) / 1000.0).clamp(0.0, 0.05))
                            .unwrap_or(0.016);
                        last_timestamp.set(Some(timestamp));

                        if !paused_signal() {
                            phase.set(phase.get() + dt);
                        }

                        draw_scene(
                            &canvas,
                            &context,
                            phase.get(),
                            orbit_speed_signal(),
                            amplitude_signal(),
                            density_signal(),
                            wireframe_signal(),
                            pulse_signal(),
                        );

                        if let Some(window) = web_sys::window() {
                            if let Some(callback) = raf_loop_handle.borrow().as_ref() {
                                let _ = window
                                    .request_animation_frame(callback.as_ref().unchecked_ref());
                            }
                        }
                    })
                        as Box<dyn FnMut(f64)>));

                    if let Some(window) = web_sys::window() {
                        if let Some(callback) = raf_loop.borrow().as_ref() {
                            let _ =
                                window.request_animation_frame(callback.as_ref().unchecked_ref());
                        }
                    }
                }
                Err(err) => status.set(format!("Viewport bootstrap failed: {err}")),
            }
        }
    });

    let orbit = orbit_speed();
    let amplitude = wave_amplitude();
    let pulse = pulse_strength();
    let density = grid_density();
    let is_wireframe = wireframe();
    let is_paused = paused();
    let estimated_vertices = (density + 1) * (density + 1);
    let energy_score = (amplitude * pulse * orbit * 42.0).round() as i64;
    let style_chip = if is_wireframe {
        "Wireframe mesh"
    } else {
        "Filled terrain"
    };
    let status_text = status();
    let forge_status_text = forge_status();
    let forge_result_value = forge_result();

    rsx! {
        div {
            style: "height: 100%; display: grid; grid-template-columns: minmax(0, 1.7fr) minmax(260px, 0.9fr); gap: 1rem; color: #e2e8f0; font-family: 'Segoe UI', sans-serif;",

            div {
                style: "display: flex; flex-direction: column; gap: 0.9rem; min-height: 0;",

                div {
                    style: "padding: 1rem 1.1rem; border-radius: 18px; border: 1px solid rgba(103,232,249,0.18); background: radial-gradient(circle at top left, rgba(56,189,248,0.18), transparent 42%), linear-gradient(180deg, rgba(7,14,25,0.95), rgba(2,6,17,0.98)); box-shadow: 0 18px 50px rgba(2,6,23,0.35);",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; margin-bottom: 0.8rem;",
                        div {
                            h2 { style: "margin: 0 0 0.28rem 0; font-size: 1.05rem; font-weight: 700; color: #f8fafc;", "Physics Simulator" }
                            p { style: "margin: 0; font-size: 0.82rem; line-height: 1.5; color: rgba(226,232,240,0.72);", "A procedural manifold viewport paired with an explicit QualiaDB Forge kinematics certification probe. Preview pixels never cross the Dioxus VDOM." }
                        }
                        div {
                            style: "display: flex; flex-direction: column; align-items: flex-end; gap: 0.35rem; min-width: 140px;",
                            span { style: "padding: 0.28rem 0.55rem; border-radius: 999px; background: rgba(15,23,42,0.72); border: 1px solid rgba(148,163,184,0.18); font-size: 0.7rem; color: #67e8f9; text-transform: uppercase; letter-spacing: 0.06em;", "{style_chip}" }
                            span { style: "font-size: 0.76rem; color: rgba(226,232,240,0.7);", "{status_text}" }
                        }
                    }

                    div {
                        style: "position: relative; border-radius: 16px; overflow: hidden; border: 1px solid rgba(148,163,184,0.14); background: #020617;",
                        canvas {
                            id: "{CANVAS_ID}",
                            width: "{SURFACE_WIDTH}",
                            height: "{SURFACE_HEIGHT}",
                            style: "display: block; width: 100%; aspect-ratio: 16 / 9; background: #020617; image-rendering: auto;",
                        }
                    }
                }

                div {
                    style: "display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.75rem;",
                    {summary_card("Vertex Lattice", format!("{estimated_vertices} active samples"), "#67e8f9")}
                    {summary_card("Energy Field", format!("{energy_score} flux"), "#f59e0b")}
                    {summary_card("Camera Orbit", format!("{orbit:.2} rad/s"), "#f472b6")}
                }
            }

            div {
                style: "display: flex; flex-direction: column; gap: 0.8rem; min-height: 0;",

                div {
                    style: "padding: 0.95rem 1rem; border-radius: 16px; border: 1px solid rgba(148,163,184,0.16); background: linear-gradient(180deg, rgba(15,23,42,0.96), rgba(2,6,23,0.98));",
                    h3 { style: "margin: 0 0 0.85rem 0; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.08em; color: rgba(148,163,184,0.9);", "Engine Controls" }

                    {control_slider("Orbit speed", orbit, "0.30", "2.50", "0.05", move |value| orbit_speed.set(value))}
                    {control_slider("Wave amplitude", amplitude, "0.60", "5.50", "0.10", move |value| wave_amplitude.set(value))}
                    {control_slider("Pulse strength", pulse, "0.10", "1.40", "0.05", move |value| pulse_strength.set(value))}

                    div {
                        style: "margin-top: 0.9rem;",
                        label { style: "display: flex; justify-content: space-between; font-size: 0.78rem; color: #cbd5e1; margin-bottom: 0.35rem;", span { "Grid density" } span { "{density}" } }
                        input {
                            type: "range",
                            min: "10",
                            max: "32",
                            step: "1",
                            value: "{density}",
                            style: "width: 100%; accent-color: #38bdf8;",
                            oninput: move |event| {
                                if let Ok(value) = event.value().parse::<u32>() {
                                    grid_density.set(value);
                                }
                            }
                        }
                    }

                    div {
                        style: "display: flex; gap: 0.55rem; margin-top: 1rem; flex-wrap: wrap;",
                        button {
                            style: "flex: 1 1 120px; padding: 0.7rem 0.85rem; border-radius: 12px; border: 1px solid rgba(103,232,249,0.25); background: rgba(8,47,73,0.65); color: #e0f2fe; font-weight: 600; cursor: pointer;",
                            onclick: move |_| paused.set(!paused()),
                            if is_paused { "Resume engine" } else { "Pause engine" }
                        }
                        button {
                            style: "flex: 1 1 120px; padding: 0.7rem 0.85rem; border-radius: 12px; border: 1px solid rgba(244,114,182,0.25); background: rgba(80,7,36,0.55); color: #fce7f3; font-weight: 600; cursor: pointer;",
                            onclick: move |_| wireframe.set(!wireframe()),
                            if is_wireframe { "Switch to terrain" } else { "Switch to mesh" }
                        }
                    }
                }

                div {
                    style: "padding: 0.95rem 1rem; border-radius: 16px; border: 1px solid rgba(148,163,184,0.16); background: rgba(15,23,42,0.94);",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; gap: 0.75rem; margin-bottom: 0.7rem;",
                        h3 { style: "margin: 0; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.08em; color: rgba(148,163,184,0.9);", "Forge Certification" }
                        button {
                            disabled: forge_running(),
                            style: "padding: 0.5rem 0.7rem; border-radius: 10px; border: 1px solid rgba(103,232,249,0.28); background: rgba(8,47,73,0.72); color: #e0f2fe; font-size: 0.72rem; font-weight: 700; cursor: pointer;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    forge_running.set(true);
                                    forge_status.set("Dispatching the bounded Forge WGPU probe...".to_string());
                                    let mut forge_result = forge_result;
                                    spawn_local(async move {
                                        match invoke_forge_physics().await {
                                            Ok(result) => {
                                                forge_status.set(result.note.clone());
                                                forge_result.set(Some(result));
                                            }
                                            Err(err) => {
                                                forge_status.set(format!("Native Forge probe unavailable: {err}"));
                                            }
                                        }
                                        forge_running.set(false);
                                    });
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    forge_status.set("Forge certification is invoked by the desktop webview.".to_string());
                                }
                            },
                            if forge_running() { "Running..." } else { "Certify step" }
                        }
                    }
                    p {
                        style: "margin: 0 0 0.7rem 0; color: rgba(226,232,240,0.76); font-size: 0.76rem; line-height: 1.5;",
                        "{forge_status_text}"
                    }
                    if let Some(result) = forge_result_value.as_ref() {
                        div {
                            style: "display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.45rem; margin-bottom: 0.65rem;",
                            {probe_metric("Result", if result.certified { "Certified".to_string() } else { "Oracle fallback".to_string() })}
                            {probe_metric("Backend", result.backend.clone())}
                            {probe_metric("Max |error|", format!("{:.3e}", result.max_abs_error))}
                            {probe_metric("Momentum drift", format!("{:.3e}", result.momentum_drift))}
                            {probe_metric("Particles", result.particle_count.to_string())}
                            {probe_metric("Elapsed", format!("{:.2} ms", result.elapsed_ms))}
                        }
                        div {
                            style: "padding: 0.55rem 0.65rem; border-radius: 10px; background: rgba(2,6,23,0.65); border: 1px solid rgba(148,163,184,0.12);",
                            div { style: "font-size: 0.68rem; color: #67e8f9; margin-bottom: 0.25rem;", "{result.kernel} · QualiaDB {result.engine_version} · schema {result.forge_schema_version}" }
                            code { style: "display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.65rem; color: rgba(226,232,240,0.62);", "{result.q42_provenance}" }
                        }
                    }
                }

                div {
                    style: "padding: 0.95rem 1rem; border-radius: 16px; border: 1px solid rgba(148,163,184,0.16); background: rgba(15,23,42,0.94);",
                    h3 { style: "margin: 0 0 0.7rem 0; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.08em; color: rgba(148,163,184,0.9);", "Execution Notes" }
                    ul {
                        style: "margin: 0; padding-left: 1.1rem; display: grid; gap: 0.45rem; color: rgba(226,232,240,0.78); font-size: 0.78rem; line-height: 1.5;",
                        li { "The viewport is a procedural examination surface; it is not presented as the N-body result." }
                        li { "The native probe runs the 0.0.23 softened inverse-square kernel and compares WGPU output with its scalar CPU oracle." }
                        li { "The bounded result is content-addressed as a Q42 provenance root; allocations stay at the explicit IPC/certification boundary." }
                    }
                }
            }
        }
    }
}

fn probe_metric(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            style: "padding: 0.5rem 0.58rem; border-radius: 9px; background: rgba(2,6,23,0.58); border: 1px solid rgba(148,163,184,0.1);",
            div { style: "font-size: 0.62rem; color: rgba(148,163,184,0.82); text-transform: uppercase; letter-spacing: 0.05em;", "{label}" }
            div { style: "font-size: 0.75rem; color: #e2e8f0; margin-top: 0.18rem;", "{value}" }
        }
    }
}

fn summary_card(title: &'static str, value: String, accent: &'static str) -> Element {
    rsx! {
        div {
            style: "padding: 0.9rem 1rem; border-radius: 16px; border: 1px solid rgba(148,163,184,0.14); background: linear-gradient(180deg, rgba(15,23,42,0.94), rgba(2,6,23,0.98));",
            div { style: "font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.08em; color: rgba(148,163,184,0.82); margin-bottom: 0.3rem;", "{title}" }
            div { style: "font-size: 1rem; font-weight: 700; color: {accent};", "{value}" }
        }
    }
}

fn control_slider(
    label: &'static str,
    value: f64,
    min: &'static str,
    max: &'static str,
    step: &'static str,
    mut on_change: impl FnMut(f64) + 'static,
) -> Element {
    rsx! {
        div {
            style: "margin-bottom: 0.9rem;",
            label {
                style: "display: flex; justify-content: space-between; font-size: 0.78rem; color: #cbd5e1; margin-bottom: 0.35rem;",
                span { "{label}" }
                span { "{value:.2}" }
            }
            input {
                type: "range",
                min: "{min}",
                max: "{max}",
                step: "{step}",
                value: "{value}",
                style: "width: 100%; accent-color: #38bdf8;",
                oninput: move |event| {
                    if let Ok(parsed) = event.value().parse::<f64>() {
                        on_change(parsed);
                    }
                }
            }
        }
    }
}
