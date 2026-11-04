//! Browser transport credential and cancellable authenticated inference stream.
//! This is the existing Webizen token, never an identity/policy credential.
use std::cell::RefCell;
use wasm_bindgen::{prelude::*, JsCast};
use super::native_daemon::NativeLlmJobEvent;

thread_local! {
    static TOKEN: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
    static STREAM: RefCell<Option<web_sys::AbortController>> = const { RefCell::new(None) };
}

#[wasm_bindgen(module = "/src/browser/native_llm_stream.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = readNativeLlmStream)]
    async fn read_stream(url: &str, token: &str, signal: &web_sys::AbortSignal,
        on_data: &js_sys::Function) -> Result<JsValue, JsValue>;
}

/// Native embedding may hand over its already configured Webizen token. It is
/// retained only in memory and scoped to the selected daemon URL.
#[wasm_bindgen]
pub fn set_native_daemon_token(base_url: String, token: String) -> Result<(), JsValue> {
    if token.is_empty() || token.len() > 4096 || token.chars().any(|c| c == '\r' || c == '\n') {
        return Err(JsValue::from_str("A bounded Webizen transport token is required"));
    }
    let selected = super::native_daemon::get_connected_daemon_url();
    if selected.as_deref() != Some(base_url.as_str()) {
        return Err(JsValue::from_str("Select the connected daemon before setting its token"));
    }
    close();
    TOKEN.with(|slot| *slot.borrow_mut() = Some((base_url, token)));
    Ok(())
}

pub(super) fn token(base_url: &str) -> Result<String, String> {
    if let Some(token) = TOKEN.with(|slot| slot.borrow().as_ref()
        .filter(|(url, _)| url == base_url).map(|(_, token)| token.clone())) {
        return Ok(token);
    }
    // This is reached by the actual LLM UI action, not health probing. A native
    // embedding can instead call the explicit setter above before inference.
    let token = web_sys::window().ok_or("Window unavailable")?
        .prompt_with_message("Enter the existing Webizen token configured for this native daemon. This authorizes transport; native actor and policy setup are also required.")
        .map_err(|_| "Could not request Webizen token")?
        .ok_or("Webizen transport authentication cancelled")?;
    set_native_daemon_token(base_url.to_owned(), token.clone())
        .map_err(|_| "Invalid Webizen transport token")?;
    Ok(token)
}

pub(super) fn close() {
    STREAM.with(|slot| {
        if let Some(controller) = slot.borrow_mut().take() { controller.abort(); }
    });
}

pub(super) fn open(base_url: &str, job_id: &str,
    mut on_event: impl FnMut(NativeLlmJobEvent) + 'static) -> Result<(), String> {
    close();
    let token = token(base_url)?;
    let controller = web_sys::AbortController::new().map_err(|_| "Abort controller unavailable")?;
    let signal = controller.signal();
    let encoded = js_sys::encode_uri_component(job_id).as_string()
        .ok_or("Could not encode local-model job identifier")?;
    let url = format!("{base_url}/llm/jobs/events?job_id={encoded}");
    STREAM.with(|slot| *slot.borrow_mut() = Some(controller));
    wasm_bindgen_futures::spawn_local(async move {
        let callback = Closure::wrap(Box::new(move |text: String| {
            match serde_json::from_str::<NativeLlmJobEvent>(&text) {
                Ok(event) => on_event(event),
                Err(error) => on_event(NativeLlmJobEvent { seq: 0, kind: "error".into(),
                    data: serde_json::json!({"diagnostic": format!("Malformed daemon event: {error}")}) }),
            }
        }) as Box<dyn FnMut(String)>);
        if let Err(error) = read_stream(&url, &token, &signal, callback.as_ref().unchecked_ref()).await {
            if !signal.aborted() {
                // Keep errors on the same consuming event route as decode errors.
                let event = serde_json::json!({"seq":0,"kind":"error", "data":{
                    "diagnostic": format!("Local-model stream failed: {error:?}")}}).to_string();
                let _ = callback.as_ref().unchecked_ref::<js_sys::Function>()
                    .call1(&JsValue::NULL, &JsValue::from_str(&event));
            }
        }
        // Closure drops here on terminal/error/abort; no forgotten callbacks.
    });
    Ok(())
}
