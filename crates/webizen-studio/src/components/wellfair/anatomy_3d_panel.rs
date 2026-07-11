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
use dioxus::document::eval;

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
            style: "
                position: relative;
                padding: 1.5rem;
                border: 1px solid var(--qualia-border, rgba(255, 255, 255, 0.1));
                border-radius: 16px;
                background: var(--qualia-surface, rgba(10, 15, 30, 0.6));
                backdrop-filter: blur(24px);
                -webkit-backdrop-filter: blur(24px);
                box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.1);
                color: var(--qualia-text, #fff);
                margin-top: 1rem;
                overflow: hidden;
            ",
            // Subtle animated gradient background behind the glass
            div {
                style: "
                    position: absolute;
                    inset: -50%;
                    background: radial-gradient(circle at 50% 50%, rgba(42, 111, 151, 0.15), transparent 60%);
                    animation: pulse-bg 8s ease-in-out infinite alternate;
                    pointer-events: none;
                    z-index: -1;
                "
            }
            // CSS for the background pulse
            style {
                "@keyframes pulse-bg {{ 0% {{ transform: scale(1); opacity: 0.8; }} 100% {{ transform: scale(1.1); opacity: 1; }} }}"
                ".anatomy-btn {{ transition: all 0.2s; }}"
                ".anatomy-btn:hover {{ background: rgba(255, 255, 255, 0.15) !important; }}"
            }

            div {
                style: "display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1.2rem;",
                div {
                    h2 { 
                        style: "margin: 0 0 0.4rem; font-size: 1.4rem; font-weight: 600; letter-spacing: -0.02em; text-shadow: 0 2px 4px rgba(0,0,0,0.5);", 
                        "Your Physical State" 
                    }
                    p {
                        style: "margin: 0; font-size: 0.85rem; color: var(--qualia-text-muted, #a0aec0); max-width: 500px; line-height: 1.5;",
                        "A holistic structural projection. Regions pulse indicating accumulated physiological load."
                    }
                }
            }

            if !native {
                div {
                    style: "margin: 1rem 0; padding: 1rem; background: rgba(229, 62, 62, 0.1); border: 1px solid rgba(229, 62, 62, 0.3); border-radius: 12px; font-size: 0.9rem; color: #fc8181; display: flex; align-items: center; gap: 0.75rem;",
                    span { style: "font-size: 1.2rem;", "⚠️" }
                    span { "The 3D body view requires the native Webizen desktop engine for GPU acceleration." }
                }
            } else {
                // ── The rendered body (real-mesh portal OR interim PNG) ───────────────────────
                div {
                    style: "
                        position: relative;
                        width: 100%;
                        height: 500px;
                        background: radial-gradient(circle at center, #111827, #030712);
                        border: 1px solid rgba(255, 255, 255, 0.05);
                        border-radius: 12px;
                        overflow: hidden;
                        margin-bottom: 1.2rem;
                        box-shadow: inset 0 2px 10px rgba(0,0,0,0.8);
                    ",
                    if real_mesh_ready {
                        div {
                            id: "anatomy-portal-webview",
                            style: "position: absolute; inset: 0; width: 100%; height: 100%;",
                            onmounted: move |_| {
                                let url = portal_src.clone();
                                spawn(async move {
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                    let script = format!(r#"
                                        const container = document.getElementById('anatomy-portal-webview');
                                        if (container && window.__TAURI__) {{
                                            const invoke = window.__TAURI__.core.invoke;
                                            const r = container.getBoundingClientRect();
                                            const id = 'anatomy-portal';
                                            
                                            invoke('spawn_native_webview', {{
                                                id: id, url: '{}', x: r.left, y: r.top, width: r.width, height: r.height
                                            }}).then(() => {{
                                                invoke('show_native_webview', {{ id }});
                                                invoke('resize_native_webview', {{ id, x: r.left, y: r.top, width: r.width, height: r.height }});
                                            }}).catch(console.error);

                                            if (!window.anatomyWebviewObserver) {{
                                                window.anatomyWebviewObserver = new ResizeObserver(() => {{
                                                    const r2 = container.getBoundingClientRect();
                                                    invoke('resize_native_webview', {{ 
                                                        id: 'anatomy-portal', 
                                                        x: r2.left, y: r2.top, width: r2.width, height: r2.height 
                                                    }}).catch(console.error);
                                                }});
                                                window.anatomyWebviewObserver.observe(container);
                                            }}
                                        }}
                                    "#, url);
                                    let _ = eval(&script);
                                });
                            }
                        }
                    } else if has_frame {
                        img {
                            src: "{frame_src}",
                            alt: "Whole-body 3D anatomy snapshot, coloured by accumulated burden",
                            style: "position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; display: block; mix-blend-mode: screen;",
                        }
                    } else {
                        div {
                            style: "position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--qualia-accent, #63b3ed); font-size: 0.95rem; text-transform: uppercase; letter-spacing: 2px;",
                            div {
                                style: "width: 40px; height: 40px; border: 3px solid rgba(99, 179, 237, 0.2); border-top-color: #63b3ed; border-radius: 50%; animation: spin 1s linear infinite; margin-bottom: 1rem;"
                            }
                            "{state.status}"
                        }
                    }

                    // Floating Glass Overlay Controls
                    div {
                        style: "
                            position: absolute;
                            bottom: 1rem;
                            left: 1rem;
                            right: 1rem;
                            display: flex;
                            justify-content: space-between;
                            align-items: flex-end;
                            pointer-events: none;
                        ",
                        // Left: Orbit Controls
                        div {
                            style: "
                                pointer-events: auto;
                                background: rgba(10, 15, 30, 0.7);
                                backdrop-filter: blur(16px);
                                -webkit-backdrop-filter: blur(16px);
                                padding: 1rem;
                                border-radius: 12px;
                                border: 1px solid rgba(255, 255, 255, 0.08);
                                display: flex;
                                flex-direction: column;
                                gap: 0.8rem;
                                min-width: 200px;
                            ",
                            div {
                                style: "display: flex; align-items: center; justify-content: space-between; gap: 1rem;",
                                label { style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 1px; color: #a0aec0;", "Azimuth" }
                                span { style: "font-family: monospace; font-size: 0.8rem; color: #e2e8f0;", "{state.azimuth:.0}°" }
                            }
                            input {
                                r#type: "range", min: "0", max: "360", step: "5", value: "{state.azimuth}",
                                oninput: move |evt| { ui.write().azimuth = evt.value().parse().unwrap_or(0.0); },
                                onchange: move |_| { if !real_mesh_ready { spawn(render_frame(ui)); } },
                                style: "accent-color: var(--qualia-accent, #63b3ed); cursor: pointer;"
                            }
                            div {
                                style: "display: flex; align-items: center; justify-content: space-between; gap: 1rem;",
                                label { style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 1px; color: #a0aec0;", "Elevation" }
                                span { style: "font-family: monospace; font-size: 0.8rem; color: #e2e8f0;", "{state.elevation:.0}°" }
                            }
                            input {
                                r#type: "range", min: "-60", max: "60", step: "5", value: "{state.elevation}",
                                oninput: move |evt| { ui.write().elevation = evt.value().parse().unwrap_or(0.0); },
                                onchange: move |_| { if !real_mesh_ready { spawn(render_frame(ui)); } },
                                style: "accent-color: var(--qualia-accent, #63b3ed); cursor: pointer;"
                            }
                        }

                        // Right: Asset Cache Controls
                        div {
                            style: "
                                pointer-events: auto;
                                background: rgba(10, 15, 30, 0.7);
                                backdrop-filter: blur(16px);
                                -webkit-backdrop-filter: blur(16px);
                                padding: 1rem;
                                border-radius: 12px;
                                border: 1px solid rgba(255, 255, 255, 0.08);
                                display: flex;
                                flex-direction: column;
                                align-items: flex-end;
                                gap: 0.8rem;
                            ",
                            div {
                                style: "display: flex; align-items: center; gap: 0.75rem;",
                                select {
                                    style: "
                                        background: rgba(255, 255, 255, 0.1);
                                        border: 1px solid rgba(255, 255, 255, 0.2);
                                        color: #fff;
                                        padding: 0.4rem 0.8rem;
                                        border-radius: 8px;
                                        font-size: 0.8rem;
                                        outline: none;
                                        cursor: pointer;
                                    ",
                                    value: "{state.model}",
                                    onchange: move |evt| {
                                        ui.write().model = evt.value();
                                        ui.write().cache_status = None;
                                        ui.write().cache_checked = false;
                                        spawn(refresh_cache_status(ui));
                                    },
                                    option { value: "male", style: "background: #111;", "XY Form" }
                                    option { value: "female", style: "background: #111;", "XX Form" }
                                }
                                if let Some(c) = cache {
                                    div {
                                        style: "font-size: 0.75rem; padding: 0.4rem 0.8rem; border-radius: 8px; background: rgba(255,255,255,0.05); color: #cbd5e1;",
                                        if c.cached {
                                            "Loaded: {c.organ_count} organs · {c.total_ten_d_bytes / 1_000_000} MB"
                                        } else {
                                            "No local meshes"
                                        }
                                    }
                                }
                            }
                            
                            div {
                                style: "display: flex; gap: 0.5rem;",
                                button {
                                    style: "
                                        padding: 0.5rem 1rem;
                                        border: none;
                                        border-radius: 8px;
                                        background: var(--qualia-accent, #3182ce);
                                        color: #fff;
                                        font-size: 0.8rem;
                                        font-weight: 600;
                                        cursor: pointer;
                                        transition: all 0.2s;
                                        animation: glow-btn 2s infinite alternate;
                                    ",
                                    disabled: state.acquiring,
                                    onclick: move |_| { spawn(acquire_assets(ui)); },
                                    if state.acquiring { "Acquiring..." } else if cache.map(|c| c.cached).unwrap_or(false) { "Update Meshes" } else { "Download HD Meshes" }
                                }
                                if cache.map(|c| c.cached).unwrap_or(false) {
                                    button {
                                        style: "
                                            padding: 0.5rem 1rem;
                                            border: 1px solid rgba(255, 255, 255, 0.2);
                                            border-radius: 8px;
                                            background: transparent;
                                            color: #e2e8f0;
                                            font-size: 0.8rem;
                                            cursor: pointer;
                                            transition: all 0.2s;
                                        ",
                                        disabled: state.acquiring,
                                        onclick: move |_| {
                                            spawn(async move {
                                                let model = ui.read().model.clone();
                                                let _ = super::host_client::clear_body_cache(&model).await;
                                                ui.write().cache_status = None;
                                                spawn(refresh_cache_status(ui));
                                            });
                                        },
                                        "Purge Cache"
                                    }
                                }
                            }
                            if !state.acquire_message.is_empty() {
                                div { style: "font-size: 0.75rem; color: #90cdf4; max-width: 250px; text-align: right;", "{state.acquire_message}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
