//! Browser-visible graphics capability probe for the WASM portal.
//!
//! This reports runtime availability separately from compile-time engine
//! capabilities. Probes use detached canvases and a bounded WebGPU adapter
//! request; they do not claim the caller's presentation canvas or request a
//! device.

use js_sys::{Array, Function, Object, Promise, Reflect};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

use crate::render::acceptance_contract::{
    BrowserWebGpuEvidence, CapabilityAcceptance, CapabilityEvidence,
};
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
    let webgpu_api_state = browser_webgpu_api_state();
    let webgpu_api = webgpu_api_state.is_confirmed();
    let webgpu_adapter_state = match webgpu_api_state {
        CapabilityEvidence::Confirmed => probe_webgpu_adapter_bounded().await,
        CapabilityEvidence::Refused => CapabilityEvidence::Refused,
        CapabilityEvidence::Unknown => CapabilityEvidence::Unknown,
    };
    let webgpu_adapter = webgpu_adapter_state.is_confirmed();
    let webgl2 = probe_webgl2()?;
    let canvas2d = probe_canvas2d()?;

    let recommended = recommend_backend(webgpu_adapter, webgl2, canvas2d);
    let acceptance = CapabilityAcceptance {
        browser_webgpu: BrowserWebGpuEvidence {
            api: webgpu_api_state,
            adapter: webgpu_adapter_state,
            // This detached probe intentionally never requests a device. Initialization is a
            // separate host-owned receipt and therefore remains unknown here.
            device: CapabilityEvidence::Unknown,
        },
        ..CapabilityAcceptance::default()
    };

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
        "webgpu_api_state",
        &JsValue::from_str(webgpu_api_state.as_str()),
    )?;
    set(
        &report,
        "webgpu_adapter_state",
        &JsValue::from_str(webgpu_adapter_state.as_str()),
    )?;
    set(
        &report,
        "browser_webgpu_adapter_gate",
        &JsValue::from_str(acceptance.browser_webgpu_adapter_gate().as_str()),
    )?;
    set(
        &report,
        "browser_webgpu_gate",
        &JsValue::from_str(acceptance.browser_webgpu_gate().as_str()),
    )?;
    set(
        &report,
        "temporal_producer_gate",
        &JsValue::from_str(acceptance.temporal_gate().as_str()),
    )?;
    set(
        &report,
        "environment_probe_gate",
        &JsValue::from_str(acceptance.environment_probe_gate().as_str()),
    )?;
    set(
        &report,
        "pixel_readback_gate",
        &JsValue::from_str(acceptance.pixel_readback_gate().as_str()),
    )?;
    set(
        &report,
        "performance_evidence_gate",
        &JsValue::from_str(acceptance.performance_gate().as_str()),
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

fn browser_webgpu_api_state() -> CapabilityEvidence {
    let Some(window) = web_sys::window() else {
        return CapabilityEvidence::Unknown;
    };
    match Reflect::get(&window.navigator(), &JsValue::from_str("gpu")) {
        Ok(gpu) if !gpu.is_null() && !gpu.is_undefined() => CapabilityEvidence::Confirmed,
        Ok(_) => CapabilityEvidence::Refused,
        Err(_) => CapabilityEvidence::Unknown,
    }
}

async fn probe_webgpu_adapter_bounded() -> CapabilityEvidence {
    let adapter_probe = wasm_bindgen_futures::future_to_promise(async {
        Ok(JsValue::from_f64(if PortalGpu::adapter_responds().await {
            1.0
        } else {
            -1.0
        }))
    });
    let timeout = Promise::new(&mut |resolve, _reject| {
        let fallback_resolve = resolve.clone();
        let timer_resolve = resolve.clone();
        let callback = Closure::once(move || {
            let _ = timer_resolve.call1(&JsValue::NULL, &JsValue::from_f64(0.0));
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
    match wasm_bindgen_futures::JsFuture::from(Promise::race(&racers))
        .await
        .ok()
        .and_then(|value| value.as_f64())
    {
        Some(value) if value > 0.0 => CapabilityEvidence::Confirmed,
        Some(value) if value < 0.0 => CapabilityEvidence::Refused,
        _ => CapabilityEvidence::Unknown,
    }
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

#[allow(dead_code)]
pub fn recommend_feature_admission(tier: &str, feature: &str) -> bool {
    use vibe::{eval_function, load_program, Value};

    let Ok(program) = load_program(include_str!("graphics_backend.vibe")) else {
        return false;
    };
    let mut host = vibe::LocalHost::default();
    let mut env = vibe::Env::default();
    match eval_function(
        &program,
        "admit_feature",
        vec![
            Value::String(tier.to_owned()),
            Value::String(feature.to_owned()),
        ],
        &mut host,
        &mut env,
    ) {
        Ok(Value::Bool(admitted)) => admitted,
        _ => false,
    }
}

#[allow(dead_code)]
pub fn recommend_vegetation_density(tier: &str) -> i64 {
    use vibe::{eval_function, load_program, Value};

    let Ok(program) = load_program(include_str!("graphics_backend.vibe")) else {
        return 0;
    };
    let mut host = vibe::LocalHost::default();
    let mut env = vibe::Env::default();
    match eval_function(
        &program,
        "select_vegetation_density",
        vec![Value::String(tier.to_owned())],
        &mut host,
        &mut env,
    ) {
        Ok(Value::I64(d)) => d,
        Ok(Value::U64(d)) => d as i64,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        recommend_backend, recommend_feature_admission, recommend_recovery_backend,
        recommend_vegetation_density,
    };

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

    #[test]
    fn test_vibe_feature_admission_and_quality() {
        assert!(recommend_feature_admission("ultra", "temporal_aa"));
        assert!(recommend_feature_admission("ultra", "water_foam"));
        assert!(recommend_feature_admission("balanced", "temporal_aa"));
        assert!(!recommend_feature_admission("low", "temporal_aa"));
        assert!(recommend_feature_admission("low", "water_foam"));
        assert_eq!(recommend_vegetation_density("ultra"), 128);
        assert_eq!(recommend_vegetation_density("balanced"), 32);
        assert_eq!(recommend_vegetation_density("conservative"), 0);
    }
}
