//! Browser-visible graphics capability probe for the WASM portal.
//!
//! This reports runtime availability separately from compile-time engine
//! capabilities. Probes use detached canvases and a bounded WebGPU adapter
//! request; they do not claim the caller's presentation canvas or request a
//! device.

use js_sys::{Array, Function, Object, Promise, Reflect};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

use crate::render::gpu::PortalGpu;

const ADAPTER_PROBE_TIMEOUT_MS: i32 = 4_000;

/// Probe browser graphics APIs without claiming the page's presentation canvas.
///
/// `selected_backend` is a recommendation from API/adapter availability only;
/// actual initialization can still fail because of surface compatibility,
/// memory pressure, or device loss. Callers must use the result of
/// `portal_init_webgpu` / `portal_init_webgl2` as the final selection receipt.
#[wasm_bindgen]
pub async fn probe_portal_graphics() -> Result<JsValue, JsValue> {
    let webgpu_api = browser_has_webgpu();
    let webgpu_adapter = webgpu_api && probe_webgpu_adapter_bounded().await;
    let webgl2 = probe_webgl2()?;
    let canvas2d = probe_canvas2d()?;

    let recommended = recommend_backend(webgpu_adapter, webgl2, canvas2d);

    let report = Object::new();
    set(&report, "target", &JsValue::from_str("wasm32"))?;
    set(
        &report,
        "compiled_profile",
        &JsValue::from_str(crate::wasm_capabilities::compiled_profile()),
    )?;
    set(
        &report,
        "compiled_capabilities",
        &serde_wasm_bindgen::to_value(crate::wasm_capabilities::compiled_capabilities())?,
    )?;
    set(
        &report,
        "native_only_capabilities",
        &serde_wasm_bindgen::to_value(crate::wasm_capabilities::native_only_capabilities())?,
    )?;
    set(
        &report,
        "webgpu_api_exposed",
        &JsValue::from_bool(webgpu_api),
    )?;
    set(
        &report,
        "webgpu_adapter_responded",
        &JsValue::from_bool(webgpu_adapter),
    )?;
    set(
        &report,
        "webgl2_context_created",
        &JsValue::from_bool(webgl2),
    )?;
    set(
        &report,
        "canvas2d_context_created",
        &JsValue::from_bool(canvas2d),
    )?;
    set(
        &report,
        "recommended_backend",
        &JsValue::from_str(&recommended),
    )?;
    set(
        &report,
        "adapter_probe_timeout_ms",
        &JsValue::from_f64(ADAPTER_PROBE_TIMEOUT_MS as f64),
    )?;

    let order = Array::new();
    for backend in ["webgpu", "webgl2", "canvas2d"] {
        order.push(&JsValue::from_str(backend));
    }
    set(&report, "fallback_order", &order.into())?;
    Ok(report.into())
}

/// Select the next recovery backend using the shared VibeScript policy.
#[wasm_bindgen]
pub fn recommend_graphics_recovery(webgpu_retry_allowed: bool, webgl2_allowed: bool) -> String {
    recommend_recovery_backend(webgpu_retry_allowed, webgl2_allowed)
}

fn set(target: &Object, key: &str, value: &JsValue) -> Result<(), JsValue> {
    Reflect::set(target, &JsValue::from_str(key), value).map(|_| ())
}

fn recommend_backend(webgpu_adapter: bool, webgl2: bool, canvas2d: bool) -> String {
    use vibe::{eval_function, load_program, Value};

    let Ok(program) = load_program(include_str!("graphics_backend.vibe")) else {
        return "unavailable".to_owned();
    };
    let mut host = vibe::LocalHost::default();
    let mut env = vibe::Env::default();
    match eval_function(
        &program,
        "select_backend",
        vec![
            Value::Bool(webgpu_adapter),
            Value::Bool(webgl2),
            Value::Bool(canvas2d),
        ],
        &mut host,
        &mut env,
    ) {
        Ok(Value::String(backend)) => backend,
        _ => "unavailable".to_owned(),
    }
}

fn recommend_recovery_backend(webgpu_retry_allowed: bool, webgl2_allowed: bool) -> String {
    use vibe::{eval_function, load_program, Value};

    let Ok(program) = load_program(include_str!("graphics_backend.vibe")) else {
        return "canvas2d".to_owned();
    };
    let mut host = vibe::LocalHost::default();
    let mut env = vibe::Env::default();
    match eval_function(
        &program,
        "select_recovery_backend",
        vec![
            Value::Bool(webgpu_retry_allowed),
            Value::Bool(webgl2_allowed),
        ],
        &mut host,
        &mut env,
    ) {
        Ok(Value::String(backend)) => backend,
        _ => "canvas2d".to_owned(),
    }
}

fn browser_has_webgpu() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    Reflect::get(&window.navigator(), &JsValue::from_str("gpu"))
        .map(|gpu| !gpu.is_null() && !gpu.is_undefined())
        .unwrap_or(false)
}

async fn probe_webgpu_adapter_bounded() -> bool {
    let adapter_probe = wasm_bindgen_futures::future_to_promise(async {
        Ok(JsValue::from_bool(PortalGpu::adapter_responds().await))
    });
    let timeout = Promise::new(&mut |resolve, _reject| {
        let fallback_resolve = resolve.clone();
        let timer_resolve = resolve.clone();
        let callback = Closure::once(move || {
            let _ = timer_resolve.call1(&JsValue::NULL, &JsValue::FALSE);
        });
        let timer = web_sys::window().and_then(|window| {
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.as_ref().unchecked_ref(),
                    ADAPTER_PROBE_TIMEOUT_MS,
                )
                .ok()
        });
        if timer.is_some() {
            // The callback owns its captured resolver until the timeout fires.
            callback.forget();
        } else {
            let _ = fallback_resolve.call1(&JsValue::NULL, &JsValue::FALSE);
        }
    });
    let racers = Array::of2(&adapter_probe, &timeout);
    wasm_bindgen_futures::JsFuture::from(Promise::race(&racers))
        .await
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn detached_canvas() -> Result<HtmlCanvasElement, JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("graphics_probe_document_unavailable"))?;
    document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("graphics_probe_canvas_unavailable"))
}

fn probe_webgl2() -> Result<bool, JsValue> {
    let canvas = detached_canvas()?;
    let Some(context) = canvas.get_context("webgl2")? else {
        return Ok(false);
    };
    let context = context.dyn_into::<WebGl2RenderingContext>()?;
    if let Some(extension) = context.get_extension("WEBGL_lose_context")? {
        if let Ok(lose_context) = Reflect::get(&extension, &JsValue::from_str("loseContext"))
            .and_then(JsValue::dyn_into::<Function>)
        {
            let _ = lose_context.call0(&extension);
        }
    }
    Ok(true)
}

fn probe_canvas2d() -> Result<bool, JsValue> {
    let canvas = detached_canvas()?;
    Ok(canvas.get_context("2d")?.is_some())
}

#[cfg(test)]
mod tests {
    use super::{recommend_backend, recommend_recovery_backend};

    #[test]
    fn recommendation_uses_quality_order_and_degrades_to_canvas() {
        assert_eq!(recommend_backend(true, true, true), "webgpu");
        assert_eq!(recommend_backend(false, true, true), "webgl2");
        assert_eq!(recommend_backend(false, false, true), "canvas2d");
        assert_eq!(recommend_backend(false, false, false), "unavailable");
    }

    #[test]
    fn recovery_policy_retries_webgpu_then_falls_back() {
        assert_eq!(recommend_recovery_backend(true, true), "webgpu");
        assert_eq!(recommend_recovery_backend(false, true), "webgl2");
        assert_eq!(recommend_recovery_backend(false, false), "canvas2d");
    }
}
