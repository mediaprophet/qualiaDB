//! QApps component - the catalog browser.

use super::*;
use crate::Route;
use dioxus::prelude::*;

fn qapp_capabilities(id: &str) -> Vec<&'static str> {
    let lower = id.to_ascii_lowercase();
    let mut labels = vec!["Local execution", "Semantic Library"];
    if lower.contains("medical")
        || lower.contains("clinical")
        || lower.contains("comorbid")
        || lower.contains("anatom")
    {
        labels.push("Health ontologies");
    } else if lower.contains("finance") || lower.contains("portfolio") || lower.contains("risk") {
        labels.push("Financial ontology");
    } else if lower.contains("ontology")
        || lower.contains("semantic")
        || lower.contains("sparql")
        || lower.contains("knowledge")
    {
        labels.push("Ontology / graph");
    } else if lower.contains("llm") || lower.contains("chat") || lower.contains("agent") {
        labels.push("AI model");
    } else {
        labels.push("Domain graph");
    }
    labels
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn QApps() -> Element {
    let all_apps = qapp_catalog();
    let mut selected_cat = use_signal(|| Cat::All);
    let mut search = use_signal(String::new);
    // Default: only Active + Beta (full catalog incl. Soon is opt-in).
    let mut show_soon = use_signal(|| false);
    let current_cat = selected_cat();
    let search_text = search().trim().to_ascii_lowercase();
    let cats = cat_list();

    let n_active = all_apps.iter().filter(|a| a.stat == Stat::Active).count();
    let n_beta = all_apps.iter().filter(|a| a.stat == Stat::Beta).count();
    let n_soon = all_apps.iter().filter(|a| a.stat == Stat::Soon).count();

    let cards: Vec<CardData> = all_apps
        .iter()
        .filter(|a| current_cat == Cat::All || a.cat == current_cat)
        .filter(|a| show_soon() || matches!(a.stat, Stat::Active | Stat::Beta))
        .filter(|a| {
            search_text.is_empty()
                || [a.id, a.name, a.tagline, a.desc]
                    .iter()
                    .any(|value| value.to_ascii_lowercase().contains(&search_text))
        })
        .map(|a| {
            let (status_label, status_color, opacity) = match a.stat {
                Stat::Active => ("Active", "#10b981", "1"),
                Stat::Beta => ("Beta", "#f59e0b", "1"),
                Stat::Soon => ("Soon", "#9ca3af", "0.60"),
            };
            let btn = match (a.stat, a.route) {
                (Stat::Active, Some(AppRoute::ContextStudio)) => BtnKind::LaunchContext,
                (Stat::Active, Some(AppRoute::QAppStudio)) => BtnKind::LaunchQAppStudio,
                (Stat::Active, Some(AppRoute::Nexus)) => BtnKind::LaunchNexus,
                (Stat::Soon, _) => BtnKind::ComingSoon,
                _ => BtnKind::OpenInStudio,
            };
            CardData {
                id: a.id,
                name: a.name,
                tagline: a.tagline,
                desc: a.desc,
                icon: a.icon,
                status_label,
                status_color,
                opacity,
                btn,
            }
        })
        .collect();

    let templates = featured_templates();

    rsx! {
        div {
            style: "width: 100%; height: 100%; overflow-y: auto; padding: 2rem 2rem 4rem;",

            // ── Header ─────────────────────────────────────────────────────────
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 1.25rem; flex-wrap: wrap; gap: 0.75rem;",
                div {
                    h1 {
                        style: "margin: 0 0 0.25rem 0; font-size: 1.4rem; font-weight: 700; color: var(--qualia-text); letter-spacing: -0.025em;",
                        "QApps"
                    }
                    p {
                        style: "margin: 0 0 0.45rem 0; font-size: 0.82rem; color: var(--qualia-text-muted); max-width: 36rem; line-height: 1.45;",
                        "Governed apps on your node. Active means a real launch path; Soon is a catalogue placeholder — not a broken product."
                    }
                    p {
                        style: "margin: 0; font-size: 0.72rem; color: #94a3b8; max-width: 36rem; line-height: 1.4;",
                        "Daily work lives under life domains (Memory · Relations · Care · World · Practice · Instruments). This catalog is Advanced discovery — default list is Active + Beta only."
                    }
                }
                div { style: "display: flex; gap: 0.4rem; flex-shrink: 0; margin-top: 0.2rem; flex-wrap: wrap; align-items: center;",
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #10b981; background: rgba(16,185,129,0.1); border: 1px solid rgba(16,185,129,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_active} Active"
                    }
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #f59e0b; background: rgba(245,158,11,0.1); border: 1px solid rgba(245,158,11,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_beta} Beta"
                    }
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #9ca3af; background: rgba(156,163,175,0.1); border: 1px solid rgba(156,163,175,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_soon} Soon (catalog)"
                    }
                    button {
                        r#type: "button",
                        style: "font-size: 0.69rem; font-weight: 600; color: var(--qualia-text); background: rgba(139,92,246,0.12); border: 1px solid rgba(139,92,246,0.35); border-radius: 12px; padding: 0.2rem 0.55rem; cursor: pointer;",
                        title: "Show academic / Soon placeholders (full map opt-in)",
                        onclick: move |_| show_soon.set(!show_soon()),
                        if show_soon() { "Hide Soon rows" } else { "Show full catalog (Soon)" }
                    }
                }
            }

            // ── Featured Templates ──────────────────────────────────────────────
            div {
                style: "display:grid;grid-template-columns:minmax(240px,1fr) repeat(3,minmax(150px,.45fr));gap:.6rem;margin-bottom:1.35rem;",
                div {
                    style: "position:relative;",
                    sl-icon { "name": "search", style: "position:absolute;left:.8rem;top:.72rem;color:var(--qualia-text-muted);" }
                    input {
                        r#type: "search",
                        aria_label: "Search QApps",
                        placeholder: "Search by task, domain, or capability…",
                        value: "{search}",
                        oninput: move |event| search.set(event.value()),
                        style: "width:100%;box-sizing:border-box;padding:.65rem .8rem .65rem 2.35rem;border:1px solid var(--qualia-border);border-radius:11px;background:var(--qualia-surface);color:var(--qualia-text);font:inherit;font-size:.78rem;",
                    }
                }
                for (number, title, detail) in [
                    ("1", "Choose", "Find the job you want done"),
                    ("2", "Check", "Review data and ontology needs"),
                    ("3", "Run", "Open its working canvas"),
                ] {
                    div { style: "display:flex;gap:.55rem;align-items:center;border:1px solid var(--qualia-border);border-radius:11px;background:var(--qualia-surface);padding:.55rem .65rem;",
                        span { style: "width:1.45rem;height:1.45rem;border-radius:7px;background:var(--qualia-accent-glow);color:var(--qualia-accent);display:grid;place-items:center;font-size:.65rem;font-weight:800;flex:0 0 auto;", "{number}" }
                        div {
                            div { style: "font-size:.7rem;font-weight:750;color:var(--qualia-text);", "{title}" }
                            div { style: "font-size:.61rem;color:var(--qualia-text-muted);margin-top:1px;", "{detail}" }
                        }
                    }
                }
            }

            div { style: "margin-bottom: 1.75rem;",
                div {
                    style: "font-size: 0.7rem; font-weight: 700; color: var(--qualia-text-muted); letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 0.75rem;",
                    "Featured Templates"
                }
                div {
                    style: "display: flex; gap: 0.875rem; overflow-x: auto; padding-bottom: 0.625rem; scrollbar-width: thin;",
                    for tmpl in templates.iter() {
                        div {
                            key: "{tmpl.name}",
                            class: "panel-card",
                            style: "flex-shrink: 0; width: 210px; background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 14px; padding: 1rem; backdrop-filter: blur(20px); box-shadow: 0 4px 20px rgba(0,0,0,0.06);",

                            div { style: "display: flex; gap: 0.35rem; margin-bottom: 0.7rem; flex-wrap: wrap;",
                                for icon in tmpl.icons.iter() {
                                    div {
                                        key: "{icon}",
                                        style: "width: 26px; height: 26px; border-radius: 7px; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center;",
                                        sl-icon { "name": "{icon}", style: "font-size: 0.75rem; color: var(--qualia-accent);" }
                                    }
                                }
                            }
                            div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.3rem; line-height: 1.3;", "{tmpl.name}" }
                            p { style: "margin: 0 0 0.7rem; font-size: 0.7rem; color: var(--qualia-text-muted); line-height: 1.45;", "{tmpl.desc}" }
                            Link {
                                to: Route::StudioRoute {},
                                style: "display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; font-weight: 600; color: var(--qualia-accent); text-decoration: none;",
                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                "Open in Studio"
                            }
                        }
                    }
                }
            }

            // ── Category filter ─────────────────────────────────────────────────
            div {
                style: "display: flex; gap: 0.375rem; margin-bottom: 1.25rem; flex-wrap: wrap;",
                for cat in cats.iter() {
                    {
                        let c = *cat;
                        let is_active = current_cat == c;
                        let bg = if is_active { "var(--qualia-accent)" } else { "rgba(128,128,128,0.08)" };
                        let col = if is_active { "white" } else { "var(--qualia-text-muted)" };
                        let border = if is_active { "var(--qualia-accent)" } else { "var(--qualia-border)" };
                        rsx! {
                            button {
                                key: "{c.label()}",
                                onclick: move |_| selected_cat.set(c),
                                style: "background: {bg}; color: {col}; border: 1px solid {border}; border-radius: 20px; padding: 0.28rem 0.7rem; font-size: 0.74rem; font-weight: 500; font-family: 'Inter', sans-serif; cursor: pointer; transition: all 0.15s; white-space: nowrap;",
                                "{c.label()}"
                            }
                        }
                    }
                }
            }

            // ── App grid ────────────────────────────────────────────────────────
            div {
                style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(288px, 1fr)); gap: 1rem;",
                for card in cards.iter() {
                    div {
                        key: "{card.id}",
                        class: "panel-card",
                        style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1.2rem; backdrop-filter: blur(24px); box-shadow: 0 6px 28px rgba(0,0,0,0.07); display: flex; flex-direction: column; gap: 0.65rem; opacity: {card.opacity};",

                        // Icon + status badge
                        div { style: "display: flex; align-items: flex-start; justify-content: space-between;",
                            div {
                                style: "width: 40px; height: 40px; border-radius: 12px; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                sl-icon { "name": "{card.icon}", style: "font-size: 1.15rem; color: var(--qualia-accent);" }
                            }
                            span {
                                style: "font-size: 0.66rem; font-weight: 600; color: {card.status_color}; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 20px; padding: 0.18rem 0.5rem; letter-spacing: 0.04em; flex-shrink: 0; margin-top: 2px;",
                                "{card.status_label}"
                            }
                        }

                        // Name + tagline
                        div {
                            div { style: "font-size: 0.875rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.1rem;", "{card.name}" }
                            div { style: "font-size: 0.69rem; color: var(--qualia-accent); font-weight: 500; letter-spacing: 0.01em;", "{card.tagline}" }
                        }

                        // Description
                        p {
                            style: "margin: 0; font-size: 0.745rem; color: var(--qualia-text-muted); line-height: 1.52; flex: 1;",
                            "{card.desc}"
                        }

                        div { style: "display:flex;gap:.3rem;flex-wrap:wrap;",
                            for capability in qapp_capabilities(card.id) {
                                span { style: "font-size:.61rem;padding:.18rem .42rem;border-radius:999px;background:var(--qualia-accent-glow);color:var(--qualia-accent);", "{capability}" }
                            }
                        }

                        // Action buttons
                        {
                            let app_id_str = card.id.to_string();
                            let app_id_str2 = card.id.to_string();
                            rsx! {
                                div { style: "display: flex; gap: 0.45rem; margin-top: auto; padding-top: 0.2rem; flex-wrap: wrap;",
                                    match card.btn {
                                        BtnKind::LaunchContext => rsx! {
                                            Link {
                                                to: Route::ContextStudioRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.1); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "pencil", style: "font-size: 0.68rem;" }
                                                "Edit"
                                            }
                                        },
                                        BtnKind::LaunchQAppStudio => rsx! {
                                            Link {
                                                to: Route::StudioRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                        },
                                        BtnKind::LaunchNexus => rsx! {
                                            Link {
                                                to: Route::NexusRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.1); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "pencil", style: "font-size: 0.68rem;" }
                                                "Edit"
                                            }
                                        },
                                        BtnKind::OpenInStudio => rsx! {
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str2 },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "layers", style: "font-size: 0.68rem;" }
                                                "Run QApp"
                                            }
                                        },
                                        BtnKind::ComingSoon => rsx! {
                                            button {
                                                disabled: true,
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.08); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; font-family: 'Inter', sans-serif; cursor: not-allowed;",
                                                sl-icon { "name": "clock", style: "font-size: 0.68rem;" }
                                                "Coming Soon"
                                            }
                                        },
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
