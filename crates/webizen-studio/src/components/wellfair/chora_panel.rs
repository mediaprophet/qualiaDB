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
    let downloading = use_signal(|| false);
    let camera_mode = use_signal(|| "earth".to_string());

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
            style: "display:flex;flex-direction:column;gap:1rem;padding:0.5rem;",

            header {
                h2 { style: "margin:0;font-size:1.1rem;", "WellFair · Chora — Spatio-Temporal Explorer" }
                p {
                    style: "margin:0.25rem 0 0;font-size:0.85rem;color:#666;",
                    "Chora is the spatio-temporal canvas inside WellFair (Peace Infrastructure). "
                    "A permissive-commons omniverse substrate — worlds are configurations; pan, scrub, and explore."
                }
            }

            if !status().is_empty() {
                div {
                    role: "status",
                    style: "padding:0.5rem;background:#e9f5ff;border-radius:6px;font-size:0.85rem;",
                    "{status}"
                }
            }

            div {
                style: "display:flex;gap:0.5rem;flex-wrap:wrap;",
                button {
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
                    "Seed demo world"
                }
                button {
                    onclick: move |_| refresh(),
                    "Refresh"
                }
            }

            div {
                style: "padding:0.75rem;background:#f8f9fa;border-radius:8px;",
                strong { "Navigation" }
                div { style: "font-size:0.85rem;margin-top:0.25rem;",
                    "Active: {str_field(&nav(), \"activeWorldId\")}"
                }
                label {
                    style: "display:flex;align-items:center;gap:0.5rem;margin-top:0.5rem;font-size:0.85rem;",
                    "Time scrub (unix): "
                    input {
                        r#type: "range",
                        min: "1700000000",
                        max: "1900000000",
                        step: "86400",
                        value: "{temporal_t()}",
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
                    span { "{temporal_t():.0}" }
                }
            }

            div {
                style: "padding:0.75rem;background:#f8f9fa;border-radius:8px;",
                strong { "Worlds ({worlds().len()})" }
                if worlds().is_empty() {
                    p { style: "font-size:0.85rem;color:#888;", "No worlds yet — seed the demo." }
                } else {
                    ul {
                        style: "list-style:none;padding:0;margin:0.5rem 0 0;",
                        for w in worlds().iter().cloned() {
                            li {
                                style: "padding:0.4rem 0;border-bottom:1px solid #eee;font-size:0.85rem;",
                                button {
                                    style: "margin-right:0.5rem;",
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
                                    "Select"
                                }
                                "{str_field(&w, \"title\")} ({str_field(&w, \"id\")})"
                            }
                        }
                    }
                }
            }

            div {
                style: "padding:0.75rem;background:#f8f9fa;border-radius:8px;",
                strong { "Region query (Sydney bbox)" }
                button {
                    style: "margin-left:0.5rem;",
                    onclick: move |_| {
                        spawn(async move {
                            // Sydney-ish bbox
                            match query_canvas_region(151.0, -34.0, 152.0, -33.0).await {
                                Ok(hits) => {
                                    status.set(format!("{} assets in region", hits.len()));
                                    region_hits.set(hits);
                                }
                                Err(e) => status.set(e),
                            }
                        });
                    },
                    "Query"
                }
                if !region_hits().is_empty() {
                    ul {
                        style: "list-style:none;padding:0;margin:0.5rem 0 0;font-size:0.8rem;",
                        for hit in region_hits().iter().cloned() {
                            li {
                                "{str_field(&hit, \"assetId\")} — α={hit.get(\"alpha\").and_then(|x| x.as_f64()).unwrap_or(0.0):.2} — {str_field(&hit, \"licence\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}