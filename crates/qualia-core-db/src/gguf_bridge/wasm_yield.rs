//! Browser cooperative multitasking for WASM LLM init.
//!
//! `initialize_webgpu_engine` used to run multi‑hundred‑MB parse + pipeline compile +
//! weight upload **synchronously** after the first `await`. That freezes the main
//! thread so:
//! - the UI heartbeat stuck on "Initialising WebGPU" never advances
//! - JS `setTimeout` init races never fire
//! - phones look permanently hung
//!
//! Yield via `setTimeout(0)` between phases so the event loop can paint and the
//! demo can surface stage text via [`init_status`].

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

thread_local! {
    static INIT_STATUS: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Human-readable stage for the UI (polled from JS).
pub fn init_status() -> String {
    INIT_STATUS.with(|s| s.borrow().clone())
}

pub fn set_init_status(msg: impl Into<String>) {
    let msg = msg.into();
    INIT_STATUS.with(|s| *s.borrow_mut() = msg.clone());
    // Always mirror to console so remote debug / chrome://inspect sees progress
    // even when the page UI is frozen for a short stretch between yields.
    web_sys::console::log_1(&JsValue::from_str(&format!("[webgpu-init] {msg}")));
}

pub fn clear_init_status() {
    INIT_STATUS.with(|s| s.borrow_mut().clear());
}

/// Yield to the browser event loop (one macrotask via `setTimeout(0)`).
///
/// Required on phones: without this, pipeline compile + weight upload monopolise
/// the main thread and the demo appears stuck on "Initialising WebGPU".
pub async fn yield_to_browser() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let resolve = resolve.clone();
        let cb = Closure::once(move || {
            let _ = resolve.call0(&JsValue::UNDEFINED);
        });
        if window
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 0)
            .is_ok()
        {
            // Timeout owns the callback until it fires; forget so Rust does not drop early.
            cb.forget();
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Publish a stage message, then yield so the UI can paint it.
pub async fn phase(msg: &str) {
    set_init_status(msg);
    yield_to_browser().await;
}
