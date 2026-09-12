//! Catalog · Lexicon → local graph daemon `:4242`.
//!
//! Soft-waits Native Connected (`GET /health`) then posts
//! `GraphDatabase.lexicon_manifest`. Held / not yet if the daemon is down.
//! Never "unavailable". No Host widen.

use super::engine::{DaemonProbe, PoetEvalResult};
use serde::Deserialize;
use webizen_studio::lexicon_catalog::{
    health_url, invoke_body, invoke_url, ok_from_invoke_json, value_from_invoke_json, HELD_WHY,
    NATIVE_CONNECTED_LABEL, PRIMARY_DAEMON_PORT,
};

const WAIT_STEPS: u32 = 8;
const WAIT_MS: i32 = 250;

#[derive(Debug, Deserialize, Default)]
struct DaemonEvalResponse {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    value: serde_json::Value,
    diagnostic: Option<String>,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    committed: usize,
    #[serde(default)]
    honesty: String,
}

fn into_eval(resp: DaemonEvalResponse) -> PoetEvalResult {
    let envelope = serde_json::json!({
        "ok": resp.ok,
        "value": resp.value,
    });
    let value = value_from_invoke_json(&envelope);
    let ok = ok_from_invoke_json(&envelope, &value);
    PoetEvalResult {
        ok,
        value,
        diagnostic: resp.diagnostic,
        revision: resp.revision,
        committed: resp.committed,
        honesty: if resp.honesty.is_empty() {
            if ok {
                "live".into()
            } else {
                "held".into()
            }
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
    let port = port.ok_or_else(|| HELD_WHY.to_string())?;
    let body = invoke_body(path);
    let text = post_json(&invoke_url(port), &body).await?;
    let resp: DaemonEvalResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(into_eval(resp))
}

/// One-shot probe + invoke. No wait — Desktop host fallback stays snappy.
pub async fn daemon_lexicon_manifest(path: &str) -> Result<PoetEvalResult, String> {
    invoke_if_connected(path).await
}

/// Soft-wait Native Connected, then invoke (webview without a working Tauri arg).
pub async fn daemon_lexicon_manifest_wait(path: &str) -> Result<PoetEvalResult, String> {
    for _ in 0..WAIT_STEPS {
        if let Ok(result) = invoke_if_connected(path).await {
            return Ok(result);
        }
        sleep_ms(WAIT_MS).await;
    }
    Err(HELD_WHY.to_string())
}

/// Loopback `/health` → Native Connected chip without Tauri.
pub async fn daemon_http_probe() -> Option<DaemonProbe> {
    for port in candidate_ports() {
        if let Some(probe) = get_health_probe(port).await {
            return Some(probe);
        }
    }
    None
}

async fn probe_health(port: u16) -> bool {
    get_ok(&health_url(port)).await
}

async fn get_health_probe(port: u16) -> Option<DaemonProbe> {
    let text = get_text(&health_url(port)).await.ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let engine = v.get("engine").and_then(|x| x.as_str()).map(str::to_string);
    if engine.is_none() && v.get("engine_version").is_none() {
        return None;
    }
    Some(DaemonProbe {
        reachable: true,
        url: format!("http://127.0.0.1:{port}"),
        port,
        engine,
        version: v
            .get("version")
            .or_else(|| v.get("engine_version"))
            .and_then(|x| x.as_str())
            .map(str::to_string),
        graph_quin_count: v
            .get("graph_quin_count")
            .and_then(|x| x.as_u64())
            .map(|n| n as usize),
        honesty: "live".into(),
        label: NATIVE_CONNECTED_LABEL.to_string(),
    })
}

#[cfg(target_arch = "wasm32")]
async fn get_ok(url: &str) -> bool {
    get_text(url).await.is_ok()
}

#[cfg(target_arch = "wasm32")]
async fn get_text(url: &str) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or_else(|| HELD_WHY.to_string())?;
    let value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: web_sys::Response = value
        .dyn_into()
        .map_err(|_| HELD_WHY.to_string())?;
    if !resp.ok() {
        return Err(HELD_WHY.to_string());
    }
    let text_p = resp.text().map_err(|e| format!("{e:?}"))?;
    let text_v = wasm_bindgen_futures::JsFuture::from(text_p)
        .await
        .map_err(|e| format!("{e:?}"))?;
    text_v
        .as_string()
        .ok_or_else(|| HELD_WHY.to_string())
}

#[cfg(target_arch = "wasm32")]
async fn post_json(url: &str, body: &serde_json::Value) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let window = web_sys::window().ok_or_else(|| HELD_WHY.to_string())?;
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
    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| HELD_WHY.to_string())?;
    let ok = resp.ok();
    let text_p = resp.text().map_err(|e| format!("{e:?}"))?;
    let text_v = wasm_bindgen_futures::JsFuture::from(text_p)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text = text_v
        .as_string()
        .ok_or_else(|| HELD_WHY.to_string())?;
    if !ok {
        return Err(HELD_WHY.to_string());
    }
    Ok(text)
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_ok(url: &str) -> bool {
    get_text(url).await.is_ok()
}

#[cfg(not(target_arch = "wasm32"))]
async fn get_text(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .build()
        .map_err(|_| HELD_WHY.to_string())?;
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|_| HELD_WHY.to_string())?;
    if !res.status().is_success() {
        return Err(HELD_WHY.to_string());
    }
    res.text().await.map_err(|_| HELD_WHY.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn post_json(url: &str, body: &serde_json::Value) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|_| HELD_WHY.to_string())?;
    let res = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|_| HELD_WHY.to_string())?;
    if !res.status().is_success() {
        return Err(HELD_WHY.to_string());
    }
    res.text()
        .await
        .map_err(|_| HELD_WHY.to_string())
}
