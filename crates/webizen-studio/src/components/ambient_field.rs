//! Canvas2D ambient knowledge field (BACKGROUND_VISUALISATION.md).
//! Cheap particle cymatics driven by `SystemTelemetry` uniforms.

use dioxus::prelude::*;
use serde::Deserialize;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub const AMBIENT_CANVAS_ID: &str = "ambient-knowledge-canvas";

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct AmbientTelemetry {
    pub memory_pressure: f32,
    pub network_ripple: f32,
    pub baking_crystallization: f32,
    pub logic_flashes: f32,
    pub llm_heat: f32,
    pub quantum_activity: f32,
    pub spectral_shift: f32,
    pub temporal_pulse: f32,
    pub epistemic_density: f32,
    pub manifold_pressure: f32,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn fetch_telemetry() -> Result<AmbientTelemetry, String> {
    let js_args = serde_wasm_bindgen::to_value(&json!({})).map_err(|e| e.to_string())?;
    let value = tauri_invoke("get_system_telemetry", js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
fn canvas_ctx(canvas_id: &str) -> Result<(HtmlCanvasElement, CanvasRenderingContext2d), String> {
    let document = web_sys::window()
        .ok_or("no window")?
        .document()
        .ok_or("no document")?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id(canvas_id)
        .ok_or("canvas not mounted")?
        .dyn_into()
        .map_err(|_| "canvas cast failed".to_string())?;
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(|_| "no 2d ctx")?
        .ok_or("2d ctx missing")?
        .dyn_into()
        .map_err(|_| "ctx cast failed".to_string())?;
    Ok((canvas, ctx))
}

#[cfg(target_arch = "wasm32")]
fn paint_ambient(
    ctx: &CanvasRenderingContext2d,
    w: f64,
    h: f64,
    time: f64,
    t: &AmbientTelemetry,
    particle_cap: usize,
) {
    let heat = t.llm_heat as f64;
    let ripple = t.network_ripple as f64;
    let logic = t.logic_flashes as f64;
    let bake = t.baking_crystallization as f64;
    let quantum = t.quantum_activity as f64;
    let spectral = t.spectral_shift as f64;
    let pressure = t.memory_pressure as f64;

    let (r0, g0, b0) = sigma_rgb(spectral);
    let grad = ctx.create_linear_gradient(0.0, 0.0, w, h);
    let _ = grad.add_color_stop(0.0, &format!("rgb({r0},{g0},{b0})"));
    let _ = grad.add_color_stop(1.0, "#060a10");
    ctx.set_fill_style(&wasm_bindgen::JsValue::from(grad));
    ctx.fill_rect(0.0, 0.0, w, h);

    let compress = 1.0 - pressure * 0.35;
    for i in 0..particle_cap {
        let fi = i as f64;
        let lattice = bake * 0.15 * (fi * 0.04).sin();
        let px = w * 0.5
            + w * 0.42
                * compress
                * (time * (0.32 + heat * 0.55) + fi * 0.011 + ripple * 2.2 + lattice).sin()
                * (fi * 0.003 + quantum * 0.12).cos();
        let py = h * 0.5
            + h * 0.42
                * compress
                * (time * (0.26 + logic * 0.45) + fi * 0.018).cos()
                * (fi * 0.005 + ripple).sin();
        let (r, g, b) = sigma_rgb(((fi * 0.017 + spectral as f64) % 1.0) as f32);
        let alpha = 0.06 + (fi * 0.001 + heat + bake * 0.5).sin().abs() * 0.38;
        ctx.set_fill_style(&wasm_bindgen::JsValue::from_str(&format!(
            "rgba({r},{g},{b},{alpha:.2})"
        )));
        ctx.begin_path();
        let _ = ctx.arc(
            px,
            py,
            0.7 + (fi % 3.0) + heat * 2.2 + bake,
            0.0,
            std::f64::consts::TAU,
        );
        ctx.fill();
    }
}

#[cfg(target_arch = "wasm32")]
fn sigma_rgb(sigma: f32) -> (u8, u8, u8) {
    let t = sigma.clamp(0.0, 1.0);
    let r = (32.0 + 180.0 * t) as u8;
    let g = (48.0 + 90.0 * (1.0 - t)) as u8;
    let b = (96.0 + 120.0 * (0.5 - (t - 0.5).abs())) as u8;
    (r, g, b)
}

#[component]
pub fn AmbientFieldCanvas(canvas_id: String, height_px: u32) -> Element {
    let mut telemetry = use_signal(AmbientTelemetry::default);
    let mut frame = use_signal(|| 0u64);

    #[cfg(target_arch = "wasm32")]
    {
        let canvas_id_poll = canvas_id.clone();
        use_effect(move || {
            spawn(async move {
                loop {
                    if let Ok(t) = fetch_telemetry().await {
                        telemetry.set(t);
                    }
                    frame.write().wrapping_add(1);
                    gloo_timers::future::TimeoutFuture::new(33).await;
                }
            });
        });

        use_effect(move || {
            let _tick = *frame.read();
            let t = telemetry.read().clone();
            let id = canvas_id_poll.clone();
            spawn(async move {
                let Ok((canvas, ctx)) = canvas_ctx(&id) else {
                    return;
                };
                let w = canvas.client_width().max(320) as f64;
                let h = canvas.client_height().max(height_px as i32) as f64;
                if canvas.width() != w as u32 {
                    canvas.set_width(w as u32);
                }
                if canvas.height() != h as u32 {
                    canvas.set_height(h as u32);
                }
                let time = js_sys::Date::now() / 1000.0;
                let cap = (800.0
                    + t.llm_heat as f64 * 4200.0
                    + t.baking_crystallization as f64 * 2000.0) as usize;
                paint_ambient(&ctx, w, h, time, &t, cap.min(6000));
            });
        });
    }

    rsx! {
        canvas {
            id: "{canvas_id}",
            width: "100%",
            height: "{height_px}px",
            style: "display:block;width:100%;border-radius:12px;border:1px solid var(--qualia-border, #2d3a4f);",
        }
    }
}
