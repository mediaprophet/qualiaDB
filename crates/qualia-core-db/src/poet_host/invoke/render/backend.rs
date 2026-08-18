//! `Render.gpu_backend_info` invoke handler — runtime backend detection and
//! fallback selection (plan §7.3 W3 + W8).
//!
//! Probes WebGPU adapter availability and returns the selected backend type
//! plus capability information. On native, WebGPU is the only backend. On
//! WASM, this probes `navigator.gpu` and falls back to WebGL2 when WebGPU is
//! unavailable.
//!
//! ## Invoke surface
//!
//! | ID | Arguments | Returns |
//! |----|-----------|---------|
//! | `Render.gpu_backend_info` | `{}` | `{ backend, available, webgl2_fallback, device_name?, limits? }` |

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};

/// `Render.gpu_backend_info` — probe the best available GPU backend.
///
/// On native: returns `backend: "webgpu"` if an adapter is available, else
/// `backend: "none"`. WebGL2 is not a native fallback (it's browser-only).
///
/// On WASM: returns `backend: "webgpu"` if `navigator.gpu` is present, else
/// `backend: "webgl2"` if WebGL2 context is available, else `backend: "none"`.
/// The `webgl2_fallback` field indicates whether WebGL2 is available as a
/// fallback even when WebGPU is the selected backend.
pub fn gpu_backend_info(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let ctx = crate::gpu_context::try_shared_gpu();
        let Some(ctx) = ctx else {
            return Ok(args::record([
                ("backend", Value::String("none".into())),
                ("available", Value::Bool(false)),
                ("webgl2_fallback", Value::Bool(false)),
                ("device_name", Value::String("no GPU adapter".into())),
            ]));
        };
        let caps = &ctx.adapter_caps;
        Ok(args::record([
            ("backend", Value::String("webgpu".into())),
            ("available", Value::Bool(true)),
            // WebGL2 is a browser-only fallback; not available on native.
            ("webgl2_fallback", Value::Bool(false)),
            ("device_name", Value::String(caps.name.clone())),
            ("backend_label", Value::String(caps.backend_label().into())),
            ("device_type", Value::String(caps.device_type_label().into())),
            ("shader_f16", Value::Bool(caps.features.shader_f16)),
            ("subgroup", Value::Bool(caps.features.subgroup)),
            ("cooperative_matrix", Value::Bool(caps.features.cooperative_matrix)),
            ("max_compute_workgroup_size_x", Value::U64(caps.limits.max_compute_workgroup_size_x as u64)),
            ("max_storage_buffer_binding_size", Value::U64(caps.limits.max_storage_buffer_binding_size)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        // On WASM, probe WebGPU availability via the global `navigator.gpu`.
        // If unavailable, report WebGL2 as the fallback backend. The actual
        // WebGL2 context creation requires a canvas element (handled by the
        // portal facade), so here we report availability only.
        let webgpu_available = webgpu_available_wasm();
        let webgl2_available = if webgpu_available {
            // WebGPU is available; WebGL2 may still be present as a fallback.
            webgl2_context_available_wasm()
        } else {
            // WebGPU is not available; check if WebGL2 is the fallback.
            webgl2_context_available_wasm()
        };
        let backend = if webgpu_available {
            "webgpu"
        } else if webgl2_available {
            "webgl2"
        } else {
            "none"
        };
        Ok(args::record([
            ("backend", Value::String(backend.into())),
            ("available", Value::Bool(webgpu_available || webgl2_available)),
            ("webgl2_fallback", Value::Bool(webgl2_available)),
            ("device_name", Value::String(
                if webgpu_available { "WebGPU (browser)" }
                else if webgl2_available { "WebGL2 (browser fallback)" }
                else { "no GPU adapter" }
                .into(),
            )),
        ]))
    }
}

/// Probe whether `navigator.gpu` is present in the browser.
#[cfg(target_arch = "wasm32")]
fn webgpu_available_wasm() -> bool {
    use wasm_bindgen::JsCast;
    let navigator = web_sys::window().and_then(|w| w.navigator());
    let Some(navigator) = navigator else {
        return false;
    };
    // `navigator.gpu` is the WebGPU entry point. If it's `undefined`, WebGPU
    // is not available in this browser.
    let gpu = js_sys::Reflect::get(&navigator, &"gpu".into());
    match gpu {
        Ok(v) => !v.is_undefined() && !v.is_null(),
        Err(_) => false,
    }
}

/// Probe whether a WebGL2 context can be created on a test canvas.
/// This checks `navigator` + canvas support without requiring an actual
/// canvas element from the caller.
#[cfg(target_arch = "wasm32")]
fn webgl2_context_available_wasm() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(Some(document)) = window.document().map(|d| Some(d)) else {
        return false;
    };
    // Create a 1×1 test canvas and try to get a webgl2 context.
    let Ok(canvas) = document.create_element("canvas") else {
        return false;
    };
    let Some(canvas) = canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok() else {
        return false;
    };
    canvas.set_width(1);
    canvas.set_height(1);
    let ctx = canvas.get_context("webgl2");
    matches!(ctx, Ok(Some(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> crate::poet_host::PoetSnapshot {
        crate::poet_host::PoetSnapshot::default()
    }

    fn eval(src: &str) -> Value {
        let mut snap = snap();
        snap.eval_fn(src, "go", vec![]).expect("script should eval")
    }

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn g_gpu_backend_info_returns_record() {
        let result = gpu_backend_info(&Value::Record(Default::default()), dummy_span())
            .expect("backend_info");
        assert!(matches!(result, Value::Record(_)), "expected record");
        if let Value::Record(m) = &result {
            assert!(
                m.contains_key("backend"),
                "should have backend field: {m:?}"
            );
            assert!(
                m.contains_key("available"),
                "should have available field: {m:?}"
            );
        }
    }

    #[test]
    fn g_gpu_backend_info_via_vibescript() {
        let src = r#"
        requires [ capability("capability.invoke") ];
        effect fn go() {
            return capability.invoke("Render.gpu_backend_info", {});
        }
        "#;
        let result = eval(src);
        assert!(matches!(result, Value::Record(_)), "expected record, got {result:?}");
        if let Value::Record(m) = &result {
            let backend = m.get("backend").expect("has backend");
            match backend {
                Value::String(s) => {
                    assert!(
                        matches!(s.as_str(), "webgpu" | "webgl2" | "none"),
                        "backend should be webgpu|webgl2|none, got {s}"
                    );
                }
                other => panic!("backend should be a string, got {other:?}"),
            }
        }
    }
}
