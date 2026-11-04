//! Browser numerical execution is an explicit developer bypass, never authority.
use std::cell::Cell;
use wasm_bindgen::prelude::*;

thread_local! {
    static DEVELOPER: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn developer() -> bool {
    DEVELOPER.with(Cell::get)
}

pub(super) fn name() -> &'static str {
    if developer() {
        "developer"
    } else {
        "governed"
    }
}

pub(super) fn require_developer() -> Result<(), String> {
    if developer() {
        Ok(())
    } else {
        Err("Browser governed inference is unavailable: no Qualia/Webizen token acceptance is connected. For ungoverned numerical debugging, explicitly call configureBrowserExecutionMode('developer', true). Sentinel is disabled in developer mode.".into())
    }
}

/// Explicitly acknowledge the absence of Sentinel token acceptance for browser
/// CPU/WebGPU compute. Selecting governed mode closes admission; it does not
/// install or authenticate a browser authority. Reconfiguration cancels pending
/// requests through the same generation mechanism as model/session replacement.
#[wasm_bindgen(js_name = configureBrowserExecutionMode)]
pub fn configure_browser_execution_mode(
    mode: &str,
    acknowledge_unguarded: bool,
) -> Result<(), JsValue> {
    let enabled = match mode {
        "developer" if acknowledge_unguarded => true,
        "developer" => return Err(JsValue::from_str("Developer mode requires acknowledgeUnguarded=true; Sentinel token acceptance is disabled")),
        "governed" => false,
        _ => return Err(JsValue::from_str("Browser execution mode must be 'governed' or 'developer'")),
    };
    if enabled != developer() {
        super::lifecycle::invalidate_model().map_err(|e| JsValue::from_str(&e))?;
        super::cpu::release();
        crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|slot| *slot.borrow_mut() = None);
        DEVELOPER.with(|mode| mode.set(enabled));
    }
    Ok(())
}
