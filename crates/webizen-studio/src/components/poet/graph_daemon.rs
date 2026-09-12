//! Catalog · Lexicon → local graph daemon `:4242`.
//!
//! Soft-waits Native Connected (`GET /health`) then posts
//! `GraphDatabase.lexicon_manifest`. Held / not yet if the daemon is down.
//! Never "unavailable". No Host widen.

use super::engine::PoetEvalResult;
use serde::Deserialize;
use webizen_studio::lexicon_catalog::{
    health_url, invoke_body, invoke_url, PRIMARY_DAEMON_PORT,
};

const WAIT_STEPS: u32 = 20;
const WAIT_MS: i32 = 250;

#[derive(Debug, Deserialize, Default)]
struct DaemonEvalResponse {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    value: String,
    diagnostic: Option<String>,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    committed: usize,
    #[serde(default)]
    honesty: String,
}

fn into_eval(resp: DaemonEvalResponse) -> PoetEvalResult {
    PoetEvalResult {
        ok: resp.ok,
        value: resp.value,
        diagnostic: resp.diagnostic,
        revision: resp.revision,
        committed: resp.committed,
        honesty: if resp.honesty.is_empty() {
            "live".into()
        } else {
            resp.honesty
        },
        language: String::new(),
        value_cbor_hex: String::new(),
    }
}

fn candidate_ports() -> [u16; 1] {
    [PRIMARY_DAEMON_PORT]
}

async fn sleep_ms(ms: i32) {
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(ms as u32).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    }
}

async fn invoke_if_connected(path: &str) -> Result<PoetEvalResult, String> {
    let mut port = None;
    for p in candidate_ports() {
        if probe_health(p).await {
            port = Some(p);
            break;
        }
    }
    let port = port.ok_or_else(|| webizen_studio::lexicon_catalog::HELD_WHY.to_string())?;
    let body = invoke_body(path);
    let text = post_json(&invoke_url(port), &body).await?;
    let resp: DaemonEvalResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(into_eval(resp))
}

/// One-shot probe + invoke. No wait — Desktop host fallback stays snappy.
pub async fn daemon_lexicon_manifest(path: &str) -> Result<PoetEvalResult, String> {
    invoke_if_connected(path).await
}

/// Soft-wait Native Connected, then invoke (webview without Tauri).
pub async fn daemon_lexicon_manifest_wait(path: &str) -> Result<PoetEvalResult, String> {
    for _ in 0..WAIT_STEPS {
        if let Ok(result) = invoke_if_connected(path).await {
            return Ok(result);
        }
        sleep_ms(WAIT_MS).await;
    }
    Err(webizen_studio::lexicon_catalog::HELD_WHY.to_string())
}

async fn probe_health(port: u16) -> bool {
    get_ok(&health_url(port)).await
}

#[cfg(target_arch = "wasm32")]
async fn get_ok(url: &str) -> bool {
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(value) = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await else {
        return false;
    };
    let Ok(resp) = value.dyn_into::<web_sys::Response>() else {
        return false;
    };
    resp.ok()
}

#[cfg(target_arch = "wasm32")]
async fn post_json(url: &str, body: &serde_json::Value) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let window = web_sys::window().ok_or_else(|| {
        webizen_studio::lexicon_catalog::HELD_WHY.to_string()
    })?;
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    let payload = serde_json::to_string(body).map_err(|e| e.to_string())?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&payload));
    let request = Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_val.dyn_into().map_err(|_| {
        webizen_studio::lexicon_catalog::HELD_WHY.to_string()
    })?;
    let ok = resp.ok();
    let text_p = resp.text().map_err(|e| format!("{e:?}"))?;
    let text_v = wasm_bindgen_futures::JsFuture::from(text_p)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text = text_v
        .as_string()
        .ok_or_else(|| webizen_studio::lexicon_catalog::HELD_WHY.to_string())?;
    if !ok {
        return Err(webizen_studio::lexicon_catalog::HELD_WHY.to_string());
    }
    Ok(text)
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_ok(url: &str) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .build()
    else {
        return false;
    };
    client
        .get(url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
async fn post_json(url: &str, body: &serde_json::Value) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|_| webizen_studio::lexicon_catalog::HELD_WHY.to_string())?;
    let res = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|_| webizen_studio::lexicon_catalog::HELD_WHY.to_string())?;
    if !res.status().is_success() {
        return Err(webizen_studio::lexicon_catalog::HELD_WHY.to_string());
    }
    res.text()
        .await
        .map_err(|_| webizen_studio::lexicon_catalog::HELD_WHY.to_string())
}
