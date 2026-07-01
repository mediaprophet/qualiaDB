use crate::theme_engine::{self, ResolvedTheme, ThemeDefinition};
use crate::Route;
use dioxus::prelude::*;
use std::collections::HashMap;

// ── Helper functions ──────────────────────────────────────────────────────────

fn hex_is_dark(hex: &str) -> bool {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return true;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32;
    (0.299 * r + 0.587 * g + 0.114 * b) < 140.0
}

fn hex_to_rgba(hex: &str, alpha: f32) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return format!("rgba(0,0,0,{alpha})");
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    format!("rgba({r},{g},{b},{alpha})")
}

// ── localStorage helpers (WASM only) ─────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn persist_custom_theme(theme: &ThemeDefinition) {
    use web_sys::window;
    let Some(win) = window() else { return };
    let Ok(Some(storage)) = win.local_storage() else {
        return;
    };
    let Ok(json) = serde_json::to_string(theme) else {
        return;
    };
    let _ = storage.set_item(&format!("webizen_theme_{}", theme.id), &json);

    let existing = storage
        .get_item("webizen_custom_ids")
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut ids: Vec<String> = serde_json::from_str(&existing).unwrap_or_default();
    if !ids.contains(&theme.id) {
        ids.push(theme.id.clone());
        let _ = storage.set_item(
            "webizen_custom_ids",
            &serde_json::to_string(&ids).unwrap_or_default(),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_custom_theme(_theme: &ThemeDefinition) {}

#[cfg(target_arch = "wasm32")]
fn read_custom_themes() -> Vec<ThemeDefinition> {
    use web_sys::window;
    let Some(win) = window() else { return vec![] };
    let Ok(Some(storage)) = win.local_storage() else {
        return vec![];
    };
    let Some(ids_json) = storage.get_item("webizen_custom_ids").ok().flatten() else {
        return vec![];
    };
    let ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
    ids.iter()
        .filter_map(|id| {
            let json = storage.get_item(&format!("webizen_theme_{id}")).ok()??;
            serde_json::from_str(&json).ok()
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_custom_themes() -> Vec<ThemeDefinition> {
    vec![]
}

// ── SVG sparkline data ────────────────────────────────────────────────────────

fn health_sparkline() -> &'static str {
    "0,32 16,26 28,29 42,16 56,21 70,17 84,10 98,14 112,11"
}

fn cpu_sparkline() -> &'static str {
    "0,18 10,14 20,16 30,8 40,12 50,9 60,6"
}

fn mem_sparkline() -> &'static str {
    "0,12 10,15 20,10 30,17 40,11 50,14 60,9"
}

fn lat_sparkline() -> &'static str {
    "0,16 10,8 20,19 30,11 40,14 50,7 60,13"
}

// ── Bar chart data ────────────────────────────────────────────────────────────

fn bar_data() -> Vec<(u32, u32, u32)> {
    [30u32, 40, 25, 45, 35, 42, 38, 48, 30, 44, 36, 50]
        .iter()
        .enumerate()
        .map(|(i, &h)| (i as u32 * 24 + 2, 50 - h, h))
        .collect()
}

// ── Dashboard component ───────────────────────────────────────────────────────

#[component]
pub fn Dashboard() -> Element {
    let mut theme_state = consume_context::<Signal<ResolvedTheme>>();
    let mut custom_themes = use_signal(read_custom_themes);
    let mut show_creator = use_signal(|| false);
    let mut new_name = use_signal(|| "My Theme".to_string());
    let mut new_bg = use_signal(|| "#1e1e2e".to_string());
    let mut new_accent = use_signal(|| "#7c3aed".to_string());
    let mut engine_active = use_signal(|| true);
    let mut temperature = use_signal(|| 50u32);

    let current_id = theme_state()
        .theme_key
        .clone()
        .unwrap_or("fiduciary-dark".to_string());

    let all_themes: Vec<ThemeDefinition> = {
        let mut v = theme_engine::builtin_theme_catalog();
        v.extend(custom_themes());
        v
    };

    let theme_options: Vec<(String, String)> = all_themes
        .iter()
        .map(|t| {
            let label =
                t.id.split('-')
                    .map(|w| {
                        let mut c = w.chars();
                        c.next()
                            .map(|ch| ch.to_uppercase().collect::<String>() + c.as_str())
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
            (t.id.clone(), label)
        })
        .collect();

    let handle_theme_change = move |evt: Event<FormData>| {
        let id = evt.value();
        let mut catalog = theme_engine::builtin_theme_catalog();
        catalog.extend(custom_themes());
        let binding = theme_engine::ThemeBinding {
            theme_id: Some(id),
            ..Default::default()
        };
        theme_state.set(theme_engine::resolve_theme(Some(&binding), &catalog));
    };

    let on_save_theme = move |_| {
        let name = new_name();
        let bg = new_bg();
        let acc = new_accent();
        let dark = hex_is_dark(&bg);
        let text = if dark {
            "#f0f0f0".to_string()
        } else {
            "#1a1a1a".to_string()
        };
        let text_muted = if dark {
            "#a0a0b0".to_string()
        } else {
            "#5a5a6a".to_string()
        };
        let id = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let surface_alpha = if dark { 0.65_f32 } else { 0.75_f32 };
        let bg_gradient = if dark {
            format!(
                "radial-gradient(ellipse at 25% 20%, {} 0%, transparent 50%), linear-gradient(160deg, {bg} 0%, {bg} 100%)",
                hex_to_rgba(&acc, 0.14)
            )
        } else {
            format!(
                "radial-gradient(ellipse at 20% 15%, {} 0%, transparent 55%), linear-gradient(160deg, {bg} 0%, {bg} 100%)",
                hex_to_rgba(&acc, 0.20)
            )
        };

        let theme = ThemeDefinition {
            id: id.clone(),
            class_name: Some(format!("theme-{id}")),
            stylesheet_href: None,
            tokens: HashMap::from([
                ("bg".to_string(), bg.clone()),
                ("surface".to_string(), hex_to_rgba(&bg, surface_alpha)),
                ("border".to_string(), hex_to_rgba(&acc, 0.25)),
                ("text".to_string(), text),
                ("text-muted".to_string(), text_muted),
                ("accent".to_string(), acc.clone()),
                ("accent-glow".to_string(), hex_to_rgba(&acc, 0.18)),
                ("bg-gradient".to_string(), bg_gradient),
            ]),
        };

        persist_custom_theme(&theme);
        custom_themes.write().push(theme.clone());

        let mut catalog = theme_engine::builtin_theme_catalog();
        catalog.extend(custom_themes());
        let binding = theme_engine::ThemeBinding {
            theme_id: Some(id),
            ..Default::default()
        };
        theme_state.set(theme_engine::resolve_theme(Some(&binding), &catalog));
        show_creator.set(false);
    };

    let bars = bar_data();

    rsx! {
        div {
            style: "width: 100%; height: 100%; overflow-y: auto; padding: 2rem 2rem 2rem 2rem;",

            // ── Page header ───────────────────────────────────────────────
            div {
                style: "margin-bottom: 1.75rem;",
                h1 {
                    style: "margin: 0 0 0.2rem 0; font-size: 1.4rem; font-weight: 600; color: var(--qualia-text); letter-spacing: -0.02em;",
                    "Agent Settings & System Telemetry"
                }
                p {
                    style: "margin: 0; font-size: 0.825rem; color: var(--qualia-text-muted);",
                    "Configure your Webizen node and monitor system health in real time."
                }
            }

            // ── Two-column layout ─────────────────────────────────────────
            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; align-items: start;",

                // ────────────────────────────────────────────────────────
                // LEFT — Agent Settings
                // ────────────────────────────────────────────────────────
                div {
                    h2 {
                        style: "margin: 0 0 0.875rem 0; font-size: 0.9rem; font-weight: 600; color: var(--qualia-text); letter-spacing: 0.01em;",
                        "Agent Settings"
                    }

                    div {
                        class: "panel-card",
                        style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1.4rem; backdrop-filter: blur(24px); box-shadow: 0 8px 32px rgba(0,0,0,0.08); display: flex; flex-direction: column; gap: 1.1rem;",

                        h3 {
                            style: "margin: 0; font-size: 0.825rem; font-weight: 600; color: var(--qualia-text-muted); text-transform: uppercase; letter-spacing: 0.06em;",
                            "Global Agent Configuration"
                        }

                        // Agent Name
                        div { style: "display: flex; flex-direction: column; gap: 0.4rem;",
                            label { style: "font-size: 0.775rem; font-weight: 500; color: var(--qualia-text-muted);", "Agent Name" }
                            input {
                                r#type: "text",
                                value: "Webizen Node",
                                style: "background: rgba(128,128,128,0.07); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.5rem 0.75rem; color: var(--qualia-text); font-size: 0.85rem; outline: none; font-family: 'Inter', sans-serif; width: 100%; transition: border-color 0.2s;",
                            }
                        }

                        // Active Status toggle
                        div { style: "display: flex; align-items: center; justify-content: space-between;",
                            label { style: "font-size: 0.775rem; font-weight: 500; color: var(--qualia-text-muted);", "Active Status" }
                            div { style: "display: flex; align-items: center; gap: 0.6rem;",
                                span {
                                    style: "font-size: 0.78rem; font-weight: 700; color: var(--qualia-accent); letter-spacing: 0.04em;",
                                    if engine_active() { "ON" } else { "OFF" }
                                }
                                div {
                                    onclick: move |_| engine_active.set(!engine_active()),
                                    style: {
                                        let c = if engine_active() { "var(--qualia-accent)" } else { "rgba(128,128,128,0.25)" };
                                        format!("width: 42px; height: 23px; border-radius: 12px; background: {c}; cursor: pointer; position: relative; transition: background 0.25s; flex-shrink: 0;")
                                    },
                                    div {
                                        style: {
                                            let l = if engine_active() { "21px" } else { "2px" };
                                            format!("position: absolute; top: 2px; left: {l}; width: 19px; height: 19px; border-radius: 50%; background: white; transition: left 0.22s; box-shadow: 0 1px 4px rgba(0,0,0,0.28);")
                                        }
                                    }
                                }
                            }
                        }

                        // Response Creativity slider
                        div { style: "display: flex; flex-direction: column; gap: 0.4rem;",
                            div { style: "display: flex; justify-content: space-between; align-items: center;",
                                label { style: "font-size: 0.775rem; font-weight: 500; color: var(--qualia-text-muted);", "Response Creativity" }
                                span {
                                    style: "font-size: 0.75rem; color: var(--qualia-text-muted);",
                                    if temperature() < 34 { "Conservative" } else if temperature() < 67 { "Moderate" } else { "Creative" }
                                }
                            }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "100",
                                value: "{temperature}",
                                oninput: move |e| { if let Ok(v) = e.value().parse::<u32>() { temperature.set(v); } },
                                style: "width: 100%; accent-color: var(--qualia-accent); cursor: pointer; background: var(--qualia-border);",
                            }
                        }

                        // Colour Scheme selector
                        div { style: "display: flex; flex-direction: column; gap: 0.5rem;",
                            label { style: "font-size: 0.775rem; font-weight: 500; color: var(--qualia-text-muted);", "Colour Scheme" }
                            select {
                                onchange: handle_theme_change,
                                style: "background: rgba(128,128,128,0.07); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.5rem 0.75rem; color: var(--qualia-text); font-size: 0.85rem; outline: none; font-family: 'Inter', sans-serif; width: 100%; cursor: pointer;",
                                for (id, label) in theme_options.iter() {
                                    option {
                                        value: "{id}",
                                        selected: id == &current_id,
                                        "{label}"
                                    }
                                }
                            }
                            button {
                                onclick: move |_| show_creator.set(!show_creator()),
                                style: "background: transparent; border: 1px dashed var(--qualia-border); border-radius: 8px; padding: 0.4rem 0.75rem; color: var(--qualia-accent); font-size: 0.775rem; cursor: pointer; font-family: 'Inter', sans-serif; text-align: left; transition: border-color 0.2s, background 0.2s;",
                                if show_creator() { "— Cancel" } else { "+ Create custom scheme..." }
                            }

                            // Custom theme creator panel
                            if show_creator() {
                                div {
                                    style: "background: rgba(128,128,128,0.05); border: 1px solid var(--qualia-border); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; gap: 0.75rem;",

                                    div { style: "display: flex; flex-direction: column; gap: 0.3rem;",
                                        label { style: "font-size: 0.72rem; color: var(--qualia-text-muted);", "Theme Name" }
                                        input {
                                            r#type: "text",
                                            value: "{new_name}",
                                            oninput: move |e| new_name.set(e.value()),
                                            placeholder: "My Custom Theme",
                                            style: "background: rgba(128,128,128,0.07); border: 1px solid var(--qualia-border); border-radius: 7px; padding: 0.4rem 0.6rem; color: var(--qualia-text); font-size: 0.8rem; outline: none; font-family: 'Inter', sans-serif; width: 100%;",
                                        }
                                    }

                                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem;",
                                        div { style: "display: flex; flex-direction: column; gap: 0.3rem;",
                                            label { style: "font-size: 0.72rem; color: var(--qualia-text-muted);", "Background" }
                                            div { style: "display: flex; align-items: center; gap: 0.5rem;",
                                                input {
                                                    r#type: "color",
                                                    value: "{new_bg}",
                                                    oninput: move |e| new_bg.set(e.value()),
                                                }
                                                span { style: "font-size: 0.7rem; font-family: monospace; color: var(--qualia-text-muted);", "{new_bg}" }
                                            }
                                        }
                                        div { style: "display: flex; flex-direction: column; gap: 0.3rem;",
                                            label { style: "font-size: 0.72rem; color: var(--qualia-text-muted);", "Accent" }
                                            div { style: "display: flex; align-items: center; gap: 0.5rem;",
                                                input {
                                                    r#type: "color",
                                                    value: "{new_accent}",
                                                    oninput: move |e| new_accent.set(e.value()),
                                                }
                                                span { style: "font-size: 0.7rem; font-family: monospace; color: var(--qualia-text-muted);", "{new_accent}" }
                                            }
                                        }
                                    }

                                    // Preview swatch
                                    div {
                                        style: {
                                            let bg = new_bg();
                                            let acc = new_accent();
                                            format!("height: 28px; border-radius: 6px; background: linear-gradient(90deg, {bg} 0%, {acc} 100%); border: 1px solid var(--qualia-border);")
                                        }
                                    }

                                    button {
                                        onclick: on_save_theme,
                                        style: "background: var(--qualia-accent); color: white; border: none; border-radius: 8px; padding: 0.5rem 1rem; font-size: 0.8rem; font-weight: 600; cursor: pointer; font-family: 'Inter', sans-serif; transition: opacity 0.2s;",
                                        "Save & Apply Theme"
                                    }
                                }
                            }
                        }

                        // Language selector
                        div { style: "display: flex; flex-direction: column; gap: 0.4rem;",
                            label { style: "font-size: 0.775rem; font-weight: 500; color: var(--qualia-text-muted);", "Language" }
                            select {
                                style: "background: rgba(128,128,128,0.07); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.5rem 0.75rem; color: var(--qualia-text); font-size: 0.85rem; outline: none; font-family: 'Inter', sans-serif; width: 100%; cursor: pointer;",
                                option { value: "en-AU", "English (AU)" }
                                option { value: "en-US", "English (US)" }
                                option { value: "fr-FR", "Français" }
                                option { value: "de-DE", "Deutsch" }
                                option { value: "ja-JP", "日本語" }
                            }
                        }

                        // Capabilities
                        div { style: "display: flex; flex-direction: column; gap: 0.6rem; padding-top: 0.25rem;",
                            label { style: "font-size: 0.825rem; font-weight: 600; color: var(--qualia-text);", "Capabilities" }
                            {[
                                ("bar-chart-line", "Data Analysis (SPARQL)"),
                                ("chat-dots",      "Natural Language (LLM)"),
                                ("globe2",         "WebTorrent Search"),
                            ].iter().map(|(icon, label)| rsx! {
                                div {
                                    key: "{label}",
                                    style: "display: flex; align-items: center; justify-content: space-between; background: rgba(128,128,128,0.05); border: 1px solid var(--qualia-border); border-radius: 10px; padding: 0.55rem 0.8rem;",
                                    div { style: "display: flex; align-items: center; gap: 0.6rem;",
                                        sl-icon { "name": "{icon}", style: "font-size: 0.85rem; color: var(--qualia-text-muted);" }
                                        span { style: "font-size: 0.8rem; color: var(--qualia-text);", "{label}" }
                                    }
                                    div {
                                        style: "width: 17px; height: 17px; border-radius: 4px; background: var(--qualia-accent); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                        span { style: "font-size: 0.65rem; color: white; font-weight: 800; line-height: 1;", "✓" }
                                    }
                                }
                            })}
                        }
                    }
                }

                // ────────────────────────────────────────────────────────
                // RIGHT — System Telemetry
                // ────────────────────────────────────────────────────────
                div {
                    h2 {
                        style: "margin: 0 0 0.875rem 0; font-size: 0.9rem; font-weight: 600; color: var(--qualia-text); letter-spacing: 0.01em;",
                        "System Telemetry"
                    }
                    div { style: "display: flex; flex-direction: column; gap: 1rem;",

                        // System Health card
                        div {
                            class: "panel-card",
                            style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1.25rem 1.4rem; backdrop-filter: blur(24px); box-shadow: 0 8px 32px rgba(0,0,0,0.08);",
                            div { style: "display: flex; justify-content: space-between; align-items: flex-start;",
                                div {
                                    div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.5rem;", "System Health" }
                                    div { style: "font-size: 0.7rem; color: var(--qualia-text-muted); margin-bottom: 0.3rem;", "Overall Status" }
                                    div { style: "display: flex; align-items: center; gap: 0.4rem;",
                                        div { style: "width: 7px; height: 7px; border-radius: 50%; background: #10b981; box-shadow: 0 0 5px #10b981;" }
                                        span { style: "font-size: 1.2rem; font-weight: 700; color: var(--qualia-text);", "Healthy" }
                                    }
                                }
                                div { style: "text-align: right; display: flex; flex-direction: column; align-items: flex-end; gap: 0.3rem;",
                                    span { style: "font-size: 0.7rem; color: var(--qualia-text-muted);", "98% Uptime" }
                                    svg {
                                        width: "120",
                                        height: "44",
                                        view_box: "0 0 112 44",
                                        style: "overflow: visible;",
                                        polyline {
                                            points: "{health_sparkline()}",
                                            fill: "none",
                                            stroke: "#10b981",
                                            stroke_width: "2",
                                            stroke_linejoin: "round",
                                            stroke_linecap: "round",
                                        }
                                    }
                                }
                            }
                        }

                        // Resource Usage card
                        div {
                            class: "panel-card",
                            style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1.25rem 1.4rem; backdrop-filter: blur(24px); box-shadow: 0 8px 32px rgba(0,0,0,0.08);",
                            div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.875rem;", "Resource Usage" }
                            div { style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 0.75rem;",
                                // CPU
                                div {
                                    div { style: "font-size: 0.65rem; color: var(--qualia-text-muted); margin-bottom: 0.2rem;", "CPU" }
                                    div { style: "font-size: 1.15rem; font-weight: 700; color: var(--qualia-text); margin-bottom: 0.3rem;", "14%" }
                                    svg { width: "64", height: "26", view_box: "0 0 60 24",
                                        polyline { points: "{cpu_sparkline()}", fill: "none", stroke: "#60a5fa", stroke_width: "1.5", stroke_linejoin: "round", stroke_linecap: "round" }
                                    }
                                }
                                // Memory
                                div {
                                    div { style: "font-size: 0.65rem; color: var(--qualia-text-muted); margin-bottom: 0.2rem;", "Memory" }
                                    div { style: "font-size: 1.15rem; font-weight: 700; color: var(--qualia-text); margin-bottom: 0.3rem;", "128 MB" }
                                    svg { width: "64", height: "26", view_box: "0 0 60 24",
                                        polyline { points: "{mem_sparkline()}", fill: "none", stroke: "#fb923c", stroke_width: "1.5", stroke_linejoin: "round", stroke_linecap: "round" }
                                    }
                                }
                                // Graph latency
                                div {
                                    div { style: "font-size: 0.65rem; color: var(--qualia-text-muted); margin-bottom: 0.2rem;", "Graph Latency" }
                                    div { style: "font-size: 1.15rem; font-weight: 700; color: var(--qualia-text); margin-bottom: 0.3rem;", "8 ms" }
                                    svg { width: "64", height: "26", view_box: "0 0 60 24",
                                        polyline { points: "{lat_sparkline()}", fill: "none", stroke: "#f472b6", stroke_width: "1.5", stroke_linejoin: "round", stroke_linecap: "round" }
                                    }
                                }
                            }
                        }

                        // AI Performance card
                        div {
                            class: "panel-card",
                            style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1.25rem 1.4rem; backdrop-filter: blur(24px); box-shadow: 0 8px 32px rgba(0,0,0,0.08);",
                            div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.875rem;", "AI Performance" }
                            div { style: "display: flex; justify-content: space-between; margin-bottom: 0.875rem;",
                                div {
                                    div { style: "font-size: 0.65rem; color: var(--qualia-text-muted); margin-bottom: 0.15rem;", "Sentinel Pass Rate" }
                                    div { style: "font-size: 1.4rem; font-weight: 700; color: var(--qualia-text);", "99.1%" }
                                }
                                div { style: "text-align: right;",
                                    div { style: "font-size: 0.65rem; color: var(--qualia-text-muted); margin-bottom: 0.15rem;", "Inference Speed" }
                                    div { style: "font-size: 1.4rem; font-weight: 700; color: var(--qualia-text);", "1.2s avg" }
                                }
                            }
                            svg {
                                width: "100%",
                                height: "52",
                                view_box: "0 0 290 52",
                                preserve_aspect_ratio: "none",
                                for (x, y, h) in bars.iter() {
                                    rect {
                                        key: "{x}",
                                        x: "{x}",
                                        y: "{y}",
                                        width: "20",
                                        height: "{h}",
                                        rx: "4",
                                        fill: "var(--qualia-accent)",
                                        opacity: "0.72",
                                    }
                                }
                            }
                        }

                        // QApps panel
                        div {
                            class: "panel-card",
                            style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1.25rem 1.4rem; backdrop-filter: blur(24px); box-shadow: 0 8px 32px rgba(0,0,0,0.08);",
                            div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.875rem;",
                                div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text);", "QApps" }
                                Link {
                                    to: Route::QAppsRoute {},
                                    style: "font-size: 0.72rem; color: var(--qualia-accent); text-decoration: none; font-weight: 500;",
                                    "View all →"
                                }
                            }
                            div { style: "display: flex; flex-direction: column; gap: 0.5rem;",
                                Link {
                                    to: Route::ContextStudioRoute {},
                                    style: "display: flex; align-items: center; gap: 0.75rem; background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 10px; padding: 0.65rem 0.875rem; text-decoration: none; transition: all 0.2s;",
                                    div { style: "width: 28px; height: 28px; border-radius: 8px; background: var(--qualia-surface); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                        sl-icon { "name": "diagram-3", style: "font-size: 0.85rem; color: var(--qualia-accent);" }
                                    }
                                    div {
                                        div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-accent);", "Context Studio" }
                                        div { style: "font-size: 0.7rem; color: var(--qualia-text-muted);", "Context Studio · Active" }
                                    }
                                }
                                Link {
                                    to: Route::StudioRoute {},
                                    style: "display: flex; align-items: center; gap: 0.75rem; background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 10px; padding: 0.65rem 0.875rem; text-decoration: none; transition: all 0.2s;",
                                    div { style: "width: 28px; height: 28px; border-radius: 8px; background: var(--qualia-surface); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                        sl-icon { "name": "layers", style: "font-size: 0.85rem; color: var(--qualia-accent);" }
                                    }
                                    div {
                                        div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-accent);", "QApp Studio" }
                                        div { style: "font-size: 0.7rem; color: var(--qualia-text-muted);", "Layout Builder · Active" }
                                    }
                                }
                                div {
                                    style: "display: flex; align-items: center; gap: 0.75rem; background: rgba(128,128,128,0.05); border: 1px solid var(--qualia-border); border-radius: 10px; padding: 0.65rem 0.875rem; opacity: 0.7;",
                                    div { style: "width: 28px; height: 28px; border-radius: 8px; background: var(--qualia-surface); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                        sl-icon { "name": "chat-dots", style: "font-size: 0.85rem; color: var(--qualia-text-muted);" }
                                    }
                                    div {
                                        div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text-muted);", "Neuro-Symbolic Chat" }
                                        div { style: "font-size: 0.7rem; color: var(--qualia-text-muted);", "Beta · Install via QApps" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            crate::components::diffusion_visualizer::DiffusionVisualizer {}
        }
    }
}
