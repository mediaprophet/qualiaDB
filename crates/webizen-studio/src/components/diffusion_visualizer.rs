use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8ClampedArray;
#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;
use serde::Deserialize;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData, Response};

const CANVAS_ID: &str = "diffusion-surface";

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct RuntimeSnapshotRecord {
    epoch: u64,
    dimensions: (u32, u32),
    frame_slot: u8,
    state_hash: [u8; 32],
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct RuntimeLedgerHealth {
    persisted_epoch: u64,
    dropped_events: u64,
    gap_events: u64,
    recovery_events: u64,
    write_failures: u64,
    last_gap_from_epoch: Option<u64>,
    last_gap_to_epoch: Option<u64>,
    degraded: bool,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(
        event: &str,
        handler: &js_sys::Function,
    ) -> Result<js_sys::Function, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_tauri_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
fn get_canvas_context() -> Result<(HtmlCanvasElement, CanvasRenderingContext2d), String> {
    let document = web_sys::window()
        .ok_or_else(|| "window unavailable".to_string())?
        .document()
        .ok_or_else(|| "document unavailable".to_string())?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id(CANVAS_ID)
        .ok_or_else(|| "diffusion canvas not mounted".to_string())?
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
async fn draw_snapshot(snapshot: RuntimeSnapshotRecord) -> Result<(), String> {
    let frame_uri = format!(
        "webizen://localhost/diffusion/frame/{}?epoch={}",
        snapshot.frame_slot, snapshot.epoch
    );
    let window = web_sys::window().ok_or_else(|| "window unavailable".to_string())?;
    let response = JsFuture::from(window.fetch_with_str(&frame_uri))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into::<Response>()
        .map_err(|_| "failed to cast frame response".to_string())?;
    if !response.ok() {
        return Err(format!(
            "frame fetch failed with status {}",
            response.status()
        ));
    }

    let frame_bytes = JsFuture::from(
        response
            .array_buffer()
            .map_err(|_| "failed to request frame buffer".to_string())?,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    let frame_bytes = Uint8ClampedArray::new(&frame_bytes);

    let (canvas, context) = get_canvas_context()?;
    if canvas.width() != snapshot.dimensions.0 {
        canvas.set_width(snapshot.dimensions.0);
    }
    if canvas.height() != snapshot.dimensions.1 {
        canvas.set_height(snapshot.dimensions.1);
    }

    let image_data = ImageData::new_with_js_u8_clamped_array_and_sh(
        &frame_bytes,
        snapshot.dimensions.0,
        snapshot.dimensions.1,
    )
    .map_err(|_| "failed to construct image data".to_string())?;

    context
        .put_image_data(&image_data, 0.0, 0.0)
        .map_err(|_| "failed to blit diffusion frame".to_string())?;

    Ok(())
}

#[component]
pub fn DiffusionVisualizer() -> Element {
    let latest_epoch = use_signal(|| 0u64);
    let dimensions = use_signal(|| (128u32, 128u32));
    let status = use_signal(|| "Awaiting diffusion kernel...".to_string());
    let ledger_health = use_signal(|| None::<RuntimeLedgerHealth>);
    #[cfg(target_arch = "wasm32")]
    let listener_started = use_signal(|| false);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let mut listener_started = listener_started;
            if listener_started() {
                return;
            }
            listener_started.set(true);

            let mut latest_epoch = latest_epoch;
            let mut dimensions = dimensions;
            let mut status = status;
            let mut ledger_health = ledger_health;

            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(Some(snapshot)) = invoke_tauri_json::<Option<RuntimeSnapshotRecord>>(
                    "get_latest_diffusion_snapshot",
                    json!({}),
                )
                .await
                {
                    latest_epoch.set(snapshot.epoch);
                    dimensions.set(snapshot.dimensions);
                    match draw_snapshot(snapshot).await {
                        Ok(_) => status.set("Streaming deterministic epochs".to_string()),
                        Err(err) => status.set(format!("Initial blit failed: {err}")),
                    }
                }

                if let Ok(health) = invoke_tauri_json::<RuntimeLedgerHealth>(
                    "get_diffusion_ledger_health",
                    json!({}),
                )
                .await
                {
                    ledger_health.set(Some(health));
                }

                let callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |event: JsValue| {
                        let payload = js_sys::Reflect::get(&event, &JsValue::from_str("payload"));
                        let mut latest_epoch = latest_epoch;
                        let mut dimensions = dimensions;
                        let mut status = status;

                        if let Ok(payload) = payload {
                            match serde_wasm_bindgen::from_value::<RuntimeSnapshotRecord>(payload) {
                                Ok(snapshot) => {
                                    latest_epoch.set(snapshot.epoch);
                                    dimensions.set(snapshot.dimensions);
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if let Err(err) = draw_snapshot(snapshot).await {
                                            status.set(format!("Blit failed: {err}"));
                                        } else {
                                            status
                                                .set("Streaming deterministic epochs".to_string());
                                        }
                                    });
                                }
                                Err(err) => status.set(format!("Snapshot decode failed: {err}")),
                            }
                        } else {
                            status.set("Diffusion event payload missing".to_string());
                        }
                    }));

                match tauri_listen("diffusion-epoch-ready", callback.as_ref().unchecked_ref()).await
                {
                    Ok(_unlisten) => {
                        callback.forget();
                    }
                    Err(err) => {
                        status.set(format!("Event listener failed: {err:?}"));
                    }
                }

                let ledger_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |event: JsValue| {
                        let payload = js_sys::Reflect::get(&event, &JsValue::from_str("payload"));
                        let mut ledger_health = ledger_health;

                        if let Ok(payload) = payload {
                            match serde_wasm_bindgen::from_value::<RuntimeLedgerHealth>(payload) {
                                Ok(health) => ledger_health.set(Some(health)),
                                Err(err) => {
                                    web_sys::console::error_1(
                                        &format!("Ledger health decode failed: {err}").into(),
                                    );
                                }
                            }
                        }
                    }));

                match tauri_listen(
                    "diffusion-ledger-health",
                    ledger_callback.as_ref().unchecked_ref(),
                )
                .await
                {
                    Ok(_unlisten) => {
                        ledger_callback.forget();
                    }
                    Err(err) => {
                        status.set(format!("Ledger listener failed: {err:?}"));
                    }
                }
            });
        }
    });

    let dims = dimensions();
    let epoch = latest_epoch();
    let status_text = status();
    let ledger_health_value = ledger_health();
    let ledger_alert = ledger_health_value
        .as_ref()
        .filter(|health| health.degraded)
        .map(
            |health| match (health.last_gap_from_epoch, health.last_gap_to_epoch) {
                (Some(from_epoch), Some(to_epoch)) => format!(
                "Ledger degraded: {} dropped handoffs, {} recovery baselines, last gap {} -> {}.",
                health.dropped_events, health.recovery_events, from_epoch, to_epoch
            ),
                _ => format!(
                    "Ledger degraded: {} dropped handoffs, {} write failures.",
                    health.dropped_events, health.write_failures
                ),
            },
        );
    let ledger_status_text = match ledger_health_value.as_ref() {
        Some(health) if health.degraded => "Degraded",
        Some(_) => "Nominal",
        None => "Pending",
    };
    let ledger_status_color = match ledger_health_value.as_ref() {
        Some(health) if health.degraded => "#f59e0b",
        Some(_) => "var(--qualia-accent)",
        None => "var(--qualia-text-muted)",
    };

    rsx! {
        div {
            class: "panel-card",
            style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 18px; padding: 1.15rem 1.2rem 1.25rem; backdrop-filter: blur(24px); box-shadow: 0 8px 32px rgba(0,0,0,0.08); margin-bottom: 1.5rem;",

            if let Some(alert) = ledger_alert.clone() {
                div {
                    style: "margin-bottom: 0.9rem; border-radius: 14px; border: 1px solid rgba(245,158,11,0.28); background: rgba(245,158,11,0.12); color: #fbbf24; padding: 0.75rem 0.9rem; font-size: 0.78rem; line-height: 1.5;",
                    "{alert}"
                }
            }

            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; margin-bottom: 0.9rem;",
                div {
                    h2 {
                        style: "margin: 0 0 0.25rem 0; font-size: 0.98rem; font-weight: 700; color: var(--qualia-text);",
                        "Discrete Diffusion Proving Ground"
                    }
                    p {
                        style: "margin: 0; font-size: 0.76rem; color: var(--qualia-text-muted); line-height: 1.45;",
                        "Canvas blits consume completed runtime epochs directly from the native substrate. No RGBA frame bytes pass through the Dioxus Virtual DOM."
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; align-items: flex-end; gap: 0.25rem; min-width: 140px;",
                    span { style: "font-size: 0.72rem; color: var(--qualia-text-muted); text-transform: uppercase; letter-spacing: 0.05em;", "Latest Epoch" }
                    span { style: "font-size: 1.15rem; font-weight: 700; color: var(--qualia-accent);", "{epoch}" }
                }
            }

            div {
                style: "display: grid; grid-template-columns: minmax(0, 1fr) 220px; gap: 1rem; align-items: start;",

                div {
                    style: "background: rgba(0,0,0,0.12); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 0.8rem; min-height: 340px;",
                    canvas {
                        id: "{CANVAS_ID}",
                        width: "{dims.0}",
                        height: "{dims.1}",
                        style: "display: block; width: 100%; aspect-ratio: 1 / 1; image-rendering: pixelated; border-radius: 10px; background: radial-gradient(circle at 30% 25%, rgba(255,255,255,0.09), transparent 45%), linear-gradient(180deg, rgba(10,12,18,0.96), rgba(4,6,12,0.98));",
                    }
                }

                div {
                    style: "display: flex; flex-direction: column; gap: 0.75rem;",

                    div {
                        style: "background: rgba(128,128,128,0.05); border: 1px solid var(--qualia-border); border-radius: 14px; padding: 0.8rem 0.9rem;",
                        div { style: "font-size: 0.68rem; color: var(--qualia-text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.35rem;", "Surface" }
                        div { style: "font-size: 0.98rem; font-weight: 700; color: var(--qualia-text);", "{dims.0} x {dims.1}" }
                    }

                    div {
                        style: "background: rgba(128,128,128,0.05); border: 1px solid var(--qualia-border); border-radius: 14px; padding: 0.8rem 0.9rem;",
                        div { style: "font-size: 0.68rem; color: var(--qualia-text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.35rem;", "Observation Path" }
                        div { style: "font-size: 0.82rem; font-weight: 600; color: var(--qualia-text); line-height: 1.45;", "Wake signal -> frame fetch -> direct canvas blit" }
                    }

                    div {
                        style: "background: rgba(128,128,128,0.05); border: 1px solid var(--qualia-border); border-radius: 14px; padding: 0.8rem 0.9rem;",
                        div { style: "font-size: 0.68rem; color: var(--qualia-text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.35rem;", "Status" }
                        div { style: "font-size: 0.8rem; color: var(--qualia-text); line-height: 1.45;", "{status_text}" }
                    }

                    div {
                        style: "background: rgba(128,128,128,0.05); border: 1px solid var(--qualia-border); border-radius: 14px; padding: 0.8rem 0.9rem;",
                        div { style: "font-size: 0.68rem; color: var(--qualia-text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.35rem;", "Ledger" }
                        div { style: "font-size: 0.92rem; font-weight: 700; color: {ledger_status_color}; margin-bottom: 0.35rem;", "{ledger_status_text}" }
                        div {
                            style: "font-size: 0.75rem; color: var(--qualia-text-muted); line-height: 1.45;",
                            match ledger_health_value.as_ref() {
                                Some(health) => format!(
                                    "Persisted epoch {} | dropped {} | recoveries {}",
                                    health.persisted_epoch, health.dropped_events, health.recovery_events
                                ),
                                None => "Waiting for persistence telemetry".to_string(),
                            }
                        }
                    }
                }
            }
        }
    }
}
