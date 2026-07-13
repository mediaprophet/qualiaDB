//! **Chora** — the spatio-temporal commons canvas explorer.
//!
//! Pan (space), scrub (time), zoom (scale) over permissive-commons layers.
//! Worlds are configurations, not engine forks (doc 02 §2).

use super::chora_host_client::{
    canvas_navigation, list_canvas_worlds, query_canvas_region,
    seed_canvas_demo, set_active_canvas_world, set_canvas_temporal,
    list_layers
};
use dioxus::prelude::*;

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

#[component]
pub fn WellfairChoraPanel() -> Element {
    let mut worlds = use_signal(Vec::<serde_json::Value>::new);
    let mut nav = use_signal(|| serde_json::json!({}));
    let mut region_hits = use_signal(Vec::<serde_json::Value>::new);
    let mut status = use_signal(String::new);
    let mut temporal_t = use_signal(|| 1_750_000_000f64);
    let mut layers = use_signal(Vec::<serde_json::Value>::new);
    let _downloading = use_signal(|| false);
    let _camera_mode = use_signal(|| "earth".to_string());

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
        if loaded() { return; }
        loaded.set(true);
        refresh();
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
                background: linear-gradient(135deg, rgba(15, 23, 42, 0.95), rgba(30, 41, 59, 0.95));
                color: #e2e8f0;
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
                style: "z-index: 1; border-bottom: 1px solid rgba(255, 255, 255, 0.1); padding-bottom: 1.5rem; display: flex; justify-content: space-between; align-items: flex-start;",
                div {
                    h2 { 
                        style: "margin: 0; font-size: 1.8rem; font-weight: 600; color: #fff; letter-spacing: -0.02em; display: flex; align-items: center; gap: 0.5rem;", 
                        "Chora Explorer ", span { style: "font-weight: 300; opacity: 0.6;", "· Spatio-Temporal Commons" }
                    }
                    p {
                        style: "margin: 0.5rem 0 0; font-size: 0.9rem; color: #94a3b8; max-width: 600px; line-height: 1.5;",
                        "A permissive-commons omniverse substrate. Worlds are configurations, not engine forks. Pan through space, scrub through time, and explore active regions."
                    }
                }
                div {
                    style: "display: flex; gap: 0.75rem;",
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
                        span { style: "font-size: 1.1rem;", "🌱" } "Seed Demo"
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
                style: "z-index: 1; display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem;",
                
                // Navigation Panel
                div {
                    style: "padding: 1.5rem; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; backdrop-filter: blur(10px); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",
                        strong { style: "font-size: 1rem; color: #fff; font-weight: 500;", "Temporal Scrubber" }
                        div {
                            style: "padding: 0.25rem 0.75rem; background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.2); border-radius: 20px; font-size: 0.75rem; color: #34d399; font-family: monospace;",
                            "ACTIVE: {str_field(&nav(), \"activeWorldId\")}"
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
                            span { style: "font-size: 0.75rem; color: #64748b;", "UNIX EPOCH" }
                            span { style: "font-size: 1.25rem; font-family: monospace; color: #818cf8; font-weight: 600;", "{temporal_t():.0}" }
                        }
                    }
                }

                // Region Query Panel
                div {
                    style: "padding: 1.5rem; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 12px; backdrop-filter: blur(10px); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);",
                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",
                        strong { style: "font-size: 1rem; color: #fff; font-weight: 500;", "Region Telemetry (Sydney)" }
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
                            "Execute Scan"
                        }
                    }
                    if region_hits().is_empty() {
                        div {
                            style: "height: 60px; display: flex; align-items: center; justify-content: center; border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: 8px; color: #64748b; font-size: 0.85rem;",
                            "No targets acquired in quadrant."
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
                strong { style: "font-size: 1rem; color: #fff; font-weight: 500; display: flex; align-items: center; gap: 0.5rem; margin-bottom: 1rem;", "Multiverse Layers ", span { style: "padding: 0.15rem 0.5rem; background: rgba(255,255,255,0.1); border-radius: 20px; font-size: 0.75rem;", "{worlds().len()}" } }
                if worlds().is_empty() {
                    div {
                        style: "padding: 3rem 1rem; text-align: center; border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: 8px;",
                        p { style: "font-size: 0.9rem; color: #64748b; margin: 0;", "Void empty. Seed a demo world to construct layers." }
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
                                        div { style: "font-size: 0.7rem; font-family: monospace; color: #64748b;", "{str_field(&w, \"id\")}" }
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
                                        "Jump to World"
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