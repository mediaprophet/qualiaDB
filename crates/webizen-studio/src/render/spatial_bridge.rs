//! Spatial presentation mode — volumetric GPU preview via the native engine.
//!
//! Desktop/Tauri: headless `PortalGpu` render → `webizen://` PNG (no Three.js).
//! Public web demo: informative fallback until portal wasm is bundled for GH Pages.

use dioxus::prelude::*;
use crate::canvas_model::Page;

#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

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
fn render_preview_args(page: &Page, width: u32, height: u32) -> serde_json::Value {
    json!({
        "width": width,
        "height": height,
        "panes": page.panes,
    })
}

#[cfg(target_arch = "wasm32")]
fn spawn_spatial_refresh(page: Page, mut epoch: Signal<u64>, mut status: Signal<String>) {
    wasm_bindgen_futures::spawn_local(async move {
        match invoke_tauri_json::<()>(
            "update_render_preview",
            render_preview_args(&page, 960, 540),
        )
        .await
        {
            Ok(_) => {
                epoch.set(epoch() + 1);
                status.set(format!(
                    "PortalGpu frame — {} workspace panes",
                    page.panes.len()
                ));
            }
            Err(err) => status.set(format!("Spatial render unavailable: {err}")),
        }
    });
}

#[component]
pub fn SpatialBridgeCanvas(page: Page) -> Element {
    let page_for_effect = page.clone();
    let epoch = use_signal(|| 0u64);
    let live_portal = use_signal(|| false);
    let status = use_signal(|| {
        if crate::endpoints::is_native_host() {
            "Initializing volumetric renderer…".to_string()
        } else {
            "Spatial view requires the Webizen desktop host.".to_string()
        }
    });

    #[cfg(not(target_arch = "wasm32"))]
    use_effect({
        let page_for_effect = page_for_effect.clone();
        move || {
            if !crate::endpoints::is_native_host() {
                return;
            }
            let mut epoch = epoch;
            let mut status = status;
            let panes = page_for_effect.panes.clone();
            spawn(async move {
                if let Some(png_len) =
                    crate::render::native_headless_png_byte_len(&panes, 960, 540).await
                {
                    epoch.set(epoch() + 1);
                    status.set(format!(
                        "PortalGpu frame — {} bytes, {} panes",
                        png_len,
                        panes.len()
                    ));
                }
            });
        }
    });

    #[cfg(target_arch = "wasm32")]
    {
        // Clone into a shadow the move-closure can own, leaving the outer
        // `page_for_effect` available for `toggle_live_portal` below (mirrors the
        // non-wasm path above). Without this the wasm build fails borrow-after-move.
        let page_for_effect = page_for_effect.clone();
        let started = use_signal(|| false);
        use_effect(move || {
            if !crate::endpoints::is_native_host() {
                return;
            }
            let mut started = started;
            if started() {
                return;
            }
            started.set(true);

            let mut epoch = epoch;
            let mut status = status;
            let page_snapshot = page_for_effect.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event: JsValue| {
                        epoch.set(epoch() + 1);
                    }));
                if tauri_listen("render-preview-ready", callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    callback.forget();
                }

                let _ = invoke_tauri_json::<bool>(
                    "toggle_render_loop",
                    json!({ "isActive": true }),
                )
                .await;

                match invoke_tauri_json::<()>(
                    "update_render_preview",
                    render_preview_args(&page_snapshot, 960, 540),
                )
                .await
                {
                    Ok(_) => {
                        epoch.set(epoch() + 1);
                        status.set(format!(
                            "PortalGpu frame — {} workspace panes",
                            page_snapshot.panes.len()
                        ));
                    }
                    Err(err) => status.set(format!("Spatial render unavailable: {err}")),
                }
            });
        });
    }

    let refresh = {
        let page_snapshot = page.clone();
        move |_| {
            #[cfg(target_arch = "wasm32")]
            if crate::endpoints::is_native_host() {
                spawn_spatial_refresh(page_snapshot.clone(), epoch, status);
            }
            #[cfg(not(target_arch = "wasm32"))]
            if crate::endpoints::is_native_host() {
                let panes = page_snapshot.panes.clone();
                let mut epoch = epoch;
                let mut status = status;
                spawn(async move {
                    if let Some(png_len) =
                        crate::render::native_headless_png_byte_len(&panes, 960, 540).await
                    {
                        epoch.set(epoch() + 1);
                        status.set(format!(
                            "PortalGpu refresh — {png_len} bytes, {} panes",
                            panes.len()
                        ));
                    }
                });
            }
        }
    };
    let refresh_again = refresh.clone();

    let toggle_live_portal = {
        let mut live_portal = live_portal;
        let mut status = status;
        let _ = page_for_effect.panes.len();
        move |_| {
            let next = !live_portal();
            live_portal.set(next);
            if next {
                status.set("Live Qualia Portal (T2 WASM)".to_string());
            } else {
                status.set("PortalGpu volumetric frame".to_string());
            }
        }
    };

    let frame_src = format!("webizen://localhost/render/preview.png?t={}", epoch());
    let portal_src = crate::endpoints::portal_design_studio_url();
    let native = crate::endpoints::is_native_host();
    let has_frame = native && epoch() > 0 && !live_portal();
    let show_live = native && live_portal();

    rsx! {
        div {
            style: "position: relative; width: 100%; height: 100%; min-height: 500px; background: var(--qualia-bg, #050510); border: 1px solid var(--qualia-border, #333); border-radius: 12px; overflow: hidden;",

            if show_live {
                iframe {
                    src: "{portal_src}",
                    title: "Qualia Portal Design Studio",
                    style: "position: absolute; inset: 0; width: 100%; height: 100%; border: 0; display: block;",
                }
            } else if has_frame {
                img {
                    src: "{frame_src}",
                    style: "position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; display: block;",
                }
            } else {
                div {
                    style: "position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.75rem; padding: 2rem; text-align: center;",
                    div {
                        style: "color: var(--qualia-accent, #f59e0b); font-size: 1.25rem; font-weight: 600;",
                        "10D Manifold — PortalGpu"
                    }
                    div {
                        style: "color: var(--qualia-text-muted, #888); font-size: 0.85rem; max-width: 420px; line-height: 1.5;",
                        "{status()}"
                    }
                    if native {
                        button {
                            style: "margin-top: 0.5rem; padding: 0.45rem 1rem; border-radius: 8px; border: 1px solid var(--qualia-border); background: var(--qualia-surface); color: var(--qualia-text); cursor: pointer; font-size: 0.8rem;",
                            onclick: refresh,
                            "Refresh spatial frame"
                        }
                    }
                }
            }

            div {
                style: "position: absolute; top: 0.75rem; right: 0.75rem; display: flex; gap: 0.4rem; z-index: 20;",
                if native {
                    button {
                        style: "padding: 0.3rem 0.65rem; border-radius: 6px; border: 1px solid var(--qualia-border); background: rgba(0,0,0,0.45); color: var(--qualia-text); font-size: 0.7rem; cursor: pointer; backdrop-filter: blur(8px);",
                        onclick: toggle_live_portal,
                        if live_portal() { "PNG" } else { "Live" }
                    }
                    if !live_portal() {
                        button {
                            style: "padding: 0.3rem 0.65rem; border-radius: 6px; border: 1px solid var(--qualia-border); background: rgba(0,0,0,0.45); color: var(--qualia-text); font-size: 0.7rem; cursor: pointer; backdrop-filter: blur(8px);",
                            onclick: refresh_again,
                            "↻"
                        }
                    }
                }
                span {
                    style: "padding: 0.3rem 0.55rem; border-radius: 6px; background: rgba(0,0,0,0.45); color: var(--qualia-text-muted); font-size: 0.65rem; backdrop-filter: blur(8px);",
                    "{page.panes.len()} HUD panes"
                }
            }

            div {
                style: "position: absolute; inset: 0; pointer-events: none; z-index: 15;",
                for (idx, pane) in page.panes.iter().enumerate() {
                    div {
                        key: "{idx}",
                        style: "position: absolute; left: {pane.x * 6}px; top: {pane.y * 6}px; background: color-mix(in srgb, var(--qualia-surface) 88%, transparent); border: 1px solid var(--qualia-accent); color: var(--qualia-text); padding: 0.45rem 0.55rem; font-size: 0.72rem; border-radius: 6px; pointer-events: auto; backdrop-filter: blur(10px); box-shadow: 0 8px 24px rgba(0,0,0,0.25);",
                        "{pane.component_id}"
                    }
                }
            }
        }
    }
}