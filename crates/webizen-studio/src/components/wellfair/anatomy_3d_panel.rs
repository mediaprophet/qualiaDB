//! S5.7 + S5.8 — the **3D Anatomy render surface**.
//!
//! Two views, switched by what's available:
//!
//! 1. **Interim visual** (S5.7) — a whole-body percept snapshot rendered headlessly by the desktop GPU
//!    (`webizen_render`), coloured by accumulated burden (σ → RGBA), served as a PNG at
//!    `webizen://localhost/anatomy/body.png`. Shown when the real-mesh asset cache is **not** present.
//! 2. **Real-mesh view** (S5.8) — when the person has triggered the asset download (CCF/HRA GLBs →
//!    compiled `.10d` cached under `{storage_root}/assets/ccf/{model}/`), the panel switches to the
//!    browser portal's WebGPU canvas (`load_10d_colored` per organ) for the live 3D body.
//!
//! The orbit camera (azimuth / elevation sliders) drives both views. Everything shown is a **hypothesis
//! to explore, not a diagnosis**; the text surface (`anatomy_panel.rs`) carries the narrative + disclosure.

use super::host_client::{
    acquire_body_assets, body_assets_status, render_body_snapshot, BodyAssetsStatus,
};
use dioxus::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
struct Anatomy3dUi {
    /// Camera azimuth in degrees (0..360).
    azimuth: f64,
    /// Camera elevation in degrees (-90..90).
    elevation: f64,
    /// Bumped each time a new frame is rendered — cache-busts the PNG URL.
    epoch: u64,
    /// Status line for the interim render.
    status: String,
    /// Whether a render is in flight.
    rendering: bool,
    /// Whether the initial frame has been requested.
    started: bool,
    // --- Asset cache (S5.8) ---
    /// The model the person wants ("male" / "female"). Default "male".
    model: String,
    /// The current cache status (None = not yet checked).
    cache_status: Option<BodyAssetsStatus>,
    /// Whether an acquisition is in flight.
    acquiring: bool,
    /// The latest acquisition progress line.
    acquire_message: String,
    /// Whether the asset-cache status has been loaded.
    cache_checked: bool,
}

async fn render_frame(mut ui: Signal<Anatomy3dUi>) {
    let az = ui.read().azimuth;
    let el = ui.read().elevation;
    ui.write().rendering = true;
    ui.write().status = "Rendering body snapshot…".to_string();
    match render_body_snapshot(az, el).await {
        Ok(_) => {
            ui.write().epoch += 1;
            ui.write().status.clear();
        }
        Err(e) => ui.write().status = format!("Couldn't render the body: {e}"),
    }
    ui.write().rendering = false;
}

/// Refresh the cache status from the host.
async fn refresh_cache_status(mut ui: Signal<Anatomy3dUi>) {
    let model = ui.read().model.clone();
    match body_assets_status(&model).await {
        Ok(s) => ui.write().cache_status = Some(s),
        Err(e) => ui.write().acquire_message = format!("Couldn't check cache: {e}"),
    }
    ui.write().cache_checked = true;
}

/// Trigger the asset download + compile + cache.
async fn acquire_assets(mut ui: Signal<Anatomy3dUi>) {
    let model = ui.read().model.clone();
    ui.write().acquiring = true;
    ui.write().acquire_message = "Starting download…".to_string();
    match acquire_body_assets(&model).await {
        Ok(report) => {
            ui.write().acquire_message = format!(
                "{} body cached: {} organs · {} MB · {} failed",
                report.model,
                report.organs_cached,
                report.total_ten_d_bytes / 1_000_000,
                report.organs_failed,
            );
            // Refresh the cache status to reflect the new state.
            spawn(refresh_cache_status(ui));
        }
        Err(e) => ui.write().acquire_message = format!("Download failed: {e}"),
    }
    ui.write().acquiring = false;
}

#[component]
pub fn WellfairAnatomy3dPanel() -> Element {
    let mut ui = use_signal(|| Anatomy3dUi {
        elevation: 10.0,
        model: "male".to_string(),
        ..Default::default()
    });

    // Render the initial frame + check the cache status on mount (native host only).
    use_effect(move || {
        if !ui.read().started && crate::endpoints::is_native_host() {
            ui.write().started = true;
            spawn(render_frame(ui));
            spawn(refresh_cache_status(ui));
        }
    });

    // When the cache becomes ready, postMessage the portal iframe to load the real body.
    // The iframe needs a moment to load its content; retry for a few seconds.
    #[cfg(target_arch = "wasm32")]
    {
        let mut was_ready = use_signal(|| false);
        use_effect(move || {
            let ready = ui.read().cache_status.as_ref().map(|c| c.cached).unwrap_or(false)
                && crate::endpoints::is_native_host();
            if ready && !was_ready() {
                was_ready.set(true);
                let model = ui.read().model.clone();
                spawn(async move {
                    // Retry postMessage for up to ~10s — the iframe may still be booting the portal WASM.
                    for _ in 0..20 {
                        let js = format!(
                            r#"(function() {{
                                var f = document.getElementById('anatomy-portal-iframe');
                                if (f && f.contentWindow) {{
                                    f.contentWindow.postMessage({{ type: 'anatomy-load-body', model: '{model}' }}, '*');
                                    return true;
                                }}
                                return false;
                            }})()"#
                        );
                        if let Ok(v) = js_sys::eval(&js) {
                            if v.as_bool() == Some(true) { break; }
                        }
                        gloo_timers::future::TimeoutFuture::new(500).await;
                    }
                });
            } else if !ready && was_ready() {
                was_ready.set(false);
            }
        });
    }

    let state = ui();
    let native = crate::endpoints::is_native_host();
    let has_frame = native && state.epoch > 0;
    let frame_src = format!("webizen://localhost/anatomy/body.png?t={}", state.epoch);

    // The real-mesh portal view is shown when the cache is complete.
    let cache = state.cache_status.as_ref();
    let real_mesh_ready = native && cache.map(|c| c.cached).unwrap_or(false);
    let portal_src = crate::endpoints::portal_design_studio_url();

    rsx! {
        section {
            aria_label: "3D Anatomy body view",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.85rem;",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Your body in 3D" }
            p {
                style: "margin:0 0 0.6rem;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "A whole-body picture coloured by how the things you've logged seem to add up across your body systems. Each region is a body system; a bigger, redder, pulsing region means more accumulated strain. This is a general guide to explore with a clinician, not a diagnosis."
            }

            if !native {
                p {
                    style: "margin:0 0 0.5rem;padding:0.5rem 0.65rem;background:#2a6f9711;border:1px solid #2a6f9733;border-radius:8px;font-size:0.8rem;",
                    "The 3D body view requires the Webizen desktop host (it uses the desktop GPU to render)."
                }
            } else {
                // ── Asset cache controls (S5.8) ──────────────────────────────────────────────
                div {
                    style: "padding:0.55rem 0.65rem;background:#f6f8fa;border:1px solid var(--qualia-border,#e0e0e0);border-radius:8px;margin-bottom:0.6rem;",
                    div {
                        style: "display:flex;align-items:center;gap:0.5rem;flex-wrap:wrap;margin-bottom:0.35rem;",
                        label {
                            for: "anatomy-model",
                            style: "font-size:0.82rem;",
                            "Body model:"
                        }
                        select {
                            id: "anatomy-model",
                            style: "padding:0.2rem 0.4rem;border:1px solid var(--qualia-border,#ccc);border-radius:6px;font-size:0.8rem;",
                            value: "{state.model}",
                            onchange: move |evt| {
                                ui.write().model = evt.value();
                                ui.write().cache_status = None;
                                ui.write().cache_checked = false;
                                spawn(refresh_cache_status(ui));
                            },
                            option { value: "male", "Male (XY)" }
                            option { value: "female", "Female (XX)" }
                        }
                        if let Some(c) = cache {
                            span {
                                style: "font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                                if c.cached {
                                    "Cached: {c.organ_count} organs · {c.total_ten_d_bytes / 1_000_000} MB"
                                } else {
                                    "Not cached"
                                }
                            }
                        } else if state.cache_checked {
                            span { style: "font-size:0.78rem;color:var(--qualia-text-muted,#666);", "Checking…" }
                        }
                    }
                    div {
                        style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                        button {
                            r#type: "button",
                            style: "padding:0.3rem 0.6rem;border:1px solid var(--qualia-border,#ccc);border-radius:6px;background:#fff;cursor:pointer;font-size:0.78rem;",
                            disabled: state.acquiring,
                            onclick: move |_| {
                                spawn(acquire_assets(ui));
                            },
                            if state.acquiring { "Downloading…" } else if cache.map(|c| c.cached).unwrap_or(false) { "Re-download" } else { "Download body assets" }
                        }
                        if cache.map(|c| c.cached).unwrap_or(false) {
                            button {
                                r#type: "button",
                                style: "padding:0.3rem 0.6rem;border:1px solid var(--qualia-border,#ccc);border-radius:6px;background:#f6f6f6;cursor:pointer;font-size:0.78rem;",
                                disabled: state.acquiring,
                                onclick: move |_| {
                                    spawn(async move {
                                        let model = ui.read().model.clone();
                                        let _ = super::host_client::clear_body_cache(&model).await;
                                        ui.write().cache_status = None;
                                        spawn(refresh_cache_status(ui));
                                    });
                                },
                                "Clear cache"
                            }
                        }
                    }
                    if !state.acquire_message.is_empty() {
                        p {
                            style: "margin:0.35rem 0 0;font-size:0.76rem;color:var(--qualia-text-muted,#555);line-height:1.4;",
                            "{state.acquire_message}"
                        }
                    }
                    p {
                        style: "margin:0.3rem 0 0;font-size:0.72rem;color:var(--qualia-text-muted,#888);line-height:1.4;",
                        "Downloads the Human Reference Atlas reference-organ set (~200–290 MB) from the live HRA endpoint and caches it on your machine. You only do this once per model; subsequent runs load the cached body instantly."
                    }
                }

                // ── The rendered body (real-mesh portal OR interim PNG) ───────────────────────
                div {
                    style: "position:relative;width:100%;min-height:360px;background:#0a0f14;border:1px solid var(--qualia-border,#333);border-radius:8px;overflow:hidden;margin-bottom:0.6rem;",
                    if real_mesh_ready {
                        iframe {
                            id: "anatomy-portal-iframe",
                            src: "{portal_src}",
                            title: "3D Anatomy — real organ meshes (WebGPU portal)",
                            style: "position:absolute;inset:0;width:100%;height:100%;border:0;display:block;",
                        }
                    } else if has_frame {
                        img {
                            src: "{frame_src}",
                            alt: "Whole-body 3D anatomy snapshot, coloured by accumulated burden",
                            style: "position:absolute;inset:0;width:100%;height:100%;object-fit:contain;display:block;",
                        }
                    } else {
                        div {
                            style: "position:absolute;inset:0;display:flex;align-items:center;justify-content:center;color:var(--qualia-text-muted,#888);font-size:0.85rem;",
                            "{state.status}"
                        }
                    }
                }

                // ── Orbit camera controls (drive both views) ─────────────────────────────────
                div {
                    role: "group",
                    aria_label: "Orbit camera",
                    style: "display:flex;flex-direction:column;gap:0.5rem;margin-bottom:0.6rem;",
                    div {
                        style: "display:flex;align-items:center;gap:0.5rem;",
                        label {
                            for: "anatomy-azimuth",
                            style: "font-size:0.82rem;flex:0 0 auto;",
                            "Rotate:"
                        }
                        input {
                            id: "anatomy-azimuth",
                            r#type: "range",
                            min: "0",
                            max: "360",
                            step: "5",
                            value: "{state.azimuth}",
                            style: "flex:1 1 auto;",
                            oninput: move |evt| {
                                ui.write().azimuth = evt.value().parse().unwrap_or(0.0);
                            },
                            onchange: move |_| {
                                if !real_mesh_ready { spawn(render_frame(ui)); }
                            },
                        }
                        span {
                            style: "font-size:0.78rem;color:var(--qualia-text-muted,#666);flex:0 0 auto;width:3.5rem;text-align:right;",
                            "{state.azimuth:.0}°"
                        }
                    }
                    div {
                        style: "display:flex;align-items:center;gap:0.5rem;",
                        label {
                            for: "anatomy-elevation",
                            style: "font-size:0.82rem;flex:0 0 auto;",
                            "Tilt:"
                        }
                        input {
                            id: "anatomy-elevation",
                            r#type: "range",
                            min: "-60",
                            max: "60",
                            step: "5",
                            value: "{state.elevation}",
                            style: "flex:1 1 auto;",
                            oninput: move |evt| {
                                ui.write().elevation = evt.value().parse().unwrap_or(0.0);
                            },
                            onchange: move |_| {
                                if !real_mesh_ready { spawn(render_frame(ui)); }
                            },
                        }
                        span {
                            style: "font-size:0.78rem;color:var(--qualia-text-muted,#666);flex:0 0 auto;width:3.5rem;text-align:right;",
                            "{state.elevation:.0}°"
                        }
                    }
                    if !real_mesh_ready {
                        div {
                            style: "display:flex;gap:0.4rem;",
                            button {
                                r#type: "button",
                                style: "padding:0.3rem 0.6rem;border:1px solid var(--qualia-border,#ccc);border-radius:6px;background:#fff;cursor:pointer;font-size:0.78rem;",
                                disabled: state.rendering,
                                onclick: move |_| {
                                    spawn(render_frame(ui));
                                },
                                if state.rendering { "Rendering…" } else { "Re-render" }
                            }
                            button {
                                r#type: "button",
                                style: "padding:0.3rem 0.6rem;border:1px solid var(--qualia-border,#ccc);border-radius:6px;background:#f6f6f6;cursor:pointer;font-size:0.78rem;",
                                onclick: move |_| {
                                    ui.write().azimuth = 0.0;
                                    ui.write().elevation = 10.0;
                                    if !real_mesh_ready { spawn(render_frame(ui)); }
                                },
                                "Reset view"
                            }
                        }
                    }
                }

                if !state.status.is_empty() && !state.rendering {
                    p { style: "margin:0 0 0.4rem;font-size:0.8rem;", "{state.status}" }
                }

                p {
                    style: "margin:0;font-size:0.72rem;color:var(--qualia-text-muted,#888);line-height:1.4;",
                    if real_mesh_ready {
                        "Showing the real 3D organ meshes (Human Reference Atlas) in the WebGPU portal. Set your physiological state on the text view to see the body at your current life stage."
                    } else {
                        "Interim visual: body systems as coloured regions on a silhouette. Download the body assets above to see the real 3D organ meshes. Set your physiological state on the text view to see the body at your current life stage."
                    }
                }
            }
        }
    }
}
