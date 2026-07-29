//! **Chora** — the spatio-temporal commons canvas explorer.
//!
//! Pan (space), scrub (time), zoom (scale) over permissive-commons layers.
//! Worlds are configurations, not engine forks (doc 02 §2).

use super::chora_host_client::{
    canvas_navigation, download_layer, list_canvas_worlds, list_layers, query_canvas_region,
    seed_canvas_demo, set_active_canvas_world, set_canvas_temporal, set_gpu_camera_mode,
};
use crate::components::experience_mode::use_experience_mode;
use dioxus::prelude::*;

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn layer_source(v: &serde_json::Value) -> String {
    let Some(source) = v.get("source") else {
        return "Layer source".to_string();
    };
    let Some(source) = source.as_object() else {
        return source.as_str().unwrap_or("Layer source").to_string();
    };
    match source.keys().next().map(String::as_str) {
        Some("NasaGibs") => "NASA GIBS".to_string(),
        Some("NasaHorizons") => "NASA Horizons".to_string(),
        Some("HipparcosCatalog") => "ESA Hipparcos".to_string(),
        Some("YaleBrightStars") => "Yale Bright Star Catalog".to_string(),
        Some("UsgsAstrogeology") => "NASA / USGS".to_string(),
        Some(name) => name.to_string(),
        None => "Layer source".to_string(),
    }
}

#[component]
pub fn WellfairChoraPanel() -> Element {
    let experience_mode = use_experience_mode();
    let advanced = experience_mode().is_advanced();
    let mut worlds = use_signal(Vec::<serde_json::Value>::new);
    let mut nav = use_signal(|| serde_json::json!({}));
    let mut region_hits = use_signal(Vec::<serde_json::Value>::new);
    let mut status = use_signal(String::new);
    let mut temporal_t = use_signal(|| 1_750_000_000f64);
    let mut layers = use_signal(Vec::<serde_json::Value>::new);
    let mut downloading = use_signal(|| Option::<String>::None);
    let mut camera_mode = use_signal(|| "space".to_string());

    let refresh = move || {
        spawn(async move {
            if let Ok(w) = list_canvas_worlds().await {
                worlds.set(w);
            }
            if let Ok(n) = canvas_navigation().await {
                if let Some(t) = n.get("temporalT").and_then(|x| x.as_f64()) {
                    temporal_t.set(t);
                }
                nav.set(n);
            }
            if let Ok(l) = list_layers().await {
                layers.set(l);
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        // Auto-seed flagships when store empty so explorer is not a void.
        spawn(async move {
            let _ = super::chora_host_client::seed_canvas_demo().await;
            // Prefer flagships if the host exposes the command (desktop).
            let _ = crate::components::qapp_engine::invoke_json(
                "chora_seed_flagships",
                serde_json::json!({}),
            )
            .await;
            refresh();
        });
    });

    rsx! {
        section {
            class: "wellfair-chora-panel",
            style: "
                display: flex;
                flex-direction: column;
                gap: 1.5rem;
                padding: 2rem;
                border: 1px solid var(--qualia-border, rgba(43, 108, 176, 0.15));
                border-radius: 16px;
                background: linear-gradient(135deg, color-mix(in srgb, var(--qualia-bg) 96%, #15203b), color-mix(in srgb, var(--qualia-surface) 94%, #111827));
                color: var(--qualia-text);
                min-height: 500px;
                font-family: 'Inter', -apple-system, sans-serif;
                box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3), inset 0 1px 1px rgba(255, 255, 255, 0.1);
                position: relative;
                overflow: hidden;
            ",

            // Background cosmic/grid styling
            div {
                style: "position: absolute; top: 0; left: 0; right: 0; bottom: 0; opacity: 0.05; background-image: radial-gradient(circle at 2px 2px, white 1px, transparent 0); background-size: 30px 30px; pointer-events: none;"
            }
            div {
                style: "position: absolute; top: -20%; right: -10%; width: 50%; height: 60%; background: radial-gradient(circle, rgba(99, 102, 241, 0.15) 0%, transparent 70%); border-radius: 50%; filter: blur(40px); pointer-events: none;"
            }

            header {
                style: "z-index: 1; border-bottom: 1px solid rgba(255, 255, 255, 0.1); padding-bottom: 1.5rem; display: flex; justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 1rem;",
                div {
                    div {
                        style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;margin-bottom:0.35rem;",
                        span {
                            style: "font-size:0.62rem;font-weight:800;letter-spacing:0.06em;text-transform:uppercase;color:#67e8f9;",
                            if advanced { "Spatio-temporal world" } else { "Place through time" }
                        }
                        span {
                            style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid rgba(34,211,238,0.45);background:rgba(34,211,238,0.12);color:#a5f3fc;font-weight:700;",
                            if advanced { "Life domain" } else { "Shared place" }
                        }
                        span {
                            style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid rgba(148,163,184,0.35);color:#cbd5e1;font-weight:600;",
                            "Commons · attributed"
                        }
                    }
                    h2 {
                        style: "margin: 0; font-size: 1.8rem; font-weight: 600; color: #fff; letter-spacing: -0.02em; display: flex; align-items: center; gap: 0.5rem;",
                        if advanced { "Universe · Chora Commons" } else { "Places through time" }
                    }
                    p {
                        style: "margin: 0.5rem 0 0; font-size: 0.9rem; color: #94a3b8; max-width: 600px; line-height: 1.5;",
                        if advanced {
                            "Explore the star-scape, Earth and planetary commons. NASA imagery and public star catalogs stay visibly attributed; worlds are configurations, not engine forks."
                        } else {
                            "Move through shared places and moments. Every public layer keeps its source and licence visible, and nothing here changes your private records."
                        }
                    }
                    p { style: "margin: 0.55rem 0 0;",
                        Link {
                            to: crate::Route::LibraryRoute {},
                            style: "font-size:0.72rem;font-weight:700;padding:0.28rem 0.65rem;border-radius:999px;border:1px solid #6d28d9;background:rgba(139,92,246,0.18);color:#e9d5ff;text-decoration:none;",
                            title: "Keep loci and notes by meaning in Lived Memory",
                            "→ Lived Memory"
                        }
                        " "
                        Link {
                            to: crate::Route::BrowserRoute {},
                            style: "font-size:0.72rem;font-weight:700;padding:0.28rem 0.65rem;border-radius:999px;border:1px solid rgba(148,163,184,0.35);background:rgba(15,23,42,0.6);color:#e2e8f0;text-decoration:none;margin-left:0.35rem;",
                            title: "Web browser — project pages into the same entity session",
                            "→ Browser"
                        }
                    }
                }
                div {
                    style: "display: flex; gap: 0.75rem; flex-wrap: wrap;",
                    button {
                        style: "padding: 0.6rem 1rem; border-radius: 8px; border: 1px solid rgba(148, 163, 184, 0.3); background: rgba(255, 255, 255, 0.05); color: #e2e8f0; font-size: 0.85rem; font-weight: 500; cursor: pointer; transition: all 0.2s; display: flex; align-items: center; gap: 0.5rem;",
                        onclick: move |_| {
                            spawn(async move {
                                match seed_canvas_demo().await {
                                    Ok(true) => status.set("Demo world seeded.".into()),
                                    Ok(false) => status.set("Worlds already exist.".into()),
                                    Err(e) => status.set(format!("Seed failed: {e}")),
                                }
                                if let Ok(w) = list_canvas_worlds().await {
                                    worlds.set(w);
                                }
                            });
                        },
                        span { style: "font-size: 1.1rem;", "🌱" }
                        if advanced { "Seed demo world" } else { "Add example places" }
                    }
                    button {
                        style: "padding: 0.6rem 1rem; border-radius: 8px; border: 1px solid rgba(99, 102, 241, 0.4); background: rgba(99, 102, 241, 0.1); color: #818cf8; font-size: 0.85rem; font-weight: 500; cursor: pointer; transition: all 0.2s; display: flex; align-items: center; gap: 0.5rem;",
                        onclick: move |_| refresh(),
                        span { style: "font-size: 1.1rem;", "↻" } "Refresh"
                    }
                }
            }

            if !status().is_empty() {
                div {
                    role: "status",
                    style: "z-index: 1; padding: 0.75rem 1rem; background: rgba(56, 189, 248, 0.1); border-left: 3px solid #38bdf8; border-radius: 0 6px 6px 0; font-size: 0.85rem; color: #bae6fd; display: inline-flex; align-items: center; gap: 0.5rem;",
                    span { style: "font-weight: bold;", "INFO:" } "{status}"
                }
            }

            div {
                style: "z-index: 1; position: relative; height: min(54vh, 560px); min-height: 360px; overflow: hidden; border: 1px solid rgba(129, 140, 248, 0.32); border-radius: 16px; background: #020617; box-shadow: 0 22px 55px rgba(0, 0, 0, 0.42);",
                iframe {
                    src: "/chora-universe.html",
                    title: "Interactive Universe star-scape",
                    style: "position: absolute; inset: 0; width: 100%; height: 100%; border: 0; background: #020617;",
                }
                div {
                    style: "position: absolute; right: 12px; bottom: 12px; display: flex; gap: 6px; padding: 5px; border: 1px solid rgba(148,163,184,0.24); border-radius: 10px; background: rgba(2,6,23,0.82); backdrop-filter: blur(12px);",
                    for mode in ["earth", "space", "free"] {
                        button {
                            r#type: "button",
                            style: if camera_mode() == mode {
                                "border:1px solid #818cf8;background:rgba(99,102,241,0.28);color:#eef2ff;border-radius:7px;padding:0.38rem 0.62rem;cursor:pointer;font-size:0.72rem;font-weight:700;text-transform:capitalize;"
                            } else {
                                "border:1px solid transparent;background:transparent;color:#94a3b8;border-radius:7px;padding:0.38rem 0.62rem;cursor:pointer;font-size:0.72rem;font-weight:650;text-transform:capitalize;"
                            },
                            onclick: move |_| {
                                camera_mode.set(mode.to_string());
                                spawn(async move {
                                    if let Err(e) = set_gpu_camera_mode(mode).await {
                                        status.set(format!("Camera mode unavailable: {e}"));
                                    }
                                });
                            },
                            "{mode}"
                        }
                    }
                }
            }

            div {
                style: "z-index: 1; display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem;",

                // Navigation Panel
                div {
                    style: "padding: 1.5rem; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; backdrop-filter: blur(10px); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",
                        strong { style: "font-size: 1rem; color: var(--qualia-text); font-weight: 650;",
                            if advanced { "Temporal coordinate" } else { "Time" }
                        }
                        div {
                            style: "padding: 0.25rem 0.75rem; background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.2); border-radius: 20px; font-size: 0.75rem; color: #34d399; font-family: monospace;",
                            if advanced { "ACTIVE: {str_field(&nav(), \"activeWorldId\")}" } else { "Current place" }
                        }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 1rem;",
                        div {
                            style: "display: flex; align-items: center; gap: 1rem;",
                            input {
                                r#type: "range",
                                min: "1700000000",
                                max: "1900000000",
                                step: "86400",
                                value: "{temporal_t()}",
                                style: "flex: 1; accent-color: #6366f1; cursor: pointer;",
                                oninput: move |evt| {
                                    if let Ok(v) = evt.value().parse::<f64>() {
                                        temporal_t.set(v);
                                    }
                                },
                                onchange: move |_| {
                                    let t = temporal_t();
                                    spawn(async move {
                                        let _ = set_canvas_temporal(t).await;
                                        if let Ok(n) = canvas_navigation().await {
                                            nav.set(n);
                                        }
                                    });
                                },
                            }
                        }
                        div {
                            style: "display: flex; justify-content: space-between; align-items: center;",
                            span { style: "font-size: 0.75rem; color: #94a3b8;",
                                if advanced { "UNIX EPOCH" } else { "Selected moment" }
                            }
                            span { style: "font-size: 1.25rem; font-family: monospace; color: #818cf8; font-weight: 600;", "{temporal_t():.0}" }
                        }
                    }
                }

                // Region Query Panel
                div {
                    style: "padding: 1.5rem; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; backdrop-filter: blur(10px); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",
                        strong { style: "font-size: 1rem; color: var(--qualia-text); font-weight: 650;",
                            if advanced { "Region telemetry (Sydney)" } else { "Material near Sydney" }
                        }
                        button {
                            style: "padding: 0.35rem 0.75rem; border-radius: 6px; border: 1px solid rgba(56, 189, 248, 0.4); background: rgba(56, 189, 248, 0.1); color: #38bdf8; font-size: 0.75rem; font-weight: 500; cursor: pointer; transition: all 0.2s;",
                            onclick: move |_| {
                                spawn(async move {
                                    match query_canvas_region(151.0, -34.0, 152.0, -33.0).await {
                                        Ok(hits) => {
                                            status.set(format!("{} assets in region", hits.len()));
                                            region_hits.set(hits);
                                        }
                                        Err(e) => status.set(e),
                                    }
                                });
                            },
                            if advanced { "Execute scan" } else { "Look here" }
                        }
                    }
                    if region_hits().is_empty() {
                        div {
                            style: "height: 60px; display: flex; align-items: center; justify-content: center; border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: 8px; color: #64748b; font-size: 0.85rem;",
                            if advanced { "No targets acquired in quadrant." } else { "No shared material found here yet." }
                        }
                    } else {
                        ul {
                            style: "list-style: none; padding: 0; margin: 0; max-height: 120px; overflow-y: auto; display: flex; flex-direction: column; gap: 0.5rem;",
                            for hit in region_hits().iter().cloned() {
                                li {
                                    style: "padding: 0.5rem; background: rgba(0, 0, 0, 0.2); border-radius: 6px; font-size: 0.8rem; font-family: monospace; display: flex; justify-content: space-between; align-items: center; border-left: 2px solid #38bdf8;",
                                    span { style: "color: #cbd5e0;", "{str_field(&hit, \"assetId\")}" }
                                    div {
                                        style: "display: flex; gap: 0.75rem; align-items: center;",
                                        span { style: "color: #94a3b8;", "α={hit.get(\"alpha\").and_then(|x| x.as_f64()).unwrap_or(0.0):.2}" }
                                        span { style: "padding: 0.15rem 0.4rem; background: rgba(255, 255, 255, 0.1); border-radius: 4px; color: #cbd5e0;", "{str_field(&hit, \"licence\")}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Worlds Panel
            div {
                style: "z-index: 1; padding: 1.5rem; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; backdrop-filter: blur(10px); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05); flex: 1;",
                strong { style: "font-size: 1rem; color: var(--qualia-text); font-weight: 650; display: flex; align-items: center; gap: 0.5rem; margin-bottom: 1rem;",
                    if advanced { "World configurations" } else { "Places" }
                    span { style: "padding: 0.15rem 0.5rem; background: rgba(255,255,255,0.1); border-radius: 20px; font-size: 0.75rem;", "{worlds().len()}" }
                }
                if worlds().is_empty() {
                    div {
                        style: "padding: 3rem 1rem; text-align: center; border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: 8px;",
                        p { style: "font-size: 0.9rem; color: #94a3b8; margin: 0;",
                            if advanced { "No world configurations. Seed the bounded demo set." } else { "No places yet. Add the example places to begin." }
                        }
                    }
                } else {
                    ul {
                        style: "list-style: none; padding: 0; margin: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 1rem;",
                        for w in worlds().iter().cloned() {
                            li {
                                style: "padding: 1rem; background: rgba(0, 0, 0, 0.2); border: 1px solid rgba(255, 255, 255, 0.05); border-radius: 8px; transition: all 0.2s;",
                                div {
                                    style: "display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 0.75rem;",
                                    div {
                                        div { style: "font-size: 0.95rem; font-weight: 500; color: #fff; margin-bottom: 0.25rem;", "{str_field(&w, \"title\")}" }
                                        if advanced {
                                            div { style: "font-size: 0.7rem; font-family: monospace; color: #94a3b8;", "{str_field(&w, \"id\")}" }
                                        }
                                    }
                                    button {
                                        style: "padding: 0.3rem 0.75rem; border-radius: 6px; border: 1px solid rgba(16, 185, 129, 0.4); background: rgba(16, 185, 129, 0.1); color: #34d399; font-size: 0.75rem; font-weight: 500; cursor: pointer; transition: all 0.2s;",
                                        onclick: {
                                            let id = str_field(&w, "id");
                                            move |_| {
                                                let id = id.clone();
                                                spawn(async move {
                                                    match set_active_canvas_world(&id).await {
                                                        Ok(()) => status.set(format!("Active: {id}")),
                                                        Err(e) => status.set(e),
                                                    }
                                                    if let Ok(n) = canvas_navigation().await {
                                                        nav.set(n);
                                                    }
                                                });
                                            }
                                        },
                                        if advanced { "Set active world" } else { "Enter place" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div {
                style: "z-index: 1; padding: 1.5rem; background: rgba(255, 255, 255, 0.035); border: 1px solid rgba(255, 255, 255, 0.09); border-radius: 14px;",
                div {
                    style: "display:flex;justify-content:space-between;align-items:flex-end;gap:1rem;margin-bottom:1rem;",
                    div {
                        h3 { style: "margin:0;color:var(--qualia-text);font-size:1.05rem;",
                            if advanced { "Earth, stars & planetary layers" } else { "Public layers" }
                        }
                        p { style: "margin:0.35rem 0 0;color:#94a3b8;font-size:0.82rem;line-height:1.45;",
                            if advanced { "Download, compile and upload an attributed public-data layer to the native GPU surface." } else { "Add an attributed Earth, star or planetary layer to this view. The source and licence stay attached." }
                        }
                    }
                    span { style: "color:#64748b;font-size:0.75rem;", "{layers().len()} available" }
                }
                if layers().is_empty() {
                    div {
                        style: "padding:1.5rem;border:1px dashed rgba(148,163,184,0.2);border-radius:10px;color:#94a3b8;text-align:center;",
                        "Layer catalog is unavailable until the desktop host is ready."
                    }
                } else {
                    div {
                        style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(250px,1fr));gap:0.85rem;",
                        for layer in layers().iter().cloned() {
                            div {
                                style: "display:flex;flex-direction:column;min-height:185px;padding:1rem;border:1px solid rgba(148,163,184,0.13);border-radius:11px;background:rgba(2,6,23,0.34);",
                                div {
                                    style: "display:flex;justify-content:space-between;align-items:flex-start;gap:0.6rem;",
                                    span {
                                        style: "padding:0.2rem 0.45rem;border-radius:999px;background:rgba(56,189,248,0.10);border:1px solid rgba(56,189,248,0.20);color:#7dd3fc;font-size:0.64rem;font-weight:750;text-transform:uppercase;letter-spacing:0.04em;",
                                        "{layer_source(&layer)}"
                                    }
                                    span { style: "color:#64748b;font-size:0.68rem;", "{str_field(&layer, \"license\")}" }
                                }
                                strong { style: "display:block;margin-top:0.75rem;color:#f8fafc;font-size:0.9rem;line-height:1.35;", "{str_field(&layer, \"name\")}" }
                                p { style: "margin:0.45rem 0 0;color:#94a3b8;font-size:0.76rem;line-height:1.45;flex:1;", "{str_field(&layer, \"description\")}" }
                                button {
                                    r#type: "button",
                                    disabled: downloading().is_some(),
                                    style: "margin-top:0.85rem;border:1px solid rgba(129,140,248,0.35);background:rgba(99,102,241,0.12);color:#c7d2fe;border-radius:8px;padding:0.48rem 0.7rem;cursor:pointer;font-size:0.75rem;font-weight:700;",
                                    onclick: {
                                        let id = str_field(&layer, "id");
                                        let name = str_field(&layer, "name");
                                        move |_| {
                                            let id = id.clone();
                                            let name = name.clone();
                                            downloading.set(Some(id.clone()));
                                            status.set(format!("Downloading {name}…"));
                                            spawn(async move {
                                                match download_layer(&id, 1024).await {
                                                    Ok(report) => {
                                                        let vertices = report.get("vertexCount").and_then(|v| v.as_u64()).unwrap_or(0);
                                                        status.set(format!("{name} loaded to the GPU · {vertices} vertices"));
                                                    }
                                                    Err(e) => status.set(format!("Layer download failed: {e}")),
                                                }
                                                downloading.set(None);
                                            });
                                        }
                                    },
                                    if downloading().as_deref() == layer.get("id").and_then(|value| value.as_str()) {
                                        "Loading…"
                                    } else {
                                        if advanced { "Compile and load on GPU" } else { "Add to view" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
